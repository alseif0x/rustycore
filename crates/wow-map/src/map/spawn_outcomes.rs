use super::{
    MapObjectRecord, PoolMgrPlanErrorLikeCpp, PoolTypedSpawnPlanLikeCpp, RespawnInfoLikeCpp,
    SpawnGroupActiveChange, SpawnId, SpawnObjectType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnGroupConditionActionLikeCpp {
    Noop,
    Spawn { ignore_respawn: bool, force: bool },
    Despawn { delete_respawn_times: bool },
    SetInactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DespawnAllBySpawnIdOutcomeLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
    /// Number of live objects snapshotted from the by-spawn store and queued via
    /// `AddObjectToRemoveList`; physical deletion is deferred to
    /// `remove_all_objects_in_remove_list_like_cpp`.
    pub queued: usize,
    /// Legacy compatibility counter retained for callers from the pre-#419 seam.
    /// It is no longer incremented by `despawn_all_by_spawn_id_like_cpp`; use
    /// `queued` for C++ `Map::DespawnAll` parity and drain the map remove-list for
    /// physical removal.
    pub removed: usize,
    pub duplicates: usize,
    pub stale_index_entries: usize,
    pub remove_errors: usize,
    pub unsupported_live_despawn_type: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SpawnGroupDespawnOutcomeLikeCpp {
    pub group_id: u32,
    pub blocked_missing_group: usize,
    pub blocked_system_group: usize,
    pub metadata_entries: usize,
    pub respawn_timers_removed: usize,
    pub respawn_timers_missing: usize,
    pub respawn_timer_unsupported_types: usize,
    pub objects_removed: usize,
    pub stale_index_entries: usize,
    pub remove_errors: usize,
    pub unsupported_live_despawn_types: usize,
    pub applied_inactive_change: Option<SpawnGroupActiveChange>,
}

impl SpawnGroupDespawnOutcomeLikeCpp {
    pub const fn blocked_missing_group(group_id: u32) -> Self {
        Self {
            group_id,
            blocked_missing_group: 1,
            blocked_system_group: 0,
            metadata_entries: 0,
            respawn_timers_removed: 0,
            respawn_timers_missing: 0,
            respawn_timer_unsupported_types: 0,
            objects_removed: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_types: 0,
            applied_inactive_change: None,
        }
    }

    pub const fn blocked_system_group(group_id: u32) -> Self {
        Self {
            group_id,
            blocked_missing_group: 0,
            blocked_system_group: 1,
            metadata_entries: 0,
            respawn_timers_removed: 0,
            respawn_timers_missing: 0,
            respawn_timer_unsupported_types: 0,
            objects_removed: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_types: 0,
            applied_inactive_change: None,
        }
    }

    pub const fn executed(group_id: u32) -> Self {
        Self {
            group_id,
            blocked_missing_group: 0,
            blocked_system_group: 0,
            metadata_entries: 0,
            respawn_timers_removed: 0,
            respawn_timers_missing: 0,
            respawn_timer_unsupported_types: 0,
            objects_removed: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_types: 0,
            applied_inactive_change: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnGroupSpawnLoadPlanLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolSpawnActionLoadPlanLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
    pub respawn: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SpawnGroupSpawnOutcomeLikeCpp {
    pub group_id: u32,
    pub blocked_missing_group: usize,
    pub blocked_system_group: usize,
    pub metadata_entries: usize,
    pub stale_index_entries: usize,
    pub respawn_timers_removed: usize,
    pub respawn_timers_missing: usize,
    pub skipped_respawn_timer_active: usize,
    pub skipped_live_object_active: usize,
    /// Spawn metadata entries skipped at the C++ `GetRespawnMapForType(...) == nullptr`
    /// guard before timers, TypeHasData/live checks, difficulty, grid, or loader planning.
    pub skipped_no_respawn_map: usize,
    pub skipped_difficulty_mismatch: usize,
    pub skipped_unloaded_grid: usize,
    /// Loaded-grid Creature/GameObject `SpawnGroupSpawn` entries whose explicit
    /// caller-supplied DB/template loader returned typed records and whose
    /// primary record was accepted by map-owned `AddToMap`.
    pub executed_loaded_grid_spawns: usize,
    /// Loaded-grid Creature/GameObject `SpawnGroupSpawn` entries whose C++
    /// `LoadFromDB` attempt is represented by a caller loader returning `None`.
    /// Compatibility wrappers still also increment the legacy type-specific
    /// blocked counters below.
    pub blocked_loaded_grid_spawn_loads: usize,
    /// Loaded-grid Creature/GameObject `SpawnGroupSpawn` entries whose loader
    /// returned records, but the primary `AddToMap` insertion was rejected.
    pub blocked_loaded_grid_spawn_add_to_map: usize,
    pub blocked_loaded_grid_creature_loads: usize,
    pub blocked_loaded_grid_gameobject_loads: usize,
    pub unsupported_spawn_types: usize,
    pub load_plans: Vec<SpawnGroupSpawnLoadPlanLikeCpp>,
    pub loaded_grid_primary_records: Vec<MapObjectRecord>,
    pub applied_active_change: Option<SpawnGroupActiveChange>,
}

impl SpawnGroupSpawnOutcomeLikeCpp {
    pub fn blocked_missing_group(group_id: u32) -> Self {
        Self {
            group_id,
            blocked_missing_group: 1,
            ..Self::default()
        }
    }

    pub fn blocked_system_group(group_id: u32) -> Self {
        Self {
            group_id,
            blocked_system_group: 1,
            ..Self::default()
        }
    }

    pub fn executed(group_id: u32) -> Self {
        Self {
            group_id,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnGroupConditionUpdateOutcomeLikeCpp {
    pub group_id: u32,
    pub action: SpawnGroupConditionActionLikeCpp,
    pub applied_change: Option<SpawnGroupActiveChange>,
    pub despawn_outcome: Option<SpawnGroupDespawnOutcomeLikeCpp>,
    pub spawn_outcome: Option<SpawnGroupSpawnOutcomeLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedGridRespawnRecordsLikeCpp {
    pub pre_add_records: Vec<MapObjectRecord>,
    pub primary_record: MapObjectRecord,
}

impl LoadedGridRespawnRecordsLikeCpp {
    pub fn primary_only(primary_record: MapObjectRecord) -> Self {
        Self {
            pre_add_records: Vec::new(),
            primary_record,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LoadedGridAreaTriggerRecordsSummaryLikeCpp {
    pub grid_not_loaded: bool,
    pub metadata_entries: usize,
    pub skipped_already_loaded: usize,
    pub skipped_should_not_spawn: usize,
    pub stale_index_entries: usize,
    pub skipped_difficulty_mismatch: usize,
    pub load_record_missing: usize,
    pub pre_add_records_added: usize,
    pub loaded_grid_primary_records: Vec<MapObjectRecord>,
    pub add_to_map_errors: usize,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ProcessRespawnsSafeSideEffectsSummaryLikeCpp {
    pub deleted_inactive_spawn_group: usize,
    pub deleted_live_object_blocker: usize,
    pub rescheduled_linked_respawns: Vec<RespawnInfoLikeCpp>,
    pub processed_pool_timers: usize,
    /// C++ `DoRespawn` removes the timer before calling into `DoRespawn`; when
    /// the target grid is unloaded, `DoRespawn` returns immediately and grid
    /// load can create the object later because no respawn timer remains.
    pub processed_unloaded_grid_respawns: usize,
    pub pool_update_plans: Vec<PoolTypedSpawnPlanLikeCpp>,
    pub pool_objects_removed: usize,
    pub pool_respawn_timers_removed: usize,
    pub pool_respawn_timers_missing: usize,
    pub pool_stale_index_entries: usize,
    pub pool_remove_errors: usize,
    pub pool_spawn_actions_skipped_unloaded_grid: usize,
    pub pool_spawn_actions_blocked_loaded_grid: usize,
    pub pool_spawn_action_load_plans: Vec<PoolSpawnActionLoadPlanLikeCpp>,
    pub pool_spawn_actions_missing_spawn_data: usize,
    pub pool_unsupported_action_kind: usize,
    pub blocked_pool_plan_errors: Vec<PoolMgrPlanErrorLikeCpp>,
    pub blocked_missing_spawn_data: usize,
    /// Loaded-grid `DoRespawn` timers and pooled `Spawn1Object`/`ReSpawn1Object`
    /// actions whose caller-supplied typed `MapObjectRecord` was successfully
    /// loaded and inserted through `AddToMap`. This is only the map-owned
    /// execution seam; DB/template resolution stays with the caller-provided
    /// loader.
    pub executed_loaded_grid_respawns: usize,
    /// Loaded-grid `DoRespawn` timers that stay queued, plus pooled
    /// `Spawn1Object`/`ReSpawn1Object` loaded-grid actions that stay represented
    /// as blocked load-plan evidence, because the explicit caller loader did not
    /// return a typed DB-backed record.
    pub blocked_loaded_grid_respawn_loads: usize,
    /// Loaded-grid `DoRespawn` timers and pooled `Spawn1Object`/`ReSpawn1Object`
    /// actions whose loader returned a record, after which C++ has already
    /// popped/erased the timer or mutated pool state before `AddToMap`; the timer
    /// therefore stays removed and pool state is not reverted even when Rust
    /// `AddToMap` rejects it.
    pub blocked_loaded_grid_respawn_add_to_map: usize,
    pub loaded_grid_primary_records: Vec<MapObjectRecord>,
    /// Legacy compatibility counter for the pre-#390 seam where any pooled timer
    /// blocked `ProcessRespawns`. New pooled-timer planner errors are reported in
    /// `blocked_pool_plan_errors`; successful pooled timers increment
    /// `processed_pool_timers` and remove the map-owned respawn timer.
    pub blocked_pool_runtime: usize,
    pub blocked_do_respawn_runtime: usize,
    pub blocked_linked_respawn_non_future: usize,
    pub blocked_unsupported_spawn_type: usize,
}

pub type ProcessRespawnsDeleteOnlySummaryLikeCpp = ProcessRespawnsSafeSideEffectsSummaryLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRespawnLiveObjectGuardOutcomeLikeCpp {
    Allowed,
    AliveCreatureBlocksRespawn,
    GameObjectBlocksRespawn,
    MissingSpawnData,
    UnsupportedSpawnType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRespawnLinkedRespawnGuardOutcomeLikeCpp {
    Allowed,
    LinkedInfinite,
    LinkedSelfNeverRespawn,
    LinkedDelayed,
    UnsupportedSpawnType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRespawnCompositeOutcomeLikeCpp {
    Allowed,
    InactiveSpawnGroupDeletedTimer,
    AliveCreatureBlocksRespawn,
    GameObjectBlocksRespawn,
    LinkedInfinite,
    LinkedSelfNeverRespawn,
    LinkedDelayed,
    MissingSpawnData,
    UnsupportedSpawnType,
}

impl SpawnGroupConditionActionLikeCpp {
    pub const fn spawn_group_spawn_default() -> Self {
        Self::Spawn {
            ignore_respawn: false,
            force: false,
        }
    }

    pub const fn condition_failure_despawn() -> Self {
        Self::Despawn {
            delete_respawn_times: true,
        }
    }
}
