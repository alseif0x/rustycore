use crate::InstanceState;
use wow_core::ObjectGuid;
use wow_social::group::{GroupInstanceResetMethodLikeCpp, GroupInstanceResetResultLikeCpp};
use wow_world_core::session::{HubMut, HubRef, connection_identity::unix_now};

pub fn create_map_instance_lock_token_like_cpp(
    owner_guid: ObjectGuid,
    entries: &wow_instances::MapDb2Entries,
    lock: &wow_instances::InstanceLock,
) -> u64 {
    fn mix(hash: &mut u64, value: u64) {
        *hash ^= value;
        *hash = hash.wrapping_mul(0x1000_0000_01b3);
    }

    let mut hash = 0xcbf2_9ce4_8422_2325;
    mix(&mut hash, owner_guid.high_value() as u64);
    mix(&mut hash, owner_guid.low_value() as u64);
    mix(&mut hash, u64::from(entries.map_id));
    mix(&mut hash, u64::from(entries.lock_id));
    mix(&mut hash, u64::from(entries.difficulty_id));
    mix(&mut hash, u64::from(lock.map_id));
    mix(&mut hash, u64::from(lock.difficulty_id));
    hash
}

impl InstanceState {
    /// C++ `Player::IsLockedToDungeonEncounter(uint32)`.
    ///
    /// The encounter row is immutable process data; the completed mask is
    /// read from the shared `InstanceLockMgr` for the player's exact canonical
    /// map and difficulty. A missing/ambiguous authority fails closed for loot
    /// callers by returning `None`; an unknown encounter or absent active lock
    /// is a known unlocked state, matching C++.
    pub fn player_is_locked_to_dungeon_encounter_like_cpp(
        &self,
        hub: HubRef<'_>,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<bool> {
        let store = hub.catalogs.dungeon_encounter_store()?;
        let Some(encounter) = store.get(dungeon_encounter_id) else {
            return Some(false);
        };
        let bit = u32::try_from(encounter.bit).ok().filter(|bit| *bit < 32)?;

        let manager = hub.core.canonical_map_manager.as_ref()?.lock().ok()?;
        let mut residence = None;
        let mut ambiguous = false;
        manager.do_for_all_maps(|managed| {
            if managed.map().get_typed_player(player_guid).is_none() {
                return;
            }
            if residence.is_some() {
                ambiguous = true;
            } else {
                residence = Some((managed.map_id(), managed.difficulty()));
            }
        });
        drop(manager);
        if ambiguous {
            return None;
        }
        let (map_id, difficulty_id) = residence?;
        let entries = self.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)?;
        let now = u64::try_from(unix_now()).ok()?;
        let lock_mgr = hub.core.instance_lock_mgr.as_ref()?.read().ok()?;
        let Some(lock) = lock_mgr.find_active_instance_lock_at(player_guid, &entries, now) else {
            return Some(false);
        };
        Some(
            (lock
                .instance_initialization_data()
                .completed_encounters_mask
                & (1u32 << bit))
                != 0,
        )
    }

    pub fn prune_expired_instance_reset_times_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        now_secs: u64,
    ) {
        if hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.prune_instance_reset_times_like_cpp(now_secs);
            })
            .is_some()
        {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_instance_reset_times_like_cpp
                .retain(|_, release_time| *release_time > now_secs);
        }
    }

    pub fn cannot_enter_existing_instance_lock_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        target_lock_context: wow_map::CreateMapInstanceLockContext,
    ) -> Option<wow_instances::TransferAbortReason> {
        let player_guid = hub.core.player_guid?;
        let owner_guid_counter = i64::try_from(target_lock_context.owner_guid_counter).ok()?;
        let owner_guid = ObjectGuid::create_player(1, owner_guid_counter);
        let entries = self.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = hub.core.instance_lock_mgr.as_ref()?;
        let mgr = mgr.read().ok()?;
        let target_lock = mgr.find_active_instance_lock_at(owner_guid, &entries, now)?;
        Some(mgr.can_join_instance_lock_at(player_guid, &entries, target_lock, now))
    }

    pub fn create_instance_lock_for_new_instance_side_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
        owner_guid: ObjectGuid,
        instance_id: u32,
    ) -> Option<()> {
        let entries = self.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id)?;
        let now = u64::try_from(unix_now()).unwrap_or(0);
        let mgr = hub.core.instance_lock_mgr.as_ref()?;
        let mut mgr = mgr.write().ok()?;
        mgr.create_instance_lock_for_new_instance_at(
            owner_guid,
            &entries,
            instance_id,
            hub.config.reset_schedule_like_cpp,
            now,
        )?;
        Some(())
    }

    pub fn lfg_has_active_instance_lock_like_cpp(
        &self,
        hub: HubRef<'_>,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> bool {
        let Some(player_guid) = hub.core.player_guid else {
            return false;
        };
        let Some(entries) = self.create_map_db2_entries_like_cpp(hub, map_id, difficulty_id) else {
            return false;
        };
        let Some(mgr) = hub.core.instance_lock_mgr.as_ref() else {
            return false;
        };
        let Ok(mgr) = mgr.read() else {
            return false;
        };
        let now = u64::try_from(unix_now()).unwrap_or(0);
        mgr.find_active_instance_lock_at(player_guid, &entries, now)
            .is_some()
    }

    pub fn apply_represented_player_instance_reset_result_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        map_id: u32,
        result: GroupInstanceResetResultLikeCpp,
        method: GroupInstanceResetMethodLikeCpp,
    ) -> bool {
        match result {
            GroupInstanceResetResultLikeCpp::Success => {
                self.forget_represented_player_recent_instance_like_cpp(hub, map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty
                if method == GroupInstanceResetMethodLikeCpp::OnChangeDifficulty =>
            {
                self.forget_represented_player_recent_instance_like_cpp(hub, map_id)
            }
            GroupInstanceResetResultLikeCpp::NotEmpty
            | GroupInstanceResetResultLikeCpp::CannotReset
            | GroupInstanceResetResultLikeCpp::Other => false,
        }
    }
}
