//! Owned results of a canonical map tick; delivery happens after its guard.

use super::map::{RespawnDbDeleteLikeCpp, RespawnDbSaveLikeCpp};

/// A VALUES update built while the canonical map owned its lock. Delivery is
/// performed only after that lock is released and is bound to the recipient's
/// current registration and committed visibility set.
#[derive(Debug, Clone)]
pub(crate) struct CanonicalMapObjectValuesUpdateLikeCpp {
    pub(crate) map_id: u16,
    pub(crate) instance_id: u32,
    pub(crate) object_guid: wow_core::ObjectGuid,
    pub(crate) packet_bytes: Vec<u8>,
    /// Compatibility payload for typed Unit/Creature/Pet delivery. The
    /// canonical map tick leaves Player/Unit snapshots on the Session-side
    /// receiver-filtered P3.10 path instead of routing them through this
    /// generic rail; other producers may still populate this field.
    pub(crate) unit_values_update: Option<wow_packet::packets::update::UnitDataValuesDeltaUpdate>,
}

/// A directed Creature destroy selected by the canonical Map while the source
/// was still attached. The world loop resolves recipient registrations and
/// publishes the session command only after releasing map guards.
#[derive(Debug, Clone)]
pub(crate) struct CanonicalCreatureVisibilityDestroyLikeCpp {
    pub(crate) map_id: u32,
    pub(crate) instance_id: u32,
    pub(crate) map_incarnation: u64,
    pub(crate) creature_guid: wow_core::ObjectGuid,
    pub(crate) recipient_guids: Vec<wow_core::ObjectGuid>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CanonicalSpawnGroupConditionTickSummaryLikeCpp {
    pub(crate) player_visibility_refresh_intents:
        Vec<wow_map::PlayerVisibilityRefreshIntentLikeCpp>,
    pub(crate) object_values_updates: Vec<CanonicalMapObjectValuesUpdateLikeCpp>,
    pub(crate) creature_visibility_destroys: Vec<CanonicalCreatureVisibilityDestroyLikeCpp>,
    pub(crate) expired_pvp_combat_refs: Vec<(u32, u32, wow_core::ObjectGuid, wow_core::ObjectGuid)>,
    pub(crate) maps_evaluated: usize,
    pub(crate) outcomes: usize,
    pub(crate) applied_set_inactive: usize,
    pub(crate) planned_spawn: usize,
    pub(crate) condition_spawn_executed_loaded_grid_spawns: usize,
    pub(crate) condition_spawn_legacy_creature_mirrors: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_spawn_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_creature_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_gameobject_loads: usize,
    pub(crate) condition_spawn_blocked_loaded_grid_spawn_add_to_map: usize,
    pub(crate) condition_spawn_load_plan_count: usize,
    pub(crate) condition_spawn_unsupported_spawn_types: usize,
    pub(crate) condition_spawn_skipped_respawn_timer_active: usize,
    pub(crate) condition_spawn_skipped_live_object_active: usize,
    pub(crate) condition_spawn_skipped_unloaded_grid: usize,
    pub(crate) condition_spawn_skipped_difficulty_mismatch: usize,
    pub(crate) planned_despawn: usize,
    pub(crate) despawn_executed: usize,
    pub(crate) despawn_objects_removed: usize,
    pub(crate) despawn_respawn_timers_removed: usize,
    pub(crate) despawn_blocked_missing_group: usize,
    pub(crate) despawn_blocked_system_group: usize,
    pub(crate) despawn_unsupported_live_types: usize,
    pub(crate) despawn_respawn_timer_unsupported_types: usize,
    pub(crate) despawn_stale_index_entries: usize,
    pub(crate) despawn_remove_errors: usize,
    pub(crate) respawn_deleted_inactive_spawn_group: usize,
    pub(crate) respawn_deleted_live_object_blocker: usize,
    pub(crate) respawn_processed_pool_timers: usize,
    pub(crate) respawn_processed_unloaded_grid_respawns: usize,
    pub(crate) respawn_executed_loaded_grid_respawns: usize,
    pub(crate) respawn_legacy_creature_mirrors: usize,
    pub(crate) respawn_blocked_loaded_grid_respawn_loads: usize,
    pub(crate) respawn_blocked_loaded_grid_respawn_add_to_map: usize,
    pub(crate) respawn_pool_update_plans: usize,
    pub(crate) respawn_blocked_pool_plan_errors: usize,
    pub(crate) respawn_blocked_missing_spawn_data: usize,
    pub(crate) respawn_blocked_pool_runtime: usize,
    pub(crate) respawn_blocked_do_respawn_runtime: usize,
    pub(crate) respawn_blocked_linked_respawn_non_future: usize,
    pub(crate) respawn_blocked_unsupported_spawn_type: usize,
    pub(crate) respawn_db_delete_queued: usize,
    pub(crate) respawn_db_delete_executed: usize,
    pub(crate) respawn_db_delete_failed: usize,
    pub(crate) respawn_db_delete_skipped_non_world_map: usize,
    pub(crate) respawn_db_delete_skipped_instanceable_map: usize,
    pub(crate) respawn_db_delete_skipped_invalid_map_id: usize,
    pub(crate) respawn_db_deletes: Vec<RespawnDbDeleteLikeCpp>,
    pub(crate) respawn_db_save_queued: usize,
    pub(crate) respawn_db_save_executed: usize,
    pub(crate) respawn_db_save_failed: usize,
    pub(crate) respawn_db_save_skipped_non_world_map: usize,
    pub(crate) respawn_db_save_skipped_instanceable_map: usize,
    pub(crate) respawn_db_save_skipped_invalid_map_id: usize,
    pub(crate) respawn_db_saves: Vec<RespawnDbSaveLikeCpp>,
}
