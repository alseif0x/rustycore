//! Complete dormant canonical Aggro operation; no producer activation.
use super::{MapManager, MapObjectTickContinuation, ObjectMapUpdateToken,
    ActorTickAccessError, PlayerHandle};
use super::tick_objects::ActorStepIdentity;
use crate::map::CreatureActorWitness;
use crate::map_manager::{AggroCandidate, AggroOutcome, AggroPolicies, AggroSettings,
    AggroFrame, AggroMap, AggroTailProgress, AggroLosPending, LiveTerrainHeights};
use std::collections::HashMap;
use wow_core::{ObjectGuid, Position};

mod access;
mod pending;
pub use pending::{ActorAggroContinuation, ActorAggroLosQuery, ActorAggroLosRequest, ActorAggroProgress};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorAggroError {
    Access(ActorTickAccessError),
    PlayerIdentityMismatch { guid: ObjectGuid },
    MapIdOutOfRange { map_id: u32 },
}

pub struct ActorAggroPrepareFailure {
    pub error: ActorAggroError,
    pub candidates: Vec<AggroCandidate>,
}
pub struct ActorAggroResumeFailure {
    pub error: ActorAggroError,
    pub continuation: ActorAggroContinuation,
    pub response: bool,
}

impl std::fmt::Debug for ActorAggroPrepareFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorAggroPrepareFailure").field("error", &self.error).finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ActorAggroResumeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorAggroResumeFailure").field("error", &self.error).field("response", &self.response).finish_non_exhaustive()
    }
}

impl MapManager {
    /// Validate the whole selected primary set before the first actor mutation.
    /// The existing operation slot covers every off-guard tail LOS query.
    pub fn prepare_aggro(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        candidates: Vec<AggroCandidate>, settings: AggroSettings,
        terrain_enabled: bool, policies: &mut AggroPolicies<'_>,
    ) -> Result<ActorAggroProgress, ActorAggroPrepareFailure> {
        let admission = match self.admit_aggro(tick, token, &candidates) {
            Ok(admission) => admission,
            Err(error) => return Err(ActorAggroPrepareFailure { error, candidates }),
        };
        let Some(identity) = admission.identity else {
            return Ok(ActorAggroProgress::Complete(AggroOutcome {
                maps_seen: 1, candidates_seen: candidates.iter().filter(|candidate|
                    u32::from(candidate.map_id) == token.key().map_id
                        && candidate.instance_id == token.key().instance_id).count(),
                ..AggroOutcome::default()
            }));
        };
        let map_id = token.key().map_id as u16; // range was checked before slot/mutation
        let instance_id = token.key().instance_id;
        let runtime = &mut self.maps.get_mut(&token.key()).unwrap().runtime;
        let mut backend = AggroMap::Canonical { runtime,
            witnesses: &admission.witnesses, order: &admission.order };
        let mut frame = AggroFrame::new(map_id, instance_id, admission.primaries,
            candidates, settings, &backend);
        frame.run_primaries(&mut backend, policies);
        let progress = frame.prepare_tail(&mut backend, policies, terrain_enabled);
        Ok(pending::wrap(token, identity, admission.witnesses, admission.players,
            admission.order, terrain_enabled, progress))
    }

    /// Consume one observed LOS reply only after the original token/slot,
    /// caller, assistant and victim identities still authorize this operation.
    pub fn resume_aggro_assistance_los(
        &mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        continuation: ActorAggroContinuation, response: bool, policies: &mut AggroPolicies<'_>,
    ) -> Result<ActorAggroProgress, ActorAggroResumeFailure> {
        if let Err(error) = self.validate_aggro_resume(tick, token, &continuation) {
            return Err(ActorAggroResumeFailure { error, continuation, response });
        }
        let ActorAggroContinuation { identity, pending, witnesses, players, order, terrain_enabled } = continuation;
        let runtime = &mut self.maps.get_mut(&token.key()).unwrap().runtime;
        let mut backend = AggroMap::Canonical { runtime, witnesses: &witnesses, order: &order };
        let frame = pending.resume(response, &mut backend, policies);
        let progress = frame.prepare_tail(&mut backend, policies, terrain_enabled);
        Ok(pending::wrap(token, identity, witnesses, players, order, terrain_enabled, progress))
    }
}

#[cfg(test)]
mod tests;
