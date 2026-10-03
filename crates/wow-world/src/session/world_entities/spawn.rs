//! Represented spawn handling for observed world entities.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn drain_ready_map_respawns_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: std::time::Instant,
    ) -> Vec<crate::map_manager::PendingRespawn> {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.drain_ready_map_respawns_like_cpp(&mut hub, map_id, instance_id, now)
    }
    pub(in crate::session) fn despawn_represented_linked_trap_by_guid_like_cpp(
        &mut self,
        trap_guid: ObjectGuid,
    ) {
        if trap_guid.is_empty() || !trap_guid.is_game_object() {
            return;
        }
        if let Some(state) = self
            .world_entities
            .represented_gameobject_use_state_mut_like_cpp(trap_guid)
        {
            state.loot_state = Some(wow_entities::LootState::NotReady);
            state.loot_state_unit_guid = ObjectGuid::EMPTY;
            if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_TRANSPORT) {
                state.go_state = Some(wow_entities::GoState::Ready);
            }
        }
        self.core.client_visible_guids_like_cpp.remove(&trap_guid);
        self.loot.remove_cached_loot_for_owner_like_cpp(trap_guid);
        self.send_represented_gameobject_delete_packets_like_cpp(trap_guid);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/spawn/f3_shims.rs"]
mod f3_shims;
