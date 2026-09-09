//! Represented instance binding, occupancy and resets.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn prune_expired_instance_reset_times_like_cpp(
        &mut self,
        now_secs: u64,
    ) {
        self.represented_instance_reset_times_like_cpp
            .retain(|_, release_time| *release_time > now_secs);
    }
    pub(in crate::session) fn cannot_enter_existing_instance_lock_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        target_lock_context: wow_map::CreateMapInstanceLockContext,
    ) -> Option<wow_instances::TransferAbortReason> {
        let player_guid = self.player_guid?;
        let owner_guid_counter = i64::try_from(target_lock_context.owner_guid_counter).ok()?;
        let owner_guid = ObjectGuid::create_player(1, owner_guid_counter);
        let entries = self.create_map_db2_entries_like_cpp(map_id, difficulty_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = self.instance_lock_mgr.as_ref()?;
        let mgr = mgr.read().ok()?;
        let target_lock = mgr.find_active_instance_lock_at(owner_guid, &entries, now)?;
        Some(mgr.can_join_instance_lock_at(player_guid, &entries, target_lock, now))
    }
    pub(in crate::session) fn create_instance_lock_for_new_instance_side_effect_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        owner_guid: ObjectGuid,
        instance_id: u32,
    ) -> Option<()> {
        let entries = self.create_map_db2_entries_like_cpp(map_id, difficulty_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = self.instance_lock_mgr.as_ref()?;
        let mut mgr = mgr.write().ok()?;
        mgr.create_instance_lock_for_new_instance_at(
            owner_guid,
            &entries,
            instance_id,
            self.reset_schedule_like_cpp,
            now,
        )?;
        Some(())
    }
    pub(in crate::session) fn set_active_instance_lock_instance_id_side_effect_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        instance_id: u32,
    ) -> Option<()> {
        let entries = self.create_map_db2_entries_like_cpp(map_id, difficulty_id)?;
        let owner_guid = self.create_map_instance_owner_guid_like_cpp(map_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = self.instance_lock_mgr.as_ref()?;
        let mut mgr = mgr.write().ok()?;
        mgr.set_active_instance_lock_instance_id_at(owner_guid, &entries, now, instance_id)
            .then_some(())
    }
    pub(crate) fn create_map_active_instance_lock_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapInstanceLockContext> {
        let entries = self.create_map_db2_entries_like_cpp(map_id, difficulty_id)?;
        let owner_guid = self.create_map_instance_owner_guid_like_cpp(map_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = self.instance_lock_mgr.as_ref()?;
        let mgr = mgr.read().ok()?;
        let lock = mgr.find_active_instance_lock_at(owner_guid, &entries, now)?;

        Some(wow_map::CreateMapInstanceLockContext {
            instance_id: lock.instance_id,
            difficulty_id: lock.difficulty_id,
            token: create_map_instance_lock_token_like_cpp(owner_guid, &entries, lock),
            owner_guid_counter: owner_guid.counter() as u64,
        })
    }
    pub(crate) fn lfg_has_active_instance_lock_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> bool {
        let Some(player_guid) = self.player_guid else {
            return false;
        };
        let Some(entries) = self.create_map_db2_entries_like_cpp(map_id, difficulty_id) else {
            return false;
        };
        let Some(mgr) = self.instance_lock_mgr.as_ref() else {
            return false;
        };
        let Ok(mgr) = mgr.read() else {
            return false;
        };
        let now = u64::try_from(unix_now()).unwrap_or(0);
        mgr.find_active_instance_lock_at(player_guid, &entries, now)
            .is_some()
    }
    /// Inject the shared C++ `InstanceLockMgr` analogue.
    pub fn set_instance_lock_mgr(
        &mut self,
        mgr: Arc<std::sync::RwLock<wow_instances::InstanceLockMgr>>,
    ) {
        self.instance_lock_mgr = Some(mgr);
    }
    pub(crate) fn apply_represented_player_instance_reset_result_like_cpp(
        &mut self,
        map_id: u32,
        result: GroupInstanceResetResultLikeCpp,
        method: GroupInstanceResetMethodLikeCpp,
    ) -> bool {
        match result {
            GroupInstanceResetResultLikeCpp::Success => {
                self.forget_represented_player_recent_instance_like_cpp(map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty
                if method == GroupInstanceResetMethodLikeCpp::OnChangeDifficulty =>
            {
                self.forget_represented_player_recent_instance_like_cpp(map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty
            | GroupInstanceResetResultLikeCpp::CannotReset
            | GroupInstanceResetResultLikeCpp::Other => false,
        }
    }
}
