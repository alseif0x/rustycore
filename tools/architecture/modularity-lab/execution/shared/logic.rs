//! Identical, deliberately synthetic Rust logic compiled natively and to core Wasm.
//! C++ placement anchors: Player.cpp:2189-2205 (mutable XP hook before award),
//! TemporarySummon.cpp:249-265 (synchronous JustSummoned/IsSummonedBy callbacks).
//! This encounter is NOT a port of a boss or a full WoW gameplay parity proof.

pub const ABI_VERSION: u32 = 1;
pub const HANDLE: u64 = (7_u64 << 32) | 42;
pub const XP: u32 = 0;
pub const START: u32 = 1;
pub const CALLBACK: u32 = 2;
pub const RESET: u32 = 3;
pub const REWARD: u32 = 4;
pub const TRAP_AFTER_REWARD: u32 = 5;
pub const SHIELD: u32 = 10;
pub const SUMMON: u32 = 11;
pub const READ_SUMMONS: u32 = 12;
pub const OBSERVE: u32 = 13;
pub const GRANT: u32 = 14;
pub const CLEAR: u32 = 15;
pub const RECURSE_PROBE: u32 = 16;
pub const BURN_PROBE: u32 = 17;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State(pub u64);

impl State {
    pub fn phase(self) -> u64 {
        self.0 & 255
    }
    pub fn shield(self) -> bool {
        self.0 & 256 != 0
    }
    pub fn callbacks(self) -> u64 {
        (self.0 >> 16) & 65535
    }
    pub fn with_callback(self) -> Self {
        let next = (self.callbacks() + 1) & 65535;
        Self((self.0 & !(65535 << 16)) | (next << 16))
    }
}

pub trait Host {
    fn state(&self) -> State;
    fn save(&mut self, state: State);
    fn percent(&self) -> u32;
    fn action(&mut self, op: u32, handle: u64, argument: i64) -> Result<i64, ()>;
}

pub fn run(host: &mut impl Host, event: u32, argument: i64) -> Result<i64, ()> {
    match event {
        XP => {
            if !(0..=1_000_000).contains(&argument) {
                return Ok(-2);
            }
            Ok(argument * i64::from(host.percent()) / 100)
        }
        START => {
            // Publish phase/shield BEFORE the fallible action and its synchronous callbacks.
            host.save(State(1 | 256));
            host.action(SHIELD, HANDLE, 1)?;
            let summoned = host.action(SUMMON, HANDLE, argument)?;
            let count = host.action(READ_SUMMONS, HANDLE, 0)?;
            // Re-read, do not overwrite state mutated by the reentrant callback.
            let state = host.state();
            host.action(OBSERVE, HANDLE, state.0 as i64)?;
            Ok(if summoned < 0 { -3 } else { count })
        }
        CALLBACK => {
            let before = host.state();
            host.action(OBSERVE, HANDLE, before.0 as i64)?;
            host.save(before.with_callback());
            Ok((before.phase() * 100 + u64::from(before.shield()) * 10) as i64)
        }
        RESET => {
            host.save(State::default());
            host.action(SHIELD, HANDLE, 0)?;
            host.action(CLEAR, HANDLE, 0)
        }
        REWARD | TRAP_AFTER_REWARD => {
            let result = host.action(GRANT, HANDLE, argument)?;
            if result >= 0 && event != TRAP_AFTER_REWARD {
                let state = host.state();
                host.save(State(state.0 | 512));
            }
            Ok(result)
        }
        _ => Ok(-2),
    }
}
