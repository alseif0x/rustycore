//! Internal actor access under an exact admitted map-update token.
//!
//! C++ MapManager.cpp:287-318 and Map.cpp:666-815 establish the selected
//! map/object order. These gates do not activate a canonical runtime producer,
//! establish writer quiescence, or reproduce omitted AI/publication stages.

use super::{
    MapManager, MapObjectTickContinuation, ObjectMapFinishOutcome, ObjectMapTickError,
    ObjectMapUpdateToken,
};
use super::tick_objects::ActorStepIdentity;
use crate::map::CreatureActorWitness;
use crate::map_manager::WorldCreature;
use crate::MapKey;
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorTickAccessError {
    Tick(ObjectMapTickError),
    StaleParticipant {
        key: MapKey,
        admitted_incarnation: u64,
        current_incarnation: Option<u64>,
    },
    OutsideSelection { guid: ObjectGuid },
    ActorUnavailable { guid: ObjectGuid },
    WitnessMismatch { guid: ObjectGuid },
    NoActorOperation,
    OperationMismatch { guid: ObjectGuid },
}

impl MapManager {
    /// Capture the exact actor workset at this family's first access. No later
    /// admission expands it, and a replacement under an admitted GUID retains
    /// the original witness in the workset so access rejects the ABA.
    pub fn selected_actor_guids(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
    ) -> Result<Vec<ObjectGuid>, ActorTickAccessError> {
        self.require_current_actor_token(tick, token)?;
        let map = self.maps.get(&token.key()).expect("current token retains its map").map();
        Ok(token.continuation.actor_workset(map).iter().map(|(guid, _)| *guid).collect())
    }

    /// Validate admission, selection and actor identity before one synchronous
    /// callback. R cannot borrow from the actor; no actor/entity clone or guard
    /// escapes. During an operation only that operation's actor may be accessed.
    pub(crate) fn with_selected_actor<R>(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: Option<&CreatureActorWitness>,
        apply: impl FnOnce(&mut WorldCreature) -> R,
    ) -> Result<(R, CreatureActorWitness), ActorTickAccessError> {
        self.require_current_actor_token(tick, token)?;
        let witness = self.selected_actor_witness(token, guid, expected_witness)?;
        let actor = self.maps.get_mut(&token.key()).expect("current token retains its map")
            .map_mut().creature_actor_mut(guid)
            .ok_or(ActorTickAccessError::ActorUnavailable { guid })?;
        Ok((apply(actor), witness))
    }

    /// Reserve one operation before preparing its lazy query/request. The slot
    /// owns the identity; dropping a request or its witness clone cannot clear it.
    pub(crate) fn begin_actor_operation(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: Option<&CreatureActorWitness>,
    ) -> Result<CreatureActorWitness, ActorTickAccessError> {
        self.require_current_actor_token(tick, token)?;
        if let Some(operation) = &token.actor_operation {
            return Err(ActorTickAccessError::Tick(ObjectMapTickError::ActorOperationInFlight {
                guid: operation.guid,
            }));
        }
        let witness = self.selected_actor_witness(token, guid, expected_witness)?;
        token.actor_operation = Some(ActorStepIdentity::new(token, guid, witness.clone()));
        Ok(witness)
    }

    /// A query result may be resumed only against the same pending operation
    /// and the same current actor in the original admitted workset.
    pub(crate) fn resume_actor_operation(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: &CreatureActorWitness,
    ) -> Result<CreatureActorWitness, ActorTickAccessError> {
        self.require_current_actor_token(tick, token)?;
        require_actor_operation(token, guid, expected_witness)?;
        self.selected_actor_witness(token, guid, Some(expected_witness))
    }

    /// Release an operation only after its work is settled/disposed by the
    /// consumer. Validate the original token and slot identity, not authority
    /// to mutate a current actor. This deliberately permits disposal after a
    /// stale map or same-GUID readmission; resume/callback still reject both.
    /// It changes no actor, accounting, Creature phase or object tail.
    pub(crate) fn complete_actor_operation(
        &self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: &CreatureActorWitness,
    ) -> Result<(), ActorTickAccessError> {
        self.validate_object_map_token(tick, token).map_err(ActorTickAccessError::Tick)?;
        require_actor_operation(token, guid, expected_witness)?;
        token.actor_operation = None;
        Ok(())
    }

    pub(super) fn require_current_actor_token(
        &self,
        tick: &MapObjectTickContinuation,
        token: &ObjectMapUpdateToken,
    ) -> Result<(), ActorTickAccessError> {
        match self.validate_object_map_token(tick, token).map_err(ActorTickAccessError::Tick)? {
            Some(ObjectMapFinishOutcome::StaleParticipant {
                key, admitted_incarnation, current_incarnation,
            }) => Err(ActorTickAccessError::StaleParticipant {
                key, admitted_incarnation, current_incarnation,
            }),
            _ => Ok(()),
        }
    }

    /// Caller has already passed the shared current-token gate.
    pub(super) fn selected_actor_witness(
        &self,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: Option<&CreatureActorWitness>,
    ) -> Result<CreatureActorWitness, ActorTickAccessError> {
        let map = self.maps.get(&token.key()).expect("current token retains its map").map();
        let admitted = token.continuation.actor_workset(map).iter()
            .find(|(selected, _)| *selected == guid)
            .map(|(_, witness)| witness.clone())
            .ok_or(ActorTickAccessError::OutsideSelection { guid })?;
        let current = map.creature_actor_witness(guid)
            .ok_or(ActorTickAccessError::ActorUnavailable { guid })?;
        if !admitted.same_actor(&current)
            || expected_witness.is_some_and(|expected| !admitted.same_actor(expected))
        {
            return Err(ActorTickAccessError::WitnessMismatch { guid });
        }
        if let Some(operation) = &token.actor_operation {
            if !operation.matches_token(token)
                || operation.guid != guid
                || !operation.witness.same_actor(&admitted)
            {
                return Err(ActorTickAccessError::OperationMismatch { guid });
            }
        }
        Ok(current)
    }
}

fn require_actor_operation(
    token: &ObjectMapUpdateToken,
    guid: ObjectGuid,
    expected_witness: &CreatureActorWitness,
) -> Result<(), ActorTickAccessError> {
    let operation = token.actor_operation.as_ref().ok_or(ActorTickAccessError::NoActorOperation)?;
    if !operation.matches_token(token)
        || operation.guid != guid
        || !operation.witness.same_actor(expected_witness)
    {
        return Err(ActorTickAccessError::OperationMismatch { guid });
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod fixtures;

#[cfg(test)]
mod lifecycle_tests;
