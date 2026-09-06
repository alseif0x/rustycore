#![no_std]

#[path = "../../shared/logic.rs"]
mod logic;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use logic::{Host, State};

// No guest &mut/RefMut/static-mut reference survives an import. Reentrant exports access
// independent zero-sized Guest values and short scalar loads/stores of the same state.
// V2 adds reset_epoch in the upper 32 bits; migration from schema1 initializes it
// to 1, and actual resets advance it. The original lower 32-bit state is retained.
const MODULE_REVISION: u32 = if cfg!(feature = "v2") { 2 } else { 1 };
static STATE: AtomicU64 = AtomicU64::new(if cfg!(feature = "v2") { 1 << 32 } else { 0 });
static PERCENT: AtomicU32 = AtomicU32::new(100);
static PROBE_STAGE: AtomicU32 = AtomicU32::new(0);

#[link(wasm_import_module = "lab")]
unsafe extern "C" {
    fn action(op: u32, handle: u64, argument: i64) -> i64;
    fn payload(pointer: u32, length: u32) -> i64;
}

struct Guest;
impl Host for Guest {
    fn state(&self) -> State {
        State(STATE.load(Ordering::Relaxed))
    }
    fn save(&mut self, state: State) {
        let bits = if cfg!(feature = "v2") {
            (state.0 & u64::from(u32::MAX)) | (STATE.load(Ordering::Relaxed) & !u64::from(u32::MAX))
        } else {
            state.0
        };
        STATE.store(bits, Ordering::Relaxed);
    }
    fn percent(&self) -> u32 {
        PERCENT.load(Ordering::Relaxed)
    }
    fn action(&mut self, op: u32, handle: u64, argument: i64) -> Result<i64, ()> {
        // Imported host failures trap before returning; they are not nullable Ok(-1).
        Ok(unsafe { action(op, handle, argument) })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn abi_version() -> u32 {
    logic::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn module_revision() -> u32 {
    MODULE_REVISION
}

#[unsafe(no_mangle)]
pub extern "C" fn invoke(event: u32, argument: i64) -> i64 {
    if cfg!(feature = "v2") && event == logic::RESET {
        if STATE.load(Ordering::Relaxed) >> 32 == u64::from(u32::MAX) {
            return -2;
        }
        STATE.fetch_add(1 << 32, Ordering::Relaxed);
    }
    let result = match logic::run(&mut Guest, event, argument) {
        Ok(value) => value,
        Err(()) => core::arch::wasm32::unreachable(),
    };
    if event == logic::TRAP_AFTER_REWARD {
        core::arch::wasm32::unreachable();
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn snapshot() -> u64 {
    STATE.load(Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "C" fn restore(schema: u32, state: u64) -> i32 {
    let migrated = match (MODULE_REVISION, schema) {
        (1, 1) if state <= u64::from(u32::MAX) => state,
        (2, 1) if state <= u64::from(u32::MAX) => state | (1 << 32),
        (2, 2) if state >> 32 > 0 => state,
        _ => return -1,
    };
    STATE.store(migrated, Ordering::Relaxed);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn configure(revision: u32, percent: u32) -> i32 {
    if !(1..=2).contains(&revision) || percent > 1000 {
        return -1;
    }
    PERCENT.store(percent, Ordering::Relaxed);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_action(op: u32, handle: u64, argument: i64) -> i64 {
    unsafe { action(op, handle, argument) }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_payload(pointer: u32, length: u32) -> i64 {
    unsafe { payload(pointer, length) }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_grow(pages: u32) -> usize {
    core::arch::wasm32::memory_grow::<0>(pages as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_spin() {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_spam(count: u32) {
    for _ in 0..count {
        unsafe {
            action(logic::OBSERVE, logic::HANDLE, 0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_recurse(depth: u32) -> i64 {
    unsafe { action(logic::RECURSE_PROBE, logic::HANDLE, depth as i64) }
}

fn burn(iterations: u32) -> u64 {
    let mut value = 0xdead_beef_u64;
    for _ in 0..iterations {
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        value = core::hint::black_box(value);
    }
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_burn(iterations: u32) -> u64 {
    PROBE_STAGE.store(2, Ordering::Relaxed);
    burn(iterations)
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_nested_burn(iterations: u32) -> u64 {
    PROBE_STAGE.store(1, Ordering::Relaxed);
    let outer = burn(iterations);
    outer ^ unsafe { action(logic::BURN_PROBE, logic::HANDLE, i64::from(iterations)) as u64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn probe_stage() -> u32 {
    PROBE_STAGE.load(Ordering::Relaxed)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
