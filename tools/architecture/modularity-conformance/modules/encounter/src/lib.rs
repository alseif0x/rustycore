//! Bounded encounter fixture, not a complete Anomalus port.
//!
//! C++ boss_anomalus.cpp:154-168 publishes phase/shield before nullable summon;
//! TemporarySummon.cpp:249-264 invokes lifecycle callbacks before return.
//! Custom events below are explicit failure/CAS probes, not C++ gameplay rules.

use conformance_contract::{
    ABI_VERSION, Action, Fault, Host, Manifest, Module, Query, Result, State, capability, event,
    read_state, write_state,
};

pub const MODULE_ID: u64 = 1;
pub const STALE_OUTER_WRITE: u32 = event::CUSTOM;
pub const FAIL_AFTER_SHIELD: u32 = event::CUSTOM + 1;

/// Non-Clone module-owned component; snapshots use the explicit stable codec.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct EncounterState {
    pub phase: u32,
    pub callbacks: u64,
}

impl State for EncounterState {
    const SCHEMA: u32 = 1;

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self.phase.to_le_bytes());
        bytes.extend_from_slice(&self.callbacks.to_le_bytes());
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 12 {
            return Err(Fault::Invalid);
        }
        let phase = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| Fault::Invalid)?);
        if phase > 1 {
            return Err(Fault::Invalid);
        }
        Ok(Self {
            phase,
            callbacks: u64::from_le_bytes(bytes[4..].try_into().map_err(|_| Fault::Invalid)?),
        })
    }
}

pub struct Encounter;

impl Module for Encounter {
    type State = EncounterState;

    fn manifest() -> Manifest {
        Manifest {
            id: MODULE_ID,
            name: "encounter",
            abi: ABI_VERSION,
            schema: EncounterState::SCHEMA,
            capabilities: capability::QUERY
                | capability::SHIELD
                | capability::SUMMON
                | capability::REENTRY_PROBE,
            state_limit: 12,
            order: 10,
            exclusive: None,
        }
    }

    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        match event {
            event::UPDATE => {
                if !(0..=1).contains(&argument) {
                    return Err(Fault::Invalid);
                }
                let (revision, mut state) = read_state::<EncounterState>(host)?;
                if state.phase == 0 {
                    state.phase = 1;
                    write_state(host, revision, &state)?;
                    host.action(Action::Shield, 1)?;
                    // Zero is a normal nullable failure, not an error/rollback.
                    host.action(Action::Summon, argument)?;
                }
                let summons = host.query(Query::Summons)?;
                // Reread after synchronous callbacks; the pre-action snapshot is
                // not authority for a later write, even though it is owned.
                let (_, current) = read_state::<EncounterState>(host)?;
                if current.phase != 1 {
                    return Err(Fault::Invalid);
                }
                Ok(summons)
            }
            event::CALLBACK => {
                if argument < 0 {
                    return Err(Fault::Invalid);
                }
                let (revision, mut state) = read_state::<EncounterState>(host)?;
                host.query(Query::Shield)?;
                state.callbacks = state.callbacks.checked_add(1).ok_or(Fault::Overflow)?;
                write_state(host, revision, &state)?;
                if argument > 0 {
                    host.action(Action::Reenter, argument - 1)?;
                }
                let (_, current) = read_state::<EncounterState>(host)?;
                i64::try_from(current.callbacks).map_err(|_| Fault::Overflow)
            }
            event::RESET | event::REMOVING => {
                let (revision, _) = read_state::<EncounterState>(host)?;
                write_state(host, revision, &EncounterState::default())?;
                host.action(Action::Shield, 0)?;
                Ok(0)
            }
            event::ATTACHED | event::DETACHED => Ok(0),
            STALE_OUTER_WRITE => {
                let (revision, mut stale) = read_state::<EncounterState>(host)?;
                host.action(Action::Summon, 1)?;
                stale.phase = 1;
                // Expected Revision: nested CALLBACK already changed this state.
                write_state(host, revision, &stale)?;
                Ok(0)
            }
            FAIL_AFTER_SHIELD => {
                host.action(Action::Shield, 1)?;
                host.action(Action::Fail, 0)
            }
            _ => Ok(0),
        }
    }
}

#[cfg(target_arch = "wasm32")]
conformance_contract::export_module!(Encounter);

#[cfg(test)]
mod tests;
