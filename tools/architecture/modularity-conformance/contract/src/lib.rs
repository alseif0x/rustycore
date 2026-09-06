//! Private experimental contract, not RustyCore's production SDK.
//! Native modules depend only on this crate; no backend, database or packet types escape.

#[cfg(target_arch = "wasm32")]
pub mod guest;

pub const ABI_VERSION: u32 = 1;
pub const MAX_STATE_BYTES: usize = 256;
pub const MAX_MODULES: usize = 8;
pub const MAX_CALLS: usize = 256;
pub const MAX_DEPTH: usize = 8;
pub const MAX_TRACE: usize = 4096;
pub const FUEL: u64 = 1_000_000;
pub const MEMORY_BYTES: usize = 3 * 1024 * 1024;

pub mod event {
    pub const UPDATE: u32 = 1;
    pub const CALLBACK: u32 = 2;
    pub const RESET: u32 = 3;
    pub const REMOVING: u32 = 4;
    pub const ATTACHED: u32 = 5;
    pub const DETACHED: u32 = 6;
    pub const POLICY: u32 = 7;
    /// Module-defined events need no new central enum variant.
    pub const CUSTOM: u32 = 1024;
}

pub mod capability {
    pub const QUERY: u64 = 1;
    pub const SHIELD: u64 = 2;
    pub const SUMMON: u64 = 4;
    pub const CONTRIBUTION: u64 = 8;
    pub const REENTRY_PROBE: u64 = 16;
    pub const ALL: u64 = QUERY | SHIELD | SUMMON | CONTRIBUTION | REENTRY_PROBE;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Fault {
    Invalid = 1,
    Stale = 2,
    NotActive = 3,
    Missing = 4,
    Conflict = 5,
    Version = 6,
    Capability = 7,
    Limit = 8,
    ActionFailed = 9,
    Revision = 10,
    Trap = 11,
    Overflow = 12,
}

impl Fault {
    pub fn code(self) -> i64 {
        -(self as i64)
    }

    pub fn from_code(code: i64) -> Self {
        match code {
            -1 => Self::Invalid,
            -2 => Self::Stale,
            -3 => Self::NotActive,
            -4 => Self::Missing,
            -5 => Self::Conflict,
            -6 => Self::Version,
            -7 => Self::Capability,
            -8 => Self::Limit,
            -9 => Self::ActionFailed,
            -10 => Self::Revision,
            -11 => Self::Trap,
            -12 => Self::Overflow,
            _ => Self::Invalid,
        }
    }
}

pub type Result<T> = std::result::Result<T, Fault>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Handle {
    pub guid: u64,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Manifest {
    pub id: u64,
    pub name: &'static str,
    pub abi: u32,
    pub schema: u32,
    pub capabilities: u64,
    pub state_limit: usize,
    pub order: i32,
    pub exclusive: Option<&'static str>,
}

/// Owned short-lived projection. Never keep a storage borrow across a host action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub revision: u64,
    pub bytes: Vec<u8>,
}

/// Defined by each native module, stored as an independent private typed component.
/// Wire encoding is also used for mock replay and explicit executor interchange.
/// Accepted bytes are canonical: encode(decode(bytes)) must equal bytes exactly.
/// Codecs are deterministic and encode all authoritative module state. A decoder
/// must not silently normalize a different representation in only one executor.
pub trait State: Default + Send + Sync + 'static {
    const SCHEMA: u32;
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

/// Shared native/Rust-Wasm codec admission. Independent producers enforce the
/// same canonical encoding and bounds; no module-specific codec belongs to core.
pub fn decode_canonical<S: State>(bytes: &[u8], limit: usize) -> Result<S> {
    let limit = limit.min(MAX_STATE_BYTES);
    if bytes.len() > limit {
        return Err(Fault::Limit);
    }
    let state = S::decode(bytes)?;
    let encoded = state.encode();
    if encoded.len() > limit {
        return Err(Fault::Limit);
    }
    if encoded != bytes {
        return Err(Fault::Invalid);
    }
    Ok(state)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Query {
    Shield = 1,
    Summons = 2,
    Contribution = 3,
    /// Detached = 0; active map index + 1. Negative ABI values are reserved for faults.
    Residence = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Action {
    /// Scoped reversible contribution: 0 clears, 1 enables this module's shield.
    Shield = 1,
    /// 0 is nullable failure, 1 succeeds and synchronously invokes CALLBACK on all modules.
    Summon = 2,
    /// Scoped additive custom policy contribution, bounded to 0..=1000.
    Contribution = 3,
    /// A named diagnostic capability, not a production gameplay primitive.
    /// Synchronously invoke CALLBACK with the argument; exercises cumulative recursion limits.
    Reenter = 4,
    /// Explicit partial-effect failure probe; it never undoes earlier actions.
    Fail = 5,
}

/// Already scoped to the invoked module and incarnation. No arbitrary namespace or entity access.
/// Native implementations are trusted source, not a sandbox; Wasm gets only these bounded imports.
pub trait Host {
    fn read(&mut self) -> Result<Snapshot>;
    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()>;
    fn query(&mut self, query: Query) -> Result<i64>;
    fn action(&mut self, action: Action, argument: i64) -> Result<i64>;
}

pub fn read_state<S: State>(host: &mut dyn Host) -> Result<(u64, S)> {
    let snapshot = host.read()?;
    Ok((snapshot.revision, S::decode(&snapshot.bytes)?))
}

pub fn write_state<S: State>(host: &mut dyn Host, revision: u64, state: &S) -> Result<()> {
    host.write(revision, &state.encode())
}

/// Stateless executable: canonical mutable state belongs to the host, not a hidden guest global.
/// Portable successful results are nonnegative. Both adapters reject Ok(negative) as Invalid;
/// negative Core Wasm return values encode Fault, not a signed successful payload.
pub trait Module: Send + Sync + 'static {
    type State: State;
    fn manifest() -> Manifest;
    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64>;
}
