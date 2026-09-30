use super::{
    AddObjectToRemoveListOutcomeLikeCpp, AddRespawnInfoOutcomeLikeCpp, AddToMapError,
    AddToMapOutcome, CreatureRuntimePlan, EntityGameObjectUpdateOutcomeLikeCpp,
    GameObjectRemoveFromOwnerOutcomeLikeCpp, MapGuidSequenceErrorLikeCpp, ObjectAccessorError,
    ObjectGuid, PoolMgrPlanErrorLikeCpp, PoolTypedSpawnPlanLikeCpp, Position,
    ProcessRespawnsSafeSideEffectsSummaryLikeCpp, RemoveListOutcomeLikeCpp, RespawnInfoLikeCpp,
    SceneObject, SpawnId, TransportUpdateLikeCpp, UnitValuesUpdate,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapUpdateMetricsSummaryLikeCpp {
    pub creature_count: usize,
    pub gameobject_count: usize,
    pub map_id: u32,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GridStatesUpdateSummaryLikeCpp {
    pub diff_ms: u32,
    pub visited: usize,
    pub updated: usize,
    pub unloaded: usize,
    pub missing_after_snapshot: usize,
    pub skipped_invalid: usize,
    pub active_to_idle: usize,
    pub idle_to_removal: usize,
    pub removal_unloaded: usize,
    pub removal_deferred_or_reset: usize,
    pub skipped_battleground_or_arena: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
    pub spawn_id: SpawnId,
    pub indexed_guids: usize,
    pub live_creatures_mutated: usize,
    pub stale_index_or_wrong_kind: usize,
    pub equipment_changed: usize,
    pub display_changed: usize,
    pub model_validation_unavailable: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameEventNpcFlagValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub values_update: UnitValuesUpdate,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameEventNpcFlagLiveOutcomeLikeCpp {
    pub spawn_id: SpawnId,
    pub indexed_guids: usize,
    pub live_creatures_mutated: usize,
    pub stale_index_or_wrong_kind: usize,
    pub npc_flags_low_applied: usize,
    pub npc_flags2_applied: usize,
    pub values_updates: Vec<GameEventNpcFlagValuesUpdateLikeCpp>,
}

/// Represented map-owned evidence for C++ `GameEventMgr::RunSmartAIScripts`.
///
/// Anchor: `GameEventMgr.cpp:1618-1655`. The C++ worker visits every map and
/// dispatches only exact in-world Creature/GameObject AI callbacks. Rust does
/// not model SmartAI/`ProcessEventsFor` here; this summary only counts exact
/// typed `Map::entity_world` candidates and marks dispatch as unrepresented.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GameEventSmartAiScriptCandidateSummaryLikeCpp {
    pub maps_visited: usize,
    pub in_world_creature_candidates: usize,
    pub in_world_gameobject_candidates: usize,
    pub creature_ai_enabled_unrepresented: usize,
    pub script_dispatch_unrepresented: usize,
}

/// Represented result for C++ `DynamicMapTree::update(t_diff)`.
///
/// Anchors: `Map.cpp:666-668`, `DynamicTree.cpp:34-38,66-101,115-138`.
/// This exposes only the map-owned model-key registration, timer and unbalanced
/// seam. It does not claim real `GameObjectModel`, RegularGrid/BIH balance,
/// LOS/intersection/height, AddToWorld/RemoveFromWorld registration,
/// ObjectAccessor/session/fanout, DB, scripts, AI, or collision runtime parity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DynamicMapTreeUpdateSummaryLikeCpp {
    pub diff_ms: u32,
    pub empty: bool,
    pub timer_before_ms: u32,
    pub timer_after_ms: u32,
    pub timer_passed: bool,
    pub timer_reset_to_ms: Option<u32>,
    pub unbalanced_before: u32,
    pub balanced: bool,
    pub unbalanced_after: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PersonalPhaseTrackerUpdateSummaryLikeCpp {
    pub expired_objects: usize,
    pub remove_queued: usize,
    pub missing_or_stale: usize,
    pub unsupported_kinds: usize,
    pub duplicate_queued: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedZoneDefaultWeatherLikeCpp {
    update_call_diffs_ms: Vec<u32>,
    next_update_returns_alive: bool,
}

impl Default for RepresentedZoneDefaultWeatherLikeCpp {
    fn default() -> Self {
        Self {
            update_call_diffs_ms: Vec::new(),
            next_update_returns_alive: true,
        }
    }
}

impl RepresentedZoneDefaultWeatherLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_call_diffs_ms(&self) -> &[u32] {
        &self.update_call_diffs_ms
    }

    pub const fn next_update_returns_alive(&self) -> bool {
        self.next_update_returns_alive
    }

    pub fn set_next_update_returns_alive(&mut self, alive: bool) {
        self.next_update_returns_alive = alive;
    }

    fn update_like_cpp(&mut self, diff_ms: u32) -> bool {
        self.update_call_diffs_ms.push(diff_ms);
        let alive = self.next_update_returns_alive;
        self.next_update_returns_alive = true;
        alive
    }
}

/// Represented durable subset of C++ `ZoneDynamicInfo` (`Map.cpp:72-73`).
///
/// `DefaultWeather` is map-owned and optional like the C++ unique pointer. This
/// does not represent WeatherMgr creation, DB weather data, player counts,
/// packet fanout, script callbacks, regeneration, or zone messaging.
#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedZoneDynamicInfoLikeCpp {
    pub default_weather: Option<RepresentedZoneDefaultWeatherLikeCpp>,
    pub weather_id: u32,
    pub intensity: f32,
}

impl Default for RepresentedZoneDynamicInfoLikeCpp {
    fn default() -> Self {
        Self {
            default_weather: None,
            weather_id: 0,
            intensity: 0.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct WeatherUpdateSummaryLikeCpp {
    pub interval_ms: u32,
    pub timer_current_before: u32,
    pub timer_current_after_update: u32,
    pub timer_current_after_reset: u32,
    pub timer_passed: bool,
    pub zones_seen: usize,
    pub zones_without_default_weather: usize,
    pub default_weather_updated: usize,
    pub default_weather_removed: usize,
    pub weather_update_call_diff_ms: Option<u32>,
    pub script_update_regeneration_fanout_not_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedScriptScheduleActionLikeCpp {
    pub source_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
    /// Opaque represented command/script identifier only. This is not a real
    /// `ScriptInfo` pointer and must not trigger command side effects.
    pub command_id: u32,
    pub due_time_secs: i64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScriptScheduleProcessSummaryLikeCpp {
    pub queued_before: usize,
    pub processed: usize,
    pub remaining: usize,
    pub represented_decrease_count: usize,
    pub lock_entered: bool,
    pub empty_noop: bool,
    pub processed_actions: Vec<RepresentedScriptScheduleActionLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptScheduleStartOutcomeLikeCpp {
    pub scheduled: RepresentedScriptScheduleActionLikeCpp,
    pub represented_increase_count: usize,
    pub remaining_after_schedule: usize,
    pub immediate_process: Option<ScriptScheduleProcessSummaryLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddObjectToSwitchListStatusLikeCpp {
    Queued,
    CancelledOppositeToggle,
    DuplicateSameDirectionAbort,
    MissingOrStale,
    IgnoredNonUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddObjectToSwitchListOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub on: bool,
    pub status: AddObjectToSwitchListStatusLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetWorldObjectStatusLikeCpp {
    MissingOrStale,
    NotInWorld,
    Delegated(AddObjectToSwitchListStatusLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetWorldObjectOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub on: bool,
    pub status: SetWorldObjectStatusLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSetViewpointStatusLikeCpp {
    Applied,
    Removed,
    MissingPlayer,
    MissingTarget,
    TargetNotUnit,
    TargetNotDynamicObject,
    TargetIsVehicleBase,
    AlreadyHasViewpoint,
    ViewpointMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSetViewpointOutcomeLikeCpp {
    pub player_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
    pub apply: bool,
    pub status: PlayerSetViewpointStatusLikeCpp,
    pub set_world_object: Option<SetWorldObjectOutcomeLikeCpp>,
    pub update_visibility_requested: bool,
    pub set_seer_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicObjectCasterViewpointStatusLikeCpp {
    CasterPlayerResolved,
    MissingDynamicObject,
    MissingCaster,
    CasterNotPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicObjectCasterViewpointOutcomeLikeCpp {
    pub player_guid: ObjectGuid,
    pub dynamic_object_guid: ObjectGuid,
    pub apply: bool,
    pub status: DynamicObjectCasterViewpointStatusLikeCpp,
    pub player_set_viewpoint: PlayerSetViewpointOutcomeLikeCpp,
    pub dynamic_object_viewpoint_toggled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp {
    RemovedUnitViewpoint,
    RemovedDynamicObjectViewpoint,
    RemovedPlayerViewpoint,
    MissingTarget,
    TargetNotInWorld,
    TargetNotSeer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: ObjectGuid,
    pub status: PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp,
    pub player_set_viewpoint: Option<PlayerSetViewpointOutcomeLikeCpp>,
    pub dynamic_object_caster_viewpoint: Option<DynamicObjectCasterViewpointOutcomeLikeCpp>,
    pub update_visibility_requested: bool,
    pub set_seer_requested: bool,
    pub object_accessor_fanout_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicObjectUpdateStatusLikeCpp {
    Updated,
    ExpiredRemoveQueued,
    MissingDynamicObject,
    NotDynamicObject,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicObjectUpdateOutcomeLikeCpp {
    pub dynamic_object_guid: ObjectGuid,
    pub elapsed_ms: u32,
    pub status: DynamicObjectUpdateStatusLikeCpp,
    pub duration_before_ms: Option<i32>,
    pub duration_after_ms: Option<i32>,
    pub aura_update_owner_calls_before: Option<u32>,
    pub aura_update_owner_calls_after: Option<u32>,
    pub script_update_would_run: bool,
    pub remove_list: Option<AddObjectToRemoveListOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DynamicObjectsUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub expired_remove_queued: usize,
    pub missing_or_stale: usize,
    pub not_dynamic_object: usize,
    pub not_in_world: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectUpdateStatusLikeCpp {
    Updated,
    DespawnRemoveQueued,
    DespawnPoolUpdated,
    MissingGameObject,
    NotGameObject,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectUpdateOutcomeLikeCpp {
    pub game_object_guid: ObjectGuid,
    pub diff_ms: u32,
    pub status: GameObjectUpdateStatusLikeCpp,
    pub despawn_delay_before_ms: Option<u32>,
    pub despawn_delay_after_ms: Option<u32>,
    pub despawn_respawn_time_secs: Option<u32>,
    pub world_update_would_run: bool,
    pub ai_update_not_represented: bool,
    pub go_type_impl_update_not_represented: bool,
    pub despawn_or_unsummon_requested: bool,
    pub entity_update: Option<EntityGameObjectUpdateOutcomeLikeCpp>,
    pub remove_list: Option<AddObjectToRemoveListOutcomeLikeCpp>,
    pub linked_trap_guid: Option<ObjectGuid>,
    pub linked_trap_removed: bool,
    pub linked_trap_remove_queued: bool,
    pub linked_trap_missing_or_self: bool,
    pub loot_cleared: bool,
    pub goober_spell_cast_spell_id: Option<u32>,
    pub goober_spell_casts_represented: usize,
    pub goober_users_cleared: bool,
    pub goober_state_reset: bool,
    pub goober_nodespawn_return: bool,
    pub non_consumed_chest_or_goober_return: bool,
    pub non_consumed_restock_armed: bool,
    pub non_consumed_set_ready: bool,
    pub non_consumed_update_visibility_represented: bool,
    pub non_consumed_update_dynamic_flags_represented: bool,
    pub non_consumed_source_missing: bool,
    pub summoned_expired_delete: bool,
    pub summoned_expired_respawn_time_zeroed: bool,
    pub summoned_expired_despawn_represented: bool,
    pub summoned_expired_go_state_ready: bool,
    pub new_flag_drop_owner_in_base_command_represented: bool,
    pub new_flag_drop_owner_missing_or_empty: bool,
    pub new_flag_drop_owner_wrong_kind: bool,
    pub new_flag_drop_owner_not_new_flag: bool,
    pub generic_not_ready: bool,
    pub generic_capture_point_removed_represented: bool,
    pub generic_visual_despawn_represented: bool,
    pub generic_flags_restored_represented: bool,
    pub generic_zero_respawn_delay_return: bool,
    pub generic_despawn_at_action_source_missing: bool,
    pub generic_respawn_scheduled_time: Option<i64>,
    pub generic_spawned_by_default_branch: bool,
    pub generic_temporary_respawn_zeroed: bool,
    pub generic_respawn_timer_add: Option<AddRespawnInfoOutcomeLikeCpp>,
    pub generic_respawn_save_missing_spawn_id: bool,
    pub generic_respawn_save_missing_gameobject_data: bool,
    pub generic_respawn_compatibility_db_only_represented: bool,
    pub generic_visibility_on_destroy_represented: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDeleteOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub remove_from_owner: Option<GameObjectRemoveFromOwnerOutcomeLikeCpp>,
    pub capture_point_packet_represented: bool,
    pub despawn_packet_represented: bool,
    pub go_state_ready: bool,
    pub flags_restored: bool,
    pub pool_update_represented: bool,
    pub pool_update_plan: Option<PoolTypedSpawnPlanLikeCpp>,
    pub pool_update_error: Option<PoolMgrPlanErrorLikeCpp>,
    pub pool_update_summary: Option<ProcessRespawnsSafeSideEffectsSummaryLikeCpp>,
    pub remove_list: Option<AddObjectToRemoveListOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectVisibilityOnDestroyGuidsLikeCpp {
    guids: Vec<ObjectGuid>,
}

impl GameObjectVisibilityOnDestroyGuidsLikeCpp {
    pub fn push(&mut self, guid: ObjectGuid) {
        self.guids.push(guid);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectGuid> {
        self.guids.iter()
    }

    pub fn as_slice(&self) -> &[ObjectGuid] {
        self.guids.as_slice()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectVisualDespawnGuidsLikeCpp {
    guids: Vec<ObjectGuid>,
}

impl GameObjectVisualDespawnGuidsLikeCpp {
    pub fn push(&mut self, guid: ObjectGuid) {
        self.guids.push(guid);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectGuid> {
        self.guids.iter()
    }

    pub fn as_slice(&self) -> &[ObjectGuid] {
        self.guids.as_slice()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectCapturePointRemovedGuidsLikeCpp {
    guids: Vec<ObjectGuid>,
}

impl GameObjectCapturePointRemovedGuidsLikeCpp {
    pub fn push(&mut self, guid: ObjectGuid) {
        self.guids.push(guid);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ObjectGuid> {
        self.guids.iter()
    }

    pub fn as_slice(&self) -> &[ObjectGuid] {
        self.guids.as_slice()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectsUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub despawn_remove_queued: usize,
    pub despawn_pool_updated: usize,
    pub missing_or_stale: usize,
    pub not_game_object: usize,
    pub not_in_world: usize,
    pub linked_traps_removed: usize,
    pub linked_traps_remove_queued: usize,
    pub loot_cleared: usize,
    pub goober_spell_casts_represented: usize,
    pub goober_users_cleared: usize,
    pub goober_state_reset: usize,
    pub goober_nodespawn_returns: usize,
    pub non_consumed_chest_or_goober_returns: usize,
    pub non_consumed_restock_armed: usize,
    pub non_consumed_set_ready: usize,
    pub non_consumed_update_visibility_represented: usize,
    pub non_consumed_update_dynamic_flags_represented: usize,
    pub non_consumed_source_missing: usize,
    pub summoned_expired_deletes: usize,
    pub summoned_expired_respawn_time_zeroed: usize,
    pub summoned_expired_despawn_represented: usize,
    pub summoned_expired_go_state_ready: usize,
    pub new_flag_drop_owner_in_base_commands_represented: usize,
    pub new_flag_drop_owner_missing_or_empty: usize,
    pub new_flag_drop_owner_wrong_kind: usize,
    pub new_flag_drop_owner_not_new_flag: usize,
    pub generic_not_ready: usize,
    pub generic_capture_point_removed_represented: usize,
    pub generic_capture_point_removed_guids: GameObjectCapturePointRemovedGuidsLikeCpp,
    pub generic_visual_despawn_represented: usize,
    pub generic_visual_despawn_guids: GameObjectVisualDespawnGuidsLikeCpp,
    pub generic_flags_restored_represented: usize,
    pub generic_zero_respawn_delay_returns: usize,
    pub generic_despawn_at_action_source_missing: usize,
    pub generic_respawn_scheduled: usize,
    pub generic_spawned_by_default_branches: usize,
    pub generic_temporary_respawn_zeroed: usize,
    pub generic_respawn_timer_added: usize,
    pub generic_respawn_save_missing_spawn_id: usize,
    pub generic_respawn_save_missing_gameobject_data: usize,
    pub generic_respawn_compatibility_db_only_represented: usize,
    /// C++ `GameObject::SaveRespawnTime` DB side effects produced this update.
    /// Compatibility mode writes DB-only; non-compat mode also owns a map timer.
    pub respawn_db_saves: Vec<RespawnInfoLikeCpp>,
    pub generic_visibility_on_destroy_represented: usize,
    pub generic_visibility_on_destroy_guids: GameObjectVisibilityOnDestroyGuidsLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportUpdateStatusLikeCpp {
    Updated,
    UnsupportedNoPeriod,
    MissingTransport,
    NotTransport,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportUpdateOutcomeLikeCpp {
    pub transport_guid: ObjectGuid,
    pub diff_ms: u32,
    pub now_ms: u64,
    pub current_map_id: u32,
    pub status: TransportUpdateStatusLikeCpp,
    pub period_ms: Option<u32>,
    pub path_progress_before_ms: Option<u32>,
    pub path_progress_after_ms: Option<u32>,
    pub timer_ms: Option<u32>,
    pub expected_map_matches_current_map: bool,
    pub position_update_due: bool,
    pub position_update_represented: bool,
    pub just_stopped: bool,
    pub entity_update: Option<TransportUpdateLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TransportsUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub unsupported_no_period: usize,
    pub missing_or_stale: usize,
    pub not_transport: usize,
    pub not_in_world: usize,
    pub position_updates_represented: usize,
    pub just_stopped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureUpdateStatusLikeCpp {
    Updated,
    MissingCreature,
    NotCreature,
    NotInWorld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureUpdateOutcomeLikeCpp {
    pub creature_guid: ObjectGuid,
    pub diff_ms: u32,
    pub now_secs: i64,
    pub status: CreatureUpdateStatusLikeCpp,
    pub plan: Option<CreatureRuntimePlan>,
    pub actions_recorded: usize,
}

/// Owned projection of the transform/vitals fields read most often by systems
/// outside the canonical entity owner.
///
/// C++ resolves the live `Creature*` through `Map::_objectsStore`
/// (`Map.cpp:3444-3447`) and reads position, combat reach, and Unit health
/// directly (`Position.h:77-84`, `Unit.h:681,757-758`). Rust returns a value
/// snapshot so no storage borrow, `MapObjectRecord`, or future ECS guard can
/// escape [`EntityWorld`]. Absence remains `None`; this projection never
/// fabricates a default creature or zero-valued vitals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureTransformVitalsSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub map_id: u32,
    pub instance_id: u32,
    pub position: Position,
    pub combat_reach: f32,
    pub health: u64,
    pub max_health: u64,
    pub is_alive: bool,
    pub is_in_world: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CreatureUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub skipped_missing: usize,
    pub skipped_non_creature: usize,
    pub skipped_not_in_world: usize,
    pub actions_recorded: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaTriggerUpdateStatusLikeCpp {
    Updated,
    ExpiredRemoveQueued,
    MissingAreaTrigger,
    NotAreaTrigger,
    NotInWorld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggerUpdateOutcomeLikeCpp {
    pub area_trigger_guid: ObjectGuid,
    pub elapsed_ms: u32,
    pub status: AreaTriggerUpdateStatusLikeCpp,
    pub duration_before_ms: Option<i32>,
    pub duration_after_ms: Option<i32>,
    pub time_since_created_before_ms: Option<u32>,
    pub time_since_created_after_ms: Option<u32>,
    pub non_static_movement_would_run: bool,
    pub ai_update_would_run: bool,
    pub target_list_update_would_run: bool,
    pub remove_list: Option<AddObjectToRemoveListOutcomeLikeCpp>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AreaTriggersUpdateSummaryLikeCpp {
    pub visited: usize,
    pub updated: usize,
    pub expired_remove_queued: usize,
    pub missing_or_stale: usize,
    pub not_area_trigger: usize,
    pub not_in_world: usize,
}

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
