use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::WorldEntitiesState;

impl WorldEntitiesState {
    pub fn pause_interacted_creature_movement_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
    ) -> bool {
        hub.core
            .mutate_world_creature(guid, |creature| {
                creature.pause_interaction_movement_like_cpp()
            })
            .unwrap_or(false)
    }
}
