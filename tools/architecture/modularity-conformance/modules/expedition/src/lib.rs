//! Independent custom expedition stampbook, authored after the two-module freeze.
//! This rule is not a C++ gameplay port and has no durable reward/DB claim.

mod state;

pub use state::{ExpeditionState, HEADER_BYTES, MAX_CHECKPOINTS, STATE_LIMIT};

use conformance_contract::{
    ABI_VERSION, Action, Fault, Host, Manifest, Module, Query, Result, State, capability, event,
    read_state, write_state,
};

pub const MODULE_ID: u64 = 73;
pub const STAMP: u32 = event::CUSTOM + 64;
pub const COUNT: u32 = event::CUSTOM + 65;
pub const CONTRIBUTION_PER_CHECKPOINT: i64 = 5;

pub struct Expedition;

impl Module for Expedition {
    type State = ExpeditionState;

    fn manifest() -> Manifest {
        Manifest {
            id: MODULE_ID,
            name: "expedition",
            abi: ABI_VERSION,
            schema: ExpeditionState::SCHEMA,
            capabilities: capability::QUERY | capability::CONTRIBUTION,
            state_limit: STATE_LIMIT,
            order: 30,
            exclusive: None,
        }
    }

    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        match event {
            STAMP => {
                let checkpoint = u8::try_from(argument).map_err(|_| Fault::Invalid)?;
                if !(1..=31).contains(&checkpoint) {
                    return Err(Fault::Invalid);
                }
                if host.query(Query::Residence)? == 0 {
                    return Err(Fault::NotActive);
                }
                let (revision, mut state) = read_state::<ExpeditionState>(host)?;
                let position = match state.checkpoints.binary_search(&checkpoint) {
                    Ok(_) => return Ok(state.checkpoints.len() as i64),
                    Err(position) => position,
                };
                if state.checkpoints.len() == MAX_CHECKPOINTS {
                    return Err(Fault::Limit);
                }
                state.accepted_total =
                    state.accepted_total.checked_add(1).ok_or(Fault::Overflow)?;
                state.checkpoints.insert(position, checkpoint);
                write_state(host, revision, &state)?;
                // A later action failure does not undo the accepted stamp. Detach/attach
                // can reconcile the derived contribution; this is not a transaction.
                host.action(
                    Action::Contribution,
                    state.checkpoints.len() as i64 * CONTRIBUTION_PER_CHECKPOINT,
                )?;
                Ok(state.checkpoints.len() as i64)
            }
            COUNT => Ok(read_state::<ExpeditionState>(host)?.1.checkpoints.len() as i64),
            event::DETACHED | event::ATTACHED => {
                let residence = host.query(Query::Residence)?;
                let expected = if event == event::DETACHED {
                    0
                } else {
                    if !(0..=u8::MAX as i64).contains(&argument) {
                        return Err(Fault::Invalid);
                    }
                    argument + 1
                };
                if residence != expected {
                    return Err(Fault::Invalid);
                }
                let (_, state) = read_state::<ExpeditionState>(host)?;
                if !state.checkpoints.is_empty() {
                    host.action(
                        Action::Contribution,
                        if residence == 0 {
                            0
                        } else {
                            state.checkpoints.len() as i64 * CONTRIBUTION_PER_CHECKPOINT
                        },
                    )?;
                }
                Ok(state.checkpoints.len() as i64)
            }
            event::RESET => {
                let (revision, mut state) = read_state::<ExpeditionState>(host)?;
                state.resets = state.resets.checked_add(1).ok_or(Fault::Overflow)?;
                state.checkpoints.clear();
                write_state(host, revision, &state)?;
                host.action(Action::Contribution, 0)?;
                Ok(0)
            }
            event::REMOVING => {
                host.action(Action::Contribution, 0)?;
                Ok(0)
            }
            // Does not intercept base policies or encounter callbacks. A third producer
            // still executes in their ordered fanout, without changing their results.
            _ => Ok(0),
        }
    }
}

#[cfg(target_arch = "wasm32")]
conformance_contract::export_module!(Expedition);

#[cfg(test)]
mod tests;
