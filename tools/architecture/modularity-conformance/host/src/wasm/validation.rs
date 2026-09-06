//! Pure codec admission. Candidate bytes are an owned temporary projection, not
//! canonical module state. No semantic Host operation may run during this phase.

use super::{Data, execution_fault};
use conformance_contract::{Fault, MAX_STATE_BYTES, Result};
use wasmtime::{AsContextMut, Caller, StoreContextMut, TypedFunc};

pub(super) struct Phase {
    module: u64,
    input: Vec<u8>,
    reads: usize,
    fault: Option<Fault>,
}

/// Block even an ignored attempt. A guest cannot swallow the import's error and
/// have the validator report success after trying to act with the outer frame.
pub(super) fn reject_semantic(caller: &mut Caller<'_, Data>) -> Option<Fault> {
    let phase = caller.data_mut().validation.as_mut()?;
    phase.fault.get_or_insert(Fault::Capability);
    Some(Fault::Capability)
}

pub(super) fn read(mut caller: Caller<'_, Data>, index: u32) -> i32 {
    let Some(phase) = caller.data_mut().validation.as_mut() else {
        return caller
            .data_mut()
            .core
            .charge()
            .err()
            .unwrap_or(Fault::Capability)
            .code() as i32;
    };
    // No semantic call counter: native State::decode has no Host calls either.
    // The target function is selected by the host's module registration, and all
    // imports capable of dispatching another module are blocked for this phase.
    if let Some(fault) = phase.fault {
        return fault.code() as i32;
    }
    if phase.reads >= MAX_STATE_BYTES {
        phase.fault = Some(Fault::Limit);
        return Fault::Limit.code() as i32;
    }
    phase.reads += 1;
    match phase.input.get(index as usize) {
        Some(byte) => i32::from(*byte),
        None => {
            phase.fault = Some(Fault::Invalid);
            Fault::Invalid.code() as i32
        }
    }
}

pub(super) fn candidate(
    mut context: StoreContextMut<'_, Data>,
    module: u64,
    validator: TypedFunc<u32, i32>,
    bytes: &[u8],
) -> Result<()> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err(Fault::Limit);
    }
    if context.data().validation.is_some() {
        return Err(Fault::Capability);
    }
    context.data_mut().validation = Some(Phase {
        module,
        input: bytes.to_vec(),
        reads: 0,
        fault: None,
    });
    // No state/registry/guest-memory borrow and no fuel refill across this call.
    let result = validator
        .call(context.as_context_mut(), bytes.len() as u32)
        .map_err(execution_fault);
    let phase = context
        .data_mut()
        .validation
        .take()
        .expect("installed validation phase");
    assert_eq!(phase.module, module, "validator must leave its own phase");
    if let Some(fault) = phase.fault {
        return Err(fault);
    }
    match result? {
        0 => Ok(()),
        value if value < 0 => Err(Fault::from_code(i64::from(value))),
        _ => Err(Fault::Invalid),
    }
}

pub(super) fn registered(
    mut context: StoreContextMut<'_, Data>,
    module: u64,
    bytes: &[u8],
) -> Result<()> {
    let validator = context
        .data()
        .guests
        .get(&module)
        .ok_or(Fault::Missing)?
        .validate
        .clone();
    candidate(context.as_context_mut(), module, validator, bytes)
}
