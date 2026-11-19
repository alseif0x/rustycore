// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Caster, conversation, scene and respawn outcome types.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
    pub caster_guid: ObjectGuid,
    pub candidates: usize,
    pub removed: usize,
    pub missing_or_stale: usize,
    pub remove_errors: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationUpdateStatusLikeCpp {
    Updated,
    ExpiredRemoveQueued,
    MissingConversation,
    NotConversation,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationUpdateOutcomeLikeCpp {
    pub conversation_guid: ObjectGuid,
    pub elapsed_ms: u32,
    pub status: ConversationUpdateStatusLikeCpp,
    pub duration_before_ms: Option<i32>,
    pub duration_after_ms: Option<i32>,
    pub script_update_would_run: bool,
    pub world_update_would_run: bool,
    pub remove_list: Option<AddObjectToRemoveListOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ConversationsUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub expired_remove_queued: usize,
    pub missing_or_stale: usize,
    pub not_conversation: usize,
    pub not_in_world: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneObjectUpdateContextLikeCpp {
    pub creator_exists: bool,
    pub linked_aura_exists: bool,
}

impl Default for SceneObjectUpdateContextLikeCpp {
    fn default() -> Self {
        Self {
            creator_exists: true,
            linked_aura_exists: true,
        }
    }
}

impl SceneObjectUpdateContextLikeCpp {
    /// Conservative represented default for live `ManagedMap::update`: until real
    /// `ObjectAccessor::GetUnit` and Aura lookup by spell/cast id exist, do not
    /// delete map-owned SceneObjects merely because that runtime is absent.
    pub fn represented_default_for(scene_object: &SceneObject) -> Self {
        let _has_spell_cast = !scene_object.created_by_spell_cast().is_empty();
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneObjectUpdateStatusLikeCpp {
    Updated,
    RemoveQueued,
    MissingSceneObject,
    NotSceneObject,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneObjectUpdateOutcomeLikeCpp {
    pub scene_object_guid: ObjectGuid,
    pub elapsed_ms: u32,
    pub status: SceneObjectUpdateStatusLikeCpp,
    pub owner_guid: Option<ObjectGuid>,
    pub created_by_spell_cast: Option<ObjectGuid>,
    pub creator_exists: bool,
    pub linked_aura_exists: bool,
    pub world_update_would_run: bool,
    pub should_be_removed: bool,
    pub remove_list: Option<RemoveListOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SceneObjectsUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub remove_queued: usize,
    pub missing_or_stale: usize,
    pub not_scene_object: usize,
    pub not_in_world: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FarsightDynamicObjectCreateStatusLikeCpp {
    Created,
    MissingCasterPlayer,
    CasterNotInWorld,
    CasterWrongMap,
    InvalidDestination,
    MapIdNotRepresentableInGuid,
    SpellIdNotRepresentable,
    CastTimeNotRepresentable,
    GuidSequenceError(MapGuidSequenceErrorLikeCpp),
    DynamicObjectRecordError(ObjectAccessorError),
    AddToMapError,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FarsightDynamicObjectCreateOutcomeLikeCpp {
    pub status: FarsightDynamicObjectCreateStatusLikeCpp,
    pub caster_player_guid: ObjectGuid,
    pub dynamic_object_guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub caster_viewpoint: Option<DynamicObjectCasterViewpointOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RemoveAllObjectsInRemoveListOutcomeLikeCpp {
    pub switch_processed: usize,
    pub switch_executed: usize,
    pub switch_missing_or_stale: usize,
    pub switch_unsupported_kinds: usize,
    pub switch_permanent_world_objects: usize,
    pub switch_invalid_or_unloaded_grid: usize,
    pub processed: usize,
    pub removed: usize,
    pub missing_or_stale: usize,
    pub remove_errors: usize,
    pub unsupported_kinds: usize,
    pub creature_second_cleanup_count: usize,
    pub dynamic_object_remove_aura_cleanup_count: usize,
    pub dynamic_object_unbound_caster_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
    pub caster_guid: ObjectGuid,
    pub candidates: usize,
    pub removed: usize,
    pub missing_or_stale: usize,
    pub remove_errors: usize,
    pub dynamic_object_remove_aura_cleanup_count: usize,
    pub dynamic_object_unbound_caster_count: usize,
}

/// Bounded represented action for C++ `Map::AddFarSpellCallback` / `_farSpellCallbacks`.
///
/// C++ anchors:
/// - `Map.cpp:2514-2517` enqueues a heap-owned `FarSpellCallback`.
/// - `Map.cpp:2519-2530` drains FIFO callbacks at the start of `Map::DelayedUpdate`
///   and executes each callback before `RemoveAllObjectsInRemoveList()`.
///
/// Rust intentionally represents only closed map-owned actions. This is not a real
/// Spell/FarSpellCallback implementation: no arbitrary closures, Spell/Aura runtime,
/// caster lookup, ObjectAccessor, session fanout, packets, scripts, or AI callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedFarSpellCallbackActionLikeCpp {
    /// Records execution evidence only; useful for FIFO/order tests without mutation.
    RecordExecution,
    /// Represented map mutation: callback queues an object for same-tick remove-list
    /// drain by delegating to `Map::AddObjectToRemoveList` semantics.
    QueueObjectRemove { guid: ObjectGuid },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedFarSpellCallbackLikeCpp {
    pub id: u64,
    pub action: RepresentedFarSpellCallbackActionLikeCpp,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FarSpellCallbackDrainSummaryLikeCpp {
    pub queued_before: usize,
    pub processed: usize,
    pub record_only: usize,
    pub remove_queue_attempted: usize,
    pub remove_queued: usize,
    pub remove_missing_or_stale: usize,
    pub remove_duplicates: usize,
    pub unsupported_remove_kinds: usize,
    pub queued_after: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::map) struct SwitchGridContainersOutcomeLikeCpp {
    pub(in crate::map) executed: bool,
    pub(in crate::map) missing_or_stale: bool,
    pub(in crate::map) unsupported_kind: bool,
    pub(in crate::map) permanent_world_object: bool,
    pub(in crate::map) invalid_or_unloaded_grid: bool,
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
    pub loaded_grid_primary_records: Vec<LoadedGridObjectFacts>,
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
    pub loaded_grid_primary_records: Vec<LoadedGridObjectFacts>,
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
    pub loaded_grid_primary_records: Vec<LoadedGridObjectFacts>,
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
