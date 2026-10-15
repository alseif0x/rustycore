//! Represented instance binding, occupancy and resets.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<bool> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.player_is_locked_to_dungeon_encounter_like_cpp(hub, player_guid, dungeon_encounter_id)
    }

    pub(in crate::session) fn cannot_enter_existing_instance_lock_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        target_lock_context: wow_map::CreateMapInstanceLockContext,
    ) -> Option<wow_instances::TransferAbortReason> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.cannot_enter_existing_instance_lock_like_cpp(
            hub,
            map_id,
            difficulty_id,
            target_lock_context,
        )
    }
    pub(crate) fn lfg_has_active_instance_lock_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> bool {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.lfg_has_active_instance_lock_like_cpp(hub, map_id, difficulty_id)
    }
    /// Inject the shared C++ `InstanceLockMgr` analogue.
    pub fn set_instance_lock_mgr(
        &mut self,
        mgr: Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>,
    ) {
        self.core.instance_lock_mgr = Some(mgr);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/binding/f3_shims.rs"]
mod f3_shims;
