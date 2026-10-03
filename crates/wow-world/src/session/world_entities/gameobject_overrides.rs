//! Recorded represented gameobject overrides layered over canonical state.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_owner_guid_like_cpp(
        &mut self,
        guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_owner_guid_like_cpp(&mut hub, guid, owner_guid)
    }
    pub(crate) fn record_represented_gameobject_db_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_db_phase_shift_like_cpp(
            &mut hub,
            guid,
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/gameobject_overrides/f3_shims.rs"]
mod f3_shims;
