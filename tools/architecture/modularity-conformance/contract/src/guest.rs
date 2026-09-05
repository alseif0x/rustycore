//! Core Wasm bindings for the private conformance contract, without WASI.
//!
//! Imports use the `conformance` namespace. Pointers/capacities are wasm32 bytes;
//! state codecs and the revision output are little endian. `read` returns the
//! byte length or a negative Fault; `write` returns zero or a negative Fault.
//! Query/action results must be nonnegative; negative values encode Fault.
//! The host supplies invocation identity, checks all ranges and copies input
//! before an action can reenter. Neither module IDs nor entity IDs are imports.
//!
//! Initial state is exposed bytewise (at most MAX_STATE_BYTES), avoiding an
//! allocator ABI or persistent guest buffer. This is registration-only work.
//! Canonical state belongs to the host; snapshots below are temporary projections.
//! `validate_state` decodes a temporary record supplied through `validation_read`.
//! During that phase ordinary imports have no authority, including during a write.

use crate::{Action, Fault, Host, MAX_STATE_BYTES, Module, Query, Result, Snapshot, State};
use std::sync::atomic::{AtomicU32, Ordering};

// Diagnostic progress only, never module/gameplay state or recovery authority.
static PROBE_STAGE: AtomicU32 = AtomicU32::new(0);

pub fn probe_stage() -> u32 {
    PROBE_STAGE.load(Ordering::Relaxed)
}

pub fn probe_grow(pages: u32) -> i32 {
    core::arch::wasm32::memory_grow::<0>(pages as usize) as i32
}

pub fn probe_spin() -> ! {
    loop {
        std::hint::black_box(1_u64);
    }
}

pub fn probe_burn(iterations: u32) -> u64 {
    let mut value = 0x9e37_79b9_7f4a_7c15_u64;
    for index in 0..iterations {
        value = std::hint::black_box(
            value.rotate_left(7) ^ u64::from(index).wrapping_mul(0xd134_2543_de82_ef95),
        );
    }
    value
}

pub fn probe_nested(iterations: u32) -> i64 {
    PROBE_STAGE.store(1, Ordering::Relaxed);
    let first = probe_burn(iterations);
    PROBE_STAGE.store(2, Ordering::Relaxed);
    if let Err(fault) = Guest.action(Action::Reenter, 0) {
        return fault.code();
    }
    PROBE_STAGE.store(3, Ordering::Relaxed);
    let second = probe_burn(iterations);
    PROBE_STAGE.store(4, Ordering::Relaxed);
    // The diagnostic result stays outside the negative Fault namespace.
    ((first ^ second) & i64::MAX as u64) as i64
}

/// Diagnostic codec rejection using the real guest write import.
pub fn probe_write(length: u32, index: u32, value: u32) -> i64 {
    let attempt = || -> Result<()> {
        if length as usize > MAX_STATE_BYTES {
            return Err(Fault::Limit);
        }
        if index >= length || value > u32::from(u8::MAX) {
            return Err(Fault::Invalid);
        }
        let mut state = Guest.read()?;
        state.bytes.resize(length as usize, 0);
        state.bytes[index as usize] = value as u8;
        Guest.write(state.revision, &state.bytes)
    };
    match attempt() {
        Ok(()) => 0,
        Err(fault) => fault.code(),
    }
}

#[link(wasm_import_module = "conformance")]
unsafe extern "C" {
    #[link_name = "read"]
    fn host_read(pointer: u32, capacity: u32, revision_pointer: u32) -> i32;
    #[link_name = "write"]
    fn host_write(pointer: u32, length: u32, revision: u64) -> i32;
    #[link_name = "query"]
    fn host_query(query: u32) -> i64;
    #[link_name = "action"]
    fn host_action(action: u32, argument: i64) -> i64;
    #[link_name = "validation_read"]
    fn host_validation_read(index: u32) -> i32;
}

fn result(value: i64) -> Result<i64> {
    if value < 0 {
        Err(Fault::from_code(value))
    } else {
        Ok(value)
    }
}

/// Stateless adapter. No reference to memory/state survives a host call.
pub struct Guest;

impl Host for Guest {
    fn read(&mut self) -> Result<Snapshot> {
        let mut bytes = [0_u8; MAX_STATE_BYTES];
        let mut revision = [0_u8; 8];
        // SAFETY: both arrays are valid writable wasm memory for this call. The
        // host validates their full ranges; read is not a reentrant operation.
        let length = unsafe {
            host_read(
                bytes.as_mut_ptr() as u32,
                bytes.len() as u32,
                revision.as_mut_ptr() as u32,
            )
        };
        let length = result(i64::from(length))? as usize;
        if length > bytes.len() {
            return Err(Fault::Limit);
        }
        Ok(Snapshot {
            revision: u64::from_le_bytes(revision),
            bytes: bytes[..length].to_vec(),
        })
    }

    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        if bytes.len() > MAX_STATE_BYTES {
            return Err(Fault::Limit);
        }
        // SAFETY: the slice remains valid during the import. Before codec-only
        // reentry the host copies it and drops every guest-memory reference;
        // validation cannot invoke gameplay actions or modify this snapshot.
        let status = unsafe { host_write(bytes.as_ptr() as u32, bytes.len() as u32, revision) };
        match result(i64::from(status))? {
            0 => Ok(()),
            _ => Err(Fault::Invalid),
        }
    }

    fn query(&mut self, query: Query) -> Result<i64> {
        // SAFETY: this import has no pointers and is scoped by the host frame.
        result(unsafe { host_query(query as u32) })
    }

    fn action(&mut self, action: Action, argument: i64) -> Result<i64> {
        // SAFETY: no borrowed host state or guest memory crosses this reentrant
        // import. Nested guest invocation gets its own stack/snapshots.
        result(unsafe { host_action(action as u32, argument) })
    }
}

pub fn invoke<M: Module>(event: u32, argument: i64) -> i64 {
    match M::invoke(&mut Guest, event, argument) {
        Ok(value) if value >= 0 => value,
        Ok(_) => Fault::Invalid.code(),
        Err(fault) => fault.code(),
    }
}

pub fn initial_state_len<M: Module>() -> i32 {
    let length = M::State::default().encode().len();
    if length > MAX_STATE_BYTES || length > M::manifest().state_limit {
        Fault::Limit.code() as i32
    } else {
        length as i32
    }
}

pub fn initial_state_byte<M: Module>(index: u32) -> i32 {
    if initial_state_len::<M>() < 0 {
        return Fault::Limit.code() as i32;
    }
    M::State::default()
        .encode()
        .get(index as usize)
        .map_or(Fault::Invalid.code() as i32, |byte| i32::from(*byte))
}

/// Pure codec admission; this record is neither a state owner nor a mutable view.
pub fn validate_state<M: Module>(length: u32) -> i32 {
    let validate = || -> Result<()> {
        let length = length as usize;
        if length > MAX_STATE_BYTES || length > M::manifest().state_limit {
            return Err(Fault::Limit);
        }
        let mut bytes = [0_u8; MAX_STATE_BYTES];
        for (index, byte) in bytes[..length].iter_mut().enumerate() {
            // SAFETY: this pointer-free import only reads the bounded temporary
            // validation projection; it has no gameplay or state-write authority.
            let value = result(i64::from(unsafe { host_validation_read(index as u32) }))?;
            *byte = u8::try_from(value).map_err(|_| Fault::Invalid)?;
        }
        crate::decode_canonical::<M::State>(&bytes[..length], M::manifest().state_limit).map(|_| ())
    };
    match validate() {
        Ok(()) => 0,
        Err(fault) => fault.code() as i32,
    }
}

/// Export the same native module implementation as a Core Wasm executable.
/// Build with panic=abort. The adapter validates metadata before activating it.
#[macro_export]
macro_rules! export_module {
    ($module:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn abi_version() -> u32 {
            <$module as $crate::Module>::manifest().abi
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn module_id() -> u64 {
            <$module as $crate::Module>::manifest().id
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn state_schema() -> u32 {
            <$module as $crate::Module>::manifest().schema
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn capabilities() -> u64 {
            <$module as $crate::Module>::manifest().capabilities
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn state_limit() -> u32 {
            <$module as $crate::Module>::manifest().state_limit as u32
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn module_order() -> i32 {
            <$module as $crate::Module>::manifest().order
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn initial_state_len() -> i32 {
            $crate::guest::initial_state_len::<$module>()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn initial_state_byte(index: u32) -> i32 {
            $crate::guest::initial_state_byte::<$module>(index)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn validate_state(length: u32) -> i32 {
            $crate::guest::validate_state::<$module>(length)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn invoke(event: u32, argument: i64) -> i64 {
            $crate::guest::invoke::<$module>(event, argument)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_grow(pages: u32) -> i32 {
            $crate::guest::probe_grow(pages)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_spin() {
            $crate::guest::probe_spin()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_burn(iterations: u32) -> u64 {
            $crate::guest::probe_burn(iterations)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_nested(iterations: u32) -> i64 {
            $crate::guest::probe_nested(iterations)
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_stage() -> u32 {
            $crate::guest::probe_stage()
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn probe_write(length: u32, index: u32, value: u32) -> i64 {
            $crate::guest::probe_write(length, index, value)
        }
    };
}
