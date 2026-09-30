//! Complete map-owned Spell operation, dormant until producer migration.
use super::{MapManager, MapObjectTickContinuation, ObjectMapUpdateToken, ActorTickAccessError, PlayerHandle};
use super::tick_objects::ActorStepIdentity;
use crate::map::CreatureActorWitness;
use crate::map_manager::{SpellQueue, SpellProgress, SpellLosPending, SpellPublicationPending, SpellPolicies, SpellOutcome,
    SpellMap, SpellValidation, MapManager as LegacyMapManager, SpellAction};
use wow_core::ObjectGuid;
use std::collections::HashMap;

mod access;
mod pending;
mod disposition;
mod legacy;
pub use pending::{ActorSpellProgress, ActorSpellContinuation, ActorSpellLosQuery, ActorSpellLosRequest,
    ActorSpellPublicationContinuation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorSpellError {
    Access(ActorTickAccessError),
    PlayerIdentityMismatch { guid: ObjectGuid },
    MapIdOutOfRange { map_id: u32 },
    CastChanged(SpellValidation),
}
pub struct ActorSpellPrepareFailure { pub error: ActorSpellError }
pub struct ActorSpellResumeFailure {
    pub error: ActorSpellError, pub continuation: ActorSpellContinuation, pub response: bool,
}
pub struct ActorSpellPublicationFailure {
    pub error: ActorSpellError, pub continuation: ActorSpellPublicationContinuation,
}
impl std::fmt::Debug for ActorSpellPublicationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorSpellPublicationFailure").field("error", &self.error).finish_non_exhaustive()
    }
}
impl std::fmt::Debug for ActorSpellPrepareFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorSpellPrepareFailure").field("error", &self.error).finish()
    }
}
impl std::fmt::Debug for ActorSpellResumeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorSpellResumeFailure").field("error", &self.error)
            .field("response", &self.response).finish_non_exhaustive()
    }
}

impl MapManager {
    pub fn prepare_spell(&mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        terrain_enabled: bool, policies: &mut SpellPolicies<'_>,
    ) -> Result<ActorSpellProgress, ActorSpellPrepareFailure> {
        let admitted = self.admit_spell(tick, token).map_err(|error| ActorSpellPrepareFailure { error })?;
        let Some(identity) = admitted.identity else {
            return Ok(ActorSpellProgress::Complete(SpellOutcome { maps_seen: 1, ..SpellOutcome::default() }));
        };
        let map_id = token.key().map_id as u16;
        let instance_id = token.key().instance_id;
        let difficulty = self.maps.get(&token.key()).unwrap().difficulty();
        let map = self.maps.get_mut(&token.key()).unwrap().map_mut();
        let mut backend = SpellMap::Canonical { map, witnesses: &admitted.witnesses };
        let queue = SpellQueue::prepare(&mut backend, admitted.primaries, map_id, instance_id, difficulty, policies);
        let progress = queue.consume(&mut backend, policies, terrain_enabled);
        Ok(pending::wrap(token, identity, admitted.witnesses, admitted.players, terrain_enabled, progress))
    }

    pub fn resume_spell_los(&mut self, tick: &MapObjectTickContinuation, token: &mut ObjectMapUpdateToken,
        continuation: ActorSpellContinuation, response: bool, policies: &mut SpellPolicies<'_>,
    ) -> Result<ActorSpellProgress, ActorSpellResumeFailure> {
        if let Err(error) = self.validate_spell_resume(tick, token, &continuation) {
            return Err(ActorSpellResumeFailure { error, continuation, response });
        }
        let ActorSpellContinuation { identity, pending, witnesses, players, terrain_enabled } = continuation;
        let map = self.maps.get_mut(&token.key()).unwrap().map_mut();
        let mut backend = SpellMap::Canonical { map, witnesses: &witnesses };
        let progress = pending.resume(&mut backend, response, policies, terrain_enabled);
        Ok(pending::wrap(token, identity, witnesses, players, terrain_enabled, progress))
    }

    /// Acknowledge owned wire publication before the Hit tombstone and next action.
    pub fn resume_spell_publication(&mut self, tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken, continuation: ActorSpellPublicationContinuation,
        policies: &mut SpellPolicies<'_>,
    ) -> Result<ActorSpellProgress, ActorSpellPublicationFailure> {
        if let Err(error) = self.validate_spell_publication(tick, token, &continuation) {
            return Err(ActorSpellPublicationFailure { error, continuation });
        }
        let ActorSpellPublicationContinuation { identity, pending, witnesses, players, terrain_enabled } = continuation;
        let map = self.maps.get_mut(&token.key()).unwrap().map_mut();
        let mut backend = SpellMap::Canonical { map, witnesses: &witnesses };
        let progress = pending.resume(&mut backend, policies, terrain_enabled);
        Ok(pending::wrap(token, identity, witnesses, players, terrain_enabled, progress))
    }
}

#[cfg(test)]
mod tests;
