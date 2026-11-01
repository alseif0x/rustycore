// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Typed access to the session's represented instance-lock manager.

use wow_core::ObjectGuid;
use wow_data::{MapDifficultyStore, MapStore};
use wow_instances::{
    InstanceLock, InstanceLockUpdateEvent, MapDb2Entries, MapDifficultyResetInterval, ResetSchedule,
};
use wow_persistence::InstanceLockPersistencePlanLikeCpp;

use crate::session::{SessionWorldConfig, state::SessionCore};

/// Borrowed, operation-specific access to the session's instance-lock owner.
/// The manager, its `Arc`, and its lock guard never leave this capability.
pub struct InstanceLockManagerAccessLikeCpp<'a> {
    core: &'a SessionCore,
    reset_schedule: &'a wow_instances::ResetSchedule,
}

impl SessionCore {
    /// Build an inert capability; manager presence is queried only by an
    /// operation at the same point where the World path currently resolves it.
    pub fn instance_lock_manager_access_like_cpp<'a>(
        &'a self,
        config: &'a SessionWorldConfig,
    ) -> InstanceLockManagerAccessLikeCpp<'a> {
        InstanceLockManagerAccessLikeCpp {
            core: self,
            reset_schedule: &config.reset_schedule_like_cpp,
        }
    }
}

impl InstanceLockManagerAccessLikeCpp<'_> {
    pub fn reset_schedule_like_cpp(&self) -> ResetSchedule {
        *self.reset_schedule
    }

    pub fn raid_info_locks_like_cpp(
        &self,
        player_guid: ObjectGuid,
        map_store: Option<&MapStore>,
        map_difficulty_store: Option<&MapDifficultyStore>,
    ) -> Option<Vec<wow_instances::InstanceRaidInfoLock>> {
        let manager = self.core.instance_lock_mgr.as_ref()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let manager = manager.read().ok()?;
        Some(manager.get_raid_info_locks_for_player_at(
            player_guid,
            now,
            ResetSchedule::default(),
            |map_id, difficulty_id| {
                let map = map_store?.get(map_id)?;
                let map_difficulty = map_difficulty_store?.get(map_id, difficulty_id)?;
                Some(MapDb2Entries {
                    map_id,
                    difficulty_id,
                    lock_id: u32::from(map_difficulty.lock_id),
                    reset_interval: match map_difficulty.reset_interval {
                        1 => MapDifficultyResetInterval::Daily,
                        2 => MapDifficultyResetInterval::Weekly,
                        _ => MapDifficultyResetInterval::Anytime,
                    },
                    max_players: map_difficulty.max_players,
                    is_flex_locking: map.is_flex_locking(),
                    is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
                })
            },
        ))
    }

    pub fn active_lock_for_map_reset_like_cpp(
        &self,
        context: wow_map::CreateMapInstanceLockContext,
        map_id: u32,
        instance_id: u32,
        difficulty_id: wow_map::Difficulty,
        map_store: Option<&MapStore>,
        map_difficulty_store: Option<&MapDifficultyStore>,
    ) -> Option<(InstanceLock, MapDb2Entries)> {
        if context.instance_id != instance_id || context.difficulty_id != difficulty_id {
            return None;
        }
        let map = map_store?.get(map_id)?;
        let map_difficulty = map_difficulty_store?.get(map_id, difficulty_id)?;
        let entries = MapDb2Entries {
            map_id,
            difficulty_id,
            lock_id: u32::from(map_difficulty.lock_id),
            reset_interval: match map_difficulty.reset_interval {
                1 => MapDifficultyResetInterval::Daily,
                2 => MapDifficultyResetInterval::Weekly,
                _ => MapDifficultyResetInterval::Anytime,
            },
            max_players: map_difficulty.max_players,
            is_flex_locking: map.is_flex_locking(),
            is_using_encounter_locks: map_difficulty.is_using_encounter_locks(),
        };
        let owner_guid =
            ObjectGuid::create_player(1, i64::try_from(context.owner_guid_counter).ok()?);
        let manager = self.core.instance_lock_mgr.as_ref()?.read().ok()?;
        let lock = manager.find_instance_lock_exact_like_cpp(
            owner_guid,
            map_id,
            difficulty_id,
            instance_id,
        )?;
        Some((lock.clone(), entries))
    }

    pub fn update_lock_for_player_with_persistence_like_cpp(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        instance_id: u32,
        completed_mask: u32,
    ) -> Option<(bool, InstanceLock, InstanceLockPersistencePlanLikeCpp, u64)> {
        let instance_lock_mgr = self.core.instance_lock_mgr.as_ref()?.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut persistence_plan = InstanceLockPersistencePlanLikeCpp::default();
        let (is_new_lock, new_lock) = {
            let mut manager = instance_lock_mgr.write().ok()?;
            let is_new_lock = manager
                .find_active_instance_lock_at(player_guid, entries, now)
                .is_none_or(|lock| lock.is_new || lock.is_expired_at(now));
            let update_event = InstanceLockUpdateEvent {
                instance_id,
                new_data: String::new(),
                instance_completed_encounters_mask: completed_mask,
                completed_encounter_bit: None,
                entrance_world_safe_loc_id: None,
            };
            let schedule = *self.reset_schedule;
            let new_lock = manager.update_instance_lock_for_player_with_persistence_at(
                &mut persistence_plan,
                player_guid,
                entries,
                update_event,
                schedule,
                now,
            )?;
            (is_new_lock, new_lock)
        };
        Some((is_new_lock, new_lock, persistence_plan, now))
    }

    pub fn update_lock_extension_with_persistence_like_cpp(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        extended: bool,
    ) -> Option<((u64, u64), InstanceLockPersistencePlanLikeCpp, u64)> {
        let instance_lock_mgr = self.core.instance_lock_mgr.as_ref()?.clone();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let mut persistence_plan = InstanceLockPersistencePlanLikeCpp::default();
        let expiry_times = {
            let mut manager = instance_lock_mgr.write().ok()?;
            let schedule = *self.reset_schedule;
            manager.update_instance_lock_extension_for_player_with_persistence_at(
                &mut persistence_plan,
                player_guid,
                entries,
                extended,
                schedule,
                now,
            )?
        };
        Some((expiry_times, persistence_plan, now))
    }
}
