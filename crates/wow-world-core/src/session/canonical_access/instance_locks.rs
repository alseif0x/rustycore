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
    pub fn player_guid_for_instance_lock_owner_like_cpp(
        &self,
        owner_guid_counter: u64,
    ) -> Option<ObjectGuid> {
        Some(ObjectGuid::create_player(
            self.core.realm_id(),
            i64::try_from(owner_guid_counter).ok()?,
        ))
    }

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
            self.player_guid_for_instance_lock_owner_like_cpp(context.owner_guid_counter)?;
        let manager = self.core.instance_lock_mgr.as_ref()?.read().ok()?;
        let lock = manager.find_instance_lock_exact_like_cpp(
            owner_guid,
            map_id,
            difficulty_id,
            instance_id,
        )?;
        Some((lock.clone(), entries))
    }

    pub async fn persist_player_bind_like_cpp(
        &self,
        player_guid: ObjectGuid,
        entries: &MapDb2Entries,
        event: InstanceLockUpdateEvent,
        port: std::sync::Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    ) -> Option<(bool, InstanceLock, u64)> {
        let manager = self.core.instance_lock_mgr.as_ref()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let (is_new, updated) = {
            let manager = manager.read().ok()?;
            let is_new = manager
                .find_active_instance_lock_at(player_guid, entries, now)
                .is_none_or(|lock| lock.is_new || lock.is_expired_at(now));
            let updated = manager.prepare_player_lock_update(
                player_guid,
                entries,
                event,
                *self.reset_schedule,
                now,
            )?;
            (is_new, updated)
        };
        let plan = wow_instances::InstanceLockMgr::player_lock_persistence_plan(
            player_guid,
            entries,
            &updated,
        );
        if let wow_persistence::InstanceLockPersistenceOutcomeLikeCpp::Failed { reason } =
            port.commit_plan_like_cpp(plan).await
        {
            tracing::warn!(?player_guid, instance_id = updated.instance_id, error = %reason, "instance bind transaction failed");
            return None;
        }
        manager.write().ok()?.install_player_lock_update(
            player_guid,
            entries,
            updated.clone(),
            now,
        );
        Some((is_new, updated, now))
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
