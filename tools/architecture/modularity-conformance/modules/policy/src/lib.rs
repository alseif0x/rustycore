//! Optional custom arithmetic policy, not a complete GiveXP implementation.
//! C++ Player.cpp:2189-2226 places the XP policy hook before award. The fixture's
//! additive contribution and bounds below are a declared custom contract.

use conformance_contract::{
    ABI_VERSION, Action, Fault, Host, Manifest, Module, Query, Result, State, capability, event,
    read_state, write_state,
};

pub const MODULE_ID: u64 = 2;

#[derive(Debug, Eq, PartialEq)]
pub struct PolicyState {
    pub calls: u64,
    pub percent: u32,
}

impl Default for PolicyState {
    fn default() -> Self {
        Self {
            calls: 0,
            percent: 100,
        }
    }
}

impl State for PolicyState {
    const SCHEMA: u32 = 1;

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(12);
        bytes.extend_from_slice(&self.calls.to_le_bytes());
        bytes.extend_from_slice(&self.percent.to_le_bytes());
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 12 {
            return Err(Fault::Invalid);
        }
        let percent = u32::from_le_bytes(bytes[8..].try_into().map_err(|_| Fault::Invalid)?);
        if percent > 1000 {
            return Err(Fault::Invalid);
        }
        Ok(Self {
            calls: u64::from_le_bytes(bytes[..8].try_into().map_err(|_| Fault::Invalid)?),
            percent,
        })
    }
}

pub struct Policy;

impl Module for Policy {
    type State = PolicyState;

    fn manifest() -> Manifest {
        Manifest {
            id: MODULE_ID,
            name: "policy",
            abi: ABI_VERSION,
            schema: PolicyState::SCHEMA,
            capabilities: capability::QUERY | capability::CONTRIBUTION,
            state_limit: 12,
            order: 20,
            exclusive: None,
        }
    }

    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        match event {
            event::POLICY => {
                if !(0..=1_000_000).contains(&argument) {
                    return Err(Fault::Invalid);
                }
                let (revision, mut state) = read_state::<PolicyState>(host)?;
                let amount = argument
                    .checked_mul(i64::from(state.percent))
                    .ok_or(Fault::Overflow)?
                    / 100;
                state.calls = state.calls.checked_add(1).ok_or(Fault::Overflow)?;
                write_state(host, revision, &state)?;
                host.action(Action::Contribution, i64::from(state.percent))?;
                Ok(amount)
            }
            event::CALLBACK => {
                // Observer only: never rewrites encounter state or its shield.
                host.query(Query::Shield)?;
                let (_, state) = read_state::<PolicyState>(host)?;
                i64::try_from(state.calls).map_err(|_| Fault::Overflow)
            }
            event::RESET | event::REMOVING => {
                let (revision, _) = read_state::<PolicyState>(host)?;
                write_state(host, revision, &PolicyState::default())?;
                host.action(Action::Contribution, 0)?;
                Ok(0)
            }
            _ => Ok(0),
        }
    }
}

#[cfg(target_arch = "wasm32")]
conformance_contract::export_module!(Policy);

#[cfg(test)]
mod tests;
