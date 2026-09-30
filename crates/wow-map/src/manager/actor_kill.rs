//! Dormant selected-actor post-hook kill completion.
//! The future composition caller supplies and retains its pending operation.
//! No Session GUID lookup derives a token, witness or reservation.
use super::{
    ActorTickAccessError, MapManager, MapObjectTickContinuation, ObjectMapUpdateToken,
};
use crate::map::CreatureActorWitness;
use wow_core::ObjectGuid;
use wow_entities::UnitValuesUpdate;

impl MapManager {
    #[allow(clippy::too_many_arguments)]
    pub fn finalize_actor_kill(
        &mut self,
        tick: &MapObjectTickContinuation,
        token: &mut ObjectMapUpdateToken,
        guid: ObjectGuid,
        expected_witness: &CreatureActorWitness,
        lootable: bool,
        can_skin: bool,
    ) -> Result<UnitValuesUpdate, ActorTickAccessError> {
        self.resume_actor_operation(tick, token, guid, expected_witness)?;
        let (values, _) = self.with_selected_actor(
            tick, token, guid, Some(expected_witness),
            |actor| actor.finalize_represented_kill(lootable, can_skin),
        )?;
        Ok(values)
    }
}

#[cfg(test)]
mod tests;
