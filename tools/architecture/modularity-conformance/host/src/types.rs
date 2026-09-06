use conformance_contract::{
    Action, Handle, MAX_CALLS, MAX_DEPTH, MAX_MODULES, MAX_STATE_BYTES, MAX_TRACE,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub modules: usize,
    pub calls: usize,
    pub depth: usize,
    pub state_bytes: usize,
    pub trace: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            modules: MAX_MODULES,
            calls: MAX_CALLS,
            depth: MAX_DEPTH,
            state_bytes: MAX_STATE_BYTES,
            trace: MAX_TRACE,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Executor {
    Native,
    Wasm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Residence {
    Active(u8),
    Detached,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Invocation {
    pub handle: Handle,
    pub module: u64,
    pub event: u32,
    pub argument: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallbackPlan {
    pub handle: Handle,
    pub event: u32,
    pub argument: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionOutcome {
    pub value: i64,
    pub callback: Option<CallbackPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Trace {
    Enter(Invocation),
    Leave {
        invocation: Invocation,
        result: conformance_contract::Result<i64>,
    },
    Write {
        handle: Handle,
        module: u64,
        revision: u64,
        bytes: Vec<u8>,
    },
    Action {
        handle: Handle,
        module: u64,
        action: Action,
        argument: i64,
        value: i64,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Contribution {
    pub shield: bool,
    pub amount: i64,
    pub summons: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observables {
    pub handle: Handle,
    pub residence: Residence,
    pub payload_sentinel: u64,
    pub shield: bool,
    pub summons: u64,
    pub contribution: i64,
    pub by_module: Vec<(u64, Contribution)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedModule {
    pub id: u64,
    pub abi: u32,
    pub schema: u32,
    pub executor: Executor,
    pub revision: u64,
    pub bytes: Vec<u8>,
}

/// Mock same-incarnation checkpoint, not a DB commit or crash-recovery receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntitySnapshot {
    pub format: u32,
    pub handle: Handle,
    pub core_revision: u64,
    pub modules: Vec<SavedModule>,
    pub contributions: Vec<(u64, Contribution)>,
}
