// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map storage, update phases and object lifecycle.
//!
//! Issue #225 split the former 15,250-line `map.rs` into private runtime
//! modules. The Map owner, the two documented runtime models and every phase
//! and bridge are unchanged; this module remains the single Map state owner
//! and exposes the stable `map::*` paths for its private responsibility modules.

mod actor_access;
mod creature_loot;
mod actor_transport;
mod loaded_grid_admission;
pub use actor_transport::{CreatureActorTransportError, CreatureActorTransportSummary};
mod combat;
mod construction;
mod creature_snapshot;
mod creature_visibility;
pub use creature_visibility::{
    CreatureAuraSlotFacts, CreatureCreateFacts, CreatureInitialAuraFacts,
    CreatureMessageSourceFacts, CreatureVisibilityCandidate,
};
mod creature_melee;
pub use self::creature_melee::{
    CreatureMeleeCatalogsLikeCpp, CreatureMeleeReadiness, creature_melee_readiness,
    is_creature_melee_los_clear_like_cpp,
    CreatureDamageThreatOutcomeLikeCpp, CreatureMeleeVictimSyncIdentityLikeCpp,
    CreatureMeleeVictimSyncStateLikeCpp,
    PendingCreatureSwingLikeCpp, CreatureVictimCompatibilitySyncLikeCpp,
    MeleeThreatSpellFacts, ShareAuraIdentityLikeCpp, ShareAuraSnapshotLikeCpp,
    MeleeEffect, MeleePresentation, CreatureMeleePlayerHit,
    CreatureMeleeSwingOutcome, MeleeAbsorbConsumption,
};
mod entity_world;
mod game_object;
mod gameobject_summoning;
mod grid_helpers;
mod grid_host;
mod map_update_plans;
mod move_list;
mod object_insertion;
mod object_entry;
mod object_operation_outcomes;
mod object_record_support;
mod object_views;
mod object_removal;
mod object_update_selection;
mod other_object_updates;
mod phase_outcomes;
mod pool_data;
mod relocation;
mod relocation_plans;
mod respawn;
pub use respawn::prefix::{LegacyCreatureRespawnPrefix, RemovedCreatureCorpse};
pub(crate) use respawn::prefix::prepare_legacy_creature_respawns;
mod respawn_scaling;
mod runtime;
mod scripts_weather;
mod send_object_updates;
mod spawn_groups;
mod spawn_outcomes;
mod storage;
mod summon_position;
mod update;
mod visibility;
mod viewpoint;

use crate::map_rules::remove_spawn_id_index_entry_like_cpp;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use rand::{Rng, SeedableRng, rngs::StdRng};

use self::entity_world::EntityWorld;
use self::object_entry::ObjectEntry;
pub(crate) use self::object_entry::CreatureActorWitness;
pub(crate) use self::actor_access::{CreatureActorAdmission, CreatureActorAdmissionError};
pub use self::actor_access::{FreshCreatureActorAdmission, FreshCreatureActorAdmissionError};
pub use self::loaded_grid_admission::{
    LoadedGridRespawnOutcome,
    LoadedGridAttemptPlan, LoadedGridPoolOutcome,
    LoadedGridConditionOutcome, LoadedGridSpawnAttempt,
    LoadedGridSpawnAttemptResult, LoadedGridSpawnOutcome,
    LoadedGridAdmission, LoadedGridMaterialization, LoadedGridPrimaryAdmission,
};
pub use self::object_entry::OwnedMapObject;
pub use self::creature_snapshot::CreatureSnapshotReplaceError;
use self::object_views::{ObjectMut, ObjectRef};
pub use self::grid_helpers::{
    cell_from_grid_center, cell_from_world, is_grid_id_loaded, total_cell_count,
};
use self::grid_helpers::{
    active_cells_near_grid, checked_grid_index, grid_index, terrain_grid_coords,
};
use self::object_record_support::{
    cleanup_map_object_record_before_delete_like_cpp, detach_typed_loot_authority_like_cpp,
    insert_object_guid_in_cell_like_cpp, is_active_object_like_cpp,
    map_record_is_world_object_like_cpp, remove_from_map_in_world_eligible_type_like_cpp,
    remove_list_grid_kind_like_cpp, remove_object_guid_from_cell_like_cpp,
    set_record_temp_world_object_like_cpp, switch_list_unit_kind_like_cpp,
    typed_loot_authorities_share_storage_like_cpp,
};
pub use self::construction::{
    GridLifecycle, MapWorldObjectEnvironment, NoopGridLifecycle, NoopTerrainGridLoader,
    TerrainGridLoader,
};
pub use self::map_update_plans::{
    MapUpdatePlayerSources, MapUpdateVisitPlan, NearbyCellGuids, NearbyCellVisitCenter,
    NearbyCellVisitPlan, ObjectUpdatePlan, ProcessRelocationNotifiesOutcome,
    RelocationNotifyProcessPlan, ResetNotifyFlagsOutcome,
};
pub use self::move_list::{
    AddObjectToMoveListOutcomeLikeCpp, MapObjectCellMoveState, MapObjectCellMoveStateLikeCpp,
    MapObjectMoveListEntry, MapObjectMoveListFamilyLikeCpp, MapObjectMoveListPlan,
    MoveListDrainSummaryLikeCpp, PendingCellMoveLikeCpp, RemoveObjectFromMoveListOutcomeLikeCpp,
};
pub use self::relocation_plans::{
    AIRelocationPlan, CreatureDelayedRelocationVisibilityPlan, CreatureRelocationVisibilityPlan,
    DelayedCreatureRelocationContext, DelayedPlayerRelocationContext,
    DelayedUnitRelocationCellPlan, DelayedUnitRelocationForCellsPlan, DelayedUnitRelocationPlan,
    DelayedUnitRelocationVisibilityPlans, PlayerDelayedRelocationVisibilityPlan,
    PlayerRelocationVisibilityPlan,
};
pub use self::runtime::{
    MapCommandKindLikeCpp, MapCommandLikeCpp, MapCommandOutcomeLikeCpp, MapCommandStatusLikeCpp,
};
pub(crate) use self::runtime::{
    MapRuntime, MapRuntimePlayerAttachErrorLikeCpp, MapRuntimePlayerDetachErrorLikeCpp,
    MapRuntimePlayerRelocationErrorLikeCpp,
};
pub use self::object_operation_outcomes::{
    ActiveNonPlayerMutationOutcomeLikeCpp, ActiveNonPlayerMutationStatusLikeCpp,
    ActiveNonPlayerUnloadLockOutcomeLikeCpp, AddObjectToRemoveListOutcomeLikeCpp,
    AddToActiveOutcomeLikeCpp, AddToMapError, AddToMapOutcome,
    AddToMapPostAddToWorldOutcomeLikeCpp, CreatureRemoveFormationOutcomeLikeCpp,
    CreatureZoneScriptCreateOutcomeLikeCpp, CreatureZoneScriptRemoveOutcomeLikeCpp,
    DynamicMapTreeModelMutationOutcomeLikeCpp, DynamicMapTreeModelMutationStatusLikeCpp,
    DynamicObjectRemoveCleanupOutcomeLikeCpp, GameObjectAddToOwnerOutcomeLikeCpp,
    GameObjectAddToOwnerSlotOutcomeLikeCpp, GameObjectCollisionEnableOutcomeLikeCpp,
    GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp, GameObjectRemoveFromOwnerOutcomeLikeCpp,
    GameObjectRemoveLinkedTrapOutcomeLikeCpp, GameObjectSetDisplayIdOutcomeLikeCpp,
    GameObjectSetDisplayIdStatusLikeCpp, GameObjectSetGoStateOutcomeLikeCpp,
    GameObjectSetGoStateStatusLikeCpp, GameObjectSetLootStateOutcomeLikeCpp,
    GameObjectSetLootStateStatusLikeCpp, GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp,
    GameObjectSummonObjectForOwnerSlotStatusLikeCpp, GameObjectUpdateModelOutcomeLikeCpp,
    GameObjectUpdateModelStatusLikeCpp, GameObjectZoneScriptCreateOutcomeLikeCpp,
    GameObjectZoneScriptRemoveOutcomeLikeCpp, MapObjectRelocationError,
    MapObjectRelocationOutcome, MapObjectStoreError, RemoveFromActiveOutcomeLikeCpp,
    RemoveFromMapError, RemoveFromMapOutcome,
    RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp, RemoveListOutcomeLikeCpp,
    RepresentedGameObjectModelKeyLikeCpp, SpellEffectSummonObjectWildOutcomeLikeCpp,
    SpellEffectSummonObjectWildStatusLikeCpp, UnitRemoveGameObjectsBySpellOutcomeLikeCpp,
    WorldObjectSummonGameObjectOutcomeLikeCpp, WorldObjectSummonGameObjectStatusLikeCpp,
};
pub use self::phase_outcomes::{
    AddObjectToSwitchListOutcomeLikeCpp, AddObjectToSwitchListStatusLikeCpp,
    AreaTriggerUpdateOutcomeLikeCpp, AreaTriggerUpdateStatusLikeCpp,
    AreaTriggersUpdateSummaryLikeCpp, ConversationUpdateOutcomeLikeCpp,
    ConversationUpdateStatusLikeCpp, ConversationsUpdateSummaryLikeCpp,
    CreatureTransformVitalsSnapshotLikeCpp, CreatureUpdateOutcomeLikeCpp,
    CreatureUpdateStatusLikeCpp, CreatureUpdateSummaryLikeCpp,
    DynamicMapTreeUpdateSummaryLikeCpp, DynamicObjectCasterViewpointOutcomeLikeCpp,
    DynamicObjectCasterViewpointStatusLikeCpp, DynamicObjectUpdateOutcomeLikeCpp,
    DynamicObjectUpdateStatusLikeCpp, DynamicObjectsUpdateSummaryLikeCpp,
    FarSpellCallbackDrainSummaryLikeCpp, FarsightDynamicObjectCreateOutcomeLikeCpp,
    FarsightDynamicObjectCreateStatusLikeCpp, GameEventChangeEquipOrModelLiveOutcomeLikeCpp,
    GameEventNpcFlagLiveOutcomeLikeCpp, GameEventNpcFlagValuesUpdateLikeCpp,
    GameEventSmartAiScriptCandidateSummaryLikeCpp, GameObjectCapturePointRemovedGuidsLikeCpp,
    GameObjectDeleteOutcomeLikeCpp, GameObjectUpdateOutcomeLikeCpp,
    GameObjectUpdateStatusLikeCpp, GameObjectVisibilityOnDestroyGuidsLikeCpp,
    GameObjectVisualDespawnGuidsLikeCpp, GameObjectsUpdateSummaryLikeCpp,
    GridStatesUpdateSummaryLikeCpp, MapUpdateMetricsSummaryLikeCpp,
    PersonalPhaseTrackerUpdateSummaryLikeCpp,
    PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp,
    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp, PlayerSetViewpointOutcomeLikeCpp,
    PlayerSetViewpointStatusLikeCpp, RemoveAllAreaTriggersForCasterOutcomeLikeCpp,
    RemoveAllDynamicObjectsForCasterOutcomeLikeCpp,
    RemoveAllObjectsInRemoveListOutcomeLikeCpp, RepresentedFarSpellCallbackActionLikeCpp,
    RepresentedFarSpellCallbackLikeCpp, RepresentedScriptScheduleActionLikeCpp,
    RepresentedZoneDefaultWeatherLikeCpp, RepresentedZoneDynamicInfoLikeCpp,
    SceneObjectUpdateContextLikeCpp, SceneObjectUpdateOutcomeLikeCpp,
    SceneObjectUpdateStatusLikeCpp, SceneObjectsUpdateSummaryLikeCpp,
    ScriptScheduleProcessSummaryLikeCpp, ScriptScheduleStartOutcomeLikeCpp,
    SetWorldObjectOutcomeLikeCpp, SetWorldObjectStatusLikeCpp,
    TransportUpdateOutcomeLikeCpp, TransportUpdateStatusLikeCpp,
    TransportsUpdateSummaryLikeCpp, WeatherUpdateSummaryLikeCpp,
};
pub use self::spawn_outcomes::{
    CheckRespawnCompositeOutcomeLikeCpp, CheckRespawnLinkedRespawnGuardOutcomeLikeCpp,
    CheckRespawnLiveObjectGuardOutcomeLikeCpp, DespawnAllBySpawnIdOutcomeLikeCpp,
    LoadedGridAreaTriggerRecordsSummaryLikeCpp, LoadedGridRespawnRecordsLikeCpp,
    PoolSpawnActionLoadPlanLikeCpp, ProcessRespawnsDeleteOnlySummaryLikeCpp,
    ProcessRespawnsSafeSideEffectsSummaryLikeCpp, SpawnGroupConditionActionLikeCpp,
    SpawnGroupConditionUpdateOutcomeLikeCpp, SpawnGroupDespawnOutcomeLikeCpp,
    SpawnGroupSpawnLoadPlanLikeCpp, SpawnGroupSpawnOutcomeLikeCpp,
};
pub use self::pool_data::{SpawnedPoolDataErrorLikeCpp, SpawnedPoolDataLikeCpp};
pub use self::respawn_scaling::{
    DynamicRespawnScalingConfig, DynamicRespawnScalingContext,
    DynamicRespawnScalingNoopReason, DynamicRespawnScalingOutcome,
    apply_dynamic_mode_respawn_scaling_like_cpp,
};
pub use self::send_object_updates::{
    RepresentedAreaTriggerValuesUpdateLikeCpp, RepresentedConversationValuesUpdateLikeCpp,
    RepresentedCorpseValuesUpdateLikeCpp, RepresentedDynamicObjectValuesUpdateLikeCpp,
    RepresentedGameObjectValuesUpdateLikeCpp, RepresentedPlayerValuesUpdateLikeCpp,
    RepresentedSceneObjectValuesUpdateLikeCpp, RepresentedUnitValuesUpdateLikeCpp,
    SendObjectUpdatesSummaryLikeCpp,
};
pub use self::summon_position::{
    spell_effect_summon_object_wild_position_like_cpp,
    world_object_summon_gameobject_position_from_coords_like_cpp,
    SpellEffectSummonObjectWildPositionOutcomeLikeCpp,
    WorldObjectSummonGameObjectPositionOutcomeLikeCpp,
};
use crate::cell::{Cell, GridObjectGuids, WorldObjectGuids, calculate_cell_area_like_cpp};
use crate::coords::{
    CellCoord, GridCoord, MAX_NUMBER_OF_CELLS, MAX_NUMBER_OF_GRIDS, SIZE_OF_GRID_CELL,
    TOTAL_NUMBER_OF_CELLS_PER_MAP, compute_cell_coord, compute_grid_coord, is_valid_map_coord_2d,
};
use crate::grid::{GridStateKind, MapGridHost, NGrid, update_grid_state};
use crate::grid_unload::{
    GridObjectKind, GridUnloadAction, GridUnloadApplyOutcome, GridUnloadEntityStore,
    apply_grid_unload_actions,
};
use crate::object_grid_loader::{GridSpawnLoadFilter, ObjectGridLoader};
use crate::personal_phase::{
    MultiPersonalPhaseTracker, PersonalPhaseUnregisterTrackedObjectOutcomeLikeCpp, PhaseShift,
};
use crate::pool::{
    PoolDespawnObjectPlanLikeCpp, PoolDespawnPoolPlanLikeCpp, PoolInitForMapPlanLikeCpp,
    PoolMemberKindLikeCpp, PoolMgrLikeCpp, PoolMgrPlanErrorLikeCpp, PoolObjectLikeCpp,
    PoolSpawnObjectActionLikeCpp, PoolSpawnObjectPlanLikeCpp, PoolSpawnPoolPlanLikeCpp,
    PoolTypedDespawnPlanLikeCpp, PoolTypedSpawnPlanLikeCpp,
};
use crate::spawn::{
    AddRespawnInfoOutcomeLikeCpp, CheckRespawnOutcomeLikeCpp,
    CheckRespawnSpawnGroupGuardOutcomeLikeCpp, Difficulty, LinkedRespawnStoreLikeCpp,
    ProcessRespawnActionLikeCpp, RespawnInfoLikeCpp, RespawnStoreLikeCpp,
    SpawnGridLoadStateLikeCpp, SpawnGroupActiveChange, SpawnGroupFlags, SpawnGroupRuntimeState,
    SpawnGroupTemplateData, SpawnId, SpawnObjectType, SpawnStore,
};
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position, guid::HighGuid};
use wow_entities::{
    AccessorObjectKind, AreaTrigger, CombatBeginContextLikeCpp, CombatSubsystem, Conversation,
    Corpse, Creature, CreatureAimInitializeOutcomeLikeCpp, CreatureRuntimePlan,
    CreatureRuntimeUpdateContext, CreatureSearchFormationOutcomeLikeCpp, DynamicObject,
    GAMEOBJECT_TYPE_CAPTURE_POINT, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT,
    GAMEOBJECT_TYPE_NEW_FLAG, GAMEOBJECT_TYPE_NEW_FLAG_DROP, GAMEOBJECT_TYPE_TRANSPORT,
    GO_FLAG_NODESPAWN, GameObject, GameObjectCreateLifecycleRecord, GameObjectLifecycleError,
    GameObjectTemplateLifecycleRecord,
    GameObjectUpdateOutcomeLikeCpp as EntityGameObjectUpdateOutcomeLikeCpp,
    GameObjectUpdateStatusLikeCpp as EntityGameObjectUpdateStatusLikeCpp, GoState, INVALID_HEIGHT,
    LineOfSightQuery, LootState, MAX_VISIBILITY_DISTANCE, MapBindingError, MapObjectRecord,
    ObjectAccessorError, ObjectNotifyFlags, Pet, Player, SceneObject, TransportUpdateLikeCpp, Unit,
    UnitAddToWorldOutcomeLikeCpp, UnitRemoveFromWorldOutcomeLikeCpp,
    UnitValuesUpdate,
    VehicleKitAddToWorldResetOutcomeLikeCpp, VehicleKitInstallOutcomeLikeCpp,
    VehicleKitRemoveOutcomeLikeCpp, WorldObject, WorldObjectEnvironment, WorldObjectHeightQuery,
};

/// Map-owned visibility-destroy recipients captured before deferred publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectVisibilityDestroyRecipientsLikeCpp {
    pub object_guid: ObjectGuid,
    pub recipient_guids: Vec<ObjectGuid>,
}

pub type CreatureVisibilityDestroyRecipientsLikeCpp = ObjectVisibilityDestroyRecipientsLikeCpp;

const GRID_SLOT_COUNT: usize = (MAX_NUMBER_OF_GRIDS * MAX_NUMBER_OF_GRIDS) as usize;
#[cfg(test)]
const GAMEOBJECT_TYPE_GENERIC_LIKE_CPP: u32 = 5;
pub const DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP: f32 = 0.388_999_998_569_489;
/// C++ `DynamicTree.cpp:34-38` `CHECK_TREE_PERIOD = 200`.
const DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP: u32 = 200;
const WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP: u32 = 1_000;



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveObjectKind {
    Player,
    NonPlayer,
}

/// C++ `GOSummonType` (`ObjectDefines.h:81-85`) is intentionally separate
/// from creature temporary summon types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GameObjectSummonTypeLikeCpp {
    TimedOrCorpseDespawn = 0,
    TimedDespawn = 1,
}

impl From<AccessorObjectKind> for ActiveObjectKind {
    fn from(kind: AccessorObjectKind) -> Self {
        match kind {
            AccessorObjectKind::Player => Self::Player,
            _ => Self::NonPlayer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapGuidSequenceErrorLikeCpp {
    /// Mirrors the C++ `static_assert` in `Map::GenerateLowGuid<high>` /
    /// `Map::GetMaxLowGuid<high>` (`Map.h:514-526`) without panicking for
    /// runtime-selected Rust `HighGuid` values.
    UnsupportedSequenceSource { high: HighGuid },
}

struct MapGuidSequenceGeneratorLikeCpp {
    generator: ObjectGuidGenerator,
}

impl std::fmt::Debug for MapGuidSequenceGeneratorLikeCpp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapGuidSequenceGeneratorLikeCpp")
            .field("high", &self.generator.high_guid())
            .field("next_after_max_used", &self.generator.next_after_max_used())
            .finish()
    }
}

impl MapGuidSequenceGeneratorLikeCpp {
    fn new(high: HighGuid) -> Self {
        Self {
            generator: ObjectGuidGenerator::new(high, 1),
        }
    }
}

#[derive(Debug)]
pub struct Map<Terrain = NoopTerrainGridLoader, Lifecycle = NoopGridLifecycle> {
    map_id: u32,
    instance_id: u32,
    spawn_mode: Difficulty,
    grid_expiry_ms: i64,
    grid_unload: bool,
    visible_distance: f32,
    grids: Vec<Option<Box<NGrid>>>,
    terrain: Terrain,
    lifecycle: Lifecycle,
    active_cells: HashSet<CellCoord>,
    /// Map-owned C++ `Map::m_activeNonPlayers` (`Map.h:617-619`).
    ///
    /// Source-of-truth remains `entity_world`; this set stores only non-player active
    /// object GUID membership produced by `Map::AddToActive`/`RemoveFromActive` seams.
    /// It is not rebuilt by sessions/ObjectAccessor scans. Rust does not yet model
    /// C++ `Map::Update`'s mutating iterator adjustment; consumers snapshot/sort GUIDs.
    active_non_players_like_cpp: HashSet<ObjectGuid>,
    personal_phase_tracker: MultiPersonalPhaseTracker,
    spawn_group_state: SpawnGroupRuntimeState,
    respawn_store: RespawnStoreLikeCpp,
    pool_data: SpawnedPoolDataLikeCpp,
    grid_state_unloaded: bool,
    /// Whether the one-shot C++ `Map::LoadCorpseData` database load completed.
    ///
    /// Map creation owns this flag; login may trigger the current async DB
    /// bridge, but repeated sessions must not duplicate the same persisted
    /// corpses in the canonical object store.
    corpse_data_loaded_like_cpp: bool,
    /// Map-local typed by-spawn-id live-object stores, matching C++
    /// `_creatureBySpawnIdStore`, `_gameobjectBySpawnIdStore`, and
    /// `_areaTriggerBySpawnIdStore` beside `_objectsStore` (`Map.h:418-430`,
    /// private fields at `Map.h:793-796`).
    ///
    /// Rust keeps `entity_world` as the source-of-truth object store. These
    /// indexes are derived only from `insert_object_entry`/`take_object_entry`
    /// and store GUID sets to preserve Trinity's unordered-multimap-like
    /// cardinality without making pointers canonical state. Spawn id zero is
    /// omitted, matching C++ `if (_spawnId)` / `IsStaticSpawn()`.
    ///
    /// AreaTrigger runtime side effects outside the object/spawn-id store
    /// (`ZoneScript`, caster unregister, AI removal, unit enter/exit, visibility,
    /// movement/transport, full entity-specific AddToWorld/RemoveFromWorld) remain
    /// outside this slice.
    creatures_by_spawn_id: HashMap<SpawnId, HashSet<ObjectGuid>>,
    gameobjects_by_spawn_id: HashMap<SpawnId, HashSet<ObjectGuid>>,
    area_triggers_by_spawn_id: HashMap<SpawnId, HashSet<ObjectGuid>>,
    entity_world: EntityWorld,
    /// Map-owned membership order of this map's players, mirroring C++
    /// `Map::m_mapRefManager`. `MapReference::targetObjectBuildLink`
    /// (`Maps/MapReference.cpp:22-28`) links each Player with `insertFirst`, so
    /// `Map::Update`'s session walk (`Maps/Map.cpp:669-680`) visits the most
    /// recently added player first. A sorted GUID list is a different order and
    /// is not used for that walk (#787).
    map_reference_order_like_cpp: Vec<ObjectGuid>,
    /// Map-owned represented C++ `CreatureGroupHolder`, keyed by leader spawn id.
    ///
    /// Source-of-truth remains `entity_world` and the typed spawn-id index. This
    /// holder stores only represented formation membership GUIDs produced by
    /// explicit `Creature::SearchFormation()` input; it does not own movement,
    /// AI, DB `FormationMgr`, waypoint, combat-assist, or session fanout runtime.
    creature_group_holder_like_cpp: HashMap<SpawnId, HashSet<ObjectGuid>>,
    /// Map-owned represented C++ `_dynamicTree` model-key registration/update seam.
    ///
    /// Source-of-truth is this `Map` instance. The represented key set is a
    /// deterministic stand-in for C++ `GameObjectModel` object identity and
    /// drives `empty()`/count; insert/remove mutate the set and increment
    /// `unbalanced_times` only on actual add/remove, matching
    /// `DynamicTree.cpp:72-82`. Duplicate insert/missing remove are guarded no-ops
    /// to avoid key-count drift. No real GameObjectModel, RegularGrid/BIH,
    /// collision, LOS/intersection/height, AddToWorld/RemoveFromWorld wiring,
    /// transport delayed-add, GO_FLAG_MAP_OBJECT, EnableCollision,
    /// ObjectAccessor/session/fanout, scripts, AI, DB or model ownership is
    /// represented here.
    dynamic_tree_model_keys_like_cpp: HashSet<RepresentedGameObjectModelKeyLikeCpp>,
    dynamic_tree_rebalance_timer_remaining_ms_like_cpp: u32,
    dynamic_tree_unbalanced_times_like_cpp: u32,
    /// Map-owned deferred physical removal queue matching C++
    /// `Map::i_objectsToRemove` (`Map.cpp:2547-2555`, `2574-2646`).
    ///
    /// Source of truth remains `entity_world`: enqueue mutates the canonical
    /// record, and only `remove_all_objects_in_remove_list_like_cpp` drains this
    /// set into `remove_from_map_like_cpp(..., true)`. Session/ObjectAccessor/DB
    /// caches must not drain or reconstruct this queue.
    objects_to_remove: HashSet<ObjectGuid>,
    /// Destroy recipients captured before an in-world object is detached.
    /// Delivery is drained by the canonical world tick after every map guard
    /// is released; no session or packet state is stored here.
    pending_object_visibility_destroy_recipients_like_cpp:
        Vec<ObjectVisibilityDestroyRecipientsLikeCpp>,
    /// Map-owned temporary Unit world-object switch queue matching C++
    /// `Map::i_objectsToSwitch` (`Map.h:651-652`) and
    /// `Map::AddObjectToSwitchList` (`Map.cpp:2557-2572`).
    ///
    /// Source of truth remains `entity_world`; callers representing
    /// `WorldObject::SetWorldObject(on)` may enqueue `guid -> on`, and only
    /// `remove_all_objects_in_remove_list_like_cpp` drains this map-local queue
    /// before `objects_to_remove` (`Map.cpp:2574-2594`). Session/ObjectAccessor/DB
    /// caches must not reconstruct or drain it.
    objects_to_switch: HashMap<ObjectGuid, bool>,
    /// Map-owned represented `_farSpellCallbacks` FIFO queue for C++
    /// `Map::AddFarSpellCallback` / `Map::DelayedUpdate` (`Map.cpp:2514-2530`).
    ///
    /// Source-of-truth and drain ownership are this `Map`; callers may enqueue only
    /// explicit represented actions and only `Map::drain_far_spell_callbacks_like_cpp`
    /// consumes them. This must run before `remove_all_objects_in_remove_list_like_cpp`.
    far_spell_callbacks_like_cpp: VecDeque<RepresentedFarSpellCallbackLikeCpp>,
    represented_far_spell_callback_execution_log_like_cpp: Vec<u64>,
    /// Map-owned delayed cell/grid movement queues matching C++
    /// `_creaturesToMove`, `_gameObjectsToMove`, `_dynamicObjectsToMove`, and
    /// `_areaTriggersToMove` (`Map.h:566-579`, `Map.cpp:1163-1416`).
    ///
    /// `entity_world` remains the source-of-truth; these vectors preserve the
    /// per-family delayed move-list order and the pending maps store only the
    /// C++-like `_moveState`/`_newPosition` derivative. Future callers enqueue
    /// through `Map::add_*_to_move_list_like_cpp`; only `Map` drains and mutates
    /// canonical cell membership/positions. Session/ObjectAccessor/DB caches must
    /// not drain or reconstruct these queues.
    creatures_to_move: Vec<ObjectGuid>,
    gameobjects_to_move: Vec<ObjectGuid>,
    dynamic_objects_to_move: Vec<ObjectGuid>,
    area_triggers_to_move: Vec<ObjectGuid>,
    creature_move_states: HashMap<ObjectGuid, PendingCellMoveLikeCpp>,
    gameobject_move_states: HashMap<ObjectGuid, PendingCellMoveLikeCpp>,
    dynamic_object_move_states: HashMap<ObjectGuid, PendingCellMoveLikeCpp>,
    area_trigger_move_states: HashMap<ObjectGuid, PendingCellMoveLikeCpp>,
    creature_move_lock: bool,
    gameobject_move_lock: bool,
    dynamic_object_move_lock: bool,
    area_trigger_move_lock: bool,
    /// Map-owned represented script schedule matching C++ `m_scriptSchedule`
    /// plus `i_scriptLock` (`Map.cpp:777-795`, `MapScripts.cpp:33-98,311-321`).
    ///
    /// Source-of-truth is this `Map` instance. Entries are keyed by absolute game
    /// time seconds so the due prefix drains deterministically and future entries
    /// remain queued. Values preserve multiple actions with the same due time.
    /// Due processing records represented execution evidence only; it does not
    /// run ScriptInfo commands, look up objects/items/sessions, send packets,
    /// mutate movement/quests/chat/weather, or call a real script manager.
    script_schedule_like_cpp: BTreeMap<i64, Vec<RepresentedScriptScheduleActionLikeCpp>>,
    script_schedule_lock_like_cpp: bool,
    represented_executed_script_actions_like_cpp: Vec<RepresentedScriptScheduleActionLikeCpp>,
    /// Map-owned represented C++ `_zoneDynamicInfo` plus `_weatherUpdateTimer`.
    ///
    /// Source-of-truth is this `Map` instance. The represented zone map is only
    /// created by explicit control/test helpers; absence is a no-op and does not
    /// synthesize `WeatherMgr` data. Timer semantics mirror `IntervalTimer`:
    /// accumulate diff, pass on `>= interval`, reset with modulo to preserve
    /// overshoot. The weather seam records `Weather::Update(interval)` evidence
    /// and drops only `DefaultWeather` when represented update returns false.
    /// It does not run regeneration/RNG, packet fanout, world zone messages,
    /// script manager hooks, DB lookups, or player-count checks.
    zone_dynamic_info_like_cpp: BTreeMap<u32, RepresentedZoneDynamicInfoLikeCpp>,
    weather_update_timer_current_ms_like_cpp: u32,
    weather_update_timer_interval_ms_like_cpp: u32,
    /// C++ `Map::_guidGenerators` (`Map.h:789-791`), lazy initialized by
    /// `Map::GetGuidSequenceGenerator` (`Map.cpp:2505-2511`). This stores only
    /// map-owned sequence counters; callers must compose full ObjectGuids with
    /// their own entry/map/server/realm context and must not feed DB spawn ids
    /// back into this map-local runtime identity source. Trinity's constructor
    /// seeds Transport from global ObjectMgr (`Map.cpp:145-166`); that external
    /// synchronization is intentionally out of scope for this seam, so all
    /// supported HighGuid generators start lazily at 1 unless explicitly set.
    guid_generators: HashMap<HighGuid, MapGuidSequenceGeneratorLikeCpp>,
    /// Map-owned seam for C++ random consumers that are owned by `Map` runtime
    /// state. DB/cache callers may request creature level/model selection through
    /// `&mut Map` but must not own or replay this RNG themselves.
    creature_level_rng_like_cpp: StdRng,
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub const fn map_id(&self) -> u32 {
        self.map_id
    }

    pub const fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub const fn spawn_mode(&self) -> Difficulty {
        self.spawn_mode
    }

    /// Mirrors TrinityCore `urand(min, max)` (`Random.cpp:35-47`): assert
    /// `max >= min` and sample an inclusive integer range. Ownership remains on
    /// `Map` so loaded-grid runtime consumers advance one canonical RNG stream.
    pub fn urand_inclusive_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        assert!(max >= min, "C++ urand requires max >= min");
        self.creature_level_rng_like_cpp.gen_range(min..=max)
    }

    /// Mirrors the floating-point random draw used by C++ weighted model selection.
    /// The caller owns the exact semantic range; this helper only keeps RNG ownership
    /// on the map, matching the loaded-grid runtime path.
    pub fn frand_exclusive_like_cpp(&mut self, min: f32, max: f32) -> f32 {
        assert!(max > min, "C++ frand-like draw requires max > min");
        self.creature_level_rng_like_cpp.gen_range(min..max)
    }

    /// Mirrors `Creature::SelectLevel` for DB/template min/max rows: fixed rows
    /// use `MinLevel` without consuming RNG; variable rows call inclusive `urand`.
    pub fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8 {
        if min_level == max_level {
            return min_level;
        }
        let selected = self.urand_inclusive_like_cpp(u32::from(min_level), u32::from(max_level));
        selected as u8
    }

    #[cfg(test)]
    fn seed_creature_level_rng_for_tests_like_cpp(&mut self, seed: u64) {
        self.creature_level_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    #[cfg(test)]
    pub(crate) fn set_dynamic_tree_model_count_for_tests_like_cpp(&mut self, model_count: u32) {
        self.dynamic_tree_model_keys_like_cpp.clear();
        for counter in 0..model_count {
            self.dynamic_tree_model_keys_like_cpp
                .insert(RepresentedGameObjectModelKeyLikeCpp {
                    owner_guid: ObjectGuid::create_player(1, i64::from(counter) + 1),
                });
        }
    }

    #[cfg(test)]
    pub(crate) fn mark_dynamic_tree_unbalanced_for_tests_like_cpp(&mut self, times: u32) {
        self.dynamic_tree_unbalanced_times_like_cpp = times;
    }

    pub fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    pub fn far_spell_callbacks_count_like_cpp(&self) -> usize {
        self.far_spell_callbacks_like_cpp.len()
    }

    pub fn represented_far_spell_callback_execution_log_like_cpp(&self) -> &[u64] {
        &self.represented_far_spell_callback_execution_log_like_cpp
    }

    /// C++ `Map::DelayedUpdate` first block: drain `_farSpellCallbacks` FIFO before
    /// `RemoveAllObjectsInRemoveList()` (`Map.cpp:2519-2530`).
    ///
    /// Limits: this is a bounded represented seam only. It executes no real Spell,
    /// Aura, caster/ObjectAccessor lookup, session fanout, packet, script, AI, or
    /// arbitrary callback side effects. `QueueObjectRemove` is the minimal map-owned
    /// mutation used to prove same-tick ordering before the remove-list drain.
    pub fn drain_far_spell_callbacks_like_cpp(&mut self) -> FarSpellCallbackDrainSummaryLikeCpp {
        let queued_before = self.far_spell_callbacks_like_cpp.len();
        let mut summary = FarSpellCallbackDrainSummaryLikeCpp {
            queued_before,
            ..Default::default()
        };

        while let Some(callback) = self.far_spell_callbacks_like_cpp.pop_front() {
            summary.processed += 1;
            self.represented_far_spell_callback_execution_log_like_cpp
                .push(callback.id);
            match callback.action {
                RepresentedFarSpellCallbackActionLikeCpp::RecordExecution => {
                    summary.record_only += 1;
                }
                RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove { guid } => {
                    summary.remove_queue_attempted += 1;
                    let outcome = self.add_object_to_remove_list_like_cpp(guid);
                    if outcome.queued {
                        summary.remove_queued += 1;
                    }
                    if outcome.missing_or_stale {
                        summary.remove_missing_or_stale += 1;
                    }
                    if outcome.duplicate {
                        summary.remove_duplicates += 1;
                    }
                    if outcome.unsupported_kind.is_some() {
                        summary.unsupported_remove_kinds += 1;
                    }
                }
            }
        }

        summary.queued_after = self.far_spell_callbacks_like_cpp.len();
        summary
    }

    pub fn represented_zone_dynamic_info_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&RepresentedZoneDynamicInfoLikeCpp> {
        self.zone_dynamic_info_like_cpp.get(&zone_id)
    }

    pub fn map_object_count(&self) -> usize {
        self.entity_world.len()
    }

    pub fn objects_to_switch_count_like_cpp(&self) -> usize {
        self.objects_to_switch.len()
    }

    pub fn pending_switch_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        self.objects_to_switch.get(&guid).copied()
    }

    #[cfg(test)]
    fn enqueue_object_to_switch_for_test(&mut self, guid: ObjectGuid, on: bool) {
        self.objects_to_switch.insert(guid, on);
    }

    fn viewpoint_has_invalid_position_like_cpp(&self, viewpoint_guid: ObjectGuid) -> bool {
        self.map_object(viewpoint_guid).is_none_or(|viewpoint| {
            let position = viewpoint.position();
            !is_valid_map_coord_2d(position.x, position.y)
        })
    }

    pub(crate) fn map_object_record(&self, guid: ObjectGuid) -> Option<ObjectRef<'_>> {
        self.entity_world.get(&guid)
    }

    /// Represented tail metrics from C++ `Map::Update` after
    /// `sScriptMgr->OnMapUpdate(this, t_diff)` (`Map.cpp:804-815`).
    ///
    /// C++ emits `TC_METRIC_VALUE("map_creatures", GetObjectsStore().Size<Creature>())`
    /// and `TC_METRIC_VALUE("map_gameobjects", GetObjectsStore().Size<GameObject>())`.
    /// Rust reads only canonical typed `MapObjectRecord`s from `entity_world`: a
    /// record must have both the exact canonical kind and the corresponding typed
    /// body. Generic `WorldObject` records, Pet, Transport, DynamicObject,
    /// AreaTrigger, Player, etc. are intentionally excluded; no telemetry backend
    /// is invoked here.
    pub fn map_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_record(guid).map(|record| record.object())
    }

    fn object_is_in_world(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid)
            .is_some_and(|object| object.object().is_in_world())
    }

    pub fn map_object_by_kind(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<&WorldObject> {
        let record = self.map_object_record(guid)?;
        allowed.contains(&record.kind()).then_some(record.object())
    }

    pub fn typed_player_counts_like_cpp(&self) -> (u32, u32) {
        let mut total = 0u32;
        let mut non_game_masters = 0u32;
        for record in self.entity_world.values() {
            if record.kind() != AccessorObjectKind::Player {
                continue;
            }
            let Some(player) = record.player() else {
                continue;
            };
            total = total.saturating_add(1);
            if !player.is_game_master_like_cpp() {
                non_game_masters = non_game_masters.saturating_add(1);
            }
        }
        (total, non_game_masters)
    }

    fn validate_map_object(&self, object: &WorldObject) -> Result<(), MapObjectStoreError> {
        if object.map_id() == self.map_id && object.instance_id() == self.instance_id {
            return Ok(());
        }

        Err(MapObjectStoreError::WrongMap {
            guid: object.guid(),
            expected_map_id: self.map_id,
            expected_instance_id: self.instance_id,
            actual_map_id: object.map_id(),
            actual_instance_id: object.instance_id(),
        })
    }
}

fn sort_dedup(guids: &mut Vec<ObjectGuid>) {
    guids.sort();
    guids.dedup();
}

fn marked_cells_in_grid_like_cpp(
    grid: GridCoord,
    marked_cells: &HashSet<CellCoord>,
) -> Vec<CellCoord> {
    let cell_min_x = grid.x_coord * MAX_NUMBER_OF_CELLS;
    let cell_min_y = grid.y_coord * MAX_NUMBER_OF_CELLS;
    let cell_max_x = cell_min_x + MAX_NUMBER_OF_CELLS;
    let cell_max_y = cell_min_y + MAX_NUMBER_OF_CELLS;
    let mut cells = Vec::new();

    for x in cell_min_x..cell_max_x {
        for y in cell_min_y..cell_max_y {
            let cell = CellCoord::new(x, y);
            if marked_cells.contains(&cell) {
                cells.push(cell);
            }
        }
    }

    cells
}

#[cfg(test)]
#[path = "../map_tests.rs"]
mod tests;
