//! Map grid lifecycle skeleton.
//!
//! C++ references:
//! - `game/Maps/Map.h`
//! - `game/Maps/Map.cpp`

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::cell::{Cell, GridObjectGuids, WorldObjectGuids, calculate_cell_area_like_cpp};
use crate::coords::{
    CellCoord, GridCoord, MAX_NUMBER_OF_CELLS, MAX_NUMBER_OF_GRIDS, SIZE_OF_GRID_CELL,
    TOTAL_NUMBER_OF_CELLS_PER_MAP, compute_cell_coord, compute_grid_coord, is_valid_map_coord_2d,
    normalize_map_coord,
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
    DynamicObjectType, DynamicObjectValuesUpdate, GAMEOBJECT_TYPE_CAPTURE_POINT,
    GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_GOOBER,
    GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT, GAMEOBJECT_TYPE_NEW_FLAG, GAMEOBJECT_TYPE_NEW_FLAG_DROP,
    GAMEOBJECT_TYPE_TRANSPORT, GO_FLAG_NODESPAWN, GameObject, GameObjectCreateLifecycleRecord,
    GameObjectLifecycleError, GameObjectTemplateLifecycleRecord,
    GameObjectUpdateOutcomeLikeCpp as EntityGameObjectUpdateOutcomeLikeCpp,
    GameObjectUpdateStatusLikeCpp as EntityGameObjectUpdateStatusLikeCpp, GoState, INVALID_HEIGHT,
    LineOfSightQuery, LootState, MAX_VISIBILITY_DISTANCE, MapBindingError, MapObjectRecord,
    ObjectAccessorError, ObjectAccessorMapSource, ObjectNotifyFlags, Pet, Player,
    PlayerValuesUpdate, SceneObject, TransportUpdateLikeCpp, Unit, UnitAddToWorldOutcomeLikeCpp,
    UnitRemoveFromWorldOutcomeLikeCpp, UnitSharedVisionSetWorldObjectRequestLikeCpp,
    UnitValuesUpdate, VehicleKitAddToWorldResetOutcomeLikeCpp, VehicleKitInstallOutcomeLikeCpp,
    VehicleKitRemoveOutcomeLikeCpp, WorldObject, WorldObjectEnvironment, WorldObjectHeightQuery,
};

const GRID_SLOT_COUNT: usize = (MAX_NUMBER_OF_GRIDS * MAX_NUMBER_OF_GRIDS) as usize;
#[cfg(test)]
const GAMEOBJECT_TYPE_GENERIC_LIKE_CPP: u32 = 5;
pub const DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP: f32 = 0.388_999_998_569_489;
/// C++ `DynamicTree.cpp:34-38` `CHECK_TREE_PERIOD = 200`.
const DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP: u32 = 200;
const WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP: u32 = 1_000;

fn gameobject_local_rotation_from_orientation_like_cpp(orientation: f32) -> [f32; 4] {
    let half = orientation * 0.5;
    [0.0, 0.0, half.sin(), half.cos()]
}

/// Position resolver for C++ `WorldObject::SummonGameObject(entry, x, y, z, ang, ...)`.
///
/// C++ anchors:
/// - `Object.cpp:2096-2105`: if `x == y == z == 0`, call
///   `GetClosePoint(x, y, z, GetCombatReach())` and use the summoner orientation.
/// - `Object.cpp:3341-3408`: `GetClosePoint` delegates to `GetNearPoint(nullptr,
///   distance2d + size, orientation)`, whose 2D calculation adds the summoner
///   combat reach again when `searcher == nullptr`.
///
/// Scope: this represents the deterministic 2D coordinate/orientation branch
/// and map-coordinate normalization. Height correction, collision detection,
/// LOS fallback search and map-vmap terrain queries remain runtime gaps.
pub fn world_object_summon_gameobject_position_from_coords_like_cpp(
    summoner_position: Position,
    summoner_combat_reach: f32,
    x: f32,
    y: f32,
    z: f32,
    angle: f32,
) -> WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
    if x == 0.0 && y == 0.0 && z == 0.0 {
        let reach = summoner_combat_reach.max(0.0);
        let distance = reach + reach;
        let mut resolved_x = summoner_position.x + distance * summoner_position.orientation.cos();
        let mut resolved_y = summoner_position.y + distance * summoner_position.orientation.sin();
        let before_normalize_x = resolved_x;
        let before_normalize_y = resolved_y;
        normalize_map_coord(&mut resolved_x);
        normalize_map_coord(&mut resolved_y);
        return WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
            position: Position::new(
                resolved_x,
                resolved_y,
                summoner_position.z,
                summoner_position.orientation,
            ),
            close_point_fallback_used: true,
            normalized_map_coords: resolved_x != before_normalize_x
                || resolved_y != before_normalize_y,
            collision_los_adjustment_represented: false,
        };
    }

    WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
        position: Position::new(x, y, z, angle),
        close_point_fallback_used: false,
        normalized_map_coords: false,
        collision_los_adjustment_represented: false,
    }
}

/// Position resolver for C++ `Spell::EffectSummonObjectWild`.
///
/// C++ anchors:
/// - `SpellEffects.cpp:2946-2954`: explicit destination wins; otherwise call
///   `m_caster->GetClosePoint(..., DEFAULT_PLAYER_BOUNDING_RADIUS)` and use
///   `target->GetOrientation()`.
/// - `ObjectDefines.h:39`: `DEFAULT_PLAYER_BOUNDING_RADIUS`.
/// - `Object.cpp:3341-3408`: `GetClosePoint` delegates to `GetNearPoint`
///   with `searcher == nullptr`, so 2D distance is caster combat reach plus
///   the provided size.
///
/// Scope: this represents deterministic 2D fallback and map-coordinate
/// normalization. `focusObject` selection, height correction, collision, LOS
/// search and terrain queries remain caller/runtime gaps.
pub fn spell_effect_summon_object_wild_position_like_cpp(
    caster_position: Position,
    caster_combat_reach: f32,
    target_orientation: f32,
    explicit_destination: Option<Position>,
) -> SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
    if let Some(position) = explicit_destination {
        return SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
            position,
            explicit_destination_used: true,
            close_point_fallback_used: false,
            normalized_map_coords: false,
            focus_object_orientation_represented: target_orientation != caster_position.orientation,
            collision_los_adjustment_represented: false,
        };
    }

    let distance = caster_combat_reach.max(0.0) + DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP;
    let mut resolved_x = caster_position.x + distance * caster_position.orientation.cos();
    let mut resolved_y = caster_position.y + distance * caster_position.orientation.sin();
    let before_normalize_x = resolved_x;
    let before_normalize_y = resolved_y;
    normalize_map_coord(&mut resolved_x);
    normalize_map_coord(&mut resolved_y);

    SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
        position: Position::new(
            resolved_x,
            resolved_y,
            caster_position.z,
            target_orientation,
        ),
        explicit_destination_used: false,
        close_point_fallback_used: true,
        normalized_map_coords: resolved_x != before_normalize_x || resolved_y != before_normalize_y,
        focus_object_orientation_represented: target_orientation != caster_position.orientation,
        collision_los_adjustment_represented: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
    pub position: Position,
    pub explicit_destination_used: bool,
    pub close_point_fallback_used: bool,
    pub normalized_map_coords: bool,
    pub focus_object_orientation_represented: bool,
    pub collision_los_adjustment_represented: bool,
}

#[derive(Clone, Copy)]
struct CombatUnitSnapshotLikeCpp<'a> {
    guid: ObjectGuid,
    unit: &'a Unit,
    game_master_player: bool,
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicRespawnScalingConfig {
    pub creature_rate: f64,
    pub creature_minimum_secs: u32,
    pub gameobject_rate: f64,
    pub gameobject_minimum_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicRespawnScalingNoopReason {
    DynamicModeDisabled,
    UnsupportedMode,
    BattlegroundOrArena,
    UnsupportedSpawnType,
    MissingSpawnMetadata,
    MissingDynamicSpawnRateFlag,
    MissingZonePlayerCount,
    ZeroZonePlayers,
    AdjustFactorAtLeastOne,
    DelayAtOrBelowMinimum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnedPoolDataErrorLikeCpp {
    /// C++ `SpawnedPoolData::IsSpawnedObject(SpawnObjectType, ...)` aborts for
    /// non Creature/GameObject types (`PoolMgr.cpp:66-77`). Rust returns a typed
    /// error at the seam instead of treating AreaTrigger as pooled/spawned.
    UnsupportedSpawnObjectType(SpawnObjectType),
}

/// Map-owned parity seam for C++ `SpawnedPoolData` (`PoolMgr.h:51-83`).
///
/// This is only the map-local state shape and helpers used by C++
/// `Map::_poolData` / `Map::GetPoolData()`. It does not implement real
/// `PoolMgr::SpawnPool`, `DespawnPool`, RNG/chance, entity creation,
/// AddToMap/RemoveFromMap, DB persistence/delete, or grid/session fanout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnedPoolDataLikeCpp {
    spawned_creatures: HashSet<SpawnId>,
    spawned_gameobjects: HashSet<SpawnId>,
    spawned_pools: HashMap<u32, u32>,
}

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

/// Represented key for the map-owned C++ `_dynamicTree` model-registration seam.
///
/// C++ `DynamicMapTree` stores `GameObjectModel` object references/pointers. Rust does
/// not model real `GameObjectModel` or collision geometry in this bounded slice, so
/// the deterministic stand-in key is the owning object GUID. Duplicate insertion is
/// guarded as a no-op to avoid count drift; this is intentionally safer than raw
/// pointer duplicate behavior and is not a claim of exact model object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepresentedGameObjectModelKeyLikeCpp {
    pub owner_guid: ObjectGuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicMapTreeModelMutationStatusLikeCpp {
    Inserted,
    AlreadyPresent,
    Removed,
    Missing,
}

/// Represented result for C++ `DynamicMapTree::{insert,remove}` via Map facades.
///
/// Anchors: `DynamicTree.cpp:72-82,115-127`, `Map.h:457-460`.
/// Real `GameObjectModel`, RegularGrid/BIH, LOS/intersection/height,
/// AddToWorld/RemoveFromWorld wiring, transport delayed-add, `GO_FLAG_MAP_OBJECT`,
/// collision enable/disable, ObjectAccessor/session/fanout/scripts/AI/DB remain out
/// of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicMapTreeModelMutationOutcomeLikeCpp {
    pub key: RepresentedGameObjectModelKeyLikeCpp,
    pub status: DynamicMapTreeModelMutationStatusLikeCpp,
    pub model_count_before: usize,
    pub model_count_after: usize,
    pub unbalanced_before: u32,
    pub unbalanced_after: u32,
}

/// Represented map-owned evidence for C++ `GameEventMgr::RunSmartAIScripts`.
///
/// Anchor: `GameEventMgr.cpp:1618-1655`. The C++ worker visits every map and
/// dispatches only exact in-world Creature/GameObject AI callbacks. Rust does
/// not model SmartAI/`ProcessEventsFor` here; this summary only counts exact
/// typed `Map::map_objects` candidates and marks dispatch as unrepresented.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GameEventSmartAiScriptCandidateSummaryLikeCpp {
    pub maps_visited: usize,
    pub in_world_creature_candidates: usize,
    pub in_world_gameobject_candidates: usize,
    pub creature_ai_enabled_unrepresented: usize,
    pub script_dispatch_unrepresented: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectUpdateModelStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
    NotInWorld,
}

/// Represented map-owned result for C++ `GameObject::UpdateModel()`.
///
/// C++ anchor: `GameObject.cpp:3867-3880`. This helper operates only on the
/// canonical `Map::map_objects` exact typed GameObject record, consumes explicit
/// caller-provided `CreateModel()` evidence, and mutates only represented local
/// model/flag/collision evidence plus the map-owned represented DynamicMapTree
/// key set. It does not infer from display/template/DB and does not call
/// `EnableCollision()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectUpdateModelOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectUpdateModelStatusLikeCpp,
    pub old_model_present: bool,
    pub old_model_registered: bool,
    pub old_model_remove: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub new_has_model: bool,
    pub new_is_map_object: bool,
    pub new_model_insert: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetDisplayIdStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetDisplayId(uint32)`.
///
/// C++ anchor: `GameObject.cpp:3817-3820`. This preserves statement order over
/// canonical exact typed `Map::map_objects` GameObject records: write
/// `GameObjectData::DisplayID` first, then call represented `UpdateModel()`.
/// The model creation evidence remains caller-provided and is never inferred
/// from display/template/DB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetDisplayIdOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetDisplayIdStatusLikeCpp,
    pub previous_display_id: Option<i32>,
    pub new_display_id: Option<i32>,
    pub update_model: Option<GameObjectUpdateModelOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetGoStateStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetGoState(GOState)`.
///
/// C++ anchor: `GameObject.cpp:3771-3793`. This preserves statement order over
/// canonical exact typed `Map::map_objects` GameObject records: capture old state,
/// write `GameObjectData::State`, then run only the represented `m_model &&
/// !IsTransport() && IsInWorld()` collision branch. AI/type implementation hooks,
/// real `GameObjectModel`, BIH/LOS, ObjectAccessor/session fanout, scripts and DB
/// inference remain out of scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetGoStateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetGoStateStatusLikeCpp,
    pub previous_state: Option<i8>,
    pub new_state: Option<i8>,
    pub represented_model_present: bool,
    pub transport_type: bool,
    pub in_world_for_collision_branch: Option<bool>,
    pub collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSetLootStateStatusLikeCpp {
    Updated,
    MissingGameObject,
    WrongKind,
}

/// Represented map-owned result for C++ `GameObject::SetLootState(LootState, Unit*)`.
///
/// C++ anchor: `GameObject.cpp:3683-3709`. This preserves statement order over
/// canonical exact typed `Map::map_objects` GameObject records: write local loot
/// state/unit GUID first, expose the unimplemented AI hook as evidence, then
/// represent only explicit-caller-evidence restock and represented `m_model` collision.
/// It does not execute real AI, infer `Loot::IsChanged()`, create real
/// `GameObjectModel`/BIH geometry, fan out ObjectAccessor/session/script/DB effects, or
/// resolve a real `Unit*` from the supplied GUID evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectSetLootStateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: GameObjectSetLootStateStatusLikeCpp,
    pub previous_loot_state: Option<LootState>,
    pub new_loot_state: Option<LootState>,
    pub previous_loot_state_unit_guid: Option<ObjectGuid>,
    pub new_loot_state_unit_guid: Option<ObjectGuid>,
    pub previous_restock_time: Option<i64>,
    pub new_restock_time: Option<i64>,
    pub ai_on_loot_state_changed_not_represented: bool,
    pub restock_armed: bool,
    pub represented_model_present: bool,
    pub door_type_early_return: bool,
    pub collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
}

fn gameobject_type_is_transport_like_cpp(type_id: i8) -> bool {
    type_id == GAMEOBJECT_TYPE_TRANSPORT as i8 || type_id == GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT as i8
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

impl SpawnedPoolDataLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_spawned_objects_like_cpp(&self, pool_id: u32) -> u32 {
        self.spawned_pools.get(&pool_id).copied().unwrap_or(0)
    }

    pub fn is_spawned_creature_like_cpp(&self, spawn_id: SpawnId) -> bool {
        self.spawned_creatures.contains(&spawn_id)
    }

    pub fn is_spawned_gameobject_like_cpp(&self, spawn_id: SpawnId) -> bool {
        self.spawned_gameobjects.contains(&spawn_id)
    }

    pub fn is_spawned_pool_like_cpp(&self, sub_pool_id: u32) -> bool {
        self.spawned_pools.contains_key(&sub_pool_id)
    }

    pub fn is_spawned_object_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Result<bool, SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => Ok(self.is_spawned_creature_like_cpp(spawn_id)),
            SpawnObjectType::GameObject => Ok(self.is_spawned_gameobject_like_cpp(spawn_id)),
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn add_spawn_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        pool_id: u32,
    ) -> Result<(), SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => {
                self.spawned_creatures.insert(spawn_id);
                *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
                Ok(())
            }
            SpawnObjectType::GameObject => {
                self.spawned_gameobjects.insert(spawn_id);
                *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
                Ok(())
            }
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn remove_spawn_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        pool_id: u32,
    ) -> Result<(), SpawnedPoolDataErrorLikeCpp> {
        match object_type {
            SpawnObjectType::Creature => {
                self.spawned_creatures.remove(&spawn_id);
                Self::decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
                Ok(())
            }
            SpawnObjectType::GameObject => {
                self.spawned_gameobjects.remove(&spawn_id);
                Self::decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
                Ok(())
            }
            SpawnObjectType::AreaTrigger => Err(
                SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(object_type),
            ),
        }
    }

    pub fn add_pool_spawn_like_cpp(&mut self, sub_pool_id: u32, pool_id: u32) {
        self.spawned_pools.insert(sub_pool_id, 0);
        *self.spawned_pools.entry(pool_id).or_insert(0) += 1;
    }

    pub fn remove_pool_spawn_like_cpp(&mut self, sub_pool_id: u32, pool_id: u32) {
        self.spawned_pools.remove(&sub_pool_id);
        Self::decrement_pool_counter_like_cpp(&mut self.spawned_pools, pool_id);
    }

    pub fn spawned_objects_like_cpp(&self) -> Vec<(SpawnObjectType, SpawnId)> {
        let mut spawned = self
            .spawned_creatures
            .iter()
            .copied()
            .map(|spawn_id| (SpawnObjectType::Creature, spawn_id))
            .chain(
                self.spawned_gameobjects
                    .iter()
                    .copied()
                    .map(|spawn_id| (SpawnObjectType::GameObject, spawn_id)),
            )
            .collect::<Vec<_>>();
        spawned.sort_unstable();
        spawned
    }

    fn decrement_pool_counter_like_cpp(spawned_pools: &mut HashMap<u32, u32>, pool_id: u32) {
        let counter = spawned_pools.entry(pool_id).or_insert(0);
        if *counter > 0 {
            *counter -= 1;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicRespawnScalingOutcome {
    pub delay_secs: u32,
    pub noop_reason: Option<DynamicRespawnScalingNoopReason>,
}

impl DynamicRespawnScalingOutcome {
    pub const fn unchanged(delay_secs: u32, reason: DynamicRespawnScalingNoopReason) -> Self {
        Self {
            delay_secs,
            noop_reason: Some(reason),
        }
    }

    pub const fn scaled(delay_secs: u32) -> Self {
        Self {
            delay_secs,
            noop_reason: None,
        }
    }

    pub const fn was_scaled(self) -> bool {
        self.noop_reason.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicRespawnScalingContext {
    pub mode: u32,
    pub spawn_type: Option<SpawnObjectType>,
    pub spawn_metadata_present: bool,
    pub spawn_group_flags: Option<SpawnGroupFlags>,
    pub is_battleground_or_arena: bool,
    pub zone_player_count: Option<u32>,
    pub config: DynamicRespawnScalingConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnGroupConditionActionLikeCpp {
    Noop,
    Spawn { ignore_respawn: bool, force: bool },
    Despawn { delete_respawn_times: bool },
    SetInactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddObjectToRemoveListOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub queued: bool,
    pub duplicate: bool,
    pub missing_or_stale: bool,
    pub unsupported_kind: Option<AccessorObjectKind>,
    pub cleanup_before_delete_count: usize,
}

pub type RemoveListOutcomeLikeCpp = AddObjectToRemoveListOutcomeLikeCpp;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PersonalPhaseTrackerUpdateSummaryLikeCpp {
    pub expired_objects: usize,
    pub remove_queued: usize,
    pub missing_or_stale: usize,
    pub unsupported_kinds: usize,
    pub duplicate_queued: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedDynamicObjectValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: DynamicObjectValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedPlayerValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub values_update: PlayerValuesUpdate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepresentedUnitValuesUpdateLikeCpp {
    pub guid: ObjectGuid,
    pub kind: AccessorObjectKind,
    pub values_update: UnitValuesUpdate,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SendObjectUpdatesSummaryLikeCpp {
    /// Objects in canonical `Map::map_objects` with represented
    /// `Object::m_objectUpdated` set at snapshot time. Rust does not yet own the
    /// exact C++ `_updateObjects` pointer set, so this is a represented snapshot.
    pub queued_before: usize,
    /// In-world updated objects consumed through the represented BuildUpdate seam.
    pub processed: usize,
    /// Objects whose update masks were cleared via `ClearUpdateMask(false)`.
    pub cleared_update_masks: usize,
    /// Defense for impossible/stale Rust state where the represented update queue
    /// contains a not-in-world object. C++ asserts in `Map::SendObjectUpdates`.
    pub skipped_not_in_world: usize,
    /// Snapshot GUIDs that disappeared before mutable consumption; this should
    /// not happen in the current single-threaded map owner but stays non-panicking.
    pub missing_or_stale: usize,
    /// Evidence that C++ `UpdateDataMapType` player fanout/packet send is still
    /// intentionally not represented by this seam.
    pub fanout_not_represented: usize,
    /// Stable represented DynamicObject VALUES snapshots captured from canonical
    /// map-owned objects before the represented `BuildUpdate` clear. This is not
    /// session fanout and must not be read from live masks after clear.
    pub dynamic_object_values_updates: Vec<RepresentedDynamicObjectValuesUpdateLikeCpp>,
    /// Complete Player/Unit/ActivePlayer VALUES snapshots captured before the
    /// typed Player masks are cleared by represented `BuildUpdate`.
    pub player_values_updates: Vec<RepresentedPlayerValuesUpdateLikeCpp>,
    /// Complete Unit VALUES snapshots captured before typed Creature/Pet masks
    /// are cleared by represented `BuildUpdate`.
    pub unit_values_updates: Vec<RepresentedUnitValuesUpdateLikeCpp>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SwitchGridContainersOutcomeLikeCpp {
    executed: bool,
    missing_or_stale: bool,
    unsupported_kind: bool,
    permanent_world_object: bool,
    invalid_or_unloaded_grid: bool,
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

const WEEK_SECS_LIKE_CPP: i64 = 7 * 24 * 60 * 60;

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

/// Rust equivalent of C++ `Map::ApplyDynamicModeRespawnScaling`.
///
/// C++ anchors:
/// - `GameObject.cpp:1665-1672` calls this before persisting GO respawn time.
/// - `Map.cpp:2242-2284` contains the dynamic respawn guards and formula.
/// - `Map.h:657-660` declares the map helper.
///
/// This helper is pure because RustyCore does not yet own the canonical map
/// spawn-metadata and zone-player-count stores needed by a `Map` method. Future
/// GameObject runtime wiring must pass canonical metadata/counts into this
/// function; this function must not read or mutate session-local fallback state.
pub fn apply_dynamic_mode_respawn_scaling_like_cpp(
    respawn_delay_secs: u32,
    context: DynamicRespawnScalingContext,
) -> DynamicRespawnScalingOutcome {
    if context.mode == 0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::DynamicModeDisabled,
        );
    }

    if context.mode != 1 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedMode,
        );
    }

    if context.is_battleground_or_arena {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::BattlegroundOrArena,
        );
    }

    let Some(spawn_type) = context.spawn_type else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
        );
    };

    if !matches!(
        spawn_type,
        SpawnObjectType::Creature | SpawnObjectType::GameObject
    ) {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
        );
    }

    if !context.spawn_metadata_present {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
        );
    }

    let Some(spawn_group_flags) = context.spawn_group_flags else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingSpawnMetadata,
        );
    };

    if !spawn_group_flags.contains(SpawnGroupFlags::DYNAMIC_SPAWN_RATE) {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingDynamicSpawnRateFlag,
        );
    }

    let Some(player_count) = context.zone_player_count else {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::MissingZonePlayerCount,
        );
    };

    if player_count == 0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::ZeroZonePlayers,
        );
    }

    let (rate, time_minimum) = match spawn_type {
        SpawnObjectType::Creature => (
            context.config.creature_rate,
            context.config.creature_minimum_secs,
        ),
        SpawnObjectType::GameObject => (
            context.config.gameobject_rate,
            context.config.gameobject_minimum_secs,
        ),
        SpawnObjectType::AreaTrigger => {
            return DynamicRespawnScalingOutcome::unchanged(
                respawn_delay_secs,
                DynamicRespawnScalingNoopReason::UnsupportedSpawnType,
            );
        }
    };

    let adjust_factor = rate / f64::from(player_count);
    if adjust_factor >= 1.0 {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::AdjustFactorAtLeastOne,
        );
    }

    if respawn_delay_secs <= time_minimum {
        return DynamicRespawnScalingOutcome::unchanged(
            respawn_delay_secs,
            DynamicRespawnScalingNoopReason::DelayAtOrBelowMinimum,
        );
    }

    let scaled = (f64::from(respawn_delay_secs) * adjust_factor).ceil() as u32;
    DynamicRespawnScalingOutcome::scaled(scaled.max(time_minimum))
}

pub trait TerrainGridLoader {
    fn load_map_and_vmap(&mut self, grid_x: u32, grid_y: u32);
    fn unload_map(&mut self, grid_x: u32, grid_y: u32);
}

/// Terrain/dynamic-tree hook used by `Map` when it acts as a
/// `WorldObjectEnvironment` for `WorldObject` helpers.
///
/// This is the explicit ownership seam for C++ `Map::isInLineOfSight`,
/// `Map::GetHeight`, and `Map::GetGameObjectFloor`. Implementations may be a
/// noop while real terrain/vmap/dynamic-tree runtime is not ported, but callers
/// must still flow through `WorldObject -> WorldObjectEnvironment -> Map -> terrain`.
pub trait MapWorldObjectEnvironment {
    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool;

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32;

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopTerrainGridLoader;

impl TerrainGridLoader for NoopTerrainGridLoader {
    fn load_map_and_vmap(&mut self, _grid_x: u32, _grid_y: u32) {}
    fn unload_map(&mut self, _grid_x: u32, _grid_y: u32) {}
}

impl MapWorldObjectEnvironment for NoopTerrainGridLoader {
    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool {
        true
    }

    fn map_height(
        &self,
        _object: &WorldObject,
        _x: f32,
        _y: f32,
        _z: f32,
        _query: WorldObjectHeightQuery,
    ) -> f32 {
        INVALID_HEIGHT
    }

    fn floor_z(&self, _object: &WorldObject, _position: Position, _max_search_dist: f32) -> f32 {
        INVALID_HEIGHT
    }
}

pub trait GridLifecycle {
    fn load_grid_objects(&mut self, grid: &mut NGrid, cell: &Cell);
    fn stop_grid_objects(&mut self, grid: &NGrid);
    fn evacuate_grid(&mut self, grid: &mut NGrid);
    fn clean_grid(&mut self, grid: &mut NGrid);
    fn unload_grid_objects(&mut self, grid: &mut NGrid);
    fn take_unload_actions_like_cpp(&mut self) -> Vec<GridUnloadAction> {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopGridLifecycle;

impl GridLifecycle for NoopGridLifecycle {
    fn load_grid_objects(&mut self, _grid: &mut NGrid, _cell: &Cell) {}
    fn stop_grid_objects(&mut self, _grid: &NGrid) {}
    fn evacuate_grid(&mut self, _grid: &mut NGrid) {}
    fn clean_grid(&mut self, _grid: &mut NGrid) {}
    fn unload_grid_objects(&mut self, _grid: &mut NGrid) {}
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
    /// Source-of-truth remains `map_objects`; this set stores only non-player active
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
    /// Rust keeps `map_objects` as the source-of-truth object store. These
    /// indexes are derived only from `insert_map_object_record`/`remove_map_object`
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
    map_objects: HashMap<ObjectGuid, MapObjectRecord>,
    /// Map-owned represented C++ `CreatureGroupHolder`, keyed by leader spawn id.
    ///
    /// Source-of-truth remains `map_objects` and the typed spawn-id index. This
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
    /// Source of truth remains `map_objects`: enqueue mutates the canonical
    /// record, and only `remove_all_objects_in_remove_list_like_cpp` drains this
    /// set into `remove_from_map_like_cpp(..., true)`. Session/ObjectAccessor/DB
    /// caches must not drain or reconstruct this queue.
    objects_to_remove: HashSet<ObjectGuid>,
    /// Map-owned temporary Unit world-object switch queue matching C++
    /// `Map::i_objectsToSwitch` (`Map.h:651-652`) and
    /// `Map::AddObjectToSwitchList` (`Map.cpp:2557-2572`).
    ///
    /// Source of truth remains `map_objects`; callers representing
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
    /// `map_objects` remains the source-of-truth; these vectors preserve the
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

impl Map<NoopTerrainGridLoader, NoopGridLifecycle> {
    pub fn new(map_id: u32, instance_id: u32, spawn_mode: Difficulty, grid_expiry_ms: i64) -> Self {
        Self::with_hooks(
            map_id,
            instance_id,
            spawn_mode,
            grid_expiry_ms,
            true,
            100.0,
            NoopTerrainGridLoader,
            NoopGridLifecycle,
        )
    }
}

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    #[allow(clippy::too_many_arguments)]
    pub fn with_hooks(
        map_id: u32,
        instance_id: u32,
        spawn_mode: Difficulty,
        grid_expiry_ms: i64,
        grid_unload: bool,
        visible_distance: f32,
        terrain: Terrain,
        lifecycle: Lifecycle,
    ) -> Self {
        Self {
            map_id,
            instance_id,
            spawn_mode,
            grid_expiry_ms,
            grid_unload,
            visible_distance,
            grids: std::iter::repeat_with(|| None)
                .take(GRID_SLOT_COUNT)
                .collect(),
            terrain,
            lifecycle,
            active_cells: HashSet::new(),
            active_non_players_like_cpp: HashSet::new(),
            personal_phase_tracker: MultiPersonalPhaseTracker::default(),
            spawn_group_state: SpawnGroupRuntimeState::new(),
            respawn_store: RespawnStoreLikeCpp::new(),
            pool_data: SpawnedPoolDataLikeCpp::new(),
            grid_state_unloaded: false,
            corpse_data_loaded_like_cpp: false,
            creatures_by_spawn_id: HashMap::new(),
            gameobjects_by_spawn_id: HashMap::new(),
            area_triggers_by_spawn_id: HashMap::new(),
            map_objects: HashMap::new(),
            creature_group_holder_like_cpp: HashMap::new(),
            dynamic_tree_model_keys_like_cpp: HashSet::new(),
            dynamic_tree_rebalance_timer_remaining_ms_like_cpp:
                DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP,
            dynamic_tree_unbalanced_times_like_cpp: 0,
            objects_to_remove: HashSet::new(),
            objects_to_switch: HashMap::new(),
            far_spell_callbacks_like_cpp: VecDeque::new(),
            represented_far_spell_callback_execution_log_like_cpp: Vec::new(),
            creatures_to_move: Vec::new(),
            gameobjects_to_move: Vec::new(),
            dynamic_objects_to_move: Vec::new(),
            area_triggers_to_move: Vec::new(),
            creature_move_states: HashMap::new(),
            gameobject_move_states: HashMap::new(),
            dynamic_object_move_states: HashMap::new(),
            area_trigger_move_states: HashMap::new(),
            creature_move_lock: false,
            gameobject_move_lock: false,
            dynamic_object_move_lock: false,
            area_trigger_move_lock: false,
            script_schedule_like_cpp: BTreeMap::new(),
            script_schedule_lock_like_cpp: false,
            represented_executed_script_actions_like_cpp: Vec::new(),
            zone_dynamic_info_like_cpp: BTreeMap::new(),
            weather_update_timer_current_ms_like_cpp: 0,
            weather_update_timer_interval_ms_like_cpp: WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP,
            guid_generators: HashMap::new(),
            creature_level_rng_like_cpp: StdRng::from_entropy(),
        }
    }

    pub const fn map_id(&self) -> u32 {
        self.map_id
    }

    pub const fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub const fn spawn_mode(&self) -> Difficulty {
        self.spawn_mode
    }

    pub const fn corpse_data_loaded_like_cpp(&self) -> bool {
        self.corpse_data_loaded_like_cpp
    }

    pub fn mark_corpse_data_loaded_like_cpp(&mut self) {
        self.corpse_data_loaded_like_cpp = true;
    }

    /// Register a corpse produced by C++ `Map::LoadCorpseData`.
    ///
    /// `Map::AddCorpse` retains every loaded corpse by cell, but
    /// `ObjectWorldLoader` only calls `AddToWorld` when that corpse's grid is
    /// loaded. Rust keeps the typed record dormant in `map_objects` and
    /// activates it immediately only when the destination grid is already
    /// loaded (the async login bridge may finish after the player's grid load).
    pub fn register_loaded_corpse_like_cpp(
        &mut self,
        corpse: Corpse,
    ) -> Result<bool, AddToMapError> {
        let record = MapObjectRecord::new_corpse(corpse).map_err(MapObjectStoreError::from)?;
        let guid = record.object().guid();
        let position = record.object().position();
        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err(AddToMapError::InvalidCoordinates {
                guid,
                x: position.x,
                y: position.y,
            });
        }

        let grid = GridCoord::new(
            Cell::from_world(position.x, position.y).grid_x(),
            Cell::from_world(position.x, position.y).grid_y(),
        );
        self.insert_map_object_record(record)?;
        if self.is_grid_loaded(grid) {
            self.activate_registered_corpses_for_grid_like_cpp(grid);
        }

        Ok(self.object_is_in_world(guid))
    }

    fn activate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
        if !self.is_grid_loaded(grid) {
            return 0;
        }

        let corpses = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::Corpse
                    || record.object().object().is_in_world()
                {
                    return None;
                }
                let position = record.object().position();
                let cell = Cell::from_world(position.x, position.y);
                (GridCoord::new(cell.grid_x(), cell.grid_y()) == grid).then_some((
                    *guid,
                    cell,
                    record.object().is_world_object(),
                ))
            })
            .collect::<Vec<_>>();

        for (guid, cell, is_world_object) in &corpses {
            let ngrid = self
                .get_ngrid_mut(grid)
                .expect("registered corpse grid was checked as loaded");
            let local_cell = ngrid
                .get_grid_type_mut(cell.cell_x(), cell.cell_y())
                .expect("registered corpse coordinates must identify a local grid cell");
            insert_object_guid_in_cell_like_cpp(
                local_cell,
                AccessorObjectKind::Corpse,
                *is_world_object,
                *guid,
            );

            if let Some(corpse) = self
                .map_objects
                .get_mut(guid)
                .and_then(MapObjectRecord::corpse_mut)
            {
                corpse
                    .world_mut()
                    .set_current_cell(cell.cell_x(), cell.cell_y());
                corpse.world_mut().object_mut().add_to_world();
            }
        }

        corpses.len()
    }

    fn deactivate_registered_corpses_for_grid_like_cpp(&mut self, grid: GridCoord) -> usize {
        let corpses = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::Corpse
                    || !record.object().object().is_in_world()
                {
                    return None;
                }
                let position = record.object().position();
                let cell = Cell::from_world(position.x, position.y);
                (GridCoord::new(cell.grid_x(), cell.grid_y()) == grid).then_some(*guid)
            })
            .collect::<Vec<_>>();

        for guid in &corpses {
            if let Some(corpse) = self
                .map_objects
                .get_mut(guid)
                .and_then(MapObjectRecord::corpse_mut)
            {
                corpse.world_mut().object_mut().remove_from_world();
                corpse.world_mut().clear_current_cell();
            }
        }

        corpses.len()
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

    pub fn contains_gameobject_model_like_cpp(
        &self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> bool {
        self.dynamic_tree_model_keys_like_cpp.contains(&key)
    }

    /// Represents C++ `Map::InsertGameObjectModel` -> `DynamicMapTree::insert`.
    ///
    /// The real C++ tree receives a `GameObjectModel const&`; this represented
    /// seam stores a deterministic owner-GUID key only. A duplicate key is a
    /// guarded no-op, so represented count/unbalanced state cannot drift from
    /// repeated calls with the same owner GUID.
    pub fn insert_gameobject_model_like_cpp(
        &mut self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> DynamicMapTreeModelMutationOutcomeLikeCpp {
        let model_count_before = self.dynamic_tree_model_keys_like_cpp.len();
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let inserted = self.dynamic_tree_model_keys_like_cpp.insert(key);

        if inserted {
            self.dynamic_tree_unbalanced_times_like_cpp = self
                .dynamic_tree_unbalanced_times_like_cpp
                .saturating_add(1);
        }

        DynamicMapTreeModelMutationOutcomeLikeCpp {
            key,
            status: if inserted {
                DynamicMapTreeModelMutationStatusLikeCpp::Inserted
            } else {
                DynamicMapTreeModelMutationStatusLikeCpp::AlreadyPresent
            },
            model_count_before,
            model_count_after: self.dynamic_tree_model_keys_like_cpp.len(),
            unbalanced_before,
            unbalanced_after: self.dynamic_tree_unbalanced_times_like_cpp,
        }
    }

    /// Represents C++ `Map::RemoveGameObjectModel` -> `DynamicMapTree::remove`.
    ///
    /// C++ GameObject callers check containment before removal. Rust exposes a
    /// safe missing-key no-op at the facade so represented count cannot underflow.
    pub fn remove_gameobject_model_like_cpp(
        &mut self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> DynamicMapTreeModelMutationOutcomeLikeCpp {
        let model_count_before = self.dynamic_tree_model_keys_like_cpp.len();
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let removed = self.dynamic_tree_model_keys_like_cpp.remove(&key);

        if removed {
            self.dynamic_tree_unbalanced_times_like_cpp = self
                .dynamic_tree_unbalanced_times_like_cpp
                .saturating_add(1);
        }

        DynamicMapTreeModelMutationOutcomeLikeCpp {
            key,
            status: if removed {
                DynamicMapTreeModelMutationStatusLikeCpp::Removed
            } else {
                DynamicMapTreeModelMutationStatusLikeCpp::Missing
            },
            model_count_before,
            model_count_after: self.dynamic_tree_model_keys_like_cpp.len(),
            unbalanced_before,
            unbalanced_after: self.dynamic_tree_unbalanced_times_like_cpp,
        }
    }

    /// Represents C++ `GameObject::SetDisplayId(uint32)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3817-3820`. C++ first writes
    /// `GameObjectData::DisplayID`, then calls `UpdateModel()`. This map-owned
    /// caller seam preserves that order and delegates all represented model-key
    /// side effects to `update_gameobject_model_like_cpp`.
    pub fn set_gameobject_display_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        display_id: u32,
        new_has_model: bool,
        new_is_map_object: bool,
    ) -> GameObjectSetDisplayIdOutcomeLikeCpp {
        let Some(record) = self.map_objects.get(&guid) else {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::MissingGameObject,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::WrongKind,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        }

        let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::WrongKind,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        };

        let previous_display_id = game_object.data().display_id;
        game_object.set_display_id(display_id);
        let new_display_id = game_object.data().display_id;

        let update_model =
            self.update_gameobject_model_like_cpp(guid, new_has_model, new_is_map_object);

        GameObjectSetDisplayIdOutcomeLikeCpp {
            guid,
            status: GameObjectSetDisplayIdStatusLikeCpp::Updated,
            previous_display_id: Some(previous_display_id),
            new_display_id: Some(new_display_id),
            update_model: Some(update_model),
        }
    }

    /// Represents C++ `GameObject::SetGoState(GOState)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3771-3793`. Source-of-truth is
    /// `Map::map_objects`; this mutates only exact typed
    /// `MapObjectRecord::GameObject` records. The state write occurs before the
    /// represented `m_model && !IsTransport()` not-in-world early return, matching
    /// C++ statement order. Collision is never inferred from display/template/DB.
    pub fn set_gameobject_go_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: GoState,
    ) -> GameObjectSetGoStateOutcomeLikeCpp {
        let Some(record) = self.map_objects.get(&guid) else {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::MissingGameObject,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::WrongKind,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        }

        let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::WrongKind,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        };

        let previous_state = game_object.data().state;
        let represented_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let transport_type = gameobject_type_is_transport_like_cpp(game_object.data().type_id);
        game_object.set_go_state(state);
        let new_state = game_object.data().state;

        let (in_world_for_collision_branch, collision_enable) =
            if represented_model_present && !transport_type {
                let in_world = game_object.world().object().is_in_world();
                if in_world {
                    let collision = game_object
                        .enable_represented_gameobject_collision_like_cpp(state == GoState::Ready);
                    (
                        Some(true),
                        Some(GameObjectCollisionEnableOutcomeLikeCpp {
                            requested_enable: collision.requested_enable,
                            represented_model_present: collision.represented_model_present,
                            previous_collision_enabled: collision.previous_collision_enabled,
                            new_collision_enabled: collision.new_collision_enabled,
                        }),
                    )
                } else {
                    (Some(false), None)
                }
            } else {
                (None, None)
            };

        GameObjectSetGoStateOutcomeLikeCpp {
            guid,
            status: GameObjectSetGoStateStatusLikeCpp::Updated,
            previous_state: Some(previous_state),
            new_state: Some(new_state),
            represented_model_present,
            transport_type,
            in_world_for_collision_branch,
            collision_enable,
        }
    }

    /// Represents C++ `GameObject::SetLootState(LootState, Unit*)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3683-3709`. Source-of-truth is `Map::map_objects`;
    /// this mutates only exact typed `MapObjectRecord::GameObject` records. The `unit_guid`
    /// argument is only represented evidence for `unit->GetGUID()` and no real `Unit*` is
    /// resolved. Restock consumes explicit caller-supplied `Loot::IsChanged()` evidence; collision
    /// consumes only explicit represented `m_model` evidence and never real geometry/BIH.
    pub fn set_gameobject_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: LootState,
        unit_guid: Option<ObjectGuid>,
        game_time_secs: i64,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> GameObjectSetLootStateOutcomeLikeCpp {
        let Some(record) = self.map_objects.get(&guid) else {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::MissingGameObject,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::WrongKind,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        }

        let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::WrongKind,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        };

        let previous_loot_state = game_object.loot_state();
        let previous_loot_state_unit_guid = game_object.loot_state_unit_guid();
        let previous_restock_time = game_object.restock_time();
        let represented_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let type_id = game_object.data().type_id;

        game_object.set_loot_state(state, unit_guid);

        let restock_armed = type_id == GAMEOBJECT_TYPE_CHEST as i8
            && state == LootState::Activated
            && chest_restock_time_secs > 0
            && previous_restock_time == 0
            && shared_loot_is_changed_like_cpp;
        if restock_armed {
            let restock_time = game_time_secs.saturating_add(i64::from(chest_restock_time_secs));
            game_object.set_restock_time_like_cpp(restock_time);
        }

        let door_type_early_return = type_id == GAMEOBJECT_TYPE_DOOR as i8;
        let collision_enable = if door_type_early_return || !represented_model_present {
            None
        } else {
            let collision_enabled = (game_object.data().state != GoState::Ready as i8
                && (state == LootState::Activated || state == LootState::JustDeactivated))
                || state == LootState::Ready;
            let collision =
                game_object.enable_represented_gameobject_collision_like_cpp(collision_enabled);
            Some(GameObjectCollisionEnableOutcomeLikeCpp {
                requested_enable: collision.requested_enable,
                represented_model_present: collision.represented_model_present,
                previous_collision_enabled: collision.previous_collision_enabled,
                new_collision_enabled: collision.new_collision_enabled,
            })
        };

        GameObjectSetLootStateOutcomeLikeCpp {
            guid,
            status: GameObjectSetLootStateStatusLikeCpp::Updated,
            previous_loot_state: Some(previous_loot_state),
            new_loot_state: Some(game_object.loot_state()),
            previous_loot_state_unit_guid: Some(previous_loot_state_unit_guid),
            new_loot_state_unit_guid: Some(game_object.loot_state_unit_guid()),
            previous_restock_time: Some(previous_restock_time),
            new_restock_time: Some(game_object.restock_time()),
            ai_on_loot_state_changed_not_represented: true,
            restock_armed,
            represented_model_present,
            door_type_early_return,
            collision_enable,
        }
    }

    /// Represents C++ `GameObject::UpdateModel()` over canonical map-owned state.
    ///
    /// C++ anchors: `GameObject.cpp:3867-3880`, `GameObject.cpp:4394-4399`, and
    /// `GameObject.cpp:3818-3820`. The caller supplies explicit represented
    /// `CreateModel()` output; this helper never infers model existence or
    /// map-object-ness from display id, template, type or DB. Only exact typed
    /// `MapObjectRecord::GameObject` records are mutated; missing, untyped,
    /// wrong-kind and not-in-world records are explicit no-mutation outcomes.
    pub fn update_gameobject_model_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_has_model: bool,
        new_is_map_object: bool,
    ) -> GameObjectUpdateModelOutcomeLikeCpp {
        let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
        let Some(record) = self.map_objects.get(&guid) else {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::MissingGameObject,
                old_model_present: false,
                old_model_registered: false,
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::WrongKind,
                old_model_present: false,
                old_model_registered: false,
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        }

        let game_object = record
            .game_object()
            .expect("exact typed GameObject record checked above");
        if !game_object.world().object().is_in_world() {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::NotInWorld,
                old_model_present: game_object.has_represented_gameobject_model_like_cpp(),
                old_model_registered: self.contains_gameobject_model_like_cpp(key),
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        }

        let old_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let old_model_registered =
            old_model_present && self.contains_gameobject_model_like_cpp(key);
        let old_model_remove =
            old_model_registered.then(|| self.remove_gameobject_model_like_cpp(key));

        if let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        {
            // C++ removes `GO_FLAG_MAP_OBJECT`, deletes/nulls `m_model`, then
            // calls `CreateModel()`. The first call clears old map-object and
            // collision evidence; the second installs only the explicit new
            // model/map-object evidence and does not call `EnableCollision()`.
            game_object.apply_represented_gameobject_model_creation_like_cpp(false, false);
            game_object.apply_represented_gameobject_model_creation_like_cpp(
                new_has_model,
                new_is_map_object,
            );
        }

        let new_model_insert = new_has_model.then(|| self.insert_gameobject_model_like_cpp(key));

        GameObjectUpdateModelOutcomeLikeCpp {
            guid,
            status: GameObjectUpdateModelStatusLikeCpp::Updated,
            old_model_present,
            old_model_registered,
            old_model_remove,
            new_has_model,
            new_is_map_object,
            new_model_insert,
        }
    }

    /// Represents the first statement in C++ `Map::Update(uint32 t_diff)`:
    /// `_dynamicTree.update(t_diff)` (`Map.cpp:666-668`).
    ///
    /// This is C++-shaped map-owned state only. It mirrors
    /// `DynTreeImpl::update` (`DynamicTree.cpp:90-101`): return early when the
    /// represented model-key set is empty; otherwise consume a TimeTracker-like
    /// remaining timer; when passed, reset to `CHECK_TREE_PERIOD` (200ms) and
    /// clear `unbalanced_times` only if it was positive, representing `balance()`.
    /// No real BIH/collision/geometry runtime is claimed.
    pub fn update_dynamic_tree_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> DynamicMapTreeUpdateSummaryLikeCpp {
        let timer_before_ms = self.dynamic_tree_rebalance_timer_remaining_ms_like_cpp;
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let empty = self.dynamic_tree_model_keys_like_cpp.is_empty();

        if empty {
            return DynamicMapTreeUpdateSummaryLikeCpp {
                diff_ms,
                empty,
                timer_before_ms,
                timer_after_ms: timer_before_ms,
                timer_passed: false,
                timer_reset_to_ms: None,
                unbalanced_before,
                balanced: false,
                unbalanced_after: unbalanced_before,
            };
        }

        let timer_passed = diff_ms >= timer_before_ms;
        let mut timer_after_ms = timer_before_ms.saturating_sub(diff_ms);
        let mut balanced = false;
        let mut unbalanced_after = unbalanced_before;
        let timer_reset_to_ms = if timer_passed {
            timer_after_ms = DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP;
            if unbalanced_before > 0 {
                self.dynamic_tree_unbalanced_times_like_cpp = 0;
                unbalanced_after = 0;
                balanced = true;
            }
            Some(DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP)
        } else {
            None
        };

        self.dynamic_tree_rebalance_timer_remaining_ms_like_cpp = timer_after_ms;

        DynamicMapTreeUpdateSummaryLikeCpp {
            diff_ms,
            empty,
            timer_before_ms,
            timer_after_ms,
            timer_passed,
            timer_reset_to_ms,
            unbalanced_before,
            balanced,
            unbalanced_after,
        }
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

    /// Test seam: flip a cell-resident creature to not-in-world (post C++
    /// `RemoveFromWorld`) while leaving its record in the cell/store, so the
    /// cell-anchored `ObjectUpdater` still visits it and exercises the
    /// `NotInWorld` skip branch.
    #[cfg(test)]
    pub(crate) fn test_remove_creature_from_world_keep_cell_like_cpp(&mut self, guid: ObjectGuid) {
        if let Some(creature) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::creature_mut)
        {
            creature.unit_mut().remove_from_world_like_cpp();
        }
    }

    pub fn generate_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .generate())
    }

    pub fn get_max_low_guid_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> Result<i64, MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        Ok(self
            .guid_sequence_generator_like_cpp(high)
            .generator
            .next_after_max_used())
    }

    pub fn set_guid_sequence_like_cpp(
        &mut self,
        high: HighGuid,
        next: i64,
    ) -> Result<(), MapGuidSequenceErrorLikeCpp> {
        Self::ensure_map_guid_sequence_source_like_cpp(high)?;
        self.guid_sequence_generator_like_cpp(high)
            .generator
            .set(next);
        Ok(())
    }

    fn guid_sequence_generator_like_cpp(
        &mut self,
        high: HighGuid,
    ) -> &mut MapGuidSequenceGeneratorLikeCpp {
        self.guid_generators
            .entry(high)
            .or_insert_with(|| MapGuidSequenceGeneratorLikeCpp::new(high))
    }

    fn ensure_map_guid_sequence_source_like_cpp(
        high: HighGuid,
    ) -> Result<(), MapGuidSequenceErrorLikeCpp> {
        match high {
            HighGuid::WorldTransaction
            | HighGuid::StaticDoor
            | HighGuid::Transport
            | HighGuid::Conversation
            | HighGuid::Creature
            | HighGuid::Vehicle
            | HighGuid::Pet
            | HighGuid::GameObject
            | HighGuid::DynamicObject
            | HighGuid::AreaTrigger
            | HighGuid::Corpse
            | HighGuid::LootObject
            | HighGuid::SceneObject
            | HighGuid::Scenario
            | HighGuid::AIGroup
            | HighGuid::DynamicDoor
            | HighGuid::Vignette
            | HighGuid::CallForHelp
            | HighGuid::AIResource
            | HighGuid::AILock
            | HighGuid::AILockTicket
            | HighGuid::Cast => Ok(()),
            _ => Err(MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource { high }),
        }
    }

    pub const fn grid_expiry_ms(&self) -> i64 {
        self.grid_expiry_ms
    }

    pub const fn grid_unload(&self) -> bool {
        self.grid_unload
    }

    pub const fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    pub fn terrain(&self) -> &Terrain {
        &self.terrain
    }

    pub fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    pub fn personal_phase_tracker(&self) -> &MultiPersonalPhaseTracker {
        &self.personal_phase_tracker
    }

    #[cfg(test)]
    pub(crate) fn register_personal_phase_object_for_test(
        &mut self,
        phase_id: u32,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phase_tracker
            .register_tracked_object(phase_id, phase_owner, object);
    }

    #[cfg(test)]
    pub(crate) fn mark_personal_phases_for_deletion_for_test(&mut self, phase_owner: ObjectGuid) {
        self.personal_phase_tracker
            .mark_all_phases_for_deletion(phase_owner);
    }

    /// Map-owned bridge for C++ `Map::_respawnTimes` and the per-type respawn maps.
    ///
    /// C++ anchors:
    /// - `Map.h:472-480` returns zero when a respawn time is missing or the type has no map.
    /// - `Map.h:748-777` stores respawn queues/maps on `Map`; AreaTrigger has no respawn map.
    /// - `Map.cpp:2057-2150` adds, replaces, gets, removes, and unloads respawn info coherently.
    pub const fn respawn_store_like_cpp(&self) -> &RespawnStoreLikeCpp {
        &self.respawn_store
    }

    /// Mutable access to the map-owned respawn store for bounded tests/bridges.
    ///
    /// Future runtime callers must treat `Map` as the owner/source of truth and
    /// must not keep external respawn stores that later overwrite this state.
    pub fn respawn_store_like_cpp_mut(&mut self) -> &mut RespawnStoreLikeCpp {
        &mut self.respawn_store
    }

    pub fn add_respawn_info_like_cpp(
        &mut self,
        info: RespawnInfoLikeCpp,
    ) -> AddRespawnInfoOutcomeLikeCpp {
        self.respawn_store.add_respawn_info_like_cpp(info)
    }

    pub fn get_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> i64 {
        self.respawn_store
            .get_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn get_respawn_info_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&RespawnInfoLikeCpp> {
        self.respawn_store
            .get_respawn_info_like_cpp(object_type, spawn_id)
    }

    /// C++ `Map::GetLinkedRespawnTime` dependency slice.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:3607-3620`.
    /// The linked respawn store is read-only ObjectMgr-style metadata; the timer
    /// source of truth remains this `Map`'s map-owned `RespawnStoreLikeCpp`.
    pub fn get_linked_respawn_time_like_cpp(
        &self,
        guid: ObjectGuid,
        linked_store: &LinkedRespawnStoreLikeCpp,
    ) -> i64 {
        let linked_guid = linked_store.get_linked_respawn_guid_like_cpp(guid);
        match linked_guid.high_type() {
            HighGuid::Creature => self.get_respawn_time_like_cpp(
                SpawnObjectType::Creature,
                linked_guid.counter() as SpawnId,
            ),
            HighGuid::GameObject => self.get_respawn_time_like_cpp(
                SpawnObjectType::GameObject,
                linked_guid.counter() as SpawnId,
            ),
            _ => 0,
        }
    }

    /// Linked-respawn branch from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchor: `/home/server/woltk-trinity-legacy/src/server/game/Maps/Map.cpp:2004-2020`.
    /// This implements only the linked-time guard after earlier live-object
    /// blockers have already cleared. It never runs PoolMgr, DoRespawn, DB
    /// save/delete, entity creation, fanout, or RNG; the caller supplies the
    /// explicit jitter that represents C++ `urand(5, 15)`.
    pub fn check_respawn_linked_respawn_guard_like_cpp(
        &self,
        info: &mut RespawnInfoLikeCpp,
        linked_store: &LinkedRespawnStoreLikeCpp,
        now: i64,
        jitter_secs: u32,
    ) -> CheckRespawnLinkedRespawnGuardOutcomeLikeCpp {
        let Some(guid_high) = (match info.object_type {
            SpawnObjectType::Creature => Some(HighGuid::Creature),
            SpawnObjectType::GameObject => Some(HighGuid::GameObject),
            SpawnObjectType::AreaTrigger => None,
        }) else {
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::UnsupportedSpawnType;
        };

        let this_guid = ObjectGuid::create_world_object(
            guid_high,
            0,
            0,
            self.map_id as u16,
            0,
            info.entry,
            info.spawn_id as i64,
        );
        let linked_time = self.get_linked_respawn_time_like_cpp(this_guid, linked_store);
        if linked_time == 0 {
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed;
        }

        if linked_time == i64::MAX {
            info.respawn_time = linked_time;
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite;
        }

        if linked_store.get_linked_respawn_guid_like_cpp(this_guid) == this_guid {
            info.respawn_time = now + WEEK_SECS_LIKE_CPP;
            return CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn;
        }

        info.respawn_time = now.max(linked_time) + i64::from(jitter_secs);
        CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed
    }

    pub fn remove_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<RespawnInfoLikeCpp> {
        self.respawn_store
            .remove_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn unload_all_respawn_infos_like_cpp(&mut self) {
        self.respawn_store.unload_all_respawn_infos_like_cpp();
    }

    pub fn respawn_timer_keys_like_cpp(
        &self,
    ) -> impl Iterator<Item = (SpawnObjectType, SpawnId)> + '_ {
        self.respawn_store.respawn_timer_keys_like_cpp()
    }

    /// Delegates the C++ `Map::ProcessRespawns` action planner to the map-owned store.
    ///
    /// This only plans side effects. It does not execute PoolMgr, DoRespawn,
    /// DB persistence/delete, linked-respawn checks, entity creation, or fanout.
    pub fn process_due_respawns_like_cpp(
        &mut self,
        now: i64,
        is_part_of_pool: impl FnMut(SpawnObjectType, SpawnId) -> Option<u32>,
        check_respawn: impl FnMut(&mut RespawnInfoLikeCpp) -> CheckRespawnOutcomeLikeCpp,
    ) -> Vec<ProcessRespawnActionLikeCpp> {
        self.respawn_store
            .process_due_respawns_like_cpp(now, is_part_of_pool, check_respawn)
    }

    /// Executes the safe map-local half of actions returned by represented C++
    /// `PoolMgr::UpdatePool` planning.
    ///
    /// C++ anchors:
    /// - `PoolMgr.cpp:183-257` `DespawnObject` / `Despawn1Object` removes
    ///   current map objects and optionally removes respawn timers.
    /// - `PoolMgr.cpp:353-403` `Spawn1Object` / `ReSpawn1Object` create only
    ///   on loaded grids; RustyCore reports that missing runtime instead of
    ///   creating DB-backed entities in `wow-map`.
    fn apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp::<
            fn(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
        >(plan, spawn_store, summary, None);
    }

    fn apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolTypedSpawnPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_spawn_object_plan_loaded_grid_records_like_cpp(
                object_plan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
            );
        }
    }

    fn apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolSpawnPoolPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        for subplan in &plan.subplans {
            self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                subplan,
                spawn_store,
                summary,
                load_record.as_deref_mut(),
            );
        }
    }

    fn apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnPoolPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        for subplan in &plan.subplans {
            self.apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(subplan, summary);
        }
    }

    fn apply_pool_typed_despawn_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolTypedDespawnPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        if let Some(object_plan) = plan.object_plan.as_ref() {
            self.apply_pool_despawn_object_plan_safe_map_actions_like_cpp(object_plan, summary);
        }
    }

    fn apply_pool_despawn_object_plan_safe_map_actions_like_cpp(
        &mut self,
        plan: &PoolDespawnObjectPlanLikeCpp,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let mut child_pool_plans = plan.child_pool_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_pool_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                other => match other {
                    PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                        self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                    }
                    PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                        let Some(object_type) =
                            pool_member_kind_to_spawn_object_type_like_cpp(kind)
                        else {
                            return;
                        };
                        if self
                            .remove_respawn_time_like_cpp(object_type, guid as SpawnId)
                            .is_some()
                        {
                            summary.pool_respawn_timers_removed += 1;
                        } else {
                            summary.pool_respawn_timers_missing += 1;
                        }
                    }
                    PoolSpawnObjectActionLikeCpp::SpawnOne { .. }
                    | PoolSpawnObjectActionLikeCpp::RespawnOne { .. } => {}
                },
            }
        }
    }

    fn apply_pool_spawn_object_plan_loaded_grid_records_like_cpp<L>(
        &mut self,
        plan: &PoolSpawnObjectPlanLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        mut load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut child_spawn_plans = plan.child_pool_spawn_plans.iter();
        let mut child_despawn_plans = plan.child_pool_despawn_plans.iter();
        for action in &plan.actions {
            match *action {
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_spawn_plans.next() {
                        self.apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp(
                            child_plan,
                            spawn_store,
                            summary,
                            load_record.as_deref_mut(),
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::DespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {
                    if let Some(child_plan) = child_despawn_plans.next() {
                        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(
                            child_plan, summary,
                        );
                    } else {
                        summary.pool_unsupported_action_kind += 1;
                    }
                }
                PoolSpawnObjectActionLikeCpp::RespawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                }
                | PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::Pool,
                    ..
                } => {}
                other => self.apply_pool_spawn_object_action_loaded_grid_records_like_cpp(
                    other,
                    spawn_store,
                    summary,
                    load_record.as_deref_mut(),
                ),
            }
        }
    }

    fn apply_pool_spawn_object_action_loaded_grid_records_like_cpp<L>(
        &mut self,
        action: PoolSpawnObjectActionLikeCpp,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        match action {
            PoolSpawnObjectActionLikeCpp::DespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
            }
            PoolSpawnObjectActionLikeCpp::RespawnOne { kind, guid } => {
                self.apply_pool_despawn_one_safe_map_action_like_cpp(kind, guid, summary);
                self.report_pool_spawn_one_action_like_cpp(
                    kind,
                    guid,
                    true,
                    spawn_store,
                    summary,
                    load_record,
                );
            }
            PoolSpawnObjectActionLikeCpp::RemoveRespawnTime { kind, guid } => {
                let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
                    return;
                };
                if self
                    .remove_respawn_time_like_cpp(object_type, guid as SpawnId)
                    .is_some()
                {
                    summary.pool_respawn_timers_removed += 1;
                } else {
                    summary.pool_respawn_timers_missing += 1;
                }
            }
            PoolSpawnObjectActionLikeCpp::SpawnOne { kind, guid } => {
                self.report_pool_spawn_one_action_like_cpp(
                    kind,
                    guid,
                    false,
                    spawn_store,
                    summary,
                    load_record,
                );
            }
        }
    }

    fn apply_pool_despawn_one_safe_map_action_like_cpp(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
    ) {
        let spawn_id = spawn_id as SpawnId;
        let guids = match kind {
            PoolMemberKindLikeCpp::Creature => {
                self.creature_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::GameObject => {
                self.gameobject_spawn_id_store_guids_like_cpp(spawn_id)
            }
            PoolMemberKindLikeCpp::Pool => {
                summary.pool_unsupported_action_kind += 1;
                return;
            }
        };

        for guid in guids {
            if self.map_object_record(guid).is_none() {
                summary.pool_stale_index_entries += 1;
                continue;
            }
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(_removed) => {
                    summary.pool_objects_removed += 1;
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    summary.pool_stale_index_entries += 1;
                }
                Err(_error) => {
                    summary.pool_remove_errors += 1;
                }
            }
        }
    }

    fn report_pool_spawn_one_action_like_cpp<L>(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: u64,
        respawn: bool,
        spawn_store: &SpawnStore,
        summary: &mut ProcessRespawnsSafeSideEffectsSummaryLikeCpp,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(object_type) = pool_member_kind_to_spawn_object_type_like_cpp(kind) else {
            summary.pool_unsupported_action_kind += 1;
            return;
        };
        let spawn_id = spawn_id as SpawnId;
        let Some(spawn_data) = spawn_store.spawn_data(object_type, spawn_id) else {
            summary.pool_spawn_actions_missing_spawn_data += 1;
            return;
        };
        let cell = cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        if !self.is_grid_loaded(grid) {
            summary.pool_spawn_actions_skipped_unloaded_grid += 1;
            return;
        }

        let Some(load_record) = load_record else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        let Some(records) = load_record(self, object_type, spawn_id) else {
            summary.pool_spawn_actions_blocked_loaded_grid += 1;
            summary
                .pool_spawn_action_load_plans
                .push(PoolSpawnActionLoadPlanLikeCpp {
                    object_type,
                    spawn_id,
                    respawn,
                });
            return;
        };

        for pre_add_record in records.pre_add_records {
            let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
        }
        let primary_record = records.primary_record;
        let loaded_grid_primary_record = primary_record.clone();
        match self.add_map_object_record_to_map_like_cpp(primary_record) {
            Ok(_outcome) => {
                summary.executed_loaded_grid_respawns += 1;
                summary
                    .loaded_grid_primary_records
                    .push(loaded_grid_primary_record);
            }
            Err(_error) => {
                summary.blocked_loaded_grid_respawn_add_to_map += 1;
            }
        }
    }

    /// Safe side-effect seam for represented C++ `Map::ProcessRespawns` branches.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2191-2198` processes only due respawn timers in queue order.
    /// - `Map.cpp:2200-2211` detects `PoolMgr::IsPartOfAPool` before
    ///   `CheckRespawn`, updates map-owned `SpawnedPoolData` through
    ///   `PoolMgr::UpdatePool`, then removes the respawn timer with DB-delete
    ///   ownership left to the caller bridge.
    /// - `Map.cpp:2213-2224` allowed respawn removes+calls `DoRespawn`; blocked here.
    /// - `Map.cpp:2226-2231` removes a timer when `CheckRespawn` set respawnTime=0.
    /// - `Map.cpp:2233-2238` updates the heap position and persists a future
    ///   `respawnTime` when `CheckRespawn` rescheduled the timer.
    ///
    /// This helper executes only safe map-owned in-memory effects represented so
    /// far: pooled timer -> deterministic `UpdatePool` plan + map-owned
    /// `SpawnedPoolDataLikeCpp` mutation + timer removal, `DoRespawn`'s unloaded-grid
    /// early return after timer removal, loaded-grid non-pooled `DoRespawn` via a
    /// caller-supplied typed `MapObjectRecord` loader, zero-delete for inactive
    /// spawn-groups/live-object blockers, and linked-respawn future reschedule by
    /// replacing the same map-owned respawn timer. DB effects, live record
    /// construction, grid/session fanout, and scripts stay outside this lock-owned
    /// helper.
    /// `consume_due_timer_on_load_failure_like_cpp` selects the live C++ path,
    /// where `ProcessRespawns` has already popped the timer before a failed
    /// `LoadFromDB`, versus the older represented safe wrapper, which has no
    /// loader at all and must leave the timer intact rather than discard work.
    pub fn process_due_respawns_composite_loaded_grid_respawns_like_cpp<F, R, C, L>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        pool_mgr: &PoolMgrLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
        mut explicit_roll_for: R,
        mut choose_equal: C,
        consume_due_timer_on_load_failure_like_cpp: bool,
        mut load_record: L,
    ) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

        loop {
            let next_key = { self.respawn_timer_keys_like_cpp().next() };
            let Some((object_type, spawn_id)) = next_key else {
                break;
            };
            let Some(info) = self
                .get_respawn_info_like_cpp(object_type, spawn_id)
                .cloned()
            else {
                summary.blocked_missing_spawn_data += 1;
                break;
            };
            if now < info.respawn_time {
                break;
            }

            match pool_mgr.is_part_of_a_pool_like_cpp(object_type, spawn_id) {
                Ok(0) => {}
                Ok(pool_id) => match pool_mgr.update_pool_plan_like_cpp(
                    &mut self.pool_data,
                    pool_id,
                    object_type,
                    spawn_id,
                    &mut explicit_roll_for,
                    &mut choose_equal,
                ) {
                    Ok(plan) => {
                        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                            Some(&mut load_record),
                        );
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        summary.processed_pool_timers += 1;
                        summary.pool_update_plans.push(plan);
                        continue;
                    }
                    Err(error) => {
                        summary.blocked_pool_plan_errors.push(error);
                        break;
                    }
                },
                Err(error) => {
                    summary.blocked_pool_plan_errors.push(error);
                    break;
                }
            }

            if spawn_store.spawn_data(object_type, spawn_id).is_none() {
                summary.blocked_missing_spawn_data += 1;
                if consume_due_timer_on_load_failure_like_cpp {
                    // C++ pops the due timer before `DoRespawn`; a stale DB
                    // spawn makes `LoadFromDB` fail, but it must not pin the
                    // queue head and starve every later respawn on the map.
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    continue;
                }
                break;
            }

            let mut checked_info = info;
            match self.check_respawn_like_cpp(
                &mut checked_info,
                spawn_store,
                linked_store,
                now,
                jitter_secs,
                respawn_dynamic_escortnpc,
                &mut is_creature_escorted,
            ) {
                CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
                    if checked_info.respawn_time == 0 =>
                {
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.deleted_inactive_spawn_group += 1;
                }
                CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn
                    if checked_info.respawn_time == 0 =>
                {
                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.deleted_live_object_blocker += 1;
                }
                CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer
                | CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn => {
                    summary.blocked_do_respawn_runtime += 1;
                    break;
                }
                CheckRespawnCompositeOutcomeLikeCpp::Allowed => {
                    if is_grid_id_loaded(self, checked_info.grid_id) {
                        let Some(records) = load_record(self, object_type, spawn_id) else {
                            summary.blocked_loaded_grid_respawn_loads += 1;
                            summary.blocked_do_respawn_runtime += 1;
                            // `Map::ProcessRespawns` erases the timer before
                            // `DoRespawn`; `Creature/GameObject::LoadFromDB`
                            // failure deletes the temporary object and the loop
                            // continues with the next due timer.
                            if consume_due_timer_on_load_failure_like_cpp {
                                self.remove_respawn_time_like_cpp(object_type, spawn_id);
                                continue;
                            }
                            break;
                        };

                        // C++ `ProcessRespawns` pops/erases the timer before
                        // calling `DoRespawn`. For DB-backed GameObjects,
                        // `GameObject::Create` may also create and AddToMap a
                        // linked trap first; that AddToMap failure only deletes
                        // the trap and does not block the owner. The primary
                        // `AddToMap` result remains determinant as in C++.
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        for pre_add_record in records.pre_add_records {
                            let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
                        }
                        let primary_record = records.primary_record;
                        let loaded_grid_primary_record = primary_record.clone();
                        match self.add_map_object_record_to_map_like_cpp(primary_record) {
                            Ok(_outcome) => {
                                summary.executed_loaded_grid_respawns += 1;
                                summary
                                    .loaded_grid_primary_records
                                    .push(loaded_grid_primary_record);
                            }
                            Err(_error) => {
                                summary.blocked_loaded_grid_respawn_add_to_map += 1;
                            }
                        }
                        continue;
                    }

                    self.remove_respawn_time_like_cpp(object_type, spawn_id);
                    summary.processed_unloaded_grid_respawns += 1;
                    continue;
                }
                CheckRespawnCompositeOutcomeLikeCpp::LinkedInfinite
                | CheckRespawnCompositeOutcomeLikeCpp::LinkedSelfNeverRespawn
                | CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed => {
                    if checked_info.respawn_time == i64::MAX || checked_info.respawn_time > now {
                        let rescheduled_info = checked_info.clone();
                        self.remove_respawn_time_like_cpp(object_type, spawn_id);
                        self.add_respawn_info_like_cpp(checked_info);
                        summary.rescheduled_linked_respawns.push(rescheduled_info);
                    } else {
                        summary.blocked_linked_respawn_non_future += 1;
                        break;
                    }
                }
                CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData => {
                    summary.blocked_missing_spawn_data += 1;
                    break;
                }
                CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType => {
                    summary.blocked_unsupported_spawn_type += 1;
                    break;
                }
            }
        }

        summary
    }

    /// Compatibility wrapper that preserves the old safe-side-effects API by
    /// keeping loaded-grid non-pooled `DoRespawn` blocked through a loader that
    /// returns no typed record.
    pub fn process_due_respawns_composite_safe_side_effects_like_cpp<F, R, C>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        pool_mgr: &PoolMgrLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        is_creature_escorted: F,
        explicit_roll_for: R,
        choose_equal: C,
    ) -> ProcessRespawnsSafeSideEffectsSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    {
        self.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
            now,
            spawn_store,
            linked_store,
            pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            is_creature_escorted,
            explicit_roll_for,
            choose_equal,
            false,
            |_map, _object_type, _spawn_id| None,
        )
    }

    /// Compatibility wrapper for callers that still use the old delete-only name.
    pub fn process_due_respawns_composite_delete_only_like_cpp<F>(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        is_creature_escorted: F,
    ) -> ProcessRespawnsDeleteOnlySummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        let pool_mgr = PoolMgrLikeCpp::new();
        self.process_due_respawns_composite_safe_side_effects_like_cpp(
            now,
            spawn_store,
            linked_store,
            &pool_mgr,
            jitter_secs,
            respawn_dynamic_escortnpc,
            is_creature_escorted,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
    }

    /// Compatibility wrapper for the original inactive-spawn-group delete-only seam.
    pub fn process_due_respawns_spawn_group_delete_only_like_cpp(
        &mut self,
        now: i64,
        spawn_store: &SpawnStore,
    ) -> ProcessRespawnsDeleteOnlySummaryLikeCpp {
        let linked_store = LinkedRespawnStoreLikeCpp::new();
        self.process_due_respawns_composite_safe_side_effects_like_cpp(
            now,
            spawn_store,
            &linked_store,
            &PoolMgrLikeCpp::new(),
            5,
            false,
            |_, _| false,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
        )
    }

    /// First represented guard from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1956-1957` resolves `SpawnData` and asserts when missing.
    /// - `Map.cpp:1959-1964` clears `respawnTime` and returns false when the
    ///   spawn group is inactive.
    ///
    /// This is only the spawn-group subdependency of `CheckRespawn`. It does not
    /// implement live by-spawn existence, escort dynamic rules, gameobject live
    /// checks, linked respawn, random 5-15 reschedule, PoolMgr, `DoRespawn`, DB
    /// save/delete, or world-server tick integration. Missing `SpawnData` is a
    /// temporary defensive fallback for incomplete ownership: C++ would assert;
    /// RustyCore returns `MissingSpawnData`, does not mutate `respawn_time`, and
    /// leaves timer deletion/reschedule decisions to the caller.
    pub fn check_respawn_spawn_group_guard_like_cpp(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
    ) -> CheckRespawnSpawnGroupGuardOutcomeLikeCpp {
        let Some(spawn_data) = spawn_store.spawn_data(info.object_type, info.spawn_id) else {
            return CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData;
        };

        if !self.is_spawn_group_active_like_cpp(Some(&spawn_data.spawn_group)) {
            info.respawn_time = 0;
            return CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer;
        }

        CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed
    }

    /// Live object existence guard from C++ `Map::CheckRespawn`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1966-2002` checks whether an already-live creature/gameobject
    ///   with the same spawn id blocks respawn, clears `respawnTime`, and returns
    ///   false when blocked.
    /// - `Map.cpp:1972-1983` allows dynamic escort NPC respawn only when the
    ///   matching live creature is already escorting.
    ///
    /// Source of truth for this slice is canonical map-owned `map_objects`, with
    /// typed map-local by-spawn-id indexes mirroring Trinity's multimap stores.
    /// Callers must provide the `CONFIG_RESPAWN_DYNAMIC_ESCORTNPC` value and the
    /// real escort runtime predicate; this helper does not invent
    /// `Creature::IsEscorted`, PoolMgr, linked respawn, `DoRespawn`, DB writes, or
    /// fanout side effects.
    pub fn check_respawn_live_object_guard_like_cpp<F>(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
    ) -> CheckRespawnLiveObjectGuardOutcomeLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        let Some(spawn_data) = spawn_store.spawn_data(info.object_type, info.spawn_id) else {
            return CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData;
        };

        match info.object_type {
            SpawnObjectType::Creature => {
                let is_escort = respawn_dynamic_escortnpc
                    && spawn_data
                        .spawn_group
                        .flags
                        .contains(SpawnGroupFlags::ESCORTQUESTNPC);

                let Some(creature_guids) = self.creatures_by_spawn_id.get(&info.spawn_id) else {
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed;
                };

                for guid in creature_guids {
                    let Some(record) = self.map_objects.get(guid) else {
                        continue;
                    };
                    let Some(creature) = record.creature() else {
                        continue;
                    };
                    if creature.spawn_id() != info.spawn_id || !creature.is_alive() {
                        continue;
                    }
                    if is_escort && is_creature_escorted(creature.guid(), creature) {
                        continue;
                    }

                    info.respawn_time = 0;
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn;
                }

                CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
            }
            SpawnObjectType::GameObject => {
                if self
                    .gameobjects_by_spawn_id
                    .get(&info.spawn_id)
                    .is_some_and(|gameobject_guids| {
                        gameobject_guids.iter().any(|guid| {
                            self.map_objects.get(guid).is_some_and(|record| {
                                record.game_object().is_some_and(|gameobject| {
                                    gameobject.spawn_id() == info.spawn_id
                                })
                            })
                        })
                    })
                {
                    info.respawn_time = 0;
                    return CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn;
                }

                CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed
            }
            SpawnObjectType::AreaTrigger => {
                CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType
            }
        }
    }

    /// Composite helper preserving represented C++ `Map::CheckRespawn` guard order.
    ///
    /// C++ anchors:
    /// - `Map.cpp:1950-2023` defines the full return/mutate contract.
    /// - `Map.cpp:1956-1964` checks spawn-group activity first.
    /// - `Map.cpp:1966-2002` checks live object blockers second.
    /// - `Map.cpp:2004-2020` checks linked respawn only after earlier guards allow.
    ///
    /// Runtime timer source of truth is this map-owned `RespawnStoreLikeCpp` via
    /// `RespawnInfoLikeCpp`; metadata stays caller-supplied `SpawnStore` until
    /// ObjectMgr ownership moves into `Map`; live blockers come from `map_objects`;
    /// linked metadata is read-only. This helper deliberately does not execute
    /// PoolMgr, `DoRespawn`, DB save/delete, entity creation, fanout, or RNG.
    pub fn check_respawn_like_cpp<F>(
        &self,
        info: &mut RespawnInfoLikeCpp,
        spawn_store: &SpawnStore,
        linked_store: &LinkedRespawnStoreLikeCpp,
        now: i64,
        jitter_secs: u32,
        respawn_dynamic_escortnpc: bool,
        mut is_creature_escorted: F,
    ) -> CheckRespawnCompositeOutcomeLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> bool,
    {
        if matches!(info.object_type, SpawnObjectType::AreaTrigger) {
            return CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType;
        }

        match self.check_respawn_spawn_group_guard_like_cpp(info, spawn_store) {
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::Allowed => {}
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer => {
                return CheckRespawnCompositeOutcomeLikeCpp::InactiveSpawnGroupDeletedTimer;
            }
            CheckRespawnSpawnGroupGuardOutcomeLikeCpp::MissingSpawnData => {
                return CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData;
            }
        }

        match self.check_respawn_live_object_guard_like_cpp(
            info,
            spawn_store,
            respawn_dynamic_escortnpc,
            &mut is_creature_escorted,
        ) {
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed => {}
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn => {
                return CheckRespawnCompositeOutcomeLikeCpp::AliveCreatureBlocksRespawn;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::GameObjectBlocksRespawn => {
                return CheckRespawnCompositeOutcomeLikeCpp::GameObjectBlocksRespawn;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::MissingSpawnData => {
                return CheckRespawnCompositeOutcomeLikeCpp::MissingSpawnData;
            }
            CheckRespawnLiveObjectGuardOutcomeLikeCpp::UnsupportedSpawnType => {
                return CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType;
            }
        }

        match self.check_respawn_linked_respawn_guard_like_cpp(info, linked_store, now, jitter_secs)
        {
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::Allowed => {
                CheckRespawnCompositeOutcomeLikeCpp::Allowed
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedInfinite => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedInfinite
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedSelfNeverRespawn => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedSelfNeverRespawn
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::LinkedDelayed => {
                CheckRespawnCompositeOutcomeLikeCpp::LinkedDelayed
            }
            CheckRespawnLinkedRespawnGuardOutcomeLikeCpp::UnsupportedSpawnType => {
                CheckRespawnCompositeOutcomeLikeCpp::UnsupportedSpawnType
            }
        }
    }

    /// Map-owned bridge for C++ `Map::_toggledSpawnGroupIds`.
    ///
    /// C++ anchors:
    /// - `Map.h:780-781` stores toggled spawn group ids on `Map`.
    /// - `Map.cpp:2427-2439` toggles only non-system existing groups.
    /// - `Map.cpp:2441-2453` queries missing/system/default/manual semantics.
    ///
    /// RustyCore does not yet wire ObjectMgr/SpawnStore ownership into `Map`, so
    /// callers must pass the already-resolved template as an honest bridge.
    pub const fn spawn_group_state(&self) -> &SpawnGroupRuntimeState {
        &self.spawn_group_state
    }

    pub fn set_spawn_group_active_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        state: bool,
    ) -> SpawnGroupActiveChange {
        self.spawn_group_state
            .set_spawn_group_active_like_cpp(group, state)
    }

    pub fn set_spawn_group_inactive_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
    ) -> SpawnGroupActiveChange {
        self.set_spawn_group_active_like_cpp(group, false)
    }

    pub fn is_spawn_group_active_like_cpp(&self, group: Option<&SpawnGroupTemplateData>) -> bool {
        self.spawn_group_state.is_spawn_group_active_like_cpp(group)
    }

    pub const fn pool_data_like_cpp(&self) -> &SpawnedPoolDataLikeCpp {
        &self.pool_data
    }

    pub const fn pool_data_mut_like_cpp(&mut self) -> &mut SpawnedPoolDataLikeCpp {
        &mut self.pool_data
    }

    /// Map-owned facade for a direct C++ `PoolMgr::DespawnPool(spawns, pool_id,
    /// alwaysDeleteRespawnTime)` call.
    ///
    /// Ownership stays one-way: `PoolMgrLikeCpp` plans and mutates only this
    /// map's canonical `SpawnedPoolDataLikeCpp`; `Map` then applies only safe
    /// map-local Creature/GameObject removal and respawn-timer deletion actions
    /// already represented by the plan. It does not fabricate live records,
    /// persist DB state, or fan out packets/scripts/AI.
    pub fn despawn_pool_safe_map_actions_like_cpp(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        always_delete_respawn_time: bool,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let plan = pool_mgr.despawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            always_delete_respawn_time,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_despawn_pool_plan_safe_map_actions_like_cpp(&plan, &mut summary);
        Ok(summary)
    }

    /// Map-owned facade for a direct C++ `PoolMgr::SpawnPool(spawns, pool_id)`
    /// call over an already loaded canonical map.
    ///
    /// Ownership stays one-way: caller-owned canonical metadata and
    /// `PoolMgrLikeCpp` feed a deterministic `SpawnPool` plan that mutates this
    /// map's canonical `SpawnedPoolDataLikeCpp`; `Map` then consumes only
    /// loaded-grid `Spawn1Object`/recursive child-pool actions through the
    /// caller-supplied typed record loader. `wow-map` does not read DB, create
    /// dummy records, persist state, touch sessions/ObjectAccessor, or fan out.
    pub fn spawn_pool_loaded_grid_records_like_cpp<L>(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        pool_id: u32,
        spawn_store: &SpawnStore,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        mut load_record: L,
    ) -> Result<ProcessRespawnsSafeSideEffectsSummaryLikeCpp, PoolMgrPlanErrorLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let plan = pool_mgr.spawn_pool_plan_like_cpp(
            &mut self.pool_data,
            pool_id,
            explicit_roll_for,
            choose_equal,
        )?;
        let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
        self.apply_pool_spawn_pool_plan_loaded_grid_records_like_cpp(
            &plan,
            spawn_store,
            &mut summary,
            Some(&mut load_record),
        );
        Ok(summary)
    }

    /// C++ `Map` constructor calls `sPoolMgr->InitPoolsForMap(this)` before
    /// startup respawn and spawn-group initialization. This represented seam
    /// applies deterministic autospawn `SpawnPool` plans into the map-owned
    /// `SpawnedPoolDataLikeCpp` and returns action records for future live
    /// `Spawn1Object`/`ReSpawn1Object`/`DespawnObject` owners; it does not create
    /// entities or fan out packets.
    pub fn init_pools_for_map_like_cpp(
        &mut self,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> PoolInitForMapPlanLikeCpp {
        pool_mgr.init_pools_for_map_plan_like_cpp(
            self.map_id,
            &mut self.pool_data,
            explicit_roll_for,
            choose_equal,
        )
    }

    /// Bridge for C++ `Map::ShouldBeSpawnedOnGridLoad` callers while `Map` does
    /// not yet own the ObjectMgr spawn metadata. The canonical toggle state,
    /// respawn timers, and `SpawnedPoolData` are map-owned; spawn metadata remains
    /// caller-supplied.
    pub fn spawn_grid_load_state_like_cpp<'a>(
        &'a self,
        spawn_store: &'a SpawnStore,
    ) -> SpawnGridLoadStateLikeCpp<'a> {
        SpawnGridLoadStateLikeCpp::new(spawn_store, &self.spawn_group_state)
            .with_respawn_timers(self.respawn_store.respawn_timer_keys_like_cpp())
            .with_pool_spawned_objects(self.pool_data.spawned_objects_like_cpp())
    }

    /// Pure bridge for C++ `Map::InitSpawnGroupState` over pre-resolved group
    /// templates. It intentionally applies only active-state toggles; live
    /// spawn/despawn, pool runtime, respawn persistence, and fanout are later gaps.
    pub fn init_spawn_group_state_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        mut meets_conditions: F,
    ) -> Vec<(u32, SpawnGroupActiveChange)>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let mut changes = Vec::new();
        for group in groups {
            if group.is_system() {
                continue;
            }
            let active = meets_conditions(group);
            changes.push((
                group.group_id,
                self.set_spawn_group_active_like_cpp(Some(group), active),
            ));
        }
        changes
    }

    /// Pure action planner for C++ `Map::UpdateSpawnGroupConditions` over
    /// pre-resolved spawn-group templates.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2471-2502` loops map groups, compares
    ///   `IsSpawnGroupActive` with `ConditionMgr`, and runs spawn/despawn or
    ///   inactive branches.
    /// - `Map.cpp:2427-2453` owns `_toggledSpawnGroupIds` semantics through
    ///   `SetSpawnGroupActive` / `IsSpawnGroupActive`.
    /// - `SpawnData.h:51-63` defines manual and condition-failure flags.
    ///
    /// This does not run live `SpawnGroupSpawn`/`SpawnGroupDespawn`, touch DB,
    /// mutate toggles, simulate pools, persist respawns, create entities, or
    /// fan out updates. The closure only replaces C++
    /// `ConditionMgr::IsMapMeetingNotGroupedConditions` for already-resolved
    /// condition outcomes.
    pub fn plan_update_spawn_group_conditions_like_cpp<'a, I, F>(
        &self,
        groups: I,
        mut meets_conditions: F,
    ) -> Vec<(u32, SpawnGroupConditionActionLikeCpp)>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let mut actions = Vec::new();
        for group in groups {
            let is_active = self.is_spawn_group_active_like_cpp(Some(group));
            let should_be_active = meets_conditions(group);

            if group.flags.contains(SpawnGroupFlags::MANUAL_SPAWN) {
                if is_active
                    && !should_be_active
                    && group
                        .flags
                        .contains(SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE)
                {
                    actions.push((
                        group.group_id,
                        SpawnGroupConditionActionLikeCpp::condition_failure_despawn(),
                    ));
                } else {
                    actions.push((group.group_id, SpawnGroupConditionActionLikeCpp::Noop));
                }
                continue;
            }

            if is_active == should_be_active {
                actions.push((group.group_id, SpawnGroupConditionActionLikeCpp::Noop));
                continue;
            }

            let action = if should_be_active {
                SpawnGroupConditionActionLikeCpp::spawn_group_spawn_default()
            } else if group
                .flags
                .contains(SpawnGroupFlags::DESPAWN_ON_CONDITION_FAILURE)
            {
                SpawnGroupConditionActionLikeCpp::condition_failure_despawn()
            } else {
                SpawnGroupConditionActionLikeCpp::SetInactive
            };
            actions.push((group.group_id, action));
        }
        actions
    }

    /// C++ `Map::AddFarSpellCallback` represented as a map-owned FIFO action queue.
    ///
    /// This helper only accepts explicit represented actions; it does not expose a
    /// general closure/callback runtime or real Spell/Aura side effects.
    pub fn add_far_spell_callback_like_cpp(
        &mut self,
        callback: RepresentedFarSpellCallbackLikeCpp,
    ) {
        self.far_spell_callbacks_like_cpp.push_back(callback);
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

    /// C++ `Map::AddObjectToRemoveList` represented over canonical map records.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2547-2555` asserts same map/instance, marks destroyed, runs
    ///   `CleanupsBeforeDelete(false)`, and inserts into `i_objectsToRemove`.
    /// - `Object.cpp:1826-1835` delegates `WorldObject::AddObjectToRemoveList` to
    ///   the owning map when present.
    ///
    /// Divergence note: the C++ `std::set` insert is deduplicated, but the
    /// cleanup call happens before insertion; this Rust seam preserves that order
    /// and reports `duplicate=true` while still incrementing represented cleanup.
    pub fn add_object_to_remove_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> AddObjectToRemoveListOutcomeLikeCpp {
        let Some(record) = self.map_objects.get_mut(&guid) else {
            return AddObjectToRemoveListOutcomeLikeCpp {
                guid,
                queued: false,
                duplicate: false,
                missing_or_stale: true,
                unsupported_kind: None,
                cleanup_before_delete_count: 0,
            };
        };

        let kind = record.kind();
        debug_assert_eq!(record.object().map_id(), self.map_id);
        debug_assert_eq!(record.object().instance_id(), self.instance_id);

        let cleanup_before_delete_count =
            cleanup_map_object_record_before_delete_like_cpp(record, kind, false);
        let inserted = self.objects_to_remove.insert(guid);
        AddObjectToRemoveListOutcomeLikeCpp {
            guid,
            queued: inserted,
            duplicate: !inserted,
            missing_or_stale: false,
            unsupported_kind: remove_list_grid_kind_like_cpp(kind)
                .is_none()
                .then_some(kind),
            cleanup_before_delete_count,
        }
    }

    /// Bounded represented consumption seam for C++ `Map::SendObjectUpdates()`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:777` calls `SendObjectUpdates()` after ObjectUpdater/Transport
    ///   visitation during `Map::Update`.
    /// - `Map.cpp:1929-1948` drains `_updateObjects`, asserts each object is
    ///   in-world, calls `obj->BuildUpdate(update_players)`, then builds/sends
    ///   per-player packets from `UpdateDataMapType`.
    /// - `Object.cpp:797-806` clears changed values and resets
    ///   `m_objectUpdated` in `ClearUpdateMask(false)`.
    /// - `Object.cpp:3722-3728` `WorldObject::BuildUpdate` visits visible players
    ///   then calls `ClearUpdateMask(false)`.
    ///
    /// Rust ownership: `map_objects` is the canonical source of objects and update
    /// flags. Because RustyCore does not yet have a map-owned `_updateObjects` set,
    /// this snapshots GUIDs from `map_objects` whose `object().is_object_updated()`
    /// is true. The seam represents only the consumption/clear side effect; it
    /// does not create `UpdateDataMapType`, iterate visible players, build packets,
    /// access sessions/ObjectAccessor, or send `SendDirectMessage` fanout.
    pub fn send_object_updates_like_cpp(&mut self) -> SendObjectUpdatesSummaryLikeCpp {
        let updated_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                record
                    .object()
                    .object()
                    .is_object_updated()
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = SendObjectUpdatesSummaryLikeCpp {
            queued_before: updated_guids.len(),
            ..Default::default()
        };

        for guid in updated_guids {
            let Some(record) = self.map_objects.get_mut(&guid) else {
                summary.missing_or_stale += 1;
                continue;
            };

            if !record.object().object().is_in_world() {
                summary.skipped_not_in_world += 1;
                continue;
            }

            // Represents `obj->BuildUpdate(update_players)` only up to its durable
            // map-owned side effect: snapshot every represented typed VALUES mask
            // before eventually calling `ClearUpdateMask(false)`. Visible-player
            // iteration, `UpdateDataMapType`, packet construction, and direct sends
            // remain open fanout gaps.
            match record.kind() {
                AccessorObjectKind::Player => {
                    let player = record.player_mut().expect("typed Player record");
                    let values_update = player.values_update(true);
                    if values_update.has_data() {
                        summary
                            .player_values_updates
                            .push(RepresentedPlayerValuesUpdateLikeCpp {
                                guid,
                                values_update,
                            });
                    }
                    player.clear_data_changes();
                }
                AccessorObjectKind::Creature => {
                    let creature = record.creature_mut().expect("typed Creature record");
                    let values_update = creature.unit().values_update();
                    if values_update.has_data() {
                        summary
                            .unit_values_updates
                            .push(RepresentedUnitValuesUpdateLikeCpp {
                                guid,
                                kind: AccessorObjectKind::Creature,
                                values_update,
                            });
                    }
                    creature.clear_data_changes();
                }
                AccessorObjectKind::Pet => {
                    let pet = record.pet_mut().expect("typed Pet record");
                    let values_update = pet.creature().unit().values_update();
                    if values_update.has_data() {
                        summary
                            .unit_values_updates
                            .push(RepresentedUnitValuesUpdateLikeCpp {
                                guid,
                                kind: AccessorObjectKind::Pet,
                                values_update,
                            });
                    }
                    pet.creature_mut().clear_data_changes();
                }
                _ => {
                    if let Some(dynamic_object) = record.dynamic_object_mut() {
                        let values_update = dynamic_object.values_update();
                        if values_update.has_data() {
                            summary.dynamic_object_values_updates.push(
                                RepresentedDynamicObjectValuesUpdateLikeCpp {
                                    guid,
                                    values_update,
                                },
                            );
                        }
                        dynamic_object.clear_dynamic_object_data_changes();
                    }
                    record.object_mut().object_mut().clear_update_mask(false);
                }
            }
            summary.processed += 1;
            summary.cleared_update_masks += 1;
            summary.fanout_not_represented += 1;
        }

        summary
    }

    pub fn represented_script_schedule_count_like_cpp(&self) -> usize {
        self.script_schedule_like_cpp.values().map(Vec::len).sum()
    }

    pub fn represented_executed_script_actions_like_cpp(
        &self,
    ) -> &[RepresentedScriptScheduleActionLikeCpp] {
        &self.represented_executed_script_actions_like_cpp
    }

    pub const fn is_script_schedule_locked_like_cpp(&self) -> bool {
        self.script_schedule_lock_like_cpp
    }

    /// Bounded represented seam for C++ `Map::ScriptCommandStart` scheduling.
    ///
    /// C++ anchors:
    /// - `MapScripts.cpp:72-98` schedules one action at
    ///   `GameTime::GetGameTime() + delay`, increments the global scheduled count,
    ///   and immediately processes zero-delay actions when `!i_scriptLock`.
    /// - `MapScripts.cpp:386-893` real commands are intentionally not executed by
    ///   this Rust seam; due actions are only recorded as represented evidence.
    pub fn schedule_represented_script_action_like_cpp(
        &mut self,
        now_secs: i64,
        delay_secs: u32,
        source_guid: ObjectGuid,
        target_guid: ObjectGuid,
        owner_guid: ObjectGuid,
        command_id: u32,
    ) -> ScriptScheduleStartOutcomeLikeCpp {
        let due_time_secs = now_secs.saturating_add(i64::from(delay_secs));
        let scheduled = RepresentedScriptScheduleActionLikeCpp {
            source_guid,
            target_guid,
            owner_guid,
            command_id,
            due_time_secs,
        };
        self.script_schedule_like_cpp
            .entry(due_time_secs)
            .or_default()
            .push(scheduled);

        let immediate_process = if delay_secs == 0 && !self.script_schedule_lock_like_cpp {
            Some(self.process_script_schedule_update_order_like_cpp(now_secs))
        } else {
            None
        };

        ScriptScheduleStartOutcomeLikeCpp {
            scheduled,
            represented_increase_count: 1,
            remaining_after_schedule: self.represented_script_schedule_count_like_cpp(),
            immediate_process,
        }
    }

    /// Bounded represented C++ `Map::ScriptsProcess()` drain.
    ///
    /// Empty schedules are no-ops. Otherwise only sorted entries whose due time is
    /// `<= GameTime::GetGameTime()` are erased and recorded as represented-executed
    /// evidence; future entries remain queued and stop the drain. This does not
    /// execute talk/emote/move/teleport/quest/gossip/item/weather/script-manager
    /// commands or any DB/session/ObjectAccessor side effects.
    pub fn process_due_script_schedule_like_cpp(
        &mut self,
        now_secs: i64,
    ) -> ScriptScheduleProcessSummaryLikeCpp {
        let queued_before = self.represented_script_schedule_count_like_cpp();
        if queued_before == 0 {
            return ScriptScheduleProcessSummaryLikeCpp {
                queued_before,
                remaining: 0,
                empty_noop: true,
                ..Default::default()
            };
        }

        let mut processed_actions = Vec::new();
        loop {
            let Some((&due_time_secs, _)) = self.script_schedule_like_cpp.first_key_value() else {
                break;
            };
            if due_time_secs > now_secs {
                break;
            }
            if let Some(mut actions) = self.script_schedule_like_cpp.remove(&due_time_secs) {
                processed_actions.append(&mut actions);
            }
        }

        self.represented_executed_script_actions_like_cpp
            .extend(processed_actions.iter().copied());
        let remaining = self.represented_script_schedule_count_like_cpp();
        ScriptScheduleProcessSummaryLikeCpp {
            queued_before,
            processed: processed_actions.len(),
            remaining,
            represented_decrease_count: processed_actions.len(),
            lock_entered: false,
            empty_noop: false,
            processed_actions,
        }
    }

    /// C++ `Map::Update` order helper for the script seam.
    ///
    /// Mirrors `if (!m_scriptSchedule.empty()) { i_scriptLock = true;
    /// ScriptsProcess(); i_scriptLock = false; }` between `SendObjectUpdates()`
    /// and weather/personal phase (`Map.cpp:777-798`).
    pub fn process_script_schedule_update_order_like_cpp(
        &mut self,
        now_secs: i64,
    ) -> ScriptScheduleProcessSummaryLikeCpp {
        if self.script_schedule_like_cpp.is_empty() {
            return ScriptScheduleProcessSummaryLikeCpp {
                empty_noop: true,
                ..Default::default()
            };
        }

        self.script_schedule_lock_like_cpp = true;
        let mut summary = self.process_due_script_schedule_like_cpp(now_secs);
        self.script_schedule_lock_like_cpp = false;
        summary.lock_entered = true;
        summary
    }

    #[cfg(test)]
    fn set_script_schedule_lock_for_test(&mut self, locked: bool) {
        self.script_schedule_lock_like_cpp = locked;
    }

    pub const fn weather_update_timer_current_ms_like_cpp(&self) -> u32 {
        self.weather_update_timer_current_ms_like_cpp
    }

    pub const fn weather_update_timer_interval_ms_like_cpp(&self) -> u32 {
        self.weather_update_timer_interval_ms_like_cpp
    }

    pub fn represented_zone_dynamic_info_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&RepresentedZoneDynamicInfoLikeCpp> {
        self.zone_dynamic_info_like_cpp.get(&zone_id)
    }

    pub fn represented_zone_default_weather_update_diffs_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&[u32]> {
        self.zone_dynamic_info_like_cpp
            .get(&zone_id)?
            .default_weather
            .as_ref()
            .map(RepresentedZoneDefaultWeatherLikeCpp::update_call_diffs_ms)
    }

    #[cfg(test)]
    pub(crate) fn register_represented_zone_default_weather_for_test(&mut self, zone_id: u32) {
        self.zone_dynamic_info_like_cpp
            .entry(zone_id)
            .or_default()
            .default_weather = Some(RepresentedZoneDefaultWeatherLikeCpp::new());
    }

    #[cfg(test)]
    pub(crate) fn set_represented_zone_default_weather_next_update_alive_for_test(
        &mut self,
        zone_id: u32,
        alive: bool,
    ) -> bool {
        let Some(weather) = self
            .zone_dynamic_info_like_cpp
            .get_mut(&zone_id)
            .and_then(|zone| zone.default_weather.as_mut())
        else {
            return false;
        };
        weather.set_next_update_returns_alive(alive);
        true
    }

    /// Represented C++ `_weatherUpdateTimer` / `_zoneDynamicInfo.DefaultWeather`
    /// step from `Map::Update` (`Map.cpp:777-798`).
    ///
    /// Timer semantics mirror `IntervalTimer` (`Timer.h:62-87`): update adds the
    /// diff, `Passed()` is `current >= interval`, and `Reset()` keeps overshoot via
    /// modulo. When passed, existing represented zones are iterated and only zones
    /// with `DefaultWeather` call represented `Weather::Update(interval)`. A false
    /// represented return removes only that optional weather pointer like C++
    /// `DefaultWeather.reset()`. Weather regeneration/RNG, `UpdateWeather`, player
    /// discovery/fanout, `sWorld->SendZoneMessage`, `sScriptMgr` hooks, DB and
    /// WeatherMgr runtime are explicit gaps surfaced in the summary flag.
    pub fn update_weather_like_cpp(&mut self, diff_ms: u32) -> WeatherUpdateSummaryLikeCpp {
        let interval_ms = self.weather_update_timer_interval_ms_like_cpp;
        let timer_current_before = self.weather_update_timer_current_ms_like_cpp;
        self.weather_update_timer_current_ms_like_cpp = self
            .weather_update_timer_current_ms_like_cpp
            .saturating_add(diff_ms);
        let timer_current_after_update = self.weather_update_timer_current_ms_like_cpp;
        let timer_passed = timer_current_after_update >= interval_ms;
        let mut summary = WeatherUpdateSummaryLikeCpp {
            interval_ms,
            timer_current_before,
            timer_current_after_update,
            timer_current_after_reset: timer_current_after_update,
            timer_passed,
            script_update_regeneration_fanout_not_represented: true,
            ..Default::default()
        };

        if !timer_passed {
            return summary;
        }

        summary.zones_seen = self.zone_dynamic_info_like_cpp.len();
        for zone_info in self.zone_dynamic_info_like_cpp.values_mut() {
            let Some(default_weather) = zone_info.default_weather.as_mut() else {
                summary.zones_without_default_weather += 1;
                continue;
            };
            summary.default_weather_updated += 1;
            summary.weather_update_call_diff_ms = Some(interval_ms);
            if !default_weather.update_like_cpp(interval_ms) {
                zone_info.default_weather = None;
                summary.default_weather_removed += 1;
            }
        }

        self.weather_update_timer_current_ms_like_cpp %= interval_ms;
        summary.timer_current_after_reset = self.weather_update_timer_current_ms_like_cpp;
        summary
    }

    /// C++ `GetMultiPersonalPhaseTracker().Update(this, t_diff)` represented on `Map`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:797-798` calls the map-owned multi personal phase tracker during
    ///   `Map::Update` before deferred move/remove-list processing.
    /// - `PersonalPhaseTracker.cpp:62-78,106-113,192-202` expires per-owner phases,
    ///   calls `Map::AddObjectToRemoveList` for tracked objects, clears phase
    ///   object/grid sets, and removes empty owner trackers.
    ///
    /// Rust ownership: `personal_phase_tracker.update(diff_ms)` is the sole source
    /// of expired GUIDs; `map_objects` remains canonical for real records/removal.
    /// This seam does not drain `objects_to_remove`, rebuild objects from external
    /// caches, or claim session/ObjectAccessor/visibility/DB/script behavior.
    pub fn update_personal_phase_tracker_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> PersonalPhaseTrackerUpdateSummaryLikeCpp {
        let expired_guids = self.personal_phase_tracker.update(diff_ms);
        let mut summary = PersonalPhaseTrackerUpdateSummaryLikeCpp {
            expired_objects: expired_guids.len(),
            ..Default::default()
        };

        for guid in expired_guids {
            let outcome = self.add_object_to_remove_list_like_cpp(guid);
            if outcome.queued {
                summary.remove_queued += 1;
            }
            if outcome.duplicate {
                summary.duplicate_queued += 1;
            }
            if outcome.missing_or_stale {
                summary.missing_or_stale += 1;
            }
            if outcome.unsupported_kind.is_some() {
                summary.unsupported_kinds += 1;
            }
        }

        summary
    }

    /// C++ `WorldObject::SetWorldObject(bool)` facade owned by `Map` over the
    /// canonical `MapObjectRecord` store.
    ///
    /// C++ anchors:
    /// - `Object.cpp:910-916` returns when `!IsInWorld()`, otherwise delegates
    ///   to the owning map's `AddObjectToSwitchList(this, on)`.
    /// - `Map.cpp:2557-2572` keeps Unit validation/queue duplicate semantics in
    ///   `add_object_to_switch_list_like_cpp`; this facade does not move grid
    ///   containers or mutate temporary world-object state.
    pub fn set_world_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> SetWorldObjectOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return SetWorldObjectOutcomeLikeCpp {
                guid,
                on,
                status: SetWorldObjectStatusLikeCpp::MissingOrStale,
            };
        };

        if !record.object().object().is_in_world() {
            return SetWorldObjectOutcomeLikeCpp {
                guid,
                on,
                status: SetWorldObjectStatusLikeCpp::NotInWorld,
            };
        }

        let delegated = self.add_object_to_switch_list_like_cpp(guid, on);
        SetWorldObjectOutcomeLikeCpp {
            guid,
            on,
            status: SetWorldObjectStatusLikeCpp::Delegated(delegated.status),
        }
    }

    /// Applies the request emitted by C++-shaped Unit shared-vision transitions
    /// to this map-owned `WorldObject::SetWorldObject(bool)` facade.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:6489-6509` emits `SetWorldObject(true/false)` only at the
    ///   empty/non-empty shared-vision boundary.
    /// - `Object.cpp:910-916` keeps the in-world guard before map delegation.
    /// - `Map.cpp:2557-2572` owns switch-list validation/queue semantics, while
    ///   `Map.cpp:2574-2594` drains later.
    ///
    /// Ownership stays one-way: Unit emits a DTO, the map owner applies it over
    /// canonical `map_objects`/`objects_to_switch`; this method does not run the
    /// drain, rebuild missing records, fan out visibility, or wire sessions.
    pub fn apply_unit_shared_vision_set_world_object_request_like_cpp(
        &mut self,
        request: UnitSharedVisionSetWorldObjectRequestLikeCpp,
    ) -> SetWorldObjectOutcomeLikeCpp {
        self.set_world_object_like_cpp(request.unit_guid, request.on)
    }

    fn player_set_viewpoint_outcome_like_cpp(
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        apply: bool,
        status: PlayerSetViewpointStatusLikeCpp,
        set_world_object: Option<SetWorldObjectOutcomeLikeCpp>,
        update_visibility_requested: bool,
        set_seer_requested: bool,
    ) -> PlayerSetViewpointOutcomeLikeCpp {
        PlayerSetViewpointOutcomeLikeCpp {
            player_guid,
            target_guid,
            apply,
            status,
            set_world_object,
            update_visibility_requested,
            set_seer_requested,
        }
    }

    fn map_record_unit_mut_like_cpp(record: &mut MapObjectRecord) -> Option<&mut Unit> {
        match record.kind() {
            AccessorObjectKind::Player => record.player_mut().map(Player::unit_mut),
            AccessorObjectKind::Creature => record.creature_mut().map(Creature::unit_mut),
            AccessorObjectKind::Pet => record.pet_mut().map(|pet| pet.creature_mut().unit_mut()),
            _ => None,
        }
    }

    fn map_record_unit_like_cpp(record: &MapObjectRecord) -> Option<&Unit> {
        match record.kind() {
            AccessorObjectKind::Player => record.player().map(Player::unit),
            AccessorObjectKind::Creature => record.creature().map(Creature::unit),
            AccessorObjectKind::Pet => record.pet().map(|pet| pet.creature().unit()),
            _ => None,
        }
    }

    /// Bounded map-owned seam for the Unit-target shared-vision branch of C++
    /// `Player::SetViewpoint(WorldObject* target, bool apply)`.
    ///
    /// C++ anchors:
    /// - `Player.cpp:25344-25387` owns FarsightObject guards/mutations,
    ///   requests `UpdateVisibilityOf`, calls `Unit::Add/RemovePlayerToVision`
    ///   only for Unit targets that are not `GetVehicleBase()`, and requests
    ///   `SetSeer`.
    /// - `Unit.cpp:6489-6509` toggles Unit active state and emits
    ///   `SetWorldObject(true/false)` only at shared-vision empty boundaries.
    /// - `Object.cpp:910-916` / `Map.cpp:2557-2594` keep the SetWorldObject
    ///   map-owned switch-list enqueue/drain split.
    ///
    /// Scope: this helper mutates only canonical `Map::map_objects` typed Player
    /// and typed Creature/Pet Unit targets already in this same map. It consumes
    /// the Unit-emitted SetWorldObject DTO immediately through the Map facade, but
    /// does not drain queues, fan out visibility, implement `SetSeer`, access
    /// ObjectAccessor/session mirrors, create records, send packets, or touch DB.
    pub fn apply_player_set_viewpoint_unit_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        apply: bool,
        vehicle_base_guid: Option<ObjectGuid>,
    ) -> PlayerSetViewpointOutcomeLikeCpp {
        let Some(player) = self.get_typed_player(player_guid) else {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                None,
                false,
                false,
            );
        };

        let current_farsight = player.active_data().farsight_object;
        if apply {
            if !current_farsight.is_empty() {
                return Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint,
                    None,
                    false,
                    false,
                );
            }
        } else if current_farsight != target_guid {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                None,
                false,
                false,
            );
        }

        let Some(target_record) = self.map_object_record(target_guid) else {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingTarget,
                None,
                false,
                false,
            );
        };
        if !matches!(
            target_record.kind(),
            AccessorObjectKind::Creature | AccessorObjectKind::Pet
        ) {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                None,
                false,
                false,
            );
        }

        let vehicle_base_skip = vehicle_base_guid == Some(target_guid);
        if !vehicle_base_skip {
            let Some(target_record) = self.map_objects.get_mut(&target_guid) else {
                return Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                    None,
                    false,
                    false,
                );
            };
            if Self::map_record_unit_mut_like_cpp(target_record).is_none() {
                return Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                    None,
                    false,
                    false,
                );
            }
        }

        let Some(player) = self.get_typed_player_mut(player_guid) else {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                None,
                false,
                false,
            );
        };
        player.set_farsight_object_like_cpp(if apply {
            target_guid
        } else {
            ObjectGuid::EMPTY
        });

        if vehicle_base_skip {
            return Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                if apply {
                    PlayerSetViewpointStatusLikeCpp::Applied
                } else {
                    PlayerSetViewpointStatusLikeCpp::Removed
                },
                None,
                apply,
                true,
            );
        }

        let request = {
            let Some(target_record) = self.map_objects.get_mut(&target_guid) else {
                return Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                    None,
                    false,
                    false,
                );
            };
            let Some(target_unit) = Self::map_record_unit_mut_like_cpp(target_record) else {
                return Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                    None,
                    false,
                    false,
                );
            };
            if apply {
                target_unit.add_player_to_vision_like_cpp(player_guid)
            } else {
                target_unit.remove_player_from_vision_like_cpp(player_guid)
            }
            .set_world_object
        };
        let set_world_object = request.map(|request| {
            self.apply_unit_shared_vision_set_world_object_request_like_cpp(request)
        });

        Self::player_set_viewpoint_outcome_like_cpp(
            player_guid,
            target_guid,
            apply,
            if apply {
                PlayerSetViewpointStatusLikeCpp::Applied
            } else {
                PlayerSetViewpointStatusLikeCpp::Removed
            },
            set_world_object,
            apply,
            true,
        )
    }

    /// Map-owned seam for C++ `Spell::EffectAddFarsight` ->
    /// `DynamicObject::CreateDynamicObject` -> `SetDuration` ->
    /// `SetCasterViewpoint`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:2237-2261` runs only after HIT handling has selected
    ///   a Player caster, returns if the Player is not in world, creates
    ///   `DynamicObject(true)`, calls `CreateDynamicObject`, then sets duration
    ///   and caster viewpoint.
    /// - `DynamicObject.cpp:84-133` binds the object to the caster map, validates
    ///   the destination, creates a world-object GUID from map/spell/low guid,
    ///   inherits phase, sets entry/scale/update fields, marks world objects
    ///   active before AddToMap, and inserts through `Map::AddToMap`.
    /// - `DynamicObject.cpp:209-239` resolves the already-bound caster pointer for
    ///   `SetCasterViewpoint`; Rust represents that by `DynamicObject::bound_caster()`
    ///   and delegates to `apply_dynamic_object_caster_viewpoint_like_cpp`.
    ///
    /// Ownership: source-of-truth is this `Map::map_objects` for both the caster
    /// Player and the newly-created DynamicObject. Per #NEXT.R8.ENTITIES.428
    /// invariants, represented fallback paths validate all rejectable inputs before
    /// low-guid consumption so a missing/wrong caster or invalid destination leaves
    /// the Map seam unmutated; this is an explicitly bounded creation-seam guard even
    /// though C++ receives `guidlow` before `CreateDynamicObject` validates `pos`.
    /// This does not parse live Spell targets, create dummy records, register through
    /// ObjectAccessor, implement transport passenger offsets, UpdatePositionData,
    /// ZoneScript, aura/update lifecycle, real SetSeer/fanout, packets/session mirrors,
    /// DB, or spell handler wiring.
    #[allow(clippy::too_many_arguments)]
    pub fn create_farsight_dynamic_object_like_cpp(
        &mut self,
        caster_player_guid: ObjectGuid,
        spell_id: u32,
        spell_x_spell_visual_id: i32,
        dest: Position,
        radius: f32,
        duration_ms: i32,
        cast_time_ms: u64,
        realm_id: u16,
        server_id: u32,
    ) -> FarsightDynamicObjectCreateOutcomeLikeCpp {
        let early = |status| FarsightDynamicObjectCreateOutcomeLikeCpp {
            status,
            caster_player_guid,
            dynamic_object_guid: None,
            low_guid: None,
            add_to_map: None,
            caster_viewpoint: None,
        };

        let Some(caster_player) = self.get_typed_player(caster_player_guid) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer);
        };
        let caster_world = caster_player.unit().world();
        if !caster_world.object().is_in_world() {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CasterNotInWorld);
        }
        if caster_world.map_id() != self.map_id || caster_world.instance_id() != self.instance_id {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CasterWrongMap);
        }
        if !dest.is_valid_map_coord_like_cpp() {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::InvalidDestination);
        }
        if self.map_id > 0x1FFF {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::MapIdNotRepresentableInGuid);
        }
        let Ok(spell_id_i32) = i32::try_from(spell_id) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::SpellIdNotRepresentable);
        };
        let Ok(cast_time_ms_u32) = u32::try_from(cast_time_ms) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CastTimeNotRepresentable);
        };
        let inherited_phase_shift = caster_world.phase_shift().clone();
        let inherited_suppressed_phase_shift = caster_world.suppressed_phase_shift().clone();

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::DynamicObject) {
            Ok(low_guid) => low_guid,
            Err(error) => {
                return early(FarsightDynamicObjectCreateStatusLikeCpp::GuidSequenceError(
                    error,
                ));
            }
        };
        let dynamic_object_guid = ObjectGuid::create_world_object(
            HighGuid::DynamicObject,
            0,
            realm_id,
            self.map_id as u16,
            server_id,
            spell_id,
            low_guid,
        );

        let mut dynamic_object = DynamicObject::new(true);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        if dynamic_object
            .world_mut()
            .set_map(self.map_id, self.instance_id)
            .is_err()
        {
            return FarsightDynamicObjectCreateOutcomeLikeCpp {
                status: FarsightDynamicObjectCreateStatusLikeCpp::DynamicObjectRecordError(
                    ObjectAccessorError::ObjectHasNoMap {
                        guid: dynamic_object_guid,
                    },
                ),
                caster_player_guid,
                dynamic_object_guid: Some(dynamic_object_guid),
                low_guid: Some(low_guid),
                add_to_map: None,
                caster_viewpoint: None,
            };
        }
        dynamic_object.world_mut().relocate(dest);
        *dynamic_object.world_mut().phase_shift_mut() = inherited_phase_shift;
        *dynamic_object.world_mut().suppressed_phase_shift_mut() = inherited_suppressed_phase_shift;
        dynamic_object.world_mut().object_mut().set_entry(spell_id);
        dynamic_object.world_mut().object_mut().set_scale(1.0);
        dynamic_object.set_caster_guid(caster_player_guid);
        dynamic_object.set_dynamic_object_type(DynamicObjectType::FarsightFocus);
        dynamic_object.set_spell_visual_id(spell_x_spell_visual_id);
        dynamic_object.set_spell_id(spell_id_i32);
        dynamic_object.set_radius(radius);
        dynamic_object.set_cast_time_ms(cast_time_ms_u32);
        dynamic_object.bind_to_caster(caster_player_guid);
        dynamic_object.set_duration(duration_ms);
        if dynamic_object.world().is_world_object() {
            dynamic_object.world_mut().set_active(true);
        }

        let record = match MapObjectRecord::new_dynamic_object(dynamic_object) {
            Ok(record) => record,
            Err(error) => {
                return FarsightDynamicObjectCreateOutcomeLikeCpp {
                    status: FarsightDynamicObjectCreateStatusLikeCpp::DynamicObjectRecordError(
                        error,
                    ),
                    caster_player_guid,
                    dynamic_object_guid: Some(dynamic_object_guid),
                    low_guid: Some(low_guid),
                    add_to_map: None,
                    caster_viewpoint: None,
                };
            }
        };
        let add_to_map = match self.add_map_object_record_to_map_like_cpp(record) {
            Ok(outcome) => outcome,
            Err(_error) => {
                return FarsightDynamicObjectCreateOutcomeLikeCpp {
                    status: FarsightDynamicObjectCreateStatusLikeCpp::AddToMapError,
                    caster_player_guid,
                    dynamic_object_guid: Some(dynamic_object_guid),
                    low_guid: Some(low_guid),
                    add_to_map: None,
                    caster_viewpoint: None,
                };
            }
        };
        let caster_viewpoint =
            self.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

        FarsightDynamicObjectCreateOutcomeLikeCpp {
            status: FarsightDynamicObjectCreateStatusLikeCpp::Created,
            caster_player_guid,
            dynamic_object_guid: Some(dynamic_object_guid),
            low_guid: Some(low_guid),
            add_to_map: Some(add_to_map),
            caster_viewpoint: Some(caster_viewpoint),
        }
    }

    /// Map-owned seam for the non-aura branch of C++ `DynamicObject::Update`.
    ///
    /// C++ anchors:
    /// - `DynamicObject.cpp:136-165` asserts same-map caster, updates aura-bound
    ///   DynamicObjects through the aura path (unsupported here), otherwise
    ///   decrements `_duration` by `p_time` or marks expired, then calls `Remove()`
    ///   on expiry and `sScriptMgr->OnDynamicObjectUpdate` otherwise.
    /// - `DynamicObject.cpp:167-171` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `Map.cpp:2547-2555` owns `AddObjectToRemoveList` cleanup and deferred
    ///   remove-list insertion, represented by `add_object_to_remove_list_like_cpp`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`; this helper
    /// mutates only the typed `MapObjectRecord::DynamicObject` duration and, after
    /// dropping that mutable borrow, enqueues the same GUID through the existing
    /// remove-list facade. Aura-bound DynamicObjects only record represented
    /// `Aura::UpdateOwner` evidence and removed/expired checks. It does not drain removal, run scripts,
    /// write ObjectAccessor/session mirrors, fan out visibility, send packets, or
    /// create fallback records.
    pub fn update_dynamic_object_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> DynamicObjectUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(dynamic_object_guid) else {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::DynamicObject {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(dynamic_object) = record.dynamic_object() else {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                duration_before_ms: None,
                duration_after_ms: None,
                aura_update_owner_calls_before: None,
                aura_update_owner_calls_after: None,
                script_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = dynamic_object.duration_ms();
        let aura_update_owner_calls_before = dynamic_object.represented_aura_update_owner_count();
        if !dynamic_object.world().object().is_in_world() {
            return DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                script_update_would_run: false,
                remove_list: None,
            };
        }

        let aura_bound_before = dynamic_object.has_aura();

        let (expired, duration_after_ms, aura_update_owner_calls_after) = {
            let Some(record) = self.map_objects.get_mut(&dynamic_object_guid) else {
                return DynamicObjectUpdateOutcomeLikeCpp {
                    dynamic_object_guid,
                    elapsed_ms,
                    status: DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                    aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                    script_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(dynamic_object) = record.dynamic_object_mut() else {
                return DynamicObjectUpdateOutcomeLikeCpp {
                    dynamic_object_guid,
                    elapsed_ms,
                    status: DynamicObjectUpdateStatusLikeCpp::NotDynamicObject,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                    aura_update_owner_calls_after: Some(aura_update_owner_calls_before),
                    script_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = if aura_bound_before {
                dynamic_object.update_aura_bound_like_cpp(elapsed_ms)
            } else {
                dynamic_object.update_non_aura_duration(elapsed_ms)
            };
            (
                expired,
                dynamic_object.duration_ms(),
                dynamic_object.represented_aura_update_owner_count(),
            )
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(dynamic_object_guid);
            DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_after),
                script_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            DynamicObjectUpdateOutcomeLikeCpp {
                dynamic_object_guid,
                elapsed_ms,
                status: DynamicObjectUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                aura_update_owner_calls_before: Some(aura_update_owner_calls_before),
                aura_update_owner_calls_after: Some(aura_update_owner_calls_after),
                script_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `DynamicObject` records only.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` creates `Trinity::ObjectUpdater updater(t_diff)`
    ///   during `Map::Update` and visits object containers before
    ///   `SendObjectUpdates()` / scripts.
    /// - `GridNotifiers.cpp:258-264,296-301` visits each object and calls
    ///   `Update(i_timeDiff)` only when `IsInWorld()`, including the explicit
    ///   `DynamicObject` instantiation.
    /// - `DynamicObject.cpp:136-171` is represented by
    ///   `update_dynamic_object_like_cpp`, including duration/aura-bound evidence
    ///   and expiry enqueue through `AddObjectToRemoveList()`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. This method
    /// snapshots typed DynamicObject GUIDs only, then delegates each GUID to the
    /// existing per-object helper. It does not drain the remove-list, visit nearby
    /// cells, update players/sessions or other object families, send object
    /// updates, run scripts/AI, touch dynamic tree/collision, fan out visibility,
    /// write ObjectAccessor/session mirrors, or create fallback records.
    pub fn update_dynamic_objects_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> DynamicObjectsUpdateSummaryLikeCpp {
        let dynamic_object_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::DynamicObject
                    && record.dynamic_object().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = DynamicObjectsUpdateSummaryLikeCpp::default();
        for guid in dynamic_object_guids {
            summary.visited += 1;
            let outcome = self.update_dynamic_object_like_cpp(guid, elapsed_ms);
            match outcome.status {
                DynamicObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject => {
                    summary.missing_or_stale += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotDynamicObject => {
                    summary.not_dynamic_object += 1;
                }
                DynamicObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `GameObject::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` creates `Trinity::ObjectUpdater updater(t_diff)`
    ///   during `Map::Update`.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `GameObject`.
    /// - `GameObject.cpp:1215-1233` is represented through the entity-level
    ///   `m_despawnDelay` countdown; expiry represents `DespawnOrUnsummon(0ms,
    ///   m_despawnRespawnTime)`.
    /// - `GameObject.cpp:1575-1580` `GO_JUST_DEACTIVATED` despawns an
    ///   already-linked trap via `GetLinkedTrap()->DespawnOrUnsummon()` before
    ///   later goober/chest/generic cleanup.
    /// - `GameObject.cpp:1740-1764` `Delete()` is represented only as
    ///   `SetLootState(GO_NOT_READY)` plus `AddObjectToRemoveList()`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-GameObject and not-in-world outcomes do not mutate state. This helper
    /// never creates fallback records, reads session/ObjectAccessor mirrors,
    /// saves DB respawn times, runs PoolMgr, sends packets, fans out visibility,
    /// executes AI/go-type implementations, drains removal, or includes Transport
    /// records whose embedded body happens to be a GameObject.
    pub fn update_game_object_like_cpp(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
    ) -> GameObjectUpdateOutcomeLikeCpp {
        self.update_game_object_with_optional_pool_update_like_cpp::<fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
            game_object_guid,
            diff_ms,
            game_time_secs,
            None,
            None,
        )
    }

    pub fn update_game_object_with_pool_update_like_cpp(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) -> GameObjectUpdateOutcomeLikeCpp {
        self.update_game_object_with_optional_pool_update_like_cpp(
            game_object_guid,
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Self,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        )
    }

    pub fn update_game_object_with_pool_update_loaded_grid_records_like_cpp<L>(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> GameObjectUpdateOutcomeLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_game_object_with_optional_pool_update_like_cpp(
            game_object_guid,
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            Some(&mut load_record),
        )
    }

    fn update_game_object_with_optional_pool_update_like_cpp<L>(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectUpdateOutcomeLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(record) = self.map_object_record(game_object_guid) else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let Some(game_object) = record.game_object() else {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                despawn_delay_before_ms: None,
                despawn_delay_after_ms: None,
                despawn_respawn_time_secs: None,
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        };

        let despawn_delay_before_ms = game_object.despawn_delay();
        let despawn_respawn_time_secs = game_object.despawn_respawn_time();
        if !game_object.world().object().is_in_world() {
            return GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::NotInWorld,
                despawn_delay_before_ms: Some(despawn_delay_before_ms),
                despawn_delay_after_ms: Some(despawn_delay_before_ms),
                despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                world_update_would_run: false,
                ai_update_not_represented: false,
                go_type_impl_update_not_represented: false,
                despawn_or_unsummon_requested: false,
                entity_update: None,
                remove_list: None,
                linked_trap_guid: None,
                linked_trap_removed: false,
                linked_trap_remove_queued: false,
                linked_trap_missing_or_self: false,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented: false,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            };
        }

        let entity_update = {
            let Some(record) = self.map_objects.get_mut(&game_object_guid) else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::MissingGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            let Some(game_object) = record.game_object_mut() else {
                return GameObjectUpdateOutcomeLikeCpp {
                    game_object_guid,
                    diff_ms,
                    status: GameObjectUpdateStatusLikeCpp::NotGameObject,
                    despawn_delay_before_ms: Some(despawn_delay_before_ms),
                    despawn_delay_after_ms: Some(despawn_delay_before_ms),
                    despawn_respawn_time_secs: Some(despawn_respawn_time_secs),
                    world_update_would_run: false,
                    ai_update_not_represented: false,
                    go_type_impl_update_not_represented: false,
                    despawn_or_unsummon_requested: false,
                    entity_update: None,
                    remove_list: None,
                    linked_trap_guid: None,
                    linked_trap_removed: false,
                    linked_trap_remove_queued: false,
                    linked_trap_missing_or_self: false,
                    loot_cleared: false,
                    goober_spell_cast_spell_id: None,
                    goober_spell_casts_represented: 0,
                    goober_users_cleared: false,
                    goober_state_reset: false,
                    goober_nodespawn_return: false,
                    non_consumed_chest_or_goober_return: false,
                    non_consumed_restock_armed: false,
                    non_consumed_set_ready: false,
                    non_consumed_update_visibility_represented: false,
                    non_consumed_update_dynamic_flags_represented: false,
                    non_consumed_source_missing: false,
                    summoned_expired_delete: false,
                    summoned_expired_respawn_time_zeroed: false,
                    summoned_expired_despawn_represented: false,
                    summoned_expired_go_state_ready: false,
                    new_flag_drop_owner_in_base_command_represented: false,
                    new_flag_drop_owner_missing_or_empty: false,
                    new_flag_drop_owner_wrong_kind: false,
                    new_flag_drop_owner_not_new_flag: false,
                    generic_not_ready: false,
                    generic_capture_point_removed_represented: false,
                    generic_visual_despawn_represented: false,
                    generic_flags_restored_represented: false,
                    generic_zero_respawn_delay_return: false,
                    generic_despawn_at_action_source_missing: false,
                    generic_respawn_scheduled_time: None,
                    generic_spawned_by_default_branch: false,
                    generic_temporary_respawn_zeroed: false,
                    generic_respawn_timer_add: None,
                    generic_respawn_save_missing_spawn_id: false,
                    generic_respawn_save_missing_gameobject_data: false,
                    generic_respawn_compatibility_db_only_represented: false,
                    generic_visibility_on_destroy_represented: false,
                };
            };
            game_object.update_like_cpp(diff_ms)
        };

        let (
            linked_trap_guid,
            linked_trap_removed,
            linked_trap_remove_queued,
            linked_trap_missing_or_self,
        ) = if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            (None, false, false, false)
        } else {
            self.map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| game_object.linked_trap_guid_like_cpp())
                .map_or((None, false, false, false), |linked_guid| {
                    if linked_guid.is_empty() || linked_guid == game_object_guid {
                        return (
                            (!linked_guid.is_empty()).then_some(linked_guid),
                            false,
                            false,
                            true,
                        );
                    }

                    let linked_trap_exists = self
                        .map_object_record(linked_guid)
                        .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                        .and_then(MapObjectRecord::game_object)
                        .is_some();
                    if !linked_trap_exists {
                        return (Some(linked_guid), false, false, true);
                    }

                    match self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                        linked_guid,
                        pool_update,
                        load_record.as_mut().map(|loader| &mut **loader),
                    ) {
                        Some(delete) => (
                            Some(linked_guid),
                            false,
                            delete
                                .remove_list
                                .as_ref()
                                .is_some_and(|remove| remove.queued || remove.duplicate),
                            false,
                        ),
                        None => (Some(linked_guid), false, false, true),
                    }
                })
        };

        let mut goober_spell_cast_spell_id = None;
        let mut goober_spell_casts_represented = 0;
        let mut goober_users_cleared = false;
        let mut goober_state_reset = false;
        let mut goober_nodespawn_return = false;
        let mut non_consumed_chest_or_goober_return = false;
        let mut non_consumed_restock_armed = false;
        let mut non_consumed_set_ready = false;
        let mut non_consumed_update_visibility_represented = false;
        let mut non_consumed_update_dynamic_flags_represented = false;
        let mut non_consumed_source_missing = false;
        let mut summoned_expired_delete = false;
        let mut summoned_expired_respawn_time_zeroed = false;
        let mut summoned_expired_despawn_represented = false;
        let mut summoned_expired_go_state_ready = false;
        let mut new_flag_drop_owner_in_base_command_represented = false;
        let mut new_flag_drop_owner_missing_or_empty = false;
        let mut new_flag_drop_owner_wrong_kind = false;
        let mut new_flag_drop_owner_not_new_flag = false;
        let mut generic_not_ready = false;
        let mut generic_visual_despawn_represented = false;
        let mut generic_flags_restored_represented = false;
        let mut generic_zero_respawn_delay_return = false;
        let mut generic_despawn_at_action_source_missing = false;
        let mut generic_respawn_scheduled_time = None;
        let mut generic_spawned_by_default_branch = false;
        let mut generic_temporary_respawn_zeroed = false;
        let mut generic_respawn_timer_add = None;
        let mut generic_respawn_save_missing_spawn_id = false;
        let mut generic_respawn_save_missing_gameobject_data = false;
        let mut generic_respawn_compatibility_db_only_represented = false;
        let mut generic_visibility_on_destroy_represented = false;

        if entity_update.status != EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            if let Some(game_object) = self
                .map_objects
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .filter(|game_object| game_object.data().type_id == GAMEOBJECT_TYPE_GOOBER as i8)
            {
                if let Some(goober_source) = game_object.represented_goober_use_source_like_cpp() {
                    if goober_source.spell_id != 0 {
                        goober_spell_cast_spell_id = Some(goober_source.spell_id);
                        goober_spell_casts_represented =
                            game_object.unique_users_snapshot_like_cpp().len();
                        game_object.clear_unique_users_and_reset_use_times_like_cpp();
                        goober_users_cleared = true;
                    }

                    if goober_source.lock_id != 0 || goober_source.auto_close_ms != 0 {
                        game_object.set_go_state(GoState::Ready);
                        goober_state_reset = true;
                    }
                }

                goober_nodespawn_return = game_object.data().flags & GO_FLAG_NODESPAWN != 0;
            }
        }

        let loot_cleared = if entity_update.status
            == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested
            || goober_nodespawn_return
        {
            false
        } else if let Some(game_object) = self
            .map_objects
            .get_mut(&game_object_guid)
            .and_then(MapObjectRecord::game_object_mut)
            .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
        {
            game_object.clear_loot_like_cpp();
            true
        } else {
            false
        };

        if loot_cleared {
            if let Some(game_object) = self
                .map_objects
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => None,
                };

                if matches!(go_type, GAMEOBJECT_TYPE_CHEST | GAMEOBJECT_TYPE_GOOBER) {
                    // C++ anchor: GameObject.cpp:1609-1623. This represented seam
                    // deliberately does not call the broader SetLootState facade from
                    // GameObject.cpp:3683-3709 because line 1617 only writes
                    // GO_NOT_READY after arming the fully-looted chest restock timer;
                    // Activated-specific restock/collision semantics are not part of
                    // this branch. Owner/spell-created expiration is consumed below
                    // through the represented `Delete()` seam.
                    if let Some(despawn_at_action) = despawn_at_action {
                        let is_summoned_and_expired = (game_object.owner_guid()
                            != ObjectGuid::EMPTY
                            || game_object.spell_id() != 0)
                            && game_object.respawn_time() == 0;
                        if !despawn_at_action && !is_summoned_and_expired {
                            if go_type == GAMEOBJECT_TYPE_CHEST {
                                if let Some(source) =
                                    game_object.represented_chest_loot_source_like_cpp()
                                {
                                    if source.chest_restock_time_secs > 0 {
                                        let restock_time = game_time_secs.saturating_add(
                                            i64::from(source.chest_restock_time_secs),
                                        );
                                        game_object.set_restock_time_like_cpp(restock_time);
                                        game_object.set_loot_state(LootState::NotReady, None);
                                        non_consumed_restock_armed = true;
                                        non_consumed_update_dynamic_flags_represented = true;
                                    } else {
                                        game_object.set_loot_state(LootState::Ready, None);
                                        non_consumed_set_ready = true;
                                    }
                                }
                            } else {
                                game_object.set_loot_state(LootState::Ready, None);
                                non_consumed_set_ready = true;
                            }
                            non_consumed_chest_or_goober_return = true;
                            non_consumed_update_visibility_represented = true;
                        }
                    } else {
                        non_consumed_source_missing = true;
                    }
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return {
            let summoned_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
                .map(|game_object| {
                    (
                        game_object.data().type_id as u32,
                        game_object.owner_guid(),
                        game_object.spell_id(),
                        game_object.respawn_time(),
                    )
                });

            if let Some((go_type, owner_guid, spell_id, respawn_time)) = summoned_snapshot {
                let is_summoned_and_expired =
                    (owner_guid != ObjectGuid::EMPTY || spell_id != 0) && respawn_time == 0;
                if is_summoned_and_expired {
                    if let Some(game_object) = self
                        .map_objects
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                        game_object.set_loot_state(LootState::NotReady, None);
                        summoned_expired_respawn_time_zeroed = true;
                        summoned_expired_despawn_represented = true;
                        if go_type != GAMEOBJECT_TYPE_TRANSPORT {
                            game_object.set_go_state(GoState::Ready);
                            summoned_expired_go_state_ready = true;
                        }
                    }

                    if go_type == GAMEOBJECT_TYPE_NEW_FLAG_DROP {
                        if owner_guid == ObjectGuid::EMPTY {
                            new_flag_drop_owner_missing_or_empty = true;
                        } else {
                            match self.map_object_record(owner_guid) {
                                Some(owner_record)
                                    if owner_record.kind() == AccessorObjectKind::GameObject =>
                                {
                                    match owner_record.game_object() {
                                        Some(owner_go)
                                            if owner_go.data().type_id as u32
                                                == GAMEOBJECT_TYPE_NEW_FLAG =>
                                        {
                                            // C++ NewFlag::SetState(InBase, nullptr) has
                                            // no full Rust go-type state object yet; record
                                            // the exact typed owner command as represented
                                            // evidence only, without faking ZoneScript or
                                            // fanout.
                                            new_flag_drop_owner_in_base_command_represented = true;
                                        }
                                        Some(_) => {
                                            new_flag_drop_owner_not_new_flag = true;
                                        }
                                        None => {
                                            new_flag_drop_owner_wrong_kind = true;
                                        }
                                    }
                                }
                                Some(_) => {
                                    new_flag_drop_owner_wrong_kind = true;
                                }
                                None => {
                                    new_flag_drop_owner_missing_or_empty = true;
                                }
                            }
                        }
                    }

                    summoned_expired_delete = true;
                }
            }
        }

        if loot_cleared && !non_consumed_chest_or_goober_return && !summoned_expired_delete {
            if let Some(game_object) = self
                .map_objects
                .get_mut(&game_object_guid)
                .and_then(MapObjectRecord::game_object_mut)
                .filter(|game_object| game_object.loot_state() == LootState::JustDeactivated)
            {
                // C++ anchor: GameObject.cpp:1639-1651. This represented seam
                // preserves the `if (!m_respawnDelayTime) return;` early return;
                // the positive-delay scheduling/SaveRespawnTime tail is consumed
                // immediately below after releasing the typed GameObject borrow.
                game_object.set_loot_state(LootState::NotReady, None);
                generic_not_ready = true;

                let go_type = game_object.data().type_id as u32;
                let despawn_at_action = match go_type {
                    GAMEOBJECT_TYPE_CHEST => game_object
                        .represented_chest_loot_source_like_cpp()
                        .map(|source| source.chest_consumable),
                    GAMEOBJECT_TYPE_GOOBER => game_object
                        .represented_goober_use_source_like_cpp()
                        .map(|source| source.consumable),
                    _ => Some(false),
                };
                generic_despawn_at_action_source_missing = despawn_at_action.is_none();
                let visual_despawn = despawn_at_action.unwrap_or(false)
                    || game_object.go_anim_progress_like_cpp() > 0;
                if visual_despawn {
                    generic_visual_despawn_represented = true;
                    generic_flags_restored_represented =
                        game_object.restore_represented_baseline_flags_like_cpp();
                }
                generic_zero_respawn_delay_return = game_object.respawn_delay_time() == 0;
            }
        }

        if generic_not_ready && !generic_zero_respawn_delay_return {
            let generic_respawn_snapshot = self
                .map_object_record(game_object_guid)
                .and_then(MapObjectRecord::game_object)
                .map(|game_object| {
                    (
                        game_object.spawned_by_default(),
                        game_object.respawn_compatibility_mode(),
                        game_object.respawn_delay_time(),
                        game_object.spawn_id(),
                        game_object.has_represented_gameobject_data_like_cpp(),
                        game_object.world().object().entry(),
                        game_object.world().position(),
                    )
                });

            if let Some((
                spawned_by_default,
                respawn_compatibility_mode,
                respawn_delay_time,
                spawn_id,
                represented_gameobject_data_present,
                entry,
                position,
            )) = generic_respawn_snapshot
            {
                if spawned_by_default {
                    let scheduled_respawn_time =
                        game_time_secs.saturating_add(i64::from(respawn_delay_time));
                    if let Some(game_object) = self
                        .map_objects
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(scheduled_respawn_time);
                    }
                    generic_respawn_scheduled_time = Some(scheduled_respawn_time);
                    generic_spawned_by_default_branch = true;

                    if !represented_gameobject_data_present {
                        // C++ `GameObject::SaveRespawnTime` is guarded by `m_goData`.
                        // A nonzero spawn id is not enough evidence for map-owned
                        // respawn persistence in this represented seam.
                        generic_respawn_save_missing_gameobject_data = true;
                    } else if spawn_id == 0 {
                        generic_respawn_save_missing_spawn_id = true;
                    } else if scheduled_respawn_time > game_time_secs {
                        if respawn_compatibility_mode {
                            // C++ `SaveRespawnTime` compatibility mode calls
                            // `SaveRespawnInfoDB` only. `wow-map` owns no async DB
                            // writes, so record DB-only evidence without mutating the
                            // map-owned respawn store.
                            generic_respawn_compatibility_db_only_represented = true;
                        } else {
                            let grid = compute_grid_coord(position.x, position.y);
                            let add_outcome = self.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
                                object_type: SpawnObjectType::GameObject,
                                spawn_id,
                                entry,
                                respawn_time: scheduled_respawn_time,
                                grid_id: grid.get_id(),
                            });
                            generic_respawn_timer_add = Some(add_outcome);
                        }
                    }

                    if respawn_compatibility_mode {
                        generic_visibility_on_destroy_represented = true;
                    }
                } else {
                    if let Some(game_object) = self
                        .map_objects
                        .get_mut(&game_object_guid)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        game_object.set_respawn_time(0);
                    }
                    generic_temporary_respawn_zeroed = true;
                    generic_visibility_on_destroy_represented = spawn_id != 0;
                }
            }
        }

        if summoned_expired_delete
            || (generic_not_ready
                && !generic_zero_respawn_delay_return
                && !generic_visibility_on_destroy_represented)
        {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: generic_visual_despawn_represented
                    || delete_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        } else if entity_update.status == EntityGameObjectUpdateStatusLikeCpp::DespawnRequested {
            let delete = self.gameobject_delete_from_update_with_optional_loader_like_cpp(
                game_object_guid,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            let generic_capture_point_removed_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.capture_point_packet_represented);
            let delete_visual_despawn_represented = delete
                .as_ref()
                .is_some_and(|delete| delete.despawn_packet_represented);
            let (status, remove_list) = match delete {
                Some(delete) if delete.pool_update_represented && delete.remove_list.is_none() => {
                    (GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated, None)
                }
                Some(delete) => (
                    GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued,
                    delete.remove_list,
                ),
                None => (GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued, None),
            };
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared: false,
                goober_spell_cast_spell_id: None,
                goober_spell_casts_represented: 0,
                goober_users_cleared: false,
                goober_state_reset: false,
                goober_nodespawn_return: false,
                non_consumed_chest_or_goober_return: false,
                non_consumed_restock_armed: false,
                non_consumed_set_ready: false,
                non_consumed_update_visibility_represented: false,
                non_consumed_update_dynamic_flags_represented: false,
                non_consumed_source_missing: false,
                summoned_expired_delete: false,
                summoned_expired_respawn_time_zeroed: false,
                summoned_expired_despawn_represented: false,
                summoned_expired_go_state_ready: false,
                new_flag_drop_owner_in_base_command_represented: false,
                new_flag_drop_owner_missing_or_empty: false,
                new_flag_drop_owner_wrong_kind: false,
                new_flag_drop_owner_not_new_flag: false,
                generic_not_ready: false,
                generic_capture_point_removed_represented,
                generic_visual_despawn_represented: delete_visual_despawn_represented,
                generic_flags_restored_represented: false,
                generic_zero_respawn_delay_return: false,
                generic_despawn_at_action_source_missing: false,
                generic_respawn_scheduled_time: None,
                generic_spawned_by_default_branch: false,
                generic_temporary_respawn_zeroed: false,
                generic_respawn_timer_add: None,
                generic_respawn_save_missing_spawn_id: false,
                generic_respawn_save_missing_gameobject_data: false,
                generic_respawn_compatibility_db_only_represented: false,
                generic_visibility_on_destroy_represented: false,
            }
        } else {
            GameObjectUpdateOutcomeLikeCpp {
                game_object_guid,
                diff_ms,
                status: GameObjectUpdateStatusLikeCpp::Updated,
                despawn_delay_before_ms: Some(entity_update.despawn_delay_before_ms),
                despawn_delay_after_ms: Some(entity_update.despawn_delay_after_ms),
                despawn_respawn_time_secs: Some(entity_update.despawn_respawn_time_secs),
                world_update_would_run: entity_update.world_update_would_run,
                ai_update_not_represented: entity_update.ai_update_not_represented,
                go_type_impl_update_not_represented: entity_update
                    .go_type_impl_update_not_represented,
                despawn_or_unsummon_requested: entity_update.despawn_or_unsummon_requested,
                entity_update: Some(entity_update),
                remove_list: None,
                linked_trap_guid,
                linked_trap_removed,
                linked_trap_remove_queued,
                linked_trap_missing_or_self,
                loot_cleared,
                goober_spell_cast_spell_id,
                goober_spell_casts_represented,
                goober_users_cleared,
                goober_state_reset,
                goober_nodespawn_return,
                non_consumed_chest_or_goober_return,
                non_consumed_restock_armed,
                non_consumed_set_ready,
                non_consumed_update_visibility_represented,
                non_consumed_update_dynamic_flags_represented,
                non_consumed_source_missing,
                summoned_expired_delete,
                summoned_expired_respawn_time_zeroed,
                summoned_expired_despawn_represented,
                summoned_expired_go_state_ready,
                new_flag_drop_owner_in_base_command_represented,
                new_flag_drop_owner_missing_or_empty,
                new_flag_drop_owner_wrong_kind,
                new_flag_drop_owner_not_new_flag,
                generic_not_ready,
                generic_capture_point_removed_represented: false,
                generic_visual_despawn_represented,
                generic_flags_restored_represented,
                generic_zero_respawn_delay_return,
                generic_despawn_at_action_source_missing,
                generic_respawn_scheduled_time,
                generic_spawned_by_default_branch,
                generic_temporary_respawn_zeroed,
                generic_respawn_timer_add,
                generic_respawn_save_missing_spawn_id,
                generic_respawn_save_missing_gameobject_data,
                generic_respawn_compatibility_db_only_represented,
                generic_visibility_on_destroy_represented,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `GameObject` records only.
    ///
    /// This snapshots canonical typed GameObject GUIDs from `Map::map_objects`
    /// and delegates each GUID to `update_game_object_like_cpp`. C++ visits by
    /// nearby cell/active object order; this slice only adds the missing
    /// map-owned GameObject family and keeps the existing Rust family order.
    pub fn update_game_objects_like_cpp(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
    ) -> GameObjectsUpdateSummaryLikeCpp {
        self.update_game_objects_with_optional_pool_update_like_cpp::<fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(diff_ms, game_time_secs, None, None)
    }

    pub fn update_game_objects_with_pool_update_like_cpp(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
    ) -> GameObjectsUpdateSummaryLikeCpp {
        self.update_game_objects_with_optional_pool_update_like_cpp(
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            None::<
                &mut fn(
                    &mut Self,
                    SpawnObjectType,
                    SpawnId,
                ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
            >,
        )
    }

    pub fn update_game_objects_with_pool_update_loaded_grid_records_like_cpp<L>(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut load_record: L,
    ) -> GameObjectsUpdateSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.update_game_objects_with_optional_pool_update_like_cpp(
            diff_ms,
            game_time_secs,
            Some((spawn_store, pool_mgr)),
            Some(&mut load_record),
        )
    }

    fn update_game_objects_with_optional_pool_update_like_cpp<L>(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        mut load_record: Option<&mut L>,
    ) -> GameObjectsUpdateSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let game_object_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::GameObject && record.game_object().is_some())
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = GameObjectsUpdateSummaryLikeCpp::default();
        for guid in game_object_guids {
            summary.visited += 1;
            let outcome = self.update_game_object_with_optional_pool_update_like_cpp(
                guid,
                diff_ms,
                game_time_secs,
                pool_update,
                load_record.as_mut().map(|loader| &mut **loader),
            );
            if outcome.linked_trap_removed {
                summary.linked_traps_removed += 1;
            }
            if outcome.linked_trap_remove_queued {
                summary.linked_traps_remove_queued += 1;
            }
            if outcome.loot_cleared {
                summary.loot_cleared += 1;
            }
            summary.goober_spell_casts_represented += outcome.goober_spell_casts_represented;
            if outcome.goober_users_cleared {
                summary.goober_users_cleared += 1;
            }
            if outcome.goober_state_reset {
                summary.goober_state_reset += 1;
            }
            if outcome.goober_nodespawn_return {
                summary.goober_nodespawn_returns += 1;
            }
            if outcome.non_consumed_chest_or_goober_return {
                summary.non_consumed_chest_or_goober_returns += 1;
            }
            if outcome.non_consumed_restock_armed {
                summary.non_consumed_restock_armed += 1;
            }
            if outcome.non_consumed_set_ready {
                summary.non_consumed_set_ready += 1;
            }
            if outcome.non_consumed_update_visibility_represented {
                summary.non_consumed_update_visibility_represented += 1;
            }
            if outcome.non_consumed_update_dynamic_flags_represented {
                summary.non_consumed_update_dynamic_flags_represented += 1;
            }
            if outcome.non_consumed_source_missing {
                summary.non_consumed_source_missing += 1;
            }
            if outcome.summoned_expired_delete {
                summary.summoned_expired_deletes += 1;
            }
            if outcome.summoned_expired_respawn_time_zeroed {
                summary.summoned_expired_respawn_time_zeroed += 1;
            }
            if outcome.summoned_expired_despawn_represented {
                summary.summoned_expired_despawn_represented += 1;
            }
            if outcome.summoned_expired_go_state_ready {
                summary.summoned_expired_go_state_ready += 1;
            }
            if outcome.new_flag_drop_owner_in_base_command_represented {
                summary.new_flag_drop_owner_in_base_commands_represented += 1;
            }
            if outcome.new_flag_drop_owner_missing_or_empty {
                summary.new_flag_drop_owner_missing_or_empty += 1;
            }
            if outcome.new_flag_drop_owner_wrong_kind {
                summary.new_flag_drop_owner_wrong_kind += 1;
            }
            if outcome.new_flag_drop_owner_not_new_flag {
                summary.new_flag_drop_owner_not_new_flag += 1;
            }
            if outcome.generic_not_ready {
                summary.generic_not_ready += 1;
            }
            if outcome.generic_capture_point_removed_represented {
                summary.generic_capture_point_removed_represented += 1;
                summary
                    .generic_capture_point_removed_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_visual_despawn_represented {
                summary.generic_visual_despawn_represented += 1;
                summary
                    .generic_visual_despawn_guids
                    .push(outcome.game_object_guid);
            }
            if outcome.generic_flags_restored_represented {
                summary.generic_flags_restored_represented += 1;
            }
            if outcome.generic_zero_respawn_delay_return {
                summary.generic_zero_respawn_delay_returns += 1;
            }
            if outcome.generic_despawn_at_action_source_missing {
                summary.generic_despawn_at_action_source_missing += 1;
            }
            if outcome.generic_respawn_scheduled_time.is_some() {
                summary.generic_respawn_scheduled += 1;
            }
            if outcome.generic_spawned_by_default_branch {
                summary.generic_spawned_by_default_branches += 1;
            }
            if outcome.generic_temporary_respawn_zeroed {
                summary.generic_temporary_respawn_zeroed += 1;
            }
            let map_timer_added = matches!(
                outcome.generic_respawn_timer_add,
                Some(
                    AddRespawnInfoOutcomeLikeCpp::Inserted
                        | AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
                )
            );
            if map_timer_added {
                summary.generic_respawn_timer_added += 1;
            }
            if outcome.generic_respawn_save_missing_spawn_id {
                summary.generic_respawn_save_missing_spawn_id += 1;
            }
            if outcome.generic_respawn_save_missing_gameobject_data {
                summary.generic_respawn_save_missing_gameobject_data += 1;
            }
            if outcome.generic_respawn_compatibility_db_only_represented {
                summary.generic_respawn_compatibility_db_only_represented += 1;
            }
            if (map_timer_added || outcome.generic_respawn_compatibility_db_only_represented)
                && let (Some(respawn_time), Some(game_object)) = (
                    outcome.generic_respawn_scheduled_time,
                    self.map_object_record(outcome.game_object_guid)
                        .and_then(MapObjectRecord::game_object),
                )
            {
                let position = game_object.world().position();
                summary.respawn_db_saves.push(RespawnInfoLikeCpp {
                    object_type: SpawnObjectType::GameObject,
                    spawn_id: game_object.spawn_id(),
                    entry: game_object.world().object().entry(),
                    respawn_time,
                    grid_id: compute_grid_coord(position.x, position.y).get_id(),
                });
            }
            if outcome.generic_visibility_on_destroy_represented {
                summary.generic_visibility_on_destroy_represented += 1;
                summary
                    .generic_visibility_on_destroy_guids
                    .push(outcome.game_object_guid);
            }
            match outcome.status {
                GameObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued => {
                    summary.despawn_remove_queued += 1;
                }
                GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated => {
                    summary.despawn_pool_updated += 1;
                }
                GameObjectUpdateStatusLikeCpp::MissingGameObject => summary.missing_or_stale += 1,
                GameObjectUpdateStatusLikeCpp::NotGameObject => summary.not_game_object += 1,
                GameObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `Transport::Update(uint32 diff)` under `Map::Update`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` updates object families, transport collection, then later
    ///   `SendObjectUpdates`; exact TypeContainerVisitor and `_transports` ordering is
    ///   not fully reproduced here.
    /// - `Transport.cpp:179-251` is represented only for local timers/path progress,
    ///   stop request evidence, client path-progress field, expected-map gated
    ///   200ms position-update due evidence, and stopped state/dynflag.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Transport and untyped Transport-kind outcomes do not mutate state.
    /// Unlike `ObjectUpdater::Visit<T>`, the C++ `_transports` loop does not gate
    /// canonical transports on `IsInWorld`, so typed Transport records are delegated
    /// even when their embedded WorldObject is not in-world. This helper never
    /// creates fallback records, reads session/ObjectAccessor mirrors, runs
    /// scripts/AI/GameEvents, computes real spline position, teleports,
    /// spawns/removes static passengers, relocates passengers, fans out packets, or
    /// drains queues.
    pub fn update_transport_like_cpp(
        &mut self,
        transport_guid: ObjectGuid,
        diff_ms: u32,
        now_ms: u64,
    ) -> TransportUpdateOutcomeLikeCpp {
        let current_map_id = self.map_id;
        let Some(record) = self.map_object_record(transport_guid) else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::MissingTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        if record.kind() != AccessorObjectKind::Transport {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        }

        let Some(transport) = record.transport() else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        let period_ms = transport.get_transport_period();
        let path_progress_before_ms = transport.path_progress_ms();

        let Some(record) = self.map_objects.get_mut(&transport_guid) else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::MissingTransport,
                period_ms: Some(period_ms),
                path_progress_before_ms: Some(path_progress_before_ms),
                path_progress_after_ms: Some(path_progress_before_ms),
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };
        let Some(transport) = record.transport_mut() else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: Some(period_ms),
                path_progress_before_ms: Some(path_progress_before_ms),
                path_progress_after_ms: Some(path_progress_before_ms),
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        let entity_update = transport.update_like_cpp(diff_ms, now_ms, current_map_id);
        let status = if entity_update.unsupported_no_period {
            TransportUpdateStatusLikeCpp::UnsupportedNoPeriod
        } else {
            TransportUpdateStatusLikeCpp::Updated
        };
        TransportUpdateOutcomeLikeCpp {
            transport_guid,
            diff_ms,
            now_ms,
            current_map_id,
            status,
            period_ms: Some(entity_update.period_ms),
            path_progress_before_ms: Some(entity_update.old_path_progress_ms),
            path_progress_after_ms: Some(entity_update.new_path_progress_ms),
            timer_ms: entity_update.timer_ms,
            expected_map_matches_current_map: entity_update.expected_map_matches_current_map,
            position_update_due: entity_update.position_update_due,
            position_update_represented: entity_update.position_update_represented,
            just_stopped: entity_update.just_stopped,
            entity_update: Some(entity_update),
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// typed canonical Transport records only. This snapshots `MapObjectRecord`
    /// GUIDs before mutation and deliberately excludes generic `WorldObject`
    /// fallback records even when their kind is Transport.
    pub fn update_transports_like_cpp(
        &mut self,
        diff_ms: u32,
        now_ms: u64,
    ) -> TransportsUpdateSummaryLikeCpp {
        let transport_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Transport && record.transport().is_some())
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = TransportsUpdateSummaryLikeCpp::default();
        for guid in transport_guids {
            summary.visited += 1;
            let outcome = self.update_transport_like_cpp(guid, diff_ms, now_ms);
            match outcome.status {
                TransportUpdateStatusLikeCpp::Updated => summary.updated += 1,
                TransportUpdateStatusLikeCpp::UnsupportedNoPeriod => {
                    summary.unsupported_no_period += 1;
                }
                TransportUpdateStatusLikeCpp::MissingTransport => summary.missing_or_stale += 1,
                TransportUpdateStatusLikeCpp::NotTransport => summary.not_transport += 1,
                TransportUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
            if outcome.position_update_represented {
                summary.position_updates_represented += 1;
            }
            if outcome.just_stopped {
                summary.just_stopped += 1;
            }
        }

        summary
    }

    /// Map-owned seam for C++ `Creature::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` uses `Trinity::ObjectUpdater` during `Map::Update`.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects, including the explicit `Creature` instantiation.
    /// - `Creature.cpp:696-903` is represented here only through the existing
    ///   `Creature::runtime_update_plan(diff, GameTime::GetGameTime(), context)`
    ///   helper; real AI/scripts/Unit::Update/fanout remain outside this slice.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Creature, and not-in-world outcomes do not mutate state. This helper
    /// never creates fallback records, reads session/ObjectAccessor mirrors,
    /// sends packets, runs DB writes, or drains map queues.
    pub fn update_creature_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        diff_ms: u32,
        now_secs: i64,
        context: CreatureRuntimeUpdateContext,
    ) -> CreatureUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(creature_guid) else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::MissingCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        if record.kind() != AccessorObjectKind::Creature {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        }

        let Some(creature) = record.creature() else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        if !creature.unit().world().object().is_in_world() {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotInWorld,
                plan: None,
                actions_recorded: 0,
            };
        }

        let Some(record) = self.map_objects.get_mut(&creature_guid) else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::MissingCreature,
                plan: None,
                actions_recorded: 0,
            };
        };
        let Some(creature) = record.creature_mut() else {
            return CreatureUpdateOutcomeLikeCpp {
                creature_guid,
                diff_ms,
                now_secs,
                status: CreatureUpdateStatusLikeCpp::NotCreature,
                plan: None,
                actions_recorded: 0,
            };
        };

        let plan = creature.runtime_update_plan(diff_ms, now_secs, context);
        let actions_recorded = plan.actions().len();
        CreatureUpdateOutcomeLikeCpp {
            creature_guid,
            diff_ms,
            now_secs,
            status: CreatureUpdateStatusLikeCpp::Updated,
            plan: Some(plan),
            actions_recorded,
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `CreatureMapType` records only.
    ///
    /// This snapshots canonical typed Creature GUIDs from `Map::map_objects`,
    /// resolves a represented runtime context from the caller before mutable
    /// access, then delegates to `update_creature_like_cpp`. It intentionally
    /// excludes Pet and every non-Creature family unless already stored as a typed
    /// `MapObjectRecord::Creature`.
    pub fn update_creatures_like_cpp<F>(
        &mut self,
        diff_ms: u32,
        now_secs: i64,
        mut context_resolver: F,
    ) -> CreatureUpdateSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &Creature) -> CreatureRuntimeUpdateContext,
    {
        let creature_guids = self.object_updater_creature_guids_like_cpp();

        let mut summary = CreatureUpdateSummaryLikeCpp::default();
        for guid in creature_guids {
            summary.visited += 1;
            let Some(context) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(|creature| context_resolver(guid, creature))
            else {
                let outcome = self.update_creature_like_cpp(
                    guid,
                    diff_ms,
                    now_secs,
                    CreatureRuntimeUpdateContext::default(),
                );
                match outcome.status {
                    CreatureUpdateStatusLikeCpp::MissingCreature => summary.skipped_missing += 1,
                    CreatureUpdateStatusLikeCpp::NotCreature => summary.skipped_non_creature += 1,
                    CreatureUpdateStatusLikeCpp::NotInWorld => summary.skipped_not_in_world += 1,
                    CreatureUpdateStatusLikeCpp::Updated => {
                        summary.updated += 1;
                        summary.actions_recorded += outcome.actions_recorded;
                    }
                }
                continue;
            };

            let outcome = self.update_creature_like_cpp(guid, diff_ms, now_secs, context);
            match outcome.status {
                CreatureUpdateStatusLikeCpp::Updated => {
                    summary.updated += 1;
                    summary.actions_recorded += outcome.actions_recorded;
                }
                CreatureUpdateStatusLikeCpp::MissingCreature => summary.skipped_missing += 1,
                CreatureUpdateStatusLikeCpp::NotCreature => summary.skipped_non_creature += 1,
                CreatureUpdateStatusLikeCpp::NotInWorld => summary.skipped_not_in_world += 1,
            }
        }

        summary
    }

    /// C++ `Trinity::ObjectUpdater` visits creature containers reachable from
    /// the map's loaded grids, not the global object accessor/store. Keeping the
    /// visitation anchored to cells prevents unloaded-grid records from being
    /// updated after `Map::UnloadGrid` has removed their NGrid.
    fn object_updater_creature_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut creature_guids = Vec::new();
        for grid in self.grids.iter().filter_map(|grid| grid.as_deref()) {
            grid.visit_all_grids(|cell| {
                creature_guids.extend(cell.grid_objects.creatures.iter().copied());
                creature_guids.extend(cell.world_objects.creatures.iter().copied());
            });
        }
        sort_dedup(&mut creature_guids);
        creature_guids
    }

    /// Map-owned seam for C++ `AreaTrigger::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `AreaTrigger.cpp:297-364` runs `WorldObject::Update(diff)`, increments
    ///   `_timeSinceCreated`, runs the non-static movement/orbit/shape branch
    ///   before duration expiry, calls `Remove(); return;` on duration expiry,
    ///   and only then runs AI update plus target-list update.
    /// - `AreaTrigger.cpp:366-372` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `AreaTrigger`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. This helper
    /// mutates only typed `MapObjectRecord::AreaTrigger` time/duration state and,
    /// after dropping that mutable borrow, enqueues the same GUID through the
    /// existing remove-list facade on expiry. It does not drain removal, run real
    /// movement/shape, AI, target-list runtime, ObjectAccessor/session mirrors,
    /// fanout, packets, dynamic tree, scripts, or create fallback records.
    pub fn update_area_trigger_like_cpp(
        &mut self,
        area_trigger_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> AreaTriggerUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(area_trigger_guid) else {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::AreaTrigger {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(area_trigger) = record.area_trigger() else {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = area_trigger.duration_ms();
        let time_since_created_before_ms = area_trigger.time_since_created_ms();
        let non_static_movement_would_run = !area_trigger.is_static_spawn();
        if !area_trigger.world().object().is_in_world() {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_before_ms),
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        }

        let (expired, duration_after_ms, time_since_created_after_ms) = {
            let Some(record) = self.map_objects.get_mut(&area_trigger_guid) else {
                return AreaTriggerUpdateOutcomeLikeCpp {
                    area_trigger_guid,
                    elapsed_ms,
                    status: AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    time_since_created_before_ms: Some(time_since_created_before_ms),
                    time_since_created_after_ms: Some(time_since_created_before_ms),
                    non_static_movement_would_run: false,
                    ai_update_would_run: false,
                    target_list_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(area_trigger) = record.area_trigger_mut() else {
                return AreaTriggerUpdateOutcomeLikeCpp {
                    area_trigger_guid,
                    elapsed_ms,
                    status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    time_since_created_before_ms: Some(time_since_created_before_ms),
                    time_since_created_after_ms: Some(time_since_created_before_ms),
                    non_static_movement_would_run: false,
                    ai_update_would_run: false,
                    target_list_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = area_trigger.update_time_and_duration(elapsed_ms);
            (
                expired,
                area_trigger.duration_ms(),
                area_trigger.time_since_created_ms(),
            )
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(area_trigger_guid);
            AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_after_ms),
                non_static_movement_would_run,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_after_ms),
                non_static_movement_would_run,
                ai_update_would_run: true,
                target_list_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `AreaTrigger` records only.
    ///
    /// This follows the same partial ObjectUpdater seam as DynamicObject: it
    /// snapshots canonical typed AreaTrigger GUIDs from `Map::map_objects`, then
    /// delegates every GUID to `update_area_trigger_like_cpp`. It does not visit
    /// nearby cells, players/sessions, other object families, SendObjectUpdates,
    /// scripts/AI real runtime, visibility, dynamic tree, packets, DB, or mirrors.
    pub fn update_area_triggers_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> AreaTriggersUpdateSummaryLikeCpp {
        let area_trigger_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::AreaTrigger
                    && record.area_trigger().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = AreaTriggersUpdateSummaryLikeCpp::default();
        for guid in area_trigger_guids {
            summary.visited += 1;
            let outcome = self.update_area_trigger_like_cpp(guid, elapsed_ms);
            match outcome.status {
                AreaTriggerUpdateStatusLikeCpp::Updated => summary.updated += 1,
                AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger => {
                    summary.missing_or_stale += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger => summary.not_area_trigger += 1,
                AreaTriggerUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// C++ `Unit::RemoveAllAreaTriggers` represented over map-owned AreaTriggers.
    ///
    /// C++ anchors:
    /// - `Player.cpp:1421-1422` calls `RemoveAllAreaTriggers()` during accepted
    ///   inter-map `Player::TeleportTo`, immediately after `RemoveAllDynObjects()`.
    /// - `Unit.cpp:5347-5351` repeatedly removes every AreaTrigger owned by the
    ///   Unit (`m_areaTrigger.back()->Remove()`).
    /// - `AreaTrigger.cpp:366-372` routes `Remove()` through the owning map
    ///   remove list only while the object is in world; Rust reuses
    ///   `remove_from_map_like_cpp(..., true)` to keep physical removal in one
    ///   canonical map path.
    ///
    /// Scope: source-of-truth is this canonical `Map::map_objects` store. This
    /// does not model the exact C++ `Unit::m_areaTrigger` vector ordering,
    /// destroy-packet fanout, ObjectAccessor/session mirrors, AI target list
    /// exits, scripts, DB, or cross-map lookup beyond this map.
    pub fn remove_all_area_triggers_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
        let mut guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::AreaTrigger {
                    return None;
                }
                let area_trigger = record.area_trigger()?;
                (area_trigger.caster_guid() == caster_guid).then_some(*guid)
            })
            .collect::<Vec<_>>();
        guids.sort_by_key(ObjectGuid::to_raw_bytes);

        let mut outcome = RemoveAllAreaTriggersForCasterOutcomeLikeCpp {
            caster_guid,
            candidates: guids.len(),
            removed: 0,
            missing_or_stale: 0,
            remove_errors: 0,
        };

        for guid in guids {
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(_) => {
                    outcome.removed += 1;
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    outcome.missing_or_stale += 1;
                }
                Err(_) => {
                    outcome.remove_errors += 1;
                }
            }
        }

        outcome
    }

    /// Map-owned seam for C++ `Conversation::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Conversation.cpp:67-80` runs `sScriptMgr->OnConversationUpdate` before
    ///   duration handling; on expiry it calls `Remove(); return;`, otherwise it
    ///   runs `WorldObject::Update(diff)`.
    /// - `Conversation.cpp:82-87` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `Conversation`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Conversation, and not-in-world outcomes do not mutate, enqueue, or
    /// create fallback records. This helper represents script and WorldObject
    /// update callsites as booleans only; it does not execute scripts, fanout,
    /// visibility, ObjectAccessor/session mirrors, DB writes, or remove-list drain.
    pub fn update_conversation_like_cpp(
        &mut self,
        conversation_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> ConversationUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(conversation_guid) else {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::MissingConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::Conversation {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(conversation) = record.conversation() else {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = conversation.duration_ms();
        if !conversation.world().object().is_in_world() {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        }

        let (expired, duration_after_ms) = {
            let Some(record) = self.map_objects.get_mut(&conversation_guid) else {
                return ConversationUpdateOutcomeLikeCpp {
                    conversation_guid,
                    elapsed_ms,
                    status: ConversationUpdateStatusLikeCpp::MissingConversation,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    script_update_would_run: false,
                    world_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(conversation) = record.conversation_mut() else {
                return ConversationUpdateOutcomeLikeCpp {
                    conversation_guid,
                    elapsed_ms,
                    status: ConversationUpdateStatusLikeCpp::NotConversation,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    script_update_would_run: false,
                    world_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = conversation.update_duration(elapsed_ms);
            (expired, conversation.duration_ms())
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(conversation_guid);
            ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                script_update_would_run: true,
                world_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                script_update_would_run: true,
                world_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `Conversation` records only.
    ///
    /// This snapshots canonical typed Conversation GUIDs from `Map::map_objects`,
    /// then delegates every GUID to `update_conversation_like_cpp`. It does not
    /// model exact `TypeContainerVisitor` order/cell traversal, players/sessions,
    /// other object families, `SendObjectUpdates`, real scripts, visibility,
    /// packets, DB, ObjectAccessor/session mirrors, or remove-list drain.
    pub fn update_conversations_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> ConversationsUpdateSummaryLikeCpp {
        let conversation_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Conversation
                    && record.conversation().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = ConversationsUpdateSummaryLikeCpp::default();
        for guid in conversation_guids {
            summary.visited += 1;
            let outcome = self.update_conversation_like_cpp(guid, elapsed_ms);
            match outcome.status {
                ConversationUpdateStatusLikeCpp::Updated => summary.updated += 1,
                ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                ConversationUpdateStatusLikeCpp::MissingConversation => {
                    summary.missing_or_stale += 1;
                }
                ConversationUpdateStatusLikeCpp::NotConversation => summary.not_conversation += 1,
                ConversationUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `SceneObject::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `SceneObject.cpp:58-71` runs `WorldObject::Update(diff)` and removes the
    ///   SceneObject when `ShouldBeRemoved()` is true.
    /// - `SceneObject.cpp:73-90` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when in world, and `ShouldBeRemoved()`
    ///   depends on `ObjectAccessor::GetUnit(owner)` plus optional Aura lookup by
    ///   spell/cast id.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `SceneObjectMapType`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. ObjectAccessor
    /// Unit resolution and Aura lookup are represented by explicit caller-supplied
    /// booleans; this helper does not scan maps, create fallback records, fan out,
    /// send packets, write session/ObjectAccessor mirrors, or drain remove-list.
    pub fn update_scene_object_like_cpp(
        &mut self,
        scene_object_guid: ObjectGuid,
        elapsed_ms: u32,
        context: SceneObjectUpdateContextLikeCpp,
    ) -> SceneObjectUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(scene_object_guid) else {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::MissingSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::SceneObject {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        }

        let Some(scene_object) = record.scene_object() else {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        };

        let owner_guid = scene_object.owner_guid();
        let created_by_spell_cast = scene_object.created_by_spell_cast();
        if !scene_object.world().object().is_in_world() {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotInWorld,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        }

        let should_be_removed =
            scene_object.should_be_removed(context.creator_exists, context.linked_aura_exists);

        if should_be_removed {
            let remove_list = self.add_object_to_remove_list_like_cpp(scene_object_guid);
            SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::RemoveQueued,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: true,
                should_be_removed,
                remove_list: Some(remove_list),
            }
        } else {
            SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::Updated,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: true,
                should_be_removed,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `SceneObject` records only.
    ///
    /// This snapshots canonical typed SceneObject GUIDs from `Map::map_objects`,
    /// resolves the explicit represented ObjectAccessor/Aura context before the
    /// per-object helper, and never visits generic/untyped SceneObject records.
    pub fn update_scene_objects_like_cpp<F>(
        &mut self,
        elapsed_ms: u32,
        mut context_resolver: F,
    ) -> SceneObjectsUpdateSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &SceneObject) -> SceneObjectUpdateContextLikeCpp,
    {
        let scene_object_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::SceneObject
                    && record.scene_object().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = SceneObjectsUpdateSummaryLikeCpp::default();
        for guid in scene_object_guids {
            summary.visited += 1;
            let Some(context) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::scene_object)
                .map(|scene_object| context_resolver(guid, scene_object))
            else {
                let outcome = self.update_scene_object_like_cpp(
                    guid,
                    elapsed_ms,
                    SceneObjectUpdateContextLikeCpp::default(),
                );
                match outcome.status {
                    SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                        summary.missing_or_stale += 1;
                    }
                    SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                    SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
                    SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                    SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                }
                continue;
            };

            let outcome = self.update_scene_object_like_cpp(guid, elapsed_ms, context);
            match outcome.status {
                SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                    summary.missing_or_stale += 1;
                }
                SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Bounded map-owned caller-consumption seam for C++
    /// `DynamicObject::SetCasterViewpoint` / `RemoveCasterViewpoint`.
    ///
    /// C++ anchors:
    /// - `DynamicObject.cpp:209-225` resolves the caster from the DynamicObject's
    ///   `_caster`, calls `Player::SetViewpoint(this, apply)` only when `_caster`
    ///   is a Player, and then toggles `_isViewpoint` without checking the Player
    ///   helper's early-return result.
    /// - `DynamicObject.cpp:233-239` represents `_caster` as a previously bound
    ///   same-map Unit pointer; this helper consumes `DynamicObject::bound_caster()`
    ///   as that represented pointer equivalent and never falls back to the raw
    ///   caster GUID field or to a caller-provided Player.
    /// - `Player.cpp:25344-25387` owns FarsightObject guards/mutations,
    ///   `UpdateVisibilityOf` on apply, and `SetSeer`; DynamicObject targets do
    ///   not run the Unit shared-vision / SetWorldObject branch.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. The helper
    /// first validates the typed DynamicObject record, then resolves the Player
    /// from `DynamicObject::bound_caster()` before any Player mutation. It does
    /// not create records, silently fall back from `caster_guid`, drain
    /// switch/remove lists, fan out visibility, implement full SetSeer, write
    /// session/ObjectAccessor mirrors, send packets, or touch DB.
    pub fn apply_dynamic_object_caster_viewpoint_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
        apply: bool,
    ) -> DynamicObjectCasterViewpointOutcomeLikeCpp {
        let outcome =
            |player_guid, status, player_set_viewpoint, dynamic_object_viewpoint_toggled| {
                DynamicObjectCasterViewpointOutcomeLikeCpp {
                    player_guid,
                    dynamic_object_guid,
                    apply,
                    status,
                    player_set_viewpoint,
                    dynamic_object_viewpoint_toggled,
                }
            };
        let player_outcome = |player_guid, status| {
            Self::player_set_viewpoint_outcome_like_cpp(
                player_guid,
                dynamic_object_guid,
                apply,
                status,
                None,
                false,
                false,
            )
        };

        let Some(dynamic_object) = self
            .map_object_record(dynamic_object_guid)
            .and_then(MapObjectRecord::dynamic_object)
        else {
            return outcome(
                ObjectGuid::EMPTY,
                DynamicObjectCasterViewpointStatusLikeCpp::MissingDynamicObject,
                player_outcome(
                    ObjectGuid::EMPTY,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                ),
                false,
            );
        };

        let Some(player_guid) = dynamic_object.bound_caster() else {
            return outcome(
                ObjectGuid::EMPTY,
                DynamicObjectCasterViewpointStatusLikeCpp::MissingCaster,
                player_outcome(
                    ObjectGuid::EMPTY,
                    PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                ),
                false,
            );
        };

        let Some(player) = self.get_typed_player(player_guid) else {
            return outcome(
                player_guid,
                DynamicObjectCasterViewpointStatusLikeCpp::CasterNotPlayer,
                player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer),
                false,
            );
        };
        let current_farsight = player.active_data().farsight_object;

        let player_set_viewpoint = if apply {
            if current_farsight.is_empty() {
                if let Some(player) = self.get_typed_player_mut(player_guid) {
                    player.set_farsight_object_like_cpp(dynamic_object_guid);
                    Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        dynamic_object_guid,
                        apply,
                        PlayerSetViewpointStatusLikeCpp::Applied,
                        None,
                        true,
                        true,
                    )
                } else {
                    player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer)
                }
            } else {
                player_outcome(
                    player_guid,
                    PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint,
                )
            }
        } else if current_farsight == dynamic_object_guid {
            if let Some(player) = self.get_typed_player_mut(player_guid) {
                player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                Self::player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    dynamic_object_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::Removed,
                    None,
                    false,
                    true,
                )
            } else {
                player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer)
            }
        } else {
            player_outcome(
                player_guid,
                PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
            )
        };

        let mut dynamic_object_viewpoint_toggled = false;
        if let Some(record) = self.map_objects.get_mut(&dynamic_object_guid) {
            if let Some(dynamic_object) = record.dynamic_object_mut() {
                if apply {
                    dynamic_object.set_caster_viewpoint();
                } else {
                    dynamic_object.remove_caster_viewpoint();
                }
                dynamic_object_viewpoint_toggled = true;
            }
        }

        outcome(
            player_guid,
            DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved,
            player_set_viewpoint,
            dynamic_object_viewpoint_toggled,
        )
    }

    /// C++ `Map::AddObjectToSwitchList` represented over canonical map records.
    ///
    /// C++ anchors:
    /// - `Map.h:345-346` declares `AddObjectToRemoveList` beside
    ///   `AddObjectToSwitchList`; `Map.h:651-652` owns both queues.
    /// - `Map.cpp:2557-2572` accepts only `TYPEID_UNIT`, inserts first toggle,
    ///   cancels an opposite pending toggle, and aborts on duplicate direction.
    /// - `Object.cpp:910-915` shows `WorldObject::SetWorldObject(on)` enqueues
    ///   through the owning map only when the object is already in world.
    pub fn add_object_to_switch_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> AddObjectToSwitchListOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::MissingOrStale,
            };
        };

        debug_assert_eq!(record.object().map_id(), self.map_id);
        debug_assert_eq!(record.object().instance_id(), self.instance_id);

        if !switch_list_unit_kind_like_cpp(record.kind()) {
            return AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit,
            };
        }

        match self.objects_to_switch.get(&guid).copied() {
            None => {
                self.objects_to_switch.insert(guid, on);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::Queued,
                }
            }
            Some(existing) if existing != on => {
                self.objects_to_switch.remove(&guid);
                AddObjectToSwitchListOutcomeLikeCpp {
                    guid,
                    on,
                    status: AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle,
                }
            }
            Some(_) => AddObjectToSwitchListOutcomeLikeCpp {
                guid,
                on,
                status: AddObjectToSwitchListStatusLikeCpp::DuplicateSameDirectionAbort,
            },
        }
    }

    /// C++ `Map::RemoveAllObjectsInRemoveList` physical map-local drain.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2574-2594` drains `i_objectsToSwitch` first and calls
    ///   `SwitchGridContainers<Creature>` for non-permanent Unit objects.
    /// - `Map.cpp:2596-2646` then drains `i_objectsToRemove`; supported grid
    ///   object types call `RemoveFromMap(..., true)`, Creature runs a second
    ///   `CleanupsBeforeDelete()` immediately before removal, and non-grid types
    ///   are logged/ignored.
    /// - `Map.cpp:933-951` shows `RemoveFromMap(T*, true)` does the physical map
    ///   removal/reset/delete path.
    pub fn remove_all_objects_in_remove_list_like_cpp(
        &mut self,
    ) -> RemoveAllObjectsInRemoveListOutcomeLikeCpp {
        let mut switches = self.objects_to_switch.drain().collect::<Vec<_>>();
        switches.sort_by_key(|(guid, _)| guid.to_raw_bytes());
        let mut outcome = RemoveAllObjectsInRemoveListOutcomeLikeCpp {
            switch_processed: switches.len(),
            ..Default::default()
        };

        for (guid, on) in switches {
            let switch = self.switch_grid_containers_like_cpp(guid, on);
            if switch.executed {
                outcome.switch_executed += 1;
            } else if switch.missing_or_stale {
                outcome.switch_missing_or_stale += 1;
            } else if switch.unsupported_kind {
                outcome.switch_unsupported_kinds += 1;
            } else if switch.permanent_world_object {
                outcome.switch_permanent_world_objects += 1;
            } else if switch.invalid_or_unloaded_grid {
                outcome.switch_invalid_or_unloaded_grid += 1;
            }
        }

        while let Some(guid) = self.objects_to_remove.iter().next().copied() {
            self.objects_to_remove.remove(&guid);
            outcome.processed += 1;
            let Some(kind) = self.map_object_record(guid).map(MapObjectRecord::kind) else {
                outcome.missing_or_stale += 1;
                continue;
            };

            if remove_list_grid_kind_like_cpp(kind).is_none() {
                outcome.unsupported_kinds += 1;
                continue;
            }

            if matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet) {
                if let Some(record) = self.map_objects.get_mut(&guid) {
                    outcome.creature_second_cleanup_count +=
                        cleanup_map_object_record_before_delete_like_cpp(record, kind, true);
                }
            }

            match self.remove_from_map_like_cpp(guid, true) {
                Ok(removed) => {
                    outcome.removed += 1;
                    if let Some(cleanup) = removed.dynamic_object_remove_cleanup {
                        if cleanup.removed_aura_pending_delete {
                            outcome.dynamic_object_remove_aura_cleanup_count += 1;
                        }
                        if cleanup.unbound_caster.is_some() {
                            outcome.dynamic_object_unbound_caster_count += 1;
                        }
                    }
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => outcome.missing_or_stale += 1,
                Err(_) => outcome.remove_errors += 1,
            }
        }

        outcome
    }

    /// C++ `Unit::RemoveAllDynObjects` represented over map-owned DynamicObjects.
    ///
    /// C++ anchors:
    /// - `Player.cpp:1418-1419` calls `RemoveAllDynObjects()` during accepted
    ///   inter-map `Player::TeleportTo`.
    /// - `Unit.cpp:5169-5174` repeatedly removes every DynamicObject owned by
    ///   the Unit (`m_dynObj.back()->Remove()`).
    /// - `DynamicObject.cpp:167-171` routes `Remove()` through the owning
    ///   map remove list; Rust reuses `remove_from_map_like_cpp(..., true)` so
    ///   aura and caster-unbind cleanup stays in the canonical remove path.
    ///
    /// Scope: source-of-truth is this canonical `Map::map_objects` store. This
    /// does not model the C++ `Unit::m_dynObj` vector ordering, session fanout,
    /// destroy packets, ObjectAccessor mirrors, scripts, DB, or cross-map
    /// instance lookup beyond this map.
    pub fn remove_all_dynamic_objects_for_caster_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
    ) -> RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
        let mut guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                if record.kind() != AccessorObjectKind::DynamicObject {
                    return None;
                }
                let dynamic_object = record.dynamic_object()?;
                (dynamic_object.caster_guid() == caster_guid).then_some(*guid)
            })
            .collect::<Vec<_>>();
        guids.sort_by_key(ObjectGuid::to_raw_bytes);

        let mut outcome = RemoveAllDynamicObjectsForCasterOutcomeLikeCpp {
            caster_guid,
            candidates: guids.len(),
            removed: 0,
            missing_or_stale: 0,
            remove_errors: 0,
            dynamic_object_remove_aura_cleanup_count: 0,
            dynamic_object_unbound_caster_count: 0,
        };

        for guid in guids {
            match self.remove_from_map_like_cpp(guid, true) {
                Ok(removed) => {
                    outcome.removed += 1;
                    if let Some(cleanup) = removed.dynamic_object_remove_cleanup {
                        if cleanup.removed_aura_pending_delete {
                            outcome.dynamic_object_remove_aura_cleanup_count += 1;
                        }
                        if cleanup.unbound_caster.is_some() {
                            outcome.dynamic_object_unbound_caster_count += 1;
                        }
                    }
                }
                Err(RemoveFromMapError::ObjectNotFound { .. }) => {
                    outcome.missing_or_stale += 1;
                }
                Err(_) => {
                    outcome.remove_errors += 1;
                }
            }
        }

        outcome
    }

    /// C++ `Map::SwitchGridContainers<Creature>` represented for Creature/Pet.
    ///
    /// C++ anchors:
    /// - `Map.cpp:260-305` computes the current cell, returns on invalid coords or
    ///   unloaded grid, moves Unit GUID between `grid_objects.creatures` and
    ///   `world_objects.creatures`, then writes `Creature::m_isTempWorldObject`.
    /// - `Object.cpp:918-925` makes `WorldObject::IsWorldObject` true for a
    ///   Creature with `m_isTempWorldObject`, while `Object.h:723-724` keeps
    ///   permanent world-object state in base `m_isWorldObject`.
    fn switch_grid_containers_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> SwitchGridContainersOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return SwitchGridContainersOutcomeLikeCpp::missing_or_stale();
        };
        let kind = record.kind();
        if !switch_list_unit_kind_like_cpp(kind) {
            return SwitchGridContainersOutcomeLikeCpp::unsupported_kind();
        }
        if record.object().is_world_object() {
            return SwitchGridContainersOutcomeLikeCpp::permanent_world_object();
        }

        let position = record.object().position();
        if !is_valid_map_coord_2d(position.x, position.y) {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        }

        let cell = Cell::from_world(position.x, position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        if !self.is_grid_loaded(grid) {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        }

        let Some(ngrid) = self.get_ngrid_mut(grid) else {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        };
        let Some(local_cell) = ngrid.get_grid_type_mut(cell.cell_x(), cell.cell_y()) else {
            return SwitchGridContainersOutcomeLikeCpp::invalid_or_unloaded_grid();
        };

        if on {
            local_cell.grid_objects.creatures.remove(&guid);
            local_cell.world_objects.creatures.insert(guid);
        } else {
            local_cell.world_objects.creatures.remove(&guid);
            local_cell.grid_objects.creatures.insert(guid);
        }

        if let Some(record) = self.map_objects.get_mut(&guid) {
            set_record_temp_world_object_like_cpp(record, on);
        }

        SwitchGridContainersOutcomeLikeCpp::executed()
    }

    /// C++ `Map::DespawnAll` represented over map-local by-spawn indexes.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2034-2055` snapshots Creature/GameObject by-spawn stores and
    ///   queues each object through `AddObjectToRemoveList`.
    /// - `Map.cpp:2547-2555` marks each queued object destroyed and runs cleanup
    ///   before insertion into the map-owned remove-list.
    /// - `Map.cpp:2574-2646` later physically drains the list.
    pub fn despawn_all_by_spawn_id_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> DespawnAllBySpawnIdOutcomeLikeCpp {
        let mut outcome = DespawnAllBySpawnIdOutcomeLikeCpp {
            object_type,
            spawn_id,
            queued: 0,
            removed: 0,
            duplicates: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_type: 0,
        };

        let guids = match object_type {
            SpawnObjectType::Creature => self.creature_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::GameObject => self.gameobject_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::AreaTrigger => {
                outcome.unsupported_live_despawn_type = 1;
                return outcome;
            }
        };

        for guid in guids {
            let still_matches = match object_type {
                SpawnObjectType::Creature => self
                    .map_object_record(guid)
                    .and_then(MapObjectRecord::creature)
                    .is_some_and(|creature| creature.spawn_id() == spawn_id),
                SpawnObjectType::GameObject => self
                    .map_object_record(guid)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|gameobject| gameobject.spawn_id() == spawn_id),
                SpawnObjectType::AreaTrigger => false,
            };
            if !still_matches {
                outcome.stale_index_entries += 1;
                continue;
            }

            let queued = self.add_object_to_remove_list_like_cpp(guid);
            if queued.missing_or_stale {
                outcome.stale_index_entries += 1;
            } else if queued.unsupported_kind.is_some() {
                outcome.unsupported_live_despawn_type += 1;
            } else if queued.duplicate {
                outcome.duplicates += 1;
            } else if queued.queued {
                outcome.queued += 1;
            }
        }

        outcome
    }

    /// C++ `Map::SpawnGroupDespawn(groupId, deleteRespawnTimes)` represented over
    /// map-owned runtime state and caller-supplied ObjectMgr-like `SpawnStore`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2404-2425` validates existing/non-system group, iterates
    ///   `sObjectMgr->GetSpawnMetadataForGroup`, optionally calls
    ///   `RemoveRespawnTime`, calls `DespawnAll`, then marks the group inactive.
    /// - `Map.cpp:2140-2163` DB delete is owned by callers; this helper only
    ///   mutates map-owned respawn timers so world-server can derive before/after
    ///   `CHAR_DEL_RESPAWN` work outside the lock.
    pub fn spawn_group_despawn_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        delete_respawn_times: bool,
        spawn_store: &SpawnStore,
    ) -> SpawnGroupDespawnOutcomeLikeCpp {
        let Some(group) = group else {
            return SpawnGroupDespawnOutcomeLikeCpp::blocked_missing_group(0);
        };
        if group.is_system() {
            return SpawnGroupDespawnOutcomeLikeCpp::blocked_system_group(group.group_id);
        }

        let mut outcome = SpawnGroupDespawnOutcomeLikeCpp::executed(group.group_id);
        if let Some(members) = spawn_store.spawn_group_members(group.group_id) {
            let members = members.iter().copied().collect::<Vec<_>>();
            for member in members {
                let Some(spawn_data) = spawn_store.spawn_data(member.object_type, member.spawn_id)
                else {
                    outcome.metadata_entries += 1;
                    outcome.stale_index_entries += 1;
                    continue;
                };
                if spawn_data.map_id != self.map_id {
                    continue;
                }

                outcome.metadata_entries += 1;
                if delete_respawn_times {
                    match member.object_type {
                        SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                            if self
                                .remove_respawn_time_like_cpp(member.object_type, member.spawn_id)
                                .is_some()
                            {
                                outcome.respawn_timers_removed += 1;
                            } else {
                                outcome.respawn_timers_missing += 1;
                            }
                        }
                        SpawnObjectType::AreaTrigger => {
                            outcome.respawn_timer_unsupported_types += 1;
                        }
                    }
                }

                let despawn =
                    self.despawn_all_by_spawn_id_like_cpp(member.object_type, member.spawn_id);
                outcome.objects_removed += despawn.queued;
                outcome.stale_index_entries += despawn.stale_index_entries;
                outcome.remove_errors += despawn.remove_errors;
                outcome.unsupported_live_despawn_types += despawn.unsupported_live_despawn_type;
            }
        }
        outcome.applied_inactive_change =
            Some(self.set_spawn_group_active_like_cpp(Some(group), false));
        outcome
    }

    /// C++ `Map::SpawnGroupSpawn(groupId, ignoreRespawn, force)` represented as a
    /// safe map-local planning/execution seam over map-owned active state,
    /// respawn timers, by-spawn live-object indexes, and optional caller-supplied
    /// loaded-grid DB-backed records.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2315-2324` validates existing/non-system group and marks it
    ///   active before iterating metadata.
    /// - `Map.cpp:2326-2353` iterates ObjectMgr spawn metadata, removes respawn
    ///   timers when forced/ignoring, skips active timers and live objects.
    /// - `Map.cpp:2326-2334` skips types whose `GetRespawnMapForType` is null;
    ///   `Map.h:751-763,765-777` currently returns null for AreaTrigger, so that
    ///   type is continued before timers, TypeHasData, difficulty, grid, or loader
    ///   planning.
    /// - `Map.cpp:2356-2385` checks difficulty/grid-loaded before calling
    ///   Creature/GameObject `LoadFromDB` and retaining the loaded object.
    /// - `Map.cpp:2387-2395` contains an AreaTrigger switch branch, but it is
    ///   unreachable with the current respawn-map guard. This does not implement
    ///   `AreaTrigger::LoadFromDB` or live AreaTrigger runtime.
    ///
    /// Ownership: `Map` owns active spawn-group state, respawn timers, live indexes,
    /// and `AddToMap`. The caller owns DB/template/runtime selection and may provide
    /// typed `LoadedGridRespawnRecordsLikeCpp` records. Synchronization is strictly
    /// caller loader -> map-owned `AddToMap`; this method never fabricates fallback
    /// records and never reaches into DB/world-server/session state.
    pub fn spawn_group_spawn_loaded_grid_records_like_cpp<L>(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
        mut load_record: L,
    ) -> SpawnGroupSpawnOutcomeLikeCpp
    where
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(group) = group else {
            return SpawnGroupSpawnOutcomeLikeCpp::blocked_missing_group(0);
        };
        if group.is_system() {
            return SpawnGroupSpawnOutcomeLikeCpp::blocked_system_group(group.group_id);
        }

        let mut outcome = SpawnGroupSpawnOutcomeLikeCpp::executed(group.group_id);
        outcome.applied_active_change =
            Some(self.set_spawn_group_active_like_cpp(Some(group), true));

        if let Some(members) = spawn_store.spawn_group_members(group.group_id) {
            let members = members.iter().copied().collect::<Vec<_>>();
            for member in members {
                let Some(spawn_data) = spawn_store.spawn_data(member.object_type, member.spawn_id)
                else {
                    outcome.stale_index_entries += 1;
                    continue;
                };
                if spawn_data.map_id != self.map_id {
                    continue;
                }

                outcome.metadata_entries += 1;
                match member.object_type {
                    SpawnObjectType::Creature | SpawnObjectType::GameObject => {
                        if force || ignore_respawn {
                            if self
                                .remove_respawn_time_like_cpp(member.object_type, member.spawn_id)
                                .is_some()
                            {
                                outcome.respawn_timers_removed += 1;
                            } else {
                                outcome.respawn_timers_missing += 1;
                            }
                        }

                        if self.get_respawn_time_like_cpp(member.object_type, member.spawn_id) != 0
                        {
                            outcome.skipped_respawn_timer_active += 1;
                            continue;
                        }

                        if !force {
                            let live_blocks = match member.object_type {
                                SpawnObjectType::Creature => self
                                    .get_creature_by_spawn_id_like_cpp(member.spawn_id)
                                    .is_some_and(Creature::is_alive),
                                SpawnObjectType::GameObject => self
                                    .get_gameobject_by_spawn_id_like_cpp(member.spawn_id)
                                    .is_some(),
                                SpawnObjectType::AreaTrigger => false,
                            };
                            if live_blocks {
                                outcome.skipped_live_object_active += 1;
                                continue;
                            }
                        }
                    }
                    SpawnObjectType::AreaTrigger => {
                        outcome.skipped_no_respawn_map += 1;
                        continue;
                    }
                }

                if !spawn_data.spawn_difficulties.contains(&self.spawn_mode()) {
                    outcome.skipped_difficulty_mismatch += 1;
                    continue;
                }

                let cell = cell_from_world(spawn_data.spawn_point.x, spawn_data.spawn_point.y);
                let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
                if !self.is_grid_loaded(grid) {
                    outcome.skipped_unloaded_grid += 1;
                    continue;
                }

                outcome.load_plans.push(SpawnGroupSpawnLoadPlanLikeCpp {
                    object_type: member.object_type,
                    spawn_id: member.spawn_id,
                    force,
                });

                let Some(records) = load_record(self, member.object_type, member.spawn_id, force)
                else {
                    outcome.blocked_loaded_grid_spawn_loads += 1;
                    if member.object_type == SpawnObjectType::Creature {
                        outcome.blocked_loaded_grid_creature_loads += 1;
                    } else if member.object_type == SpawnObjectType::GameObject {
                        outcome.blocked_loaded_grid_gameobject_loads += 1;
                    }
                    continue;
                };

                for pre_add_record in records.pre_add_records {
                    let _ = self.add_map_object_record_to_map_like_cpp(pre_add_record);
                }
                let primary_record = records.primary_record;
                let loaded_grid_primary_record = primary_record.clone();
                match self.add_map_object_record_to_map_like_cpp(primary_record) {
                    Ok(_outcome) => {
                        outcome.executed_loaded_grid_spawns += 1;
                        outcome
                            .loaded_grid_primary_records
                            .push(loaded_grid_primary_record);
                    }
                    Err(_error) => outcome.blocked_loaded_grid_spawn_add_to_map += 1,
                }
            }
        }

        outcome
    }

    /// Compatibility wrapper preserving the pre-loader `SpawnGroupSpawn` seam:
    /// loaded-grid Creature/GameObject attempts are planned and counted as blocked,
    /// but no DB-backed records are fabricated or inserted.
    pub fn spawn_group_spawn_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        ignore_respawn: bool,
        force: bool,
        spawn_store: &SpawnStore,
    ) -> SpawnGroupSpawnOutcomeLikeCpp {
        self.spawn_group_spawn_loaded_grid_records_like_cpp(
            group,
            ignore_respawn,
            force,
            spawn_store,
            |_map, _object_type, _spawn_id, _force| None,
        )
    }

    /// C++-shaped `Map::UpdateSpawnGroupConditions` bridge over pre-resolved
    /// templates that executes the complete represented `SetSpawnGroupInactive`
    /// branch, the map-local `SpawnGroupDespawn(..., true)` condition-failure
    /// branch, and the safe map-local `SpawnGroupSpawn` loaded-grid branch with
    /// caller-supplied records.
    ///
    /// Ownership remains split like `spawn_group_spawn_loaded_grid_records_like_cpp`:
    /// this map owns active-state/timer/live/grid/difficulty/AddToMap decisions;
    /// the caller owns DB/template/runtime composition and may return no record to
    /// preserve the pre-loader planned/blocked outcome.
    pub fn apply_update_spawn_group_conditions_loaded_grid_records_like_cpp<'a, I, F, L>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
        mut load_record: L,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
        L: FnMut(
            &mut Self,
            SpawnObjectType,
            SpawnId,
            bool,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let groups = groups.into_iter().collect::<Vec<_>>();
        let planned_actions = self
            .plan_update_spawn_group_conditions_like_cpp(groups.iter().copied(), meets_conditions);

        planned_actions
            .into_iter()
            .zip(groups)
            .map(|((group_id, action), group)| {
                let mut applied_change = None;
                let mut despawn_outcome = None;
                let mut spawn_outcome = None;
                match action {
                    SpawnGroupConditionActionLikeCpp::SetInactive => {
                        applied_change = Some(self.set_spawn_group_inactive_like_cpp(Some(group)));
                    }
                    SpawnGroupConditionActionLikeCpp::Despawn {
                        delete_respawn_times,
                    } => {
                        despawn_outcome = Some(self.spawn_group_despawn_like_cpp(
                            Some(group),
                            delete_respawn_times,
                            spawn_store,
                        ));
                    }
                    SpawnGroupConditionActionLikeCpp::Spawn {
                        ignore_respawn,
                        force,
                    } => {
                        spawn_outcome = Some(self.spawn_group_spawn_loaded_grid_records_like_cpp(
                            Some(group),
                            ignore_respawn,
                            force,
                            spawn_store,
                            &mut load_record,
                        ));
                    }
                    SpawnGroupConditionActionLikeCpp::Noop => {}
                }

                SpawnGroupConditionUpdateOutcomeLikeCpp {
                    group_id,
                    action,
                    applied_change,
                    despawn_outcome,
                    spawn_outcome,
                }
            })
            .collect()
    }

    /// Compatibility wrapper preserving the pre-loader `UpdateSpawnGroupConditions`
    /// seam: loaded-grid Creature/GameObject spawn attempts are planned and counted
    /// as blocked, but no DB-backed records are fabricated or inserted.
    pub fn apply_update_spawn_group_conditions_represented_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        spawn_store: &SpawnStore,
        meets_conditions: F,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        self.apply_update_spawn_group_conditions_loaded_grid_records_like_cpp(
            groups,
            spawn_store,
            meets_conditions,
            |_map, _object_type, _spawn_id, _force| None,
        )
    }

    /// Legacy wrapper preserving the pre-#391 SetInactive-only seam for focused
    /// tests/callers that explicitly require planned-only despawn evidence.
    pub fn apply_update_spawn_group_conditions_set_inactive_like_cpp<'a, I, F>(
        &mut self,
        groups: I,
        meets_conditions: F,
    ) -> Vec<SpawnGroupConditionUpdateOutcomeLikeCpp>
    where
        I: IntoIterator<Item = &'a SpawnGroupTemplateData>,
        F: FnMut(&SpawnGroupTemplateData) -> bool,
    {
        let groups = groups.into_iter().collect::<Vec<_>>();
        let planned_actions = self
            .plan_update_spawn_group_conditions_like_cpp(groups.iter().copied(), meets_conditions);

        planned_actions
            .into_iter()
            .zip(groups)
            .map(|((group_id, action), group)| {
                let applied_change = if action == SpawnGroupConditionActionLikeCpp::SetInactive {
                    Some(self.set_spawn_group_inactive_like_cpp(Some(group)))
                } else {
                    None
                };

                SpawnGroupConditionUpdateOutcomeLikeCpp {
                    group_id,
                    action,
                    applied_change,
                    despawn_outcome: None,
                    spawn_outcome: None,
                }
            })
            .collect()
    }

    pub fn map_object_count(&self) -> usize {
        self.map_objects.len()
    }

    pub fn objects_to_remove_count_like_cpp(&self) -> usize {
        self.objects_to_remove.len()
    }

    pub fn objects_to_switch_count_like_cpp(&self) -> usize {
        self.objects_to_switch.len()
    }

    pub fn pending_switch_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        self.objects_to_switch.get(&guid).copied()
    }

    #[cfg(test)]
    fn enqueue_object_to_remove_for_test(&mut self, guid: ObjectGuid) {
        self.objects_to_remove.insert(guid);
    }

    #[cfg(test)]
    fn enqueue_object_to_switch_for_test(&mut self, guid: ObjectGuid, on: bool) {
        self.objects_to_switch.insert(guid, on);
    }

    pub fn creature_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn creature_group_holder_member_count_like_cpp(&self, leader_spawn_id: SpawnId) -> usize {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn creature_group_holder_contains_like_cpp(
        &self,
        leader_spawn_id: SpawnId,
        member_guid: ObjectGuid,
    ) -> bool {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .is_some_and(|members| members.contains(&member_guid))
    }

    pub fn gameobject_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.gameobjects_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn area_trigger_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.area_triggers_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn creature_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.creatures_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn gameobject_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.gameobjects_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn area_trigger_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.area_triggers_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                // C++ returns the first unordered_multimap entry; Rust sorts for deterministic tests.
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn get_creature_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&Creature> {
        let mut fallback_guid = None;
        let mut alive_guid = None;
        for guid in self.creature_spawn_id_store_guids_like_cpp(spawn_id) {
            let Some(creature) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
            else {
                continue;
            };
            if creature.spawn_id() != spawn_id {
                continue;
            }
            fallback_guid.get_or_insert(guid);
            if creature.is_alive() {
                alive_guid = Some(guid);
                break;
            }
        }

        alive_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.creature())
    }

    /// Bounded map-owned consumer for C++ `GameEventMgr::ChangeEquipOrModel` live creature loop.
    ///
    /// Mirrors the `GetCreatureBySpawnIdStore().equal_range(spawn_id)` direction over the
    /// map-owned creature by-spawn index. This only mutates canonical `MapObjectRecord::Creature`
    /// equipment/display fields; it does not implement full `Creature::LoadEquipment`, DB2
    /// `GetCreatureModelInfo`, values/session fanout, scripts, AI, or ObjectAccessor side effects.
    pub fn change_game_event_equip_or_model_by_spawn_id_like_cpp(
        &mut self,
        spawn_id: SpawnId,
        equipment_id: u8,
        model_id: u32,
        model_info_available: bool,
    ) -> GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
        let guids = self.creature_spawn_id_store_guids_like_cpp(spawn_id);
        let mut outcome = GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
            spawn_id,
            indexed_guids: guids.len(),
            ..GameEventChangeEquipOrModelLiveOutcomeLikeCpp::default()
        };

        for guid in guids {
            let Some(record) = self.map_objects.get_mut(&guid) else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            let Some(creature) = record.creature_mut() else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            if creature.spawn_id() != spawn_id {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            }

            outcome.live_creatures_mutated += 1;
            creature.set_equipment_id_like_cpp(equipment_id);
            outcome.equipment_changed += 1;

            if model_id > 0 && creature.unit().data().display_id as u32 != model_id {
                if model_info_available {
                    creature.set_display_id(model_id, true, None);
                    outcome.display_changed += 1;
                } else {
                    outcome.model_validation_unavailable += 1;
                }
            }
        }

        outcome
    }

    /// Bounded map-owned consumer for C++ `GameEventMgr::UpdateEventNPCFlags` live creature loop.
    ///
    /// Mirrors `Map::GetCreatureBySpawnIdStore().equal_range(spawnId)` and applies represented
    /// `ReplaceAllNpcFlags` plus `ReplaceAllNpcFlags2` state to canonical `MapObjectRecord::Creature`.
    /// No values/session fanout, gossip reset, ObjectAccessor, update packets, or template lookup is
    /// performed inside `wow-map`.
    pub fn update_game_event_npc_flags_by_spawn_id_like_cpp(
        &mut self,
        spawn_id: SpawnId,
        npcflag_mask_with_template: u64,
    ) -> GameEventNpcFlagLiveOutcomeLikeCpp {
        let guids = self.creature_spawn_id_store_guids_like_cpp(spawn_id);
        let mut outcome = GameEventNpcFlagLiveOutcomeLikeCpp {
            spawn_id,
            indexed_guids: guids.len(),
            ..GameEventNpcFlagLiveOutcomeLikeCpp::default()
        };
        let npc_flags_low = npcflag_mask_with_template as u32;
        let npc_flags2 = (npcflag_mask_with_template >> 32) as u32;

        for guid in guids {
            let Some(record) = self.map_objects.get_mut(&guid) else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            let Some(creature) = record.creature_mut() else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            if creature.spawn_id() != spawn_id {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            }

            creature.ai_ownership_mut().npc_flags = npc_flags_low;
            creature.ai_ownership_mut().npc_flags2 = npc_flags2;
            creature.unit_mut().set_npc_flags_like_cpp(npc_flags_low);
            creature.unit_mut().set_npc_flags2_like_cpp(npc_flags2);
            let values_update = creature.unit().values_update();
            if values_update.has_data() {
                outcome
                    .values_updates
                    .push(GameEventNpcFlagValuesUpdateLikeCpp {
                        guid,
                        map_id: self.map_id,
                        values_update,
                    });
            }
            outcome.live_creatures_mutated += 1;
            outcome.npc_flags_low_applied += 1;
            outcome.npc_flags2_applied += 1;
        }

        outcome
    }

    pub fn get_gameobject_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&GameObject> {
        let mut fallback_guid = None;
        let mut spawned_guid = None;
        for guid in self.gameobject_spawn_id_store_guids_like_cpp(spawn_id) {
            let Some(gameobject) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
            else {
                continue;
            };
            if gameobject.spawn_id() != spawn_id {
                continue;
            }
            fallback_guid.get_or_insert(guid);
            if Self::gameobject_is_spawned_like_cpp(gameobject) {
                spawned_guid = Some(guid);
                break;
            }
        }

        spawned_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.game_object())
    }

    fn gameobject_is_spawned_like_cpp(gameobject: &GameObject) -> bool {
        gameobject.respawn_delay_time() == 0
            || (gameobject.respawn_time() > 0 && !gameobject.spawned_by_default())
            || (gameobject.respawn_time() == 0 && gameobject.spawned_by_default())
    }

    pub fn get_area_trigger_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&AreaTrigger> {
        self.area_trigger_spawn_id_store_guids_like_cpp(spawn_id)
            .into_iter()
            .find_map(|guid| self.map_object_record(guid)?.area_trigger())
    }

    pub fn get_world_object_by_spawn_id_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&WorldObject> {
        match object_type {
            SpawnObjectType::Creature => self
                .get_creature_by_spawn_id_like_cpp(spawn_id)
                .map(|creature| creature.unit().world()),
            SpawnObjectType::GameObject => self
                .get_gameobject_by_spawn_id_like_cpp(spawn_id)
                .map(GameObject::world),
            SpawnObjectType::AreaTrigger => self
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .map(AreaTrigger::world),
        }
    }

    pub fn insert_map_object(
        &mut self,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) -> Result<Option<MapObjectRecord>, MapObjectStoreError> {
        let record = MapObjectRecord::new(kind, object)?;
        self.insert_map_object_record(record)
    }

    pub fn insert_map_object_record(
        &mut self,
        record: MapObjectRecord,
    ) -> Result<Option<MapObjectRecord>, MapObjectStoreError> {
        self.validate_map_object(record.object())?;
        let guid = record.object().guid();
        let mut previous = self.map_objects.remove(&guid);
        if let Some(previous_record) = previous.as_mut() {
            if !typed_loot_authorities_share_storage_like_cpp(previous_record, &record) {
                detach_typed_loot_authority_like_cpp(previous_record);
            }
            self.unindex_map_object_record_by_spawn_id_like_cpp(previous_record);
        }
        self.index_map_object_record_by_spawn_id_like_cpp(&record);
        self.map_objects.insert(guid, record);
        Ok(previous)
    }

    fn index_map_object_record_by_spawn_id_like_cpp(&mut self, record: &MapObjectRecord) {
        if let Some(creature) = record.creature() {
            let spawn_id = creature.spawn_id();
            if spawn_id != 0 {
                self.creatures_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(creature.guid());
            }
            return;
        }

        if let Some(gameobject) = record.game_object() {
            let spawn_id = gameobject.spawn_id();
            if spawn_id != 0 {
                self.gameobjects_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(gameobject.world().guid());
            }
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            let spawn_id = area_trigger.spawn_id();
            if spawn_id != 0 {
                self.area_triggers_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(area_trigger.world().guid());
            }
        }
    }

    fn unindex_map_object_record_by_spawn_id_like_cpp(&mut self, record: &MapObjectRecord) {
        if let Some(creature) = record.creature() {
            Self::remove_spawn_id_index_entry_like_cpp(
                &mut self.creatures_by_spawn_id,
                creature.spawn_id(),
                creature.guid(),
            );
            return;
        }

        if let Some(gameobject) = record.game_object() {
            Self::remove_spawn_id_index_entry_like_cpp(
                &mut self.gameobjects_by_spawn_id,
                gameobject.spawn_id(),
                gameobject.world().guid(),
            );
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            Self::remove_spawn_id_index_entry_like_cpp(
                &mut self.area_triggers_by_spawn_id,
                area_trigger.spawn_id(),
                area_trigger.world().guid(),
            );
        }
    }

    fn remove_spawn_id_index_entry_like_cpp(
        index: &mut HashMap<SpawnId, HashSet<ObjectGuid>>,
        spawn_id: SpawnId,
        guid: ObjectGuid,
    ) {
        if spawn_id == 0 {
            return;
        }

        if let Some(guids) = index.get_mut(&spawn_id) {
            guids.remove(&guid);
            if guids.is_empty() {
                index.remove(&spawn_id);
            }
        }
    }

    fn apply_creature_search_formation_like_cpp(
        &mut self,
        current_guid: ObjectGuid,
        outcome: CreatureSearchFormationOutcomeLikeCpp,
    ) {
        if !outcome.add_to_group_requested {
            return;
        }

        let Some(leader_spawn_id) = outcome.leader_spawn_id else {
            return;
        };

        let stale_member_guids = self.creature_spawn_id_store_guids_like_cpp(outcome.spawn_id);
        let group = self
            .creature_group_holder_like_cpp
            .entry(leader_spawn_id)
            .or_default();
        for stale_guid in stale_member_guids {
            if stale_guid != current_guid {
                group.remove(&stale_guid);
            }
        }
        group.insert(current_guid);
    }

    fn remove_creature_from_formation_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<CreatureRemoveFormationOutcomeLikeCpp> {
        let (spawn_id, leader_spawn_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::Creature)
            .and_then(MapObjectRecord::creature)
            .filter(|creature| creature.unit().world().object().is_in_world())
            .and_then(|creature| {
                let leader_spawn_id = creature.formation_info_like_cpp()?.leader_spawn_id;
                Some((creature.spawn_id(), leader_spawn_id))
            })?;

        let Some(group) = self
            .creature_group_holder_like_cpp
            .get_mut(&leader_spawn_id)
        else {
            return Some(CreatureRemoveFormationOutcomeLikeCpp {
                guid,
                spawn_id,
                leader_spawn_id: Some(leader_spawn_id),
                had_group: false,
                removed_member: false,
                removed_group: false,
                remaining_members: 0,
            });
        };

        let removed_member = group.remove(&guid);
        let remaining_members = group.len();
        let removed_group = remaining_members == 0;
        if removed_group {
            self.creature_group_holder_like_cpp.remove(&leader_spawn_id);
        }

        Some(CreatureRemoveFormationOutcomeLikeCpp {
            guid,
            spawn_id,
            leader_spawn_id: Some(leader_spawn_id),
            had_group: true,
            removed_member,
            removed_group,
            remaining_members,
        })
    }

    fn active_respawn_location_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<ActiveNonPlayerRespawnLocationLikeCpp> {
        let record = self.map_object_record(guid)?;
        match record.kind() {
            AccessorObjectKind::Creature => {
                let creature = record.creature()?;
                let spawn_id = creature.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: creature.ai_home_position(),
                })
            }
            AccessorObjectKind::GameObject => {
                let game_object = record.game_object()?;
                let spawn_id = game_object.spawn_id();
                (spawn_id != 0).then_some(ActiveNonPlayerRespawnLocationLikeCpp {
                    spawn_id,
                    position: game_object.stationary_position(),
                })
            }
            _ => None,
        }
    }

    fn mutate_unload_active_lock_for_respawn_location_like_cpp(
        &mut self,
        location: ActiveNonPlayerRespawnLocationLikeCpp,
        increment: bool,
    ) -> ActiveNonPlayerUnloadLockOutcomeLikeCpp {
        if !is_valid_map_coord_2d(location.position.x, location.position.y) {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: None,
                respawn_grid_missing: true,
                invalid_respawn_position: true,
                lock_incremented: false,
                lock_decremented: false,
            };
        }

        let cell = Cell::from_world(location.position.x, location.position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let Some(ngrid) = self.get_ngrid_mut(grid) else {
            return ActiveNonPlayerUnloadLockOutcomeLikeCpp {
                spawn_id: location.spawn_id,
                respawn_grid: Some(grid),
                respawn_grid_missing: true,
                invalid_respawn_position: false,
                lock_incremented: false,
                lock_decremented: false,
            };
        };

        if increment {
            ngrid.info_mut().inc_unload_active_lock();
        } else {
            ngrid.info_mut().dec_unload_active_lock();
        }

        ActiveNonPlayerUnloadLockOutcomeLikeCpp {
            spawn_id: location.spawn_id,
            respawn_grid: Some(grid),
            respawn_grid_missing: false,
            invalid_respawn_position: false,
            lock_incremented: increment,
            lock_decremented: !increment,
        }
    }

    pub fn add_to_active_like_cpp(&mut self, guid: ObjectGuid) -> AddToActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let inserted_in_active_set = self.active_non_players_like_cpp.insert(guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, true)
        });
        AddToActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set,
            removed_from_active_set: false,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }

    pub fn remove_from_active_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveFromActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let removed_from_active_set = self.active_non_players_like_cpp.remove(&guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, false)
        });
        RemoveFromActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set: false,
            removed_from_active_set,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }

    pub fn active_non_players_count_like_cpp(&self) -> usize {
        self.active_non_players_like_cpp.len()
    }

    pub fn is_active_non_player_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.active_non_players_like_cpp.contains(&guid)
    }

    fn represented_active_non_player_sources_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut guids: Vec<_> = self
            .active_non_players_like_cpp
            .iter()
            .copied()
            .filter(|guid| self.object_is_in_world(*guid))
            .collect();
        sort_dedup(&mut guids);
        guids
    }

    fn represent_add_to_map_post_add_to_world_tail_like_cpp(
        &mut self,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        active_object: bool,
    ) -> Option<AddToMapPostAddToWorldOutcomeLikeCpp> {
        let pending_move_state = match kind {
            AccessorObjectKind::Creature => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.creature().is_some())
                {
                    self.creature_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            AccessorObjectKind::GameObject => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.game_object().is_some())
                {
                    self.gameobject_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        if pending_move_state.is_some() {
            match kind {
                AccessorObjectKind::Creature => {
                    self.creatures_to_move.retain(|queued| *queued != guid)
                }
                AccessorObjectKind::GameObject => {
                    self.gameobjects_to_move.retain(|queued| *queued != guid);
                }
                _ => {}
            }
        }

        let add_to_active = active_object.then(|| self.add_to_active_like_cpp(guid));

        let mut set_true = false;
        let mut set_false = false;
        let final_is_new_object = if let Some(record) = self.map_objects.get_mut(&guid) {
            record.object_mut().object_mut().set_is_new_object(true);
            set_true = true;
            record.object_mut().object_mut().set_is_new_object(false);
            set_false = true;
            record.object().object().is_new_object()
        } else {
            false
        };

        Some(AddToMapPostAddToWorldOutcomeLikeCpp {
            initialize_object_represented: true,
            pending_move_state_cleared: pending_move_state.is_some(),
            no_pending_move_state: pending_move_state.is_none(),
            add_to_active_represented: add_to_active.is_some(),
            add_to_active_skipped_runtime_gap: false,
            add_to_active,
            set_is_new_object_true: set_true,
            update_object_visibility_on_create_represented: true,
            update_object_visibility_on_create_runtime_gap: true,
            set_is_new_object_false: set_false,
            final_is_new_object,
        })
    }

    pub fn add_to_map_like_cpp(
        &mut self,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let record = MapObjectRecord::new(kind, object).map_err(MapObjectStoreError::from)?;
        self.add_map_object_record_to_map_like_cpp(record)
    }

    pub fn add_map_object_record_to_map_like_cpp(
        &mut self,
        mut record: MapObjectRecord,
    ) -> Result<AddToMapOutcome, AddToMapError> {
        let kind = record.kind();
        let guid = record.object().guid();
        let position = record.object().position();
        let is_world_object = record.object().is_world_object();

        if record.object().object().is_in_world() {
            let cell = Cell::from_world(position.x, position.y);
            let previous = self.insert_map_object_record(record)?;
            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid: GridCoord::new(cell.grid_x(), cell.grid_y()),
                inserted: previous.is_none(),
                already_in_world: true,
                grid_created: false,
                grid_loaded: false,
                inserted_into_cell: false,
                gameobject_model_insert: None,
                gameobject_collision_enable: None,
                gameobject_zone_script_create: None,
                gameobject_store_inserted_before_add_to_world: None,
                gameobject_spawn_indexed_before_add_to_world: None,
                creature_store_inserted_before_add_to_world: None,
                creature_spawn_indexed_before_add_to_world: None,
                creature_unit_add_to_world: None,
                creature_search_formation: None,
                creature_aim_initialize: None,
                creature_vehicle_reset: None,
                creature_vehicle_install: None,
                creature_zone_script_create: None,
                add_to_map_tail: None,
            });
        }

        self.validate_map_object(record.object())?;

        if !is_valid_map_coord_2d(position.x, position.y) {
            return Err(AddToMapError::InvalidCoordinates {
                guid,
                x: position.x,
                y: position.y,
            });
        }

        let cell = Cell::from_world(position.x, position.y);
        let grid = GridCoord::new(cell.grid_x(), cell.grid_y());
        let active_object = is_active_object_like_cpp(kind, record.object());
        let grid_loaded = if active_object {
            self.ensure_grid_loaded_for_active_object(&cell, kind.into())
        } else {
            false
        };
        let grid_created = if active_object {
            false
        } else {
            self.ensure_grid_created(grid)
        };

        {
            let ngrid = self
                .get_ngrid_mut(grid)
                .expect("Map::AddToMap must have created or loaded the target grid");
            let local_cell = ngrid
                .get_grid_type_mut(cell.cell_x(), cell.cell_y())
                .expect("cell coordinates must be local to target grid");
            insert_object_guid_in_cell_like_cpp(local_cell, kind, is_world_object, guid);
        }

        if kind == AccessorObjectKind::Creature && record.creature().is_some() {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let previous = self.insert_map_object_record(record)?;

            let creature_store_inserted_before_add_to_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.creature().is_some());
            let creature_spawn_indexed_before_add_to_world = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .is_some_and(|creature| {
                    let spawn_id = creature.spawn_id();
                    spawn_id != 0
                        && self
                            .creature_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });

            let creature_unit_add_to_world = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .map(|creature| creature.unit_mut().add_to_world_like_cpp());
            let creature_search_formation = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(Creature::search_formation_like_cpp);
            if let Some(outcome) = creature_search_formation {
                self.apply_creature_search_formation_like_cpp(guid, outcome);
            }

            let creature_aim_initialize = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .map(Creature::aim_initialize_like_cpp);

            let creature_vehicle_reset = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| {
                    let context = creature
                        .add_to_world_vehicle_reset_context_like_cpp()?
                        .clone();
                    let base_is_alive = creature.is_alive();
                    creature
                        .unit_mut()
                        .subsystems_mut()
                        .vehicle
                        .reset_vehicle_kit_for_creature_add_to_world_like_cpp(
                            &context,
                            base_is_alive,
                        )
                });

            let creature_vehicle_install = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| {
                    let install = creature
                        .unit_mut()
                        .subsystems_mut()
                        .vehicle
                        .install_vehicle_kit_like_cpp();
                    install.had_kit.then_some(install)
                });

            let creature_zone_script_create = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::creature)
                .is_some()
                .then_some(CreatureZoneScriptCreateOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                });
            let add_to_map_tail = self.represent_add_to_map_post_add_to_world_tail_like_cpp(
                kind,
                guid,
                active_object,
            );

            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                inserted: previous.is_none(),
                already_in_world: false,
                grid_created,
                grid_loaded,
                inserted_into_cell: true,
                gameobject_model_insert: None,
                gameobject_collision_enable: None,
                gameobject_zone_script_create: None,
                gameobject_store_inserted_before_add_to_world: None,
                gameobject_spawn_indexed_before_add_to_world: None,
                creature_store_inserted_before_add_to_world: Some(
                    creature_store_inserted_before_add_to_world,
                ),
                creature_spawn_indexed_before_add_to_world: Some(
                    creature_spawn_indexed_before_add_to_world,
                ),
                creature_unit_add_to_world,
                creature_search_formation,
                creature_aim_initialize,
                creature_vehicle_reset,
                creature_vehicle_install,
                creature_zone_script_create,
                add_to_map_tail,
            });
        }

        if kind == AccessorObjectKind::GameObject && record.game_object().is_some() {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let object_store_present_before_callback = self
                .map_object_record(guid)
                .is_some_and(|record| record.game_object().is_some());
            let spawn_index_present_before_callback =
                record.game_object().is_some_and(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    spawn_id != 0
                        && self
                            .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });
            let gameobject_zone_script_create = Some(GameObjectZoneScriptCreateOutcomeLikeCpp {
                guid,
                represented_callback_boundary: true,
                script_dispatch_represented: false,
                object_store_present_before_callback,
                spawn_index_present_before_callback,
            });
            let previous = self.insert_map_object_record(record)?;

            let gameobject_store_inserted_before_add_to_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.game_object().is_some());
            let gameobject_spawn_indexed_before_add_to_world = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
                .is_some_and(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    spawn_id != 0
                        && self
                            .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                            .contains(&guid)
                });
            let has_represented_model = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
                .is_some_and(GameObject::has_represented_gameobject_model_like_cpp);
            let (gameobject_model_insert, gameobject_collision_enable) = if has_represented_model {
                let gameobject_model_insert =
                    self.insert_gameobject_model_like_cpp(RepresentedGameObjectModelKeyLikeCpp {
                        owner_guid: guid,
                    });
                let gameobject_collision_enable = self
                    .map_objects
                    .get_mut(&guid)
                    .and_then(MapObjectRecord::game_object_mut)
                    .map(|game_object| {
                        // C++ `GameObject::AddToWorld()` computes toggledState before
                        // `EnableCollision(toggledState)`: chests use `getLootState() == GO_READY`,
                        // exact non-Transport GameObjects use `GetGoState() == GO_STATE_READY`.
                        // `MapObjectRecord::Transport` is handled outside this exact-typed
                        // GameObject branch and remains a delayed-add runtime gap for this
                        // represented seam.
                        let toggled_state =
                            if game_object.data().type_id as u32 == GAMEOBJECT_TYPE_CHEST {
                                game_object.loot_state() == LootState::Ready
                            } else {
                                game_object.data().state == GoState::Ready as i8
                            };
                        let collision = game_object
                            .enable_represented_gameobject_collision_like_cpp(toggled_state);
                        GameObjectCollisionEnableOutcomeLikeCpp {
                            requested_enable: collision.requested_enable,
                            represented_model_present: collision.represented_model_present,
                            previous_collision_enabled: collision.previous_collision_enabled,
                            new_collision_enabled: collision.new_collision_enabled,
                        }
                    });
                (Some(gameobject_model_insert), gameobject_collision_enable)
            } else {
                (None, None)
            };

            if let Some(game_object) = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.world_mut().object_mut().add_to_world();
            }
            let add_to_map_tail = self.represent_add_to_map_post_add_to_world_tail_like_cpp(
                kind,
                guid,
                active_object,
            );

            return Ok(AddToMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                inserted: previous.is_none(),
                already_in_world: false,
                grid_created,
                grid_loaded,
                inserted_into_cell: true,
                gameobject_model_insert,
                gameobject_collision_enable,
                gameobject_zone_script_create,
                gameobject_store_inserted_before_add_to_world: Some(
                    gameobject_store_inserted_before_add_to_world,
                ),
                gameobject_spawn_indexed_before_add_to_world: Some(
                    gameobject_spawn_indexed_before_add_to_world,
                ),
                creature_store_inserted_before_add_to_world: None,
                creature_spawn_indexed_before_add_to_world: None,
                creature_unit_add_to_world: None,
                creature_search_formation: None,
                creature_aim_initialize: None,
                creature_vehicle_reset: None,
                creature_vehicle_install: None,
                creature_zone_script_create: None,
                add_to_map_tail,
            });
        }

        let creature_unit_add_to_world = {
            record
                .object_mut()
                .set_current_cell(cell.cell_x(), cell.cell_y());
            let creature_unit_add_to_world = if let Some(creature) = record.creature_mut() {
                Some(creature.unit_mut().add_to_world_like_cpp())
            } else {
                record.object_mut().object_mut().add_to_world();
                None
            };
            record.object_mut().object_mut().set_is_new_object(true);
            // Rust does not emit visibility here yet; keep the flag lifecycle identical to
            // C++ `Map::AddToMap` after `UpdateObjectVisibilityOnCreate()` returns.
            record.object_mut().object_mut().set_is_new_object(false);
            creature_unit_add_to_world
        };

        let creature_search_formation = if kind == AccessorObjectKind::Creature {
            record.creature().map(Creature::search_formation_like_cpp)
        } else {
            None
        };
        if let Some(outcome) = creature_search_formation {
            self.apply_creature_search_formation_like_cpp(guid, outcome);
        }

        let creature_aim_initialize = if kind == AccessorObjectKind::Creature {
            record.creature().map(Creature::aim_initialize_like_cpp)
        } else {
            None
        };

        let creature_vehicle_reset = if kind == AccessorObjectKind::Creature {
            record.creature_mut().and_then(|creature| {
                let context = creature
                    .add_to_world_vehicle_reset_context_like_cpp()?
                    .clone();
                let base_is_alive = creature.is_alive();
                creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .reset_vehicle_kit_for_creature_add_to_world_like_cpp(&context, base_is_alive)
            })
        } else {
            None
        };

        let creature_vehicle_install = if kind == AccessorObjectKind::Creature {
            record.creature_mut().and_then(|creature| {
                let install = creature
                    .unit_mut()
                    .subsystems_mut()
                    .vehicle
                    .install_vehicle_kit_like_cpp();
                install.had_kit.then_some(install)
            })
        } else {
            None
        };
        let creature_zone_script_create = if kind == AccessorObjectKind::Creature {
            record
                .creature()
                .is_some()
                .then_some(CreatureZoneScriptCreateOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                })
        } else {
            None
        };

        let (gameobject_model_insert, gameobject_collision_enable) =
            if kind == AccessorObjectKind::GameObject {
                if let Some(game_object) = record
                    .game_object_mut()
                    .filter(|game_object| game_object.has_represented_gameobject_model_like_cpp())
                {
                    let gameobject_model_insert = self.insert_gameobject_model_like_cpp(
                        RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid },
                    );
                    // C++ `GameObject::AddToWorld()` computes toggledState before
                    // `EnableCollision(toggledState)`: chests use `getLootState() == GO_READY`,
                    // exact non-Transport GameObjects use `GetGoState() == GO_STATE_READY`.
                    // `MapObjectRecord::Transport` is handled above by the kind gate and remains
                    // a delayed-add runtime gap for this represented seam.
                    let toggled_state =
                        if game_object.data().type_id as u32 == GAMEOBJECT_TYPE_CHEST {
                            game_object.loot_state() == LootState::Ready
                        } else {
                            game_object.data().state == GoState::Ready as i8
                        };
                    let collision =
                        game_object.enable_represented_gameobject_collision_like_cpp(toggled_state);
                    let gameobject_collision_enable = GameObjectCollisionEnableOutcomeLikeCpp {
                        requested_enable: collision.requested_enable,
                        represented_model_present: collision.represented_model_present,
                        previous_collision_enabled: collision.previous_collision_enabled,
                        new_collision_enabled: collision.new_collision_enabled,
                    };
                    (
                        Some(gameobject_model_insert),
                        Some(gameobject_collision_enable),
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

        let previous = self.insert_map_object_record(record)?;
        let add_to_map_tail =
            self.represent_add_to_map_post_add_to_world_tail_like_cpp(kind, guid, active_object);
        Ok(AddToMapOutcome {
            guid,
            cell: cell.cell_coord(),
            grid,
            inserted: previous.is_none(),
            already_in_world: false,
            grid_created,
            grid_loaded,
            inserted_into_cell: true,
            gameobject_model_insert,
            gameobject_collision_enable,
            gameobject_zone_script_create: None,
            gameobject_store_inserted_before_add_to_world: None,
            gameobject_spawn_indexed_before_add_to_world: None,
            creature_store_inserted_before_add_to_world: None,
            creature_spawn_indexed_before_add_to_world: None,
            creature_unit_add_to_world,
            creature_search_formation,
            creature_aim_initialize,
            creature_vehicle_reset,
            creature_vehicle_install,
            creature_zone_script_create,
            add_to_map_tail,
        })
    }

    pub fn remove_map_object(&mut self, guid: ObjectGuid) -> Option<MapObjectRecord> {
        let record = self.map_objects.remove(&guid)?;
        self.unindex_map_object_record_by_spawn_id_like_cpp(&record);
        Some(record)
    }

    fn map_record_is_unit_like_gameobject_owner_like_cpp(record: &MapObjectRecord) -> bool {
        matches!(
            record.kind(),
            AccessorObjectKind::Player | AccessorObjectKind::Creature | AccessorObjectKind::Pet
        ) && (record.player().is_some() || record.creature().is_some() || record.pet().is_some())
    }

    /// Bounded map-owned representation of C++ `Unit::AddGameObject(GameObject*)`.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:5192-5209`: if the object exists and has no owner, append to
    ///   `m_gameObj`, set `CreatedBy` to the Unit GUID, optionally start
    ///   event-based cooldown, and dispatch `CreatureAI::JustSummonedGameobject`.
    /// - `Object.cpp:2067-2090` and `SpellEffects.cpp:3238/3590/4456-4482`:
    ///   summon/create paths call this helper for the owning Unit before or
    ///   around `Map::AddToMap`.
    ///
    /// Scope: this does not create objects, insert into object slots, start
    /// cooldowns, execute scripts/SmartAI, send packets, or touch DB. Slot
    /// assignment is path-specific in C++ (`Spell::EffectSummonObject`) and
    /// remains a caller concern.
    pub fn gameobject_add_to_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        guid: ObjectGuid,
    ) -> GameObjectAddToOwnerOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let (gameobject_found, owner_guid_before) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .map(|game_object| (true, game_object.owner_guid()))
            .unwrap_or((false, ObjectGuid::EMPTY));
        let gameobject_owner_empty_before = gameobject_found && owner_guid_before.is_empty();

        let mut registered_owned_gameobject = false;
        let mut owner_guid_after = owner_guid_before;
        let mut creature_ai_callback_represented = false;

        if owner_found_as_unit_like && gameobject_owner_empty_before {
            if let Some(record) = self.map_objects.get_mut(&owner_guid) {
                if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
                    owner
                        .subsystems_mut()
                        .control
                        .register_owned_gameobject_like_cpp(guid);
                    registered_owned_gameobject = true;
                }
            }

            if registered_owned_gameobject {
                if let Some(game_object) = self
                    .map_objects
                    .get_mut(&guid)
                    .and_then(MapObjectRecord::game_object_mut)
                {
                    game_object.set_owner_guid_like_cpp(owner_guid);
                    owner_guid_after = game_object.owner_guid();
                }

                creature_ai_callback_represented = self
                    .map_objects
                    .get_mut(&owner_guid)
                    .map(|record| match record.kind() {
                        AccessorObjectKind::Creature => record
                            .creature_mut()
                            .map(|creature| {
                                creature
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .just_summoned_gameobject_like_cpp()
                            })
                            .unwrap_or(false),
                        AccessorObjectKind::Pet => record
                            .pet_mut()
                            .map(|pet| {
                                pet.creature_mut()
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .just_summoned_gameobject_like_cpp()
                            })
                            .unwrap_or(false),
                        _ => false,
                    })
                    .unwrap_or(false);
            }
        }

        GameObjectAddToOwnerOutcomeLikeCpp {
            guid,
            owner_guid,
            owner_found_as_unit_like,
            gameobject_found,
            owner_guid_before,
            owner_guid_after,
            gameobject_owner_empty_before,
            registered_owned_gameobject,
            owner_guid_set: owner_guid_after == owner_guid && owner_guid_before != owner_guid,
            cooldown_start_represented: false,
            creature_ai_callback_represented,
        }
    }

    /// Bounded map-owned tail for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3548-3563`: the caller clears any previous
    ///   `m_ObjectSlot[slot]` and deletes the old GameObject before creating
    ///   the replacement.
    /// - `SpellEffects.cpp:3590-3597`: after `Unit::AddGameObject(go)` and
    ///   `Map::AddToMap(go)`, the caster writes `m_ObjectSlot[slot]`.
    ///
    /// Scope: this helper represents only the post-create owner link and final
    /// slot assignment for an already map-owned GameObject. It does not create
    /// the GameObject, clear/delete an old slot occupant, compute spell
    /// duration/location, inherit phase, or send packets.
    pub fn gameobject_add_to_owner_slot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        guid: ObjectGuid,
        slot: usize,
    ) -> GameObjectAddToOwnerSlotOutcomeLikeCpp {
        let add_owner = self.gameobject_add_to_owner_like_cpp(owner_guid, guid);
        let mut slot_previous_guid = ObjectGuid::EMPTY;
        let mut slot_set = false;

        if add_owner.registered_owned_gameobject {
            if let Some(owner) = self
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
            {
                if let Some(previous) = owner
                    .subsystems()
                    .control
                    .gameobject_slots
                    .get(slot)
                    .copied()
                {
                    slot_previous_guid = previous;
                }
                slot_set = owner
                    .subsystems_mut()
                    .control
                    .set_gameobject_slot(slot, guid);
            }
        }

        GameObjectAddToOwnerSlotOutcomeLikeCpp {
            add_owner,
            slot,
            slot_previous_guid,
            slot_set,
        }
    }

    /// Bounded map-owned representation of C++ `WorldObject::SummonGameObject`.
    ///
    /// C++ anchors:
    /// - `Object.cpp:2067-2090`: `WorldObject::SummonGameObject(entry, pos,
    ///   rot, respawnTime, summonType)` requires an in-world summoner, creates
    ///   a ready dynamic GameObject from the already-resolved template,
    ///   inherits phase, sets respawn time, either calls `ToUnit()->AddGameObject`
    ///   for Player / Unit + `GO_SUMMON_TIMED_OR_CORPSE_DESPAWN`, or marks the
    ///   object not spawned by default, then calls `Map::AddToMap`.
    /// - `GameObject.cpp:1187-1200`: `GameObject::CreateGameObject` delegates
    ///   to `GameObject::Create` and returns null on missing template/create
    ///   failure.
    ///
    /// Scope: the caller supplies an already-resolved template, position and
    /// respawn seconds. This helper does not load DB/templates, compute
    /// `GetClosePoint`, inherit real phase masks, dispatch scripts, send
    /// packets, create linked traps, or emit spell execute logs.
    pub fn world_object_summon_gameobject_like_cpp(
        &mut self,
        summoner_guid: ObjectGuid,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        respawn_time_secs: i64,
        summon_type: GameObjectSummonTypeLikeCpp,
    ) -> WorldObjectSummonGameObjectOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(summoner_record) = self.map_object_record(summoner_guid) else {
            return WorldObjectSummonGameObjectOutcomeLikeCpp {
                summoner_guid,
                template_entry,
                summon_type,
                status: WorldObjectSummonGameObjectStatusLikeCpp::MissingSummoner,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner: None,
                respawn_time_secs,
                phase_inherit_represented: false,
                spawned_by_default_forced_false: false,
            };
        };
        if !summoner_record.object().object().is_in_world() {
            return WorldObjectSummonGameObjectOutcomeLikeCpp {
                summoner_guid,
                template_entry,
                summon_type,
                status: WorldObjectSummonGameObjectStatusLikeCpp::SummonerNotInWorld,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner: None,
                respawn_time_secs,
                phase_inherit_represented: false,
                spawned_by_default_forced_false: false,
            };
        }
        let summoner_is_player = summoner_record.kind() == AccessorObjectKind::Player;
        let summoner_is_unit_like =
            Self::map_record_is_unit_like_gameobject_owner_like_cpp(summoner_record);
        let should_add_to_owner = summoner_is_player
            || (summoner_is_unit_like
                && summon_type == GameObjectSummonTypeLikeCpp::TimedOrCorpseDespawn);

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return WorldObjectSummonGameObjectOutcomeLikeCpp {
                    summoner_guid,
                    template_entry,
                    summon_type,
                    status: WorldObjectSummonGameObjectStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    add_owner: None,
                    respawn_time_secs,
                    phase_inherit_represented: false,
                    spawned_by_default_forced_false: false,
                };
            }
        };
        let guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            self.map_id as u16,
            self.instance_id,
            template_entry,
            low_guid,
        );
        let record = GameObjectCreateLifecycleRecord {
            guid,
            map_id: self.map_id,
            instance_id: self.instance_id,
            position,
            rotation: gameobject_local_rotation_from_orientation_like_cpp(position.orientation),
            anim_progress: u8::MAX,
            go_state: GoState::Ready,
            art_kit: 0,
            dynamic: true,
            spawn_id: 0,
            template,
        };

        let mut game_object = match GameObject::try_create_from_lifecycle(record) {
            Ok(game_object) => game_object,
            Err(error) => {
                return WorldObjectSummonGameObjectOutcomeLikeCpp {
                    summoner_guid,
                    template_entry,
                    summon_type,
                    status: WorldObjectSummonGameObjectStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    add_owner: None,
                    respawn_time_secs,
                    phase_inherit_represented: false,
                    spawned_by_default_forced_false: false,
                };
            }
        };
        game_object.set_respawn_time(respawn_time_secs);

        let mut add_owner = None;
        let mut spawned_by_default_forced_false = false;
        if should_add_to_owner {
            game_object.set_owner_guid_like_cpp(summoner_guid);
            let mut registered_owned_gameobject = false;
            let mut creature_ai_callback_represented = false;
            if let Some(record) = self.map_objects.get_mut(&summoner_guid) {
                if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
                    owner
                        .subsystems_mut()
                        .control
                        .register_owned_gameobject_like_cpp(guid);
                    registered_owned_gameobject = true;
                }
                creature_ai_callback_represented = match record.kind() {
                    AccessorObjectKind::Creature => record
                        .creature_mut()
                        .map(|creature| {
                            creature
                                .unit_mut()
                                .subsystems_mut()
                                .ai
                                .just_summoned_gameobject_like_cpp()
                        })
                        .unwrap_or(false),
                    AccessorObjectKind::Pet => record
                        .pet_mut()
                        .map(|pet| {
                            pet.creature_mut()
                                .unit_mut()
                                .subsystems_mut()
                                .ai
                                .just_summoned_gameobject_like_cpp()
                        })
                        .unwrap_or(false),
                    _ => false,
                };
            }
            add_owner = Some(GameObjectAddToOwnerOutcomeLikeCpp {
                guid,
                owner_guid: summoner_guid,
                owner_found_as_unit_like: summoner_is_unit_like,
                gameobject_found: true,
                owner_guid_before: ObjectGuid::EMPTY,
                owner_guid_after: summoner_guid,
                gameobject_owner_empty_before: true,
                registered_owned_gameobject,
                owner_guid_set: registered_owned_gameobject,
                cooldown_start_represented: false,
                creature_ai_callback_represented,
            });
        } else {
            game_object.set_spawned_by_default(false);
            spawned_by_default_forced_false = true;
        }

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let status = if add_to_map.is_some() {
            WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
        } else {
            WorldObjectSummonGameObjectStatusLikeCpp::AddToMapFailed
        };

        WorldObjectSummonGameObjectOutcomeLikeCpp {
            summoner_guid,
            template_entry,
            summon_type,
            status,
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            add_owner,
            respawn_time_secs,
            phase_inherit_represented: false,
            spawned_by_default_forced_false,
        }
    }

    /// Bounded map-owned body for C++ `Spell::EffectSummonObjectWild`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:2937-2971`: launch-only spell effect resolves the
    ///   destination before this seam, creates a ready GameObject from
    ///   `effectInfo->MiscValue`, inherits phase from `m_caster`, sets respawn
    ///   seconds from positive duration, sets `SpellId`, executes the summon log,
    ///   and calls `Map::AddToMap` without owner linkage.
    /// - `SpellEffects.cpp:2973-2986`: flag-drop battleground state and linked
    ///   trap phase/respawn/spell/log are runtime side effects after AddToMap.
    ///
    /// Scope: the caller supplies an already-resolved template, position,
    /// duration and spell id. This helper does not load DB/templates, resolve
    /// spell targets/GetClosePoint, inherit real phase masks, dispatch scripts,
    /// send packets, update battleground state, or create/resolve linked traps.
    pub fn spell_effect_summon_object_wild_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        duration_ms: i32,
    ) -> SpellEffectSummonObjectWildOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(caster_record) = self.map_object_record(caster_guid) else {
            return SpellEffectSummonObjectWildOutcomeLikeCpp {
                caster_guid,
                spell_id,
                template_entry,
                status: SpellEffectSummonObjectWildStatusLikeCpp::MissingCaster,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                respawn_time_secs: None,
                phase_inherit_represented: false,
                execute_log_represented: false,
                owner_linked: false,
                flagdrop_type: false,
                flagdrop_player_branch_reached: false,
                flagdrop_battleground_update_represented: false,
                linked_trap_guid: None,
                linked_trap_side_effect_represented: false,
            };
        };
        let caster_is_player = caster_record.kind() == AccessorObjectKind::Player;
        let respawn_time_secs = if duration_ms > 0 {
            duration_ms / 1_000
        } else {
            0
        };
        let flagdrop_type = template.go_type == GAMEOBJECT_TYPE_FLAGDROP;

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return SpellEffectSummonObjectWildOutcomeLikeCpp {
                    caster_guid,
                    spell_id,
                    template_entry,
                    status: SpellEffectSummonObjectWildStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    owner_linked: false,
                    flagdrop_type,
                    flagdrop_player_branch_reached: false,
                    flagdrop_battleground_update_represented: false,
                    linked_trap_guid: None,
                    linked_trap_side_effect_represented: false,
                };
            }
        };
        let guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            self.map_id as u16,
            self.instance_id,
            template_entry,
            low_guid,
        );
        let record = GameObjectCreateLifecycleRecord {
            guid,
            map_id: self.map_id,
            instance_id: self.instance_id,
            position,
            rotation: gameobject_local_rotation_from_orientation_like_cpp(position.orientation),
            anim_progress: u8::MAX,
            go_state: GoState::Ready,
            art_kit: 0,
            dynamic: true,
            spawn_id: 0,
            template,
        };

        let mut game_object = match GameObject::try_create_from_lifecycle(record) {
            Ok(game_object) => game_object,
            Err(error) => {
                return SpellEffectSummonObjectWildOutcomeLikeCpp {
                    caster_guid,
                    spell_id,
                    template_entry,
                    status: SpellEffectSummonObjectWildStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    owner_linked: false,
                    flagdrop_type,
                    flagdrop_player_branch_reached: false,
                    flagdrop_battleground_update_represented: false,
                    linked_trap_guid: None,
                    linked_trap_side_effect_represented: false,
                };
            }
        };
        game_object.set_respawn_time(i64::from(respawn_time_secs));
        game_object.set_spell_id(spell_id);
        let linked_trap_guid = game_object.linked_trap_guid_like_cpp();

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let execute_log_represented = add_to_map.is_some();

        SpellEffectSummonObjectWildOutcomeLikeCpp {
            caster_guid,
            spell_id,
            template_entry,
            status: if execute_log_represented {
                SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
            } else {
                SpellEffectSummonObjectWildStatusLikeCpp::AddToMapFailed
            },
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            respawn_time_secs: Some(respawn_time_secs),
            phase_inherit_represented: false,
            execute_log_represented,
            owner_linked: false,
            flagdrop_type,
            flagdrop_player_branch_reached: flagdrop_type && caster_is_player,
            flagdrop_battleground_update_represented: false,
            linked_trap_guid: (!linked_trap_guid.is_empty()).then_some(linked_trap_guid),
            linked_trap_side_effect_represented: false,
        }
    }

    /// Bounded map-owned pre-create cleanup for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3548-3563`: before creating the replacement object,
    ///   clear the existing `m_ObjectSlot[slot]`; if the old GameObject exists,
    ///   null its spell id in the recast case, call `Unit::RemoveGameObject(obj,
    ///   true)`, then clear the slot.
    /// - `Unit.cpp:5213-5251`: pointer-overload removal clears owner/list/slot,
    ///   removes spell auras when `GetSpellId() != 0`, emits the represented AI
    ///   despawn boundary, then `SetRespawnTime(0); Delete()` when `del=true`.
    ///
    /// Scope: this represents only the old-slot cleanup before a new object is
    /// created. It does not create the new GameObject, write the replacement
    /// slot, inherit phase, execute scripts, send packets, or emit cooldown
    /// events.
    pub fn gameobject_prepare_owner_slot_for_summon_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        slot: usize,
        spell_id: u32,
    ) -> GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let slot_guid_before = self
            .map_object_record(owner_guid)
            .and_then(Self::map_record_unit_like_cpp)
            .and_then(|owner| {
                owner
                    .subsystems()
                    .control
                    .gameobject_slots
                    .get(slot)
                    .copied()
            })
            .unwrap_or(ObjectGuid::EMPTY);

        let mut gameobject_found = false;
        let mut recast_spell_id_cleared = false;
        let mut unit_pointer_owner_match = false;
        let mut remove_from_owner = None;
        let mut respawn_time_cleared = false;
        let mut delete_outcome = None;
        let mut slot_cleared = false;

        if owner_found_as_unit_like && !slot_guid_before.is_empty() {
            gameobject_found = self
                .map_object_record(slot_guid_before)
                .and_then(MapObjectRecord::game_object)
                .is_some();

            if gameobject_found {
                unit_pointer_owner_match = self
                    .map_object_record(slot_guid_before)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|gameobject| gameobject.owner_guid() == owner_guid);
                if let Some(gameobject) = self
                    .map_objects
                    .get_mut(&slot_guid_before)
                    .and_then(MapObjectRecord::game_object_mut)
                {
                    if gameobject.spell_id() == spell_id {
                        gameobject.set_spell_id(0);
                        recast_spell_id_cleared = true;
                    }
                }

                if unit_pointer_owner_match {
                    remove_from_owner =
                        self.gameobject_remove_from_owner_like_cpp(slot_guid_before);
                    if let Some(gameobject) = self
                        .map_objects
                        .get_mut(&slot_guid_before)
                        .and_then(MapObjectRecord::game_object_mut)
                    {
                        gameobject.set_respawn_time(0);
                        respawn_time_cleared = true;
                    }
                    delete_outcome = self.gameobject_delete_like_cpp(slot_guid_before);
                }
            }

            if let Some(owner) = self
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
            {
                slot_cleared = owner
                    .subsystems_mut()
                    .control
                    .set_gameobject_slot(slot, ObjectGuid::EMPTY);
            }
        }

        GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
            owner_guid,
            slot,
            spell_id,
            owner_found_as_unit_like,
            slot_guid_before,
            slot_had_guid: !slot_guid_before.is_empty(),
            gameobject_found,
            recast_spell_id_cleared,
            unit_pointer_owner_match,
            remove_from_owner,
            respawn_time_cleared,
            delete_outcome,
            slot_cleared,
            cooldown_event_represented: false,
        }
    }

    /// Bounded map-owned body for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3565-3597`: after old-slot cleanup and destination
    ///   resolution, create a ready GameObject from `effectInfo->MiscValue`,
    ///   inherit phase, copy caster faction/level, set respawn from spell
    ///   duration, set `SpellId`, call `Unit::AddGameObject`, execute the
    ///   summon-object log boundary, add to map, then write `m_ObjectSlot[slot]`.
    /// - `GameObject.cpp:179-229`: `GameObject::Create` binds the object to the
    ///   map/position/rotation/template before `Map::AddToMap`.
    ///
    /// Scope: the caller supplies an already-resolved template, destination and
    /// duration. This helper does not load DB/templates, resolve spell targets,
    /// inherit real phase masks, execute scripts, send packets, or emit cooldown
    /// events.
    pub fn gameobject_summon_object_for_owner_slot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        slot: usize,
        spell_id: u32,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        duration_ms: i32,
    ) -> GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(owner) = self
            .map_object_record(owner_guid)
            .and_then(Self::map_record_unit_like_cpp)
        else {
            return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                owner_guid,
                slot,
                spell_id,
                template_entry,
                status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::MissingOwner,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner_slot: None,
                respawn_time_secs: None,
                caster_faction: None,
                caster_level: None,
                phase_inherit_represented: false,
                execute_log_represented: false,
                cooldown_event_represented: false,
            };
        };

        let caster_faction = owner.data().faction_template.max(0) as u32;
        let caster_level = owner.data().level.max(0) as u32;
        let respawn_time_secs = if duration_ms > 0 {
            duration_ms / 1_000
        } else {
            0
        };
        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                    owner_guid,
                    slot,
                    spell_id,
                    template_entry,
                    status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    add_owner_slot: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    caster_faction: Some(caster_faction),
                    caster_level: Some(caster_level),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    cooldown_event_represented: false,
                };
            }
        };
        let guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            self.map_id as u16,
            self.instance_id,
            template_entry,
            low_guid,
        );
        let rotation = gameobject_local_rotation_from_orientation_like_cpp(position.orientation);
        let record = GameObjectCreateLifecycleRecord {
            guid,
            map_id: self.map_id,
            instance_id: self.instance_id,
            position,
            rotation,
            anim_progress: u8::MAX,
            go_state: GoState::Ready,
            art_kit: 0,
            dynamic: true,
            spawn_id: 0,
            template,
        };

        let mut game_object = match GameObject::try_create_from_lifecycle(record) {
            Ok(game_object) => game_object,
            Err(error) => {
                return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                    owner_guid,
                    slot,
                    spell_id,
                    template_entry,
                    status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    add_owner_slot: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    caster_faction: Some(caster_faction),
                    caster_level: Some(caster_level),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    cooldown_event_represented: false,
                };
            }
        };
        game_object.set_faction(caster_faction);
        game_object.set_level(caster_level);
        game_object.set_respawn_time(i64::from(respawn_time_secs));
        game_object.set_spell_id(spell_id);
        game_object.set_owner_guid_like_cpp(owner_guid);

        let mut registered_owned_gameobject = false;
        let mut creature_ai_callback_represented = false;
        if let Some(record) = self.map_objects.get_mut(&owner_guid) {
            if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
                owner
                    .subsystems_mut()
                    .control
                    .register_owned_gameobject_like_cpp(guid);
                registered_owned_gameobject = true;
            }
            creature_ai_callback_represented = match record.kind() {
                AccessorObjectKind::Creature => record
                    .creature_mut()
                    .map(|creature| {
                        creature
                            .unit_mut()
                            .subsystems_mut()
                            .ai
                            .just_summoned_gameobject_like_cpp()
                    })
                    .unwrap_or(false),
                AccessorObjectKind::Pet => record
                    .pet_mut()
                    .map(|pet| {
                        pet.creature_mut()
                            .unit_mut()
                            .subsystems_mut()
                            .ai
                            .just_summoned_gameobject_like_cpp()
                    })
                    .unwrap_or(false),
                _ => false,
            };
        }
        let add_owner = GameObjectAddToOwnerOutcomeLikeCpp {
            guid,
            owner_guid,
            owner_found_as_unit_like: true,
            gameobject_found: true,
            owner_guid_before: ObjectGuid::EMPTY,
            owner_guid_after: owner_guid,
            gameobject_owner_empty_before: true,
            registered_owned_gameobject,
            owner_guid_set: registered_owned_gameobject,
            cooldown_start_represented: false,
            creature_ai_callback_represented,
        };

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let add_owner_slot = if add_to_map.is_some() {
            let mut slot_previous_guid = ObjectGuid::EMPTY;
            let mut slot_set = false;
            if let Some(owner) = self
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
            {
                if let Some(previous) = owner
                    .subsystems()
                    .control
                    .gameobject_slots
                    .get(slot)
                    .copied()
                {
                    slot_previous_guid = previous;
                }
                slot_set = owner
                    .subsystems_mut()
                    .control
                    .set_gameobject_slot(slot, guid);
            }
            Some(GameObjectAddToOwnerSlotOutcomeLikeCpp {
                add_owner,
                slot,
                slot_previous_guid,
                slot_set,
            })
        } else {
            None
        };
        let execute_log_represented = add_owner_slot.as_ref().is_some_and(|outcome| {
            outcome.add_owner.registered_owned_gameobject && outcome.slot_set
        });

        GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
            owner_guid,
            slot,
            spell_id,
            template_entry,
            status: if execute_log_represented {
                GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
            } else {
                GameObjectSummonObjectForOwnerSlotStatusLikeCpp::AddToMapOrOwnerFailed
            },
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            add_owner_slot,
            respawn_time_secs: Some(respawn_time_secs),
            caster_faction: Some(caster_faction),
            caster_level: Some(caster_level),
            phase_inherit_represented: false,
            execute_log_represented,
            cooldown_event_represented: false,
        }
    }

    /// Bounded map-owned representation of C++ `Unit::RemoveGameObject(uint32
    /// spellid, bool del)`.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:5253-5274`: iterates `m_gameObj`, matches all when
    ///   `spellid == 0` or only objects with the requested spell id, clears
    ///   `CreatedBy`, optionally `SetRespawnTime(0); Delete();`, then erases
    ///   the list entry.
    /// - `Spell.cpp:3621-3625`: channeled spell cancellation uses this overload
    ///   with `del=true`.
    ///
    /// Scope: this overload intentionally does not clear `m_ObjectSlot`, remove
    /// auras, send cooldown events, or dispatch Creature AI despawn callbacks;
    /// those belong to the pointer overload represented by
    /// `gameobject_remove_from_owner_like_cpp`.
    pub fn unit_remove_gameobjects_by_spell_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        spell_id: u32,
        delete: bool,
    ) -> UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let owned_guids_before = self
            .map_object_record(owner_guid)
            .and_then(Self::map_record_unit_like_cpp)
            .map(|owner| owner.subsystems().control.owned_gameobjects.clone())
            .unwrap_or_default();

        let matched_guids: Vec<ObjectGuid> = owned_guids_before
            .iter()
            .copied()
            .filter(|guid| {
                if spell_id == 0 {
                    return true;
                }
                self.map_object_record(*guid)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|game_object| game_object.spell_id() == spell_id)
            })
            .collect();

        let mut owner_guid_cleared = 0;
        let mut respawn_time_cleared = 0;
        for guid in &matched_guids {
            if let Some(game_object) = self
                .map_objects
                .get_mut(guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.clear_owner_guid_like_cpp();
                owner_guid_cleared += 1;
                if delete {
                    game_object.set_respawn_time(0);
                    respawn_time_cleared += 1;
                }
            }
        }

        let mut owner_list_entries_removed = 0;
        if let Some(owner) = self
            .map_objects
            .get_mut(&owner_guid)
            .and_then(Self::map_record_unit_mut_like_cpp)
        {
            let before = owner.subsystems().control.owned_gameobjects.len();
            owner
                .subsystems_mut()
                .control
                .owned_gameobjects
                .retain(|guid| !matched_guids.contains(guid));
            owner_list_entries_removed =
                before.saturating_sub(owner.subsystems().control.owned_gameobjects.len());
        }

        let mut delete_outcomes = 0;
        if delete {
            for guid in &matched_guids {
                if self.gameobject_delete_like_cpp(*guid).is_some() {
                    delete_outcomes += 1;
                }
            }
        }

        UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
            owner_guid,
            spell_id,
            delete_requested: delete,
            owner_found_as_unit_like,
            owned_entries_before: owned_guids_before.len(),
            matched_entries: matched_guids.len(),
            owner_guid_cleared,
            respawn_time_cleared,
            owner_list_entries_removed,
            delete_outcomes,
            object_slot_cleanup_represented: false,
            aura_cleanup_represented: false,
            cooldown_event_represented: false,
            creature_ai_callback_represented: false,
        }
    }

    /// Bounded map-owned representation of C++ `GameObject::RemoveFromOwner()`
    /// during `GameObject::RemoveFromWorld()`.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:880-897`: empty owner returns; resolved Unit calls
    ///   `Unit::RemoveGameObject(this, false)`; missing owner falls back to
    ///   `SetOwnerGUID(ObjectGuid::Empty)`.
    /// - `GameObject.cpp:926-948`: this runs after ZoneScript remove and before
    ///   model removal, linked trap despawn, `WorldObject::RemoveFromWorld`,
    ///   spawn-id unindex, and map store removal.
    /// - `Unit.cpp:5213-5250`: real owner-side list/slot/aura/cooldown/AI effects
    ///   remain explicit gaps here.
    fn gameobject_remove_from_owner_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<GameObjectRemoveFromOwnerOutcomeLikeCpp> {
        let (owner_guid_before, spell_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .filter(|game_object| game_object.world().object().is_in_world())
            .map(|game_object| (game_object.owner_guid(), game_object.spell_id()))?;

        let owner_found_as_unit_like = !owner_guid_before.is_empty()
            && self
                .map_object_record(owner_guid_before)
                .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let cleared_owner = !owner_guid_before.is_empty();

        if cleared_owner {
            if let Some(game_object) = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::game_object_mut)
            {
                game_object.clear_owner_guid_like_cpp();
            }
        }

        let (
            unit_owned_gameobject_list_removed,
            unit_object_slot_cleared,
            aura_cleanup_removed_count,
            creature_ai_callback_represented,
        ) = if owner_found_as_unit_like {
            self.map_objects
                .get_mut(&owner_guid_before)
                .map(|record| {
                    let creature_ai_callback_represented = match record.kind() {
                        AccessorObjectKind::Creature => record
                            .creature_mut()
                            .map(|creature| {
                                creature
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .summoned_gameobject_despawn_like_cpp()
                            })
                            .unwrap_or(false),
                        AccessorObjectKind::Pet => record
                            .pet_mut()
                            .map(|pet| {
                                pet.creature_mut()
                                    .unit_mut()
                                    .subsystems_mut()
                                    .ai
                                    .summoned_gameobject_despawn_like_cpp()
                            })
                            .unwrap_or(false),
                        _ => false,
                    };
                    let Some(owner) = Self::map_record_unit_mut_like_cpp(record) else {
                        return (false, false, 0, creature_ai_callback_represented);
                    };
                    let subsystems = owner.subsystems_mut();
                    let control = &mut subsystems.control;
                    let unit_owned_gameobject_list_removed =
                        control.remove_owned_gameobject_like_cpp(guid);
                    let unit_object_slot_cleared =
                        control.clear_gameobject_slot_for_guid_like_cpp(guid);
                    let aura_cleanup_removed_count = (spell_id != 0)
                        .then(|| {
                            subsystems
                                .auras
                                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                                .len()
                        })
                        .unwrap_or(0);
                    (
                        unit_owned_gameobject_list_removed,
                        unit_object_slot_cleared,
                        aura_cleanup_removed_count,
                        creature_ai_callback_represented,
                    )
                })
                .unwrap_or((false, false, 0, false))
        } else {
            (false, false, 0, false)
        };

        Some(GameObjectRemoveFromOwnerOutcomeLikeCpp {
            guid,
            owner_guid_before,
            owner_guid_after: if cleared_owner {
                ObjectGuid::EMPTY
            } else {
                owner_guid_before
            },
            owner_found_as_unit_like,
            cleared_owner,
            spell_id,
            unit_side_effects_represented: owner_found_as_unit_like,
            unit_owned_gameobject_list_removed,
            unit_object_slot_cleared,
            aura_cleanup_represented: spell_id != 0 && owner_found_as_unit_like,
            aura_cleanup_removed_count,
            cooldown_event_represented: false,
            creature_ai_callback_represented,
        })
    }

    /// Bounded map-owned representation of C++ `GameObject::Delete()`.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:1740-1764`: `SetLootState(GO_NOT_READY)`,
    ///   `RemoveFromOwner()`, optional capture-point packet, `SendGameObjectDespawn()`,
    ///   GO state reset for non-transports, override flag restore, then PoolMgr or
    ///   `AddObjectToRemoveList()`.
    /// - `Map.cpp:2547-2555`: `AddObjectToRemoveList()` is the physical-removal
    ///   handoff; extraction happens later in `RemoveAllObjectsInRemoveList()`.
    fn gameobject_delete_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp> {
        let go_type = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .map(|game_object| game_object.data().type_id as u32)?;

        if let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        {
            // `GameObject::Delete` queues physical removal without calling
            // `ClearLoot`. Terminally detach only the async authority here:
            // this prevents both Arc-held claims and an async generator from
            // reactivating the deleted lifetime while preserving C++'s
            // interim object fields until remove-list drain.
            game_object.loot_authority_like_cpp().detach_like_cpp();
            game_object.set_loot_state(LootState::NotReady, None);
        }
        let remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
        let capture_point_packet_represented = go_type == GAMEOBJECT_TYPE_CAPTURE_POINT;
        let despawn_packet_represented = true;

        let (go_state_ready, flags_restored) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
            .map(|game_object| {
                let go_state_ready = go_type != GAMEOBJECT_TYPE_TRANSPORT;
                if go_state_ready {
                    game_object.set_go_state(GoState::Ready);
                }
                let flags_restored = game_object.restore_represented_baseline_flags_like_cpp();
                (go_state_ready, flags_restored)
            })
            .unwrap_or((false, false));

        let remove_list = self.add_object_to_remove_list_like_cpp(guid);
        Some(GameObjectDeleteOutcomeLikeCpp {
            guid,
            remove_from_owner,
            capture_point_packet_represented,
            despawn_packet_represented,
            go_state_ready,
            flags_restored,
            pool_update_represented: false,
            pool_update_plan: None,
            pool_update_error: None,
            pool_update_summary: None,
            remove_list: Some(remove_list),
        })
    }

    fn gameobject_delete_from_update_with_optional_loader_like_cpp<L>(
        &mut self,
        guid: ObjectGuid,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        match pool_update {
            Some((spawn_store, pool_mgr)) => match load_record {
                Some(loader) => self
                    .gameobject_delete_with_pool_update_loaded_grid_records_like_cpp(
                        guid,
                        spawn_store,
                        pool_mgr,
                        |_, _| 0.0,
                        |_candidates, count| (0..count).collect(),
                        loader,
                    ),
                None => self.gameobject_delete_with_pool_update_like_cpp(
                    guid,
                    spawn_store,
                    pool_mgr,
                    |_, _| 0.0,
                    |_candidates, count| (0..count).collect(),
                ),
            },
            None => self.gameobject_delete_like_cpp(guid),
        }
    }

    /// Bounded map-owned representation of C++ `GameObject::Delete()` with
    /// the compatibility-mode `PoolMgr::UpdatePool<GameObject>` branch.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:1759-1763`: if `m_respawnCompatibilityMode && poolid`,
    ///   call `sPoolMgr->UpdatePool<GameObject>(..., poolid, GetSpawnId())`;
    ///   otherwise call `AddObjectToRemoveList()`.
    /// - `PoolMgr.cpp:891-905`: `UpdatePool<T>` either updates a mother pool
    ///   or spawns from the typed pool using the triggering spawn id.
    ///
    /// This helper consumes only the represented map-owned PoolMgr plan. It does
    /// not perform DB writes, fabricate DB-backed GameObjects, or fan out packets.
    pub fn gameobject_delete_with_pool_update_like_cpp<R, C>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: R,
        choose_equal: C,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    {
        self.gameobject_delete_with_optional_pool_update_loader_like_cpp::<R, C, fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
            guid,
            spawn_store,
            pool_mgr,
            explicit_roll_for,
            choose_equal,
            None,
        )
    }

    pub fn gameobject_delete_with_pool_update_loaded_grid_records_like_cpp<R, C, L>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        explicit_roll_for: R,
        choose_equal: C,
        load_record: L,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.gameobject_delete_with_optional_pool_update_loader_like_cpp(
            guid,
            spawn_store,
            pool_mgr,
            explicit_roll_for,
            choose_equal,
            Some(load_record),
        )
    }

    fn gameobject_delete_with_optional_pool_update_loader_like_cpp<R, C, L>(
        &mut self,
        guid: ObjectGuid,
        spawn_store: &SpawnStore,
        pool_mgr: &PoolMgrLikeCpp,
        mut explicit_roll_for: R,
        mut choose_equal: C,
        load_record: Option<L>,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        R: FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        C: FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let (go_type, spawn_id, respawn_compatibility_mode, represented_gameobject_data_present) =
            self.map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .map(|game_object| {
                    (
                        game_object.data().type_id as u32,
                        game_object.spawn_id(),
                        game_object.respawn_compatibility_mode(),
                        game_object.has_represented_gameobject_data_like_cpp(),
                    )
                })?;

        if let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        {
            game_object.loot_authority_like_cpp().detach_like_cpp();
            game_object.set_loot_state(LootState::NotReady, None);
        }
        let remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
        let capture_point_packet_represented = go_type == GAMEOBJECT_TYPE_CAPTURE_POINT;
        let despawn_packet_represented = true;

        let (go_state_ready, flags_restored) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
            .map(|game_object| {
                let go_state_ready = go_type != GAMEOBJECT_TYPE_TRANSPORT;
                if go_state_ready {
                    game_object.set_go_state(GoState::Ready);
                }
                let flags_restored = game_object.restore_represented_baseline_flags_like_cpp();
                (go_state_ready, flags_restored)
            })
            .unwrap_or((false, false));

        let pool_id =
            if respawn_compatibility_mode && represented_gameobject_data_present && spawn_id != 0 {
                spawn_store
                    .spawn_data(SpawnObjectType::GameObject, spawn_id)
                    .map(|spawn| spawn.pool_id)
                    .unwrap_or(0)
            } else {
                0
            };

        let mut pool_update_plan = None;
        let mut pool_update_error = None;
        let mut pool_update_summary = None;
        let mut remove_list = None;

        if pool_id != 0 {
            match pool_mgr.update_pool_plan_like_cpp(
                &mut self.pool_data,
                pool_id,
                SpawnObjectType::GameObject,
                spawn_id,
                &mut explicit_roll_for,
                &mut choose_equal,
            ) {
                Ok(plan) => {
                    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();
                    if let Some(mut load_record) = load_record {
                        self.apply_pool_typed_spawn_plan_loaded_grid_records_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                            Some(&mut load_record),
                        );
                    } else {
                        self.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(
                            &plan,
                            spawn_store,
                            &mut summary,
                        );
                    }
                    pool_update_summary = Some(summary);
                    pool_update_plan = Some(plan);
                }
                Err(error) => {
                    pool_update_error = Some(error);
                }
            }
        } else {
            remove_list = Some(self.add_object_to_remove_list_like_cpp(guid));
        }

        Some(GameObjectDeleteOutcomeLikeCpp {
            guid,
            remove_from_owner,
            capture_point_packet_represented,
            despawn_packet_represented,
            go_state_ready,
            flags_restored,
            pool_update_represented: pool_update_plan.is_some(),
            pool_update_plan,
            pool_update_error,
            pool_update_summary,
            remove_list,
        })
    }

    /// Bounded map-owned representation of C++ `GameObject::RemoveFromWorld()`
    /// linked-trap cleanup.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:926-948`: after ZoneScript remove, `RemoveFromOwner`,
    ///   and represented model removal, `GetLinkedTrap()->DespawnOrUnsummon()`
    ///   runs before `WorldObject::RemoveFromWorld()` and before ObjectsStore
    ///   removal.
    /// - `Map.cpp:933-951`: `Map::RemoveFromMap<T>` calls
    ///   `obj->RemoveFromWorld()` before active/grid/reset/delete tail.
    fn gameobject_remove_linked_trap_like_cpp(
        &mut self,
        guid: ObjectGuid,
        remove_from_map_in_progress: &mut HashSet<ObjectGuid>,
    ) -> Option<GameObjectRemoveLinkedTrapOutcomeLikeCpp> {
        let linked_trap_guid = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .filter(|game_object| game_object.world().object().is_in_world())
            .map(GameObject::linked_trap_guid_like_cpp)?;

        let owner_present_before_linked_trap_remove = self.map_object_record(guid).is_some();
        let linked_trap_guid = (!linked_trap_guid.is_empty()).then_some(linked_trap_guid);
        let linked_trap_cycle_guarded = linked_trap_guid.is_some_and(|linked_guid| {
            linked_guid != guid && remove_from_map_in_progress.contains(&linked_guid)
        });
        let linked_trap_missing_or_self = linked_trap_guid.is_none_or(|linked_guid| {
            linked_guid == guid
                || (!linked_trap_cycle_guarded && self.map_object_record(linked_guid).is_none())
        });
        let linked_trap_delete = if let Some(linked_guid) = linked_trap_guid {
            if linked_guid == guid
                || linked_trap_cycle_guarded
                || self.map_object_record(linked_guid).is_none()
            {
                None
            } else {
                self.gameobject_delete_like_cpp(linked_guid)
            }
        } else {
            None
        };
        let linked_trap_remove_queued = linked_trap_delete.as_ref().is_some_and(|delete| {
            delete
                .remove_list
                .as_ref()
                .is_some_and(|remove| remove.queued || remove.duplicate)
        });

        Some(GameObjectRemoveLinkedTrapOutcomeLikeCpp {
            guid,
            linked_trap_guid,
            owner_present_before_linked_trap_remove,
            linked_trap_removed: false,
            linked_trap_remove_queued,
            linked_trap_missing_or_self,
            linked_trap_cycle_guarded,
            despawn_or_unsummon_scheduler_represented: linked_trap_delete.is_some(),
            object_accessor_fanout_represented: false,
        })
    }

    /// Bounded map-owned cleanup for the late C++ `Player::RemoveFromWorld()`
    /// `GetViewpoint()` -> `SetViewpoint(viewpoint, false)` branch.
    ///
    /// Source-of-truth anchors:
    /// - `Player.cpp:1567-1585` runs this after `Unit::RemoveFromWorld()` and
    ///   item cleanup while the Player still exists.
    /// - `Player.cpp:25344-25387` clears `FarsightObject`, removes Unit shared
    ///   vision for Unit targets, requests `SetSeer(this)`, and does not request
    ///   `UpdateVisibilityOf` on remove.
    /// - `Player.cpp:25389-25395` resolves `GetViewpoint()` from
    ///   `FarsightObject` through `TYPEMASK_SEER`.
    ///
    /// Ownership: only canonical same-map `Map::map_objects` typed records are
    /// consulted/mutated. DynamicObject targets clear only the removing Player's
    /// `FarsightObject` when it still equals the target GUID; this branch never
    /// resolves `DynamicObject::bound_caster()` or toggles DynamicObject caster
    /// viewpoint state because that lifecycle belongs to DynamicObject removal.
    /// There is no ObjectAccessor/session fallback, no packet fanout, and no real
    /// SetSeer implementation in this seam. Vehicle-base skipping stays open
    /// because this map-owned cleanup has no Player vehicle base runtime; the Unit
    /// helper is called with `vehicle_base_guid: None`.
    fn cleanup_player_remove_from_world_viewpoint_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) -> Option<PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp> {
        let player_record = self.map_object_record(player_guid)?;
        if player_record.kind() != AccessorObjectKind::Player
            || !player_record.object().object().is_in_world()
        {
            return None;
        }

        let viewpoint_guid = player_record
            .player()
            .map(|player| player.active_data().farsight_object)?;
        if viewpoint_guid.is_empty() {
            return None;
        }

        let outcome = |status,
                       player_set_viewpoint: Option<PlayerSetViewpointOutcomeLikeCpp>,
                       dynamic_object_caster_viewpoint: Option<
            DynamicObjectCasterViewpointOutcomeLikeCpp,
        >,
                       update_visibility_requested,
                       set_seer_requested| {
            PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp {
                player_guid,
                viewpoint_guid,
                status,
                player_set_viewpoint,
                dynamic_object_caster_viewpoint,
                update_visibility_requested,
                set_seer_requested,
                object_accessor_fanout_represented: false,
            }
        };

        let Some(target_record) = self.map_object_record(viewpoint_guid) else {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::MissingTarget,
                None,
                None,
                false,
                false,
            ));
        };
        let target_kind = target_record.kind();
        if !target_record.object().object().is_in_world() {
            return Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotInWorld,
                None,
                None,
                false,
                false,
            ));
        }

        match target_kind {
            AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
                let player_set_viewpoint = self.apply_player_set_viewpoint_unit_like_cpp(
                    player_guid,
                    viewpoint_guid,
                    false,
                    None,
                );
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedUnitViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::DynamicObject => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        Self::player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    Some(_) => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                    None => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            AccessorObjectKind::Player => {
                let player_set_viewpoint = match self.get_typed_player_mut(player_guid) {
                    Some(player) if player.active_data().farsight_object == viewpoint_guid => {
                        player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                        Self::player_set_viewpoint_outcome_like_cpp(
                            player_guid,
                            viewpoint_guid,
                            false,
                            PlayerSetViewpointStatusLikeCpp::Removed,
                            None,
                            false,
                            true,
                        )
                    }
                    _ => Self::player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        viewpoint_guid,
                        false,
                        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                        None,
                        false,
                        false,
                    ),
                };
                Some(outcome(
                    PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedPlayerViewpoint,
                    Some(player_set_viewpoint),
                    None,
                    player_set_viewpoint.update_visibility_requested,
                    player_set_viewpoint.set_seer_requested,
                ))
            }
            _ => Some(outcome(
                PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotSeer,
                None,
                None,
                false,
                false,
            )),
        }
    }

    pub fn remove_from_map_like_cpp(
        &mut self,
        guid: ObjectGuid,
        delete_from_world: bool,
    ) -> Result<RemoveFromMapOutcome, RemoveFromMapError> {
        let mut remove_from_map_in_progress = HashSet::new();
        self.remove_from_map_like_cpp_inner(
            guid,
            delete_from_world,
            &mut remove_from_map_in_progress,
        )
    }

    fn remove_from_map_like_cpp_inner(
        &mut self,
        guid: ObjectGuid,
        delete_from_world: bool,
        remove_from_map_in_progress: &mut HashSet<ObjectGuid>,
    ) -> Result<RemoveFromMapOutcome, RemoveFromMapError> {
        if !remove_from_map_in_progress.insert(guid) {
            return Err(RemoveFromMapError::ObjectNotFound { guid });
        }

        let outcome = (|| {
            let should_cleanup_dynamic_object_caster_viewpoint = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::dynamic_object)
                .is_some_and(|dynamic_object| {
                    dynamic_object.world().object().is_in_world()
                        && dynamic_object.is_caster_viewpoint()
                });
            let dynamic_object_caster_viewpoint = should_cleanup_dynamic_object_caster_viewpoint
                .then(|| self.apply_dynamic_object_caster_viewpoint_like_cpp(guid, false));
            let dynamic_object_remove_cleanup = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::dynamic_object_mut)
                .and_then(|dynamic_object| {
                    if !dynamic_object.world().object().is_in_world() {
                        return None;
                    }

                    let had_aura = dynamic_object.has_aura();
                    if had_aura {
                        dynamic_object.remove_aura();
                    }

                    let unbound_caster = dynamic_object.bound_caster();
                    if unbound_caster.is_some() {
                        dynamic_object.unbind_from_caster();
                    }

                    Some(DynamicObjectRemoveCleanupOutcomeLikeCpp {
                        had_aura,
                        removed_aura_pending_delete: dynamic_object
                            .has_removed_aura_pending_delete(),
                        unbound_caster,
                    })
                });
            let gameobject_model_key = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.world().object().is_in_world())
                .filter(|game_object| game_object.has_represented_gameobject_model_like_cpp())
                .map(|_| RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid });
            let gameobject_model_remove_pending_before_callback = gameobject_model_key
                .is_some_and(|key| self.contains_gameobject_model_like_cpp(key));
            let gameobject_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::GameObject)
                .and_then(MapObjectRecord::game_object)
                .filter(|game_object| game_object.world().object().is_in_world())
                .map(|game_object| {
                    let spawn_id = game_object.spawn_id();
                    GameObjectZoneScriptRemoveOutcomeLikeCpp {
                        guid,
                        represented_callback_boundary: true,
                        script_dispatch_represented: false,
                        model_remove_pending_before_callback:
                            gameobject_model_remove_pending_before_callback,
                        spawn_index_present_before_callback: spawn_id != 0
                            && self
                                .gameobject_spawn_id_store_guids_like_cpp(spawn_id)
                                .contains(&guid),
                    }
                });
            let gameobject_remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
            let gameobject_model_remove = gameobject_model_key.and_then(|key| {
                self.contains_gameobject_model_like_cpp(key)
                    .then(|| self.remove_gameobject_model_like_cpp(key))
            });
            let gameobject_linked_trap_remove =
                self.gameobject_remove_linked_trap_like_cpp(guid, remove_from_map_in_progress);
            let remove_from_map_was_in_world = self
                .map_object_record(guid)
                .is_some_and(|record| record.object().object().is_in_world());
            let creature_zone_script_remove = self
                .map_object_record(guid)
                .filter(|record| record.kind() == AccessorObjectKind::Creature)
                .and_then(MapObjectRecord::creature)
                .filter(|creature| creature.unit().world().object().is_in_world())
                .map(|_| CreatureZoneScriptRemoveOutcomeLikeCpp {
                    guid,
                    represented_callback: true,
                    script_dispatch_represented: false,
                });
            let creature_remove_formation = self.remove_creature_from_formation_like_cpp(guid);
            let creature_unit_remove_from_world = self
                .map_objects
                .get_mut(&guid)
                .and_then(MapObjectRecord::creature_mut)
                .and_then(|creature| creature.unit_mut().remove_from_world_like_cpp());
            let creature_vehicle_remove = creature_unit_remove_from_world
                .as_ref()
                .and_then(|outcome| outcome.vehicle_remove);
            let player_viewpoint_cleanup =
                self.cleanup_player_remove_from_world_viewpoint_like_cpp(guid);
            let (kind, was_active) = self
                .map_object_record(guid)
                .map(|record| {
                    (
                        record.kind(),
                        is_active_object_like_cpp(record.kind(), record.object()),
                    )
                })
                .ok_or(RemoveFromMapError::ObjectNotFound { guid })?;
            let remove_from_active = was_active.then(|| self.remove_from_active_like_cpp(guid));
            let mut record = self
                .remove_map_object(guid)
                .ok_or(RemoveFromMapError::ObjectNotFound { guid })?;
            // Rust's non-delete outcome retains only the erased
            // `WorldObject`; `MapObjectRecord::into_object` still destroys the
            // typed Creature/GameObject that owns its Loot. Until this API can
            // return the full typed record like C++ retains the object pointer,
            // both paths must terminally detach that otherwise orphaned
            // authority. A stale lease may finish only if it already crossed
            // the protected durable boundary.
            detach_typed_loot_authority_like_cpp(&mut record);
            let was_world_object_like_cpp = map_record_is_world_object_like_cpp(&record);
            let mut object = record.into_object();
            let was_in_world = remove_from_map_was_in_world;
            let cxx_in_world =
                was_in_world && remove_from_map_in_world_eligible_type_like_cpp(kind);
            let personal_phase_owner = object.phase_shift().personal_guid_like_cpp();
            let cell = Cell::from_world(object.position().x, object.position().y);
            let grid = GridCoord::new(cell.grid_x(), cell.grid_y());

            object.object_mut().remove_from_world();
            let personal_phase_unregister = self
                .personal_phase_tracker
                .unregister_tracked_object_for_phase_owner_like_cpp(personal_phase_owner, guid);
            let visibility_on_destroy = RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp {
                guid,
                cxx_in_world,
                update_object_visibility_on_destroy_represented: !cxx_in_world,
                update_object_visibility_on_destroy_runtime_gap: !cxx_in_world,
            };
            let removed_from_cell = remove_object_guid_from_cell_like_cpp(
                self,
                grid,
                &cell,
                kind,
                was_world_object_like_cpp,
                guid,
            );

            object.clear_current_cell();
            object.reset_map().map_err(RemoveFromMapError::ResetMap)?;

            Ok(RemoveFromMapOutcome {
                guid,
                cell: cell.cell_coord(),
                grid,
                was_in_world,
                cxx_in_world,
                was_active,
                remove_from_active,
                removed_from_cell,
                delete_from_world,
                dynamic_object_caster_viewpoint,
                dynamic_object_remove_cleanup,
                gameobject_zone_script_remove,
                gameobject_remove_from_owner,
                gameobject_model_remove,
                gameobject_linked_trap_remove,
                creature_zone_script_remove,
                creature_vehicle_remove,
                player_viewpoint_cleanup,
                creature_unit_remove_from_world,
                creature_remove_formation,
                personal_phase_unregister,
                visibility_on_destroy,
                object: if delete_from_world {
                    None
                } else {
                    Some(object)
                },
            })
        })();
        remove_from_map_in_progress.remove(&guid);
        outcome
    }

    pub fn relocate_map_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_position: Position,
    ) -> Result<MapObjectRelocationOutcome, MapObjectRelocationError> {
        if !is_valid_map_coord_2d(new_position.x, new_position.y) {
            return Err(MapObjectRelocationError::InvalidCoordinates {
                guid,
                x: new_position.x,
                y: new_position.y,
            });
        }

        let record = self
            .map_object_record(guid)
            .ok_or(MapObjectRelocationError::ObjectNotFound { guid })?;
        let kind = record.kind();
        let old_position = record.object().position();
        let old_cell = Cell::from_world(old_position.x, old_position.y);
        let new_cell = Cell::from_world(new_position.x, new_position.y);
        let old_grid = GridCoord::new(old_cell.grid_x(), old_cell.grid_y());
        let new_grid = GridCoord::new(new_cell.grid_x(), new_cell.grid_y());
        let diff_cell = old_cell.diff_cell(&new_cell);
        let diff_grid = old_cell.diff_grid(&new_cell);

        if !diff_cell && !diff_grid {
            let mut record = self
                .remove_map_object(guid)
                .expect("record was just observed");
            record.object_mut().relocate(new_position);
            self.insert_map_object_record(record)
                .map_err(MapObjectRelocationError::Store)?;
            return Ok(MapObjectRelocationOutcome {
                guid,
                old_cell: old_cell.cell_coord(),
                new_cell: new_cell.cell_coord(),
                old_grid,
                new_grid,
                moved_between_cells: false,
                loaded_grid: false,
                created_grid: false,
                relocated: true,
                blocked_by_unloaded_grid: false,
            });
        }

        let active_object = is_active_object_like_cpp(kind, record.object());
        let loaded_grid = if diff_grid && active_object {
            self.ensure_grid_loaded_for_active_object(&new_cell, kind.into())
        } else {
            false
        };
        let created_grid = if diff_grid && !active_object {
            if !self.is_grid_loaded(new_grid) {
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid: false,
                    created_grid: false,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            }
            self.ensure_grid_created(new_grid)
        } else {
            false
        };

        if self.get_ngrid(new_grid).is_none() {
            return Ok(MapObjectRelocationOutcome {
                guid,
                old_cell: old_cell.cell_coord(),
                new_cell: new_cell.cell_coord(),
                old_grid,
                new_grid,
                moved_between_cells: false,
                loaded_grid,
                created_grid: false,
                relocated: false,
                blocked_by_unloaded_grid: true,
            });
        }

        let mut record = self
            .remove_map_object(guid)
            .expect("record was just observed");
        let object_is_world_object = record.object().is_world_object();
        let removed = remove_object_guid_from_cell_like_cpp(
            self,
            old_grid,
            &old_cell,
            kind,
            object_is_world_object,
            guid,
        );
        let _removed_from_old_cell = removed;
        {
            let Some(ngrid) = self.get_ngrid_mut(new_grid) else {
                self.insert_map_object_record(record)
                    .map_err(MapObjectRelocationError::Store)?;
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid,
                    created_grid,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            };
            let Some(local_cell) = ngrid.get_grid_type_mut(new_cell.cell_x(), new_cell.cell_y())
            else {
                self.insert_map_object_record(record)
                    .map_err(MapObjectRelocationError::Store)?;
                return Ok(MapObjectRelocationOutcome {
                    guid,
                    old_cell: old_cell.cell_coord(),
                    new_cell: new_cell.cell_coord(),
                    old_grid,
                    new_grid,
                    moved_between_cells: false,
                    loaded_grid,
                    created_grid,
                    relocated: false,
                    blocked_by_unloaded_grid: true,
                });
            };
            insert_object_guid_in_cell_like_cpp(local_cell, kind, object_is_world_object, guid);
        }
        record.object_mut().relocate(new_position);
        record
            .object_mut()
            .set_current_cell(new_cell.cell_x(), new_cell.cell_y());
        self.insert_map_object_record(record)
            .map_err(MapObjectRelocationError::Store)?;

        Ok(MapObjectRelocationOutcome {
            guid,
            old_cell: old_cell.cell_coord(),
            new_cell: new_cell.cell_coord(),
            old_grid,
            new_grid,
            moved_between_cells: true,
            loaded_grid,
            created_grid,
            relocated: true,
            blocked_by_unloaded_grid: false,
        })
    }

    pub fn nearby_cell_guids_like_cpp(&self, x: f32, y: f32, radius: f32) -> NearbyCellGuids {
        if !is_valid_map_coord_2d(x, y) {
            return NearbyCellGuids::default();
        }

        let area = calculate_cell_area_like_cpp(x, y, radius);
        let mut result = NearbyCellGuids::default();
        for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
            for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                result.visited_cells += 1;
                let cell = Cell::from_cell_coord(CellCoord::new(cell_x, cell_y));
                let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                else {
                    continue;
                };
                let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                    continue;
                };
                result.merge_world(&local_cell.world_objects);
                result.merge_grid(&local_cell.grid_objects);
            }
        }

        result
    }

    pub fn visit_nearby_cells_of_like_cpp(
        &self,
        centers: impl IntoIterator<Item = NearbyCellVisitCenter>,
    ) -> NearbyCellVisitPlan {
        let mut marked_cells = HashSet::new();
        let mut marked_cells_in_visit_order = Vec::new();
        let mut nearby = NearbyCellGuids::default();
        let mut skipped_missing_centers = Vec::new();
        let mut skipped_invalid_position_centers = Vec::new();

        for center in centers {
            let Some(object) = self.map_object(center.guid) else {
                skipped_missing_centers.push(center.guid);
                continue;
            };
            let position = object.position();
            if !is_valid_map_coord_2d(position.x, position.y) {
                skipped_invalid_position_centers.push(center.guid);
                continue;
            }

            let area =
                calculate_cell_area_like_cpp(position.x, position.y, center.activation_radius);
            for cell_x in area.low_bound.x_coord..=area.high_bound.x_coord {
                for cell_y in area.low_bound.y_coord..=area.high_bound.y_coord {
                    let cell_coord = CellCoord::new(cell_x, cell_y);
                    if !marked_cells.insert(cell_coord) {
                        continue;
                    }

                    marked_cells_in_visit_order.push(cell_coord);
                    nearby.visited_cells += 1;
                    let cell = Cell::from_cell_coord(cell_coord);
                    let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
                    else {
                        continue;
                    };
                    let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                        continue;
                    };
                    nearby.merge_world(&local_cell.world_objects);
                    nearby.merge_grid(&local_cell.grid_objects);
                }
            }
        }

        NearbyCellVisitPlan {
            marked_cells: marked_cells_in_visit_order,
            nearby,
            skipped_missing_centers,
            skipped_invalid_position_centers,
        }
    }

    pub fn object_update_plan_for_nearby_like_cpp(
        &self,
        nearby: &NearbyCellGuids,
        diff_ms: u32,
    ) -> ObjectUpdatePlan {
        let mut update_guids = Vec::new();
        for guid in nearby
            .world
            .creatures
            .iter()
            .chain(nearby.world.dynamic_objects.iter())
            .chain(nearby.grid.creatures.iter())
            .chain(nearby.grid.gameobjects.iter())
            .chain(nearby.grid.dynamic_objects.iter())
            .chain(nearby.grid.area_triggers.iter())
            .chain(nearby.grid.scene_objects.iter())
            .chain(nearby.grid.conversations.iter())
        {
            if self
                .map_object(*guid)
                .is_some_and(|object| object.object().is_in_world())
            {
                update_guids.push(*guid);
            }
        }

        update_guids.sort();
        update_guids.dedup();
        ObjectUpdatePlan {
            diff_ms,
            update_guids,
        }
    }

    /// Live represented C++ `Map::Update` source selection for
    /// `ProcessRelocationNotifies(t_diff)` (`Map.cpp:692-717,797-805,830-905`).
    ///
    /// Source of truth stays map-owned canonical `map_objects`: typed in-world
    /// Players become player sources, typed in-world active non-Players become
    /// active object sources, and the existing visit/relocation helpers consume
    /// marked cells and reset notify flags. Unsupported far combat/aura/summon
    /// source ownership remains a gap and is represented by empty source lists;
    /// no session, ObjectAccessor, packet, AI, dynamic-tree, or fanout side
    /// effects are claimed here.
    pub fn process_live_relocation_notifies_like_cpp(
        &mut self,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> ProcessRelocationNotifiesOutcome {
        let mut player_sources = Vec::new();

        for (guid, record) in &self.map_objects {
            let object = record.object();
            if !object.object().is_in_world() || record.kind() != AccessorObjectKind::Player {
                continue;
            }

            let viewpoint_guid = record.player().and_then(|player| {
                let farsight = player.active_data().farsight_object;
                (!farsight.is_empty()).then_some(farsight)
            });
            player_sources.push(MapUpdatePlayerSources {
                player_guid: *guid,
                viewpoint_guid,
                far_combat_unit_guids: Vec::new(),
                far_aura_caster_guids: Vec::new(),
                far_summon_guids: Vec::new(),
            });
        }

        let active_non_player_guids = self.represented_active_non_player_sources_like_cpp();
        player_sources.sort_by_key(|source| source.player_guid);
        player_sources.dedup_by_key(|source| source.player_guid);

        let visit_plan = self.map_update_visit_plan_like_cpp(
            player_sources,
            active_non_player_guids,
            std::iter::empty(),
            diff_ms,
        );
        if !visit_plan.process_relocation_notifies {
            return ProcessRelocationNotifiesOutcome::default();
        }

        let centers =
            visit_plan
                .nearby_visit_centers
                .into_iter()
                .map(|guid| NearbyCellVisitCenter {
                    guid,
                    activation_radius: MAX_VISIBILITY_DISTANCE,
                });
        let nearby_plan = self.visit_nearby_cells_of_like_cpp(centers);
        self.process_relocation_notifies_like_cpp(
            nearby_plan.marked_cells,
            diff_ms,
            visibility_notify_period_ms,
            std::iter::empty(),
        )
    }

    pub fn map_update_visit_plan_like_cpp(
        &self,
        sources: impl IntoIterator<Item = MapUpdatePlayerSources>,
        active_non_player_guids: impl IntoIterator<Item = ObjectGuid>,
        transport_guids: impl IntoIterator<Item = ObjectGuid>,
        diff_ms: u32,
    ) -> MapUpdateVisitPlan {
        let mut session_update_players = Vec::new();
        let mut player_update_guids = Vec::new();
        let mut nearby_visit_centers = Vec::new();
        let mut saw_player_source = false;

        for source in sources {
            saw_player_source = true;
            if !self.object_is_in_world(source.player_guid) {
                continue;
            }

            session_update_players.push(source.player_guid);
            player_update_guids.push(source.player_guid);
            nearby_visit_centers.push(source.player_guid);

            if let Some(viewpoint) = source.viewpoint_guid
                && self.object_is_in_world(viewpoint)
            {
                nearby_visit_centers.push(viewpoint);
            }

            push_in_world_guids(
                self,
                &mut nearby_visit_centers,
                source.far_combat_unit_guids,
            );
            push_in_world_guids(
                self,
                &mut nearby_visit_centers,
                source.far_aura_caster_guids,
            );
            push_in_world_guids(self, &mut nearby_visit_centers, source.far_summon_guids);
        }

        let mut saw_active_non_player_source = false;
        for guid in active_non_player_guids {
            saw_active_non_player_source = true;
            if self.object_is_in_world(guid) {
                nearby_visit_centers.push(guid);
            }
        }

        let mut transport_update_guids = Vec::new();
        for guid in transport_guids {
            if self.map_object(guid).is_some() {
                transport_update_guids.push(guid);
            }
        }

        sort_dedup(&mut session_update_players);
        sort_dedup(&mut player_update_guids);
        sort_dedup(&mut nearby_visit_centers);
        sort_dedup(&mut transport_update_guids);
        let process_relocation_notifies = saw_player_source || saw_active_non_player_source;

        MapUpdateVisitPlan {
            diff_ms,
            session_update_players,
            player_update_guids,
            nearby_visit_centers,
            transport_update_guids,
            process_relocation_notifies,
        }
    }

    pub fn process_relocation_notifies_plan_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
    ) -> RelocationNotifyProcessPlan {
        let marked_cells: HashSet<_> = marked_cells.into_iter().collect();
        let mut delayed_relocation_cells = Vec::new();
        let mut reset_notify_cells = Vec::new();
        let mut reset_timer_grids = Vec::new();
        let mut expired_active_grids = Vec::new();

        for grid_x in 0..MAX_NUMBER_OF_GRIDS {
            for grid_y in 0..MAX_NUMBER_OF_GRIDS {
                let coord = GridCoord::new(grid_x, grid_y);
                let Some(grid) = self.get_ngrid_mut(coord) else {
                    continue;
                };
                if grid.state() != GridStateKind::Active {
                    continue;
                }

                grid.info_mut()
                    .relocation_timer_mut()
                    .tracker_update(diff_ms);
                if !grid.info().relocation_timer().tracker_passed() {
                    continue;
                }

                expired_active_grids.push(coord);
                delayed_relocation_cells
                    .extend(marked_cells_in_grid_like_cpp(coord, &marked_cells));
            }
        }

        for coord in &expired_active_grids {
            let Some(grid) = self.get_ngrid_mut(*coord) else {
                continue;
            };
            if grid.state() != GridStateKind::Active {
                continue;
            }
            if !grid.info().relocation_timer().tracker_passed() {
                continue;
            }

            grid.info_mut()
                .relocation_timer_mut()
                .tracker_reset(diff_ms, visibility_notify_period_ms);
            reset_timer_grids.push(*coord);
            reset_notify_cells.extend(marked_cells_in_grid_like_cpp(*coord, &marked_cells));
        }

        RelocationNotifyProcessPlan {
            diff_ms,
            delayed_relocation_cells,
            reset_notify_cells,
            reset_timer_grids,
        }
    }

    pub fn process_relocation_notifies_like_cpp(
        &mut self,
        marked_cells: impl IntoIterator<Item = CellCoord>,
        diff_ms: u32,
        visibility_notify_period_ms: i64,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> ProcessRelocationNotifiesOutcome {
        let process_plan = self.process_relocation_notifies_plan_like_cpp(
            marked_cells,
            diff_ms,
            visibility_notify_period_ms,
        );
        let delayed_plan = self.delayed_unit_relocation_for_cells_like_cpp(
            process_plan.delayed_relocation_cells.iter().copied(),
            invalid_non_self_viewpoints,
        );
        // C++ runs DelayedUnitRelocation's CreatureRelocationNotifier and
        // PlayerRelocationNotifier while NOTIFY_VISIBILITY_CHANGED is still set,
        // before ResetNotifier clears the cell. Rust exposes only represented
        // visibility/AI evidence here: no packets, sessions, ObjectAccessor fanout,
        // real UpdateObjectVisibility, or SendObjectUpdates are executed.
        let visibility_plans = self.delayed_unit_relocation_visibility_plans_like_cpp(
            &delayed_plan,
            self.delayed_player_relocation_contexts_from_plan_like_cpp(&delayed_plan),
            self.delayed_creature_relocation_contexts_from_plan_like_cpp(&delayed_plan),
        );
        let reset_outcome = self
            .reset_notify_flags_for_cells_like_cpp(process_plan.reset_notify_cells.iter().copied());

        ProcessRelocationNotifiesOutcome {
            process_plan,
            delayed_plan,
            visibility_plans,
            reset_outcome,
        }
    }

    pub fn reset_notify_flags_for_cells_like_cpp(
        &mut self,
        cells: impl IntoIterator<Item = CellCoord>,
    ) -> ResetNotifyFlagsOutcome {
        let mut reset_player_guids = Vec::new();
        let mut reset_creature_guids = Vec::new();
        let mut missing_guids = Vec::new();

        for cell_coord in cells {
            let cell = Cell::from_cell_coord(cell_coord);
            let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
                continue;
            };
            let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
                continue;
            };

            reset_player_guids.extend(local_cell.world_objects.players.iter().copied());
            reset_creature_guids.extend(local_cell.grid_objects.creatures.iter().copied());
            reset_creature_guids.extend(local_cell.world_objects.creatures.iter().copied());
        }

        sort_dedup(&mut reset_player_guids);
        sort_dedup(&mut reset_creature_guids);

        for guid in reset_player_guids
            .iter()
            .chain(reset_creature_guids.iter())
            .copied()
        {
            let Some(record) = self.map_objects.get_mut(&guid) else {
                missing_guids.push(guid);
                continue;
            };
            record.object_mut().object_mut().reset_all_notifies();
        }

        ResetNotifyFlagsOutcome {
            reset_player_guids,
            reset_creature_guids,
            missing_guids,
        }
    }

    pub fn delayed_unit_relocation_for_cells_like_cpp(
        &self,
        cells: impl IntoIterator<Item = CellCoord>,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> DelayedUnitRelocationForCellsPlan {
        let invalid_non_self_viewpoints: HashSet<_> =
            invalid_non_self_viewpoints.into_iter().collect();
        let mut cell_plans = Vec::new();

        for cell_coord in cells {
            let nearby = self.exact_cell_guids_like_cpp(cell_coord);
            let creatures_needing_notify = nearby
                .world
                .creatures
                .iter()
                .chain(nearby.grid.creatures.iter())
                .copied()
                .filter(|guid| self.object_needs_notify_visibility(*guid));
            let mut plan = DelayedUnitRelocationPlan::from_nearby_like_cpp(
                &nearby,
                creatures_needing_notify,
                std::iter::empty::<ObjectGuid>(),
                std::iter::empty::<ObjectGuid>(),
            );
            let mut players: Vec<_> = nearby.world.players.iter().copied().collect();
            players.sort();
            for player_guid in players {
                let Some(viewpoint_guid) = self.player_viewpoint_guid_like_cpp(player_guid) else {
                    continue;
                };
                if !self.object_needs_notify_visibility(viewpoint_guid) {
                    continue;
                }
                if player_guid != viewpoint_guid
                    && (invalid_non_self_viewpoints.contains(&player_guid)
                        || invalid_non_self_viewpoints.contains(&viewpoint_guid)
                        || self.viewpoint_has_invalid_position_like_cpp(viewpoint_guid))
                {
                    plan.skipped_invalid_viewpoints.push(player_guid);
                    continue;
                }
                plan.player_relocations.push(player_guid);
            }
            sort_dedup(&mut plan.player_relocations);
            sort_dedup(&mut plan.skipped_invalid_viewpoints);
            if !plan.creature_relocations.is_empty()
                || !plan.player_relocations.is_empty()
                || !plan.skipped_invalid_viewpoints.is_empty()
            {
                cell_plans.push(DelayedUnitRelocationCellPlan { cell_coord, plan });
            }
        }

        DelayedUnitRelocationForCellsPlan { cell_plans }
    }

    pub fn delayed_unit_relocation_visibility_plans_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
        player_contexts: impl IntoIterator<Item = DelayedPlayerRelocationContext>,
        creature_contexts: impl IntoIterator<Item = DelayedCreatureRelocationContext>,
    ) -> DelayedUnitRelocationVisibilityPlans {
        let player_contexts: HashMap<_, _> = player_contexts
            .into_iter()
            .map(|context| (context.player_guid, context))
            .collect();
        let creature_contexts: HashMap<_, _> = creature_contexts
            .into_iter()
            .map(|context| (context.creature_guid, context))
            .collect();
        let mut creature_plans = Vec::new();
        let mut player_plans = Vec::new();
        let mut skipped_missing_sources = Vec::new();
        let mut skipped_invalid_source_positions = Vec::new();
        let mut missing_player_contexts = Vec::new();

        for cell_plan in &delayed_plan.cell_plans {
            for creature_guid in &cell_plan.plan.creature_relocations {
                let Some(creature) = self.map_object(*creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let position = creature.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(*creature_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + creature.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let Some(creature_context) = creature_contexts.get(creature_guid) else {
                    skipped_missing_sources.push(*creature_guid);
                    continue;
                };
                let source_creature_alive = creature_context.source_creature_alive;
                let visibility_plan = CreatureRelocationVisibilityPlan::from_nearby_like_cpp(
                    *creature_guid,
                    source_creature_alive,
                    &nearby,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                creature_plans.push(CreatureDelayedRelocationVisibilityPlan {
                    creature_guid: *creature_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }

            for player_guid in &cell_plan.plan.player_relocations {
                let Some(context) = player_contexts.get(player_guid) else {
                    missing_player_contexts.push(*player_guid);
                    continue;
                };
                let Some(viewpoint) = self.map_object(context.viewpoint_guid) else {
                    skipped_missing_sources.push(context.viewpoint_guid);
                    continue;
                };
                let position = viewpoint.position();
                if !is_valid_map_coord_2d(position.x, position.y) {
                    skipped_invalid_source_positions.push(context.viewpoint_guid);
                    continue;
                }

                let nearby = self.nearby_cell_guids_like_cpp(
                    position.x,
                    position.y,
                    MAX_VISIBILITY_DISTANCE + viewpoint.combat_reach(),
                );
                let player_seers_needing_notify = nearby
                    .world
                    .players
                    .iter()
                    .copied()
                    .filter(|guid| self.player_seer_needs_notify_visibility_like_cpp(*guid));
                let creatures_needing_notify = nearby
                    .world
                    .creatures
                    .iter()
                    .chain(nearby.grid.creatures.iter())
                    .copied()
                    .filter(|guid| self.object_needs_notify_visibility(*guid));
                let visibility_plan = PlayerRelocationVisibilityPlan::from_nearby_like_cpp(
                    *player_guid,
                    context.previous_client_guids.iter().copied(),
                    &nearby,
                    context.relocated_for_ai,
                    player_seers_needing_notify,
                    creatures_needing_notify,
                );
                player_plans.push(PlayerDelayedRelocationVisibilityPlan {
                    player_guid: *player_guid,
                    viewpoint_guid: context.viewpoint_guid,
                    cell_coord: cell_plan.cell_coord,
                    nearby,
                    visibility_plan,
                });
            }
        }

        sort_dedup(&mut skipped_missing_sources);
        sort_dedup(&mut skipped_invalid_source_positions);
        sort_dedup(&mut missing_player_contexts);

        DelayedUnitRelocationVisibilityPlans {
            creature_plans,
            player_plans,
            skipped_missing_sources,
            skipped_invalid_source_positions,
            missing_player_contexts,
        }
    }

    fn player_viewpoint_guid_like_cpp(&self, player_guid: ObjectGuid) -> Option<ObjectGuid> {
        let record = self.map_object_record(player_guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        let Some(player) = record.player() else {
            return Some(player_guid);
        };
        let farsight = player.active_data().farsight_object;
        Some(if farsight.is_empty() {
            player_guid
        } else {
            farsight
        })
    }

    fn player_seer_needs_notify_visibility_like_cpp(&self, player_guid: ObjectGuid) -> bool {
        self.player_viewpoint_guid_like_cpp(player_guid)
            .is_some_and(|viewpoint_guid| self.object_needs_notify_visibility(viewpoint_guid))
    }

    fn viewpoint_has_invalid_position_like_cpp(&self, viewpoint_guid: ObjectGuid) -> bool {
        self.map_object(viewpoint_guid).is_none_or(|viewpoint| {
            let position = viewpoint.position();
            !is_valid_map_coord_2d(position.x, position.y)
        })
    }

    fn delayed_player_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedPlayerRelocationContext> {
        let mut player_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.player_relocations.iter().copied())
            .collect();
        sort_dedup(&mut player_guids);

        player_guids
            .into_iter()
            .filter_map(|player_guid| {
                let viewpoint_guid = self.player_viewpoint_guid_like_cpp(player_guid)?;
                Some(DelayedPlayerRelocationContext {
                    player_guid,
                    viewpoint_guid,
                    // Map-owned live relocation currently has no canonical client
                    // object-list source; keep this empty as an explicit visibility
                    // fanout gap rather than inventing session state.
                    previous_client_guids: Vec::new(),
                    relocated_for_ai: viewpoint_guid == player_guid,
                })
            })
            .collect()
    }

    fn delayed_creature_relocation_contexts_from_plan_like_cpp(
        &self,
        delayed_plan: &DelayedUnitRelocationForCellsPlan,
    ) -> Vec<DelayedCreatureRelocationContext> {
        let mut creature_guids: Vec<_> = delayed_plan
            .cell_plans
            .iter()
            .flat_map(|cell_plan| cell_plan.plan.creature_relocations.iter().copied())
            .collect();
        sort_dedup(&mut creature_guids);

        creature_guids
            .into_iter()
            .filter_map(|creature_guid| {
                let creature = self.get_typed_creature(creature_guid)?;
                Some(DelayedCreatureRelocationContext {
                    creature_guid,
                    source_creature_alive: creature.is_alive(),
                })
            })
            .collect()
    }

    pub fn process_map_object_move_list_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MapObjectMoveListEntry>,
    ) -> MapObjectMoveListPlan {
        let mut plan = MapObjectMoveListPlan::default();

        for entry in entries {
            let Some(record) = self.map_object_record(entry.guid) else {
                plan.skipped_other_map_or_missing.push(entry.guid);
                continue;
            };
            if record.kind() != entry.kind {
                plan.skipped_kind_mismatch.push(entry.guid);
                continue;
            }

            if entry.move_state != MapObjectCellMoveState::Active {
                plan.reset_inactive_or_none.push(entry.guid);
                continue;
            }

            if !record.object().object().is_in_world() {
                plan.skipped_not_in_world.push(entry.guid);
                continue;
            }

            match self.relocate_map_object_like_cpp(entry.guid, entry.new_position) {
                Ok(outcome) if outcome.relocated => {
                    plan.relocated.push(entry.guid);
                    continue;
                }
                Ok(outcome) if outcome.blocked_by_unloaded_grid => {}
                Ok(_) => {}
                Err(MapObjectRelocationError::InvalidCoordinates { .. }) => {
                    plan.failed_invalid_position.push(entry.guid);
                    continue;
                }
                Err(MapObjectRelocationError::ObjectNotFound { .. }) => {
                    plan.skipped_other_map_or_missing.push(entry.guid);
                    continue;
                }
                Err(MapObjectRelocationError::Record(_) | MapObjectRelocationError::Store(_)) => {
                    plan.failed_store.push(entry.guid);
                    continue;
                }
            }

            match entry.kind {
                AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
                    if let Some(respawn_position) = entry.respawn_position
                        && self
                            .relocate_map_object_like_cpp(entry.guid, respawn_position)
                            .is_ok_and(|outcome| outcome.relocated)
                    {
                        plan.respawn_relocated.push(entry.guid);
                        continue;
                    }

                    if entry.kind == AccessorObjectKind::Pet || entry.is_pet {
                        plan.pet_removed.push(entry.guid);
                    } else {
                        plan.remove_from_world.push(entry.guid);
                    }
                }
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
                    if let Some(respawn_position) = entry.respawn_position
                        && self
                            .relocate_map_object_like_cpp(entry.guid, respawn_position)
                            .is_ok_and(|outcome| outcome.relocated)
                    {
                        plan.respawn_relocated.push(entry.guid);
                        continue;
                    }

                    plan.remove_from_world.push(entry.guid);
                }
                AccessorObjectKind::DynamicObject | AccessorObjectKind::AreaTrigger => {
                    plan.blocked_unloaded_grid.push(entry.guid);
                }
                AccessorObjectKind::Player
                | AccessorObjectKind::Corpse
                | AccessorObjectKind::SceneObject
                | AccessorObjectKind::Conversation => {
                    plan.unsupported_kind.push(entry.guid);
                }
            }
        }

        plan
    }

    /// C++ `Map::AddCreatureToMoveList` (`Map.cpp:1163-1176`) seam.
    pub fn add_creature_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid, position)
    }

    /// C++ `Map::RemoveCreatureFromMoveList` (`Map.cpp:1178-1187`) seam.
    pub fn remove_creature_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
    }

    /// C++ `Map::AddGameObjectToMoveList` (`Map.cpp:1189-1202`) seam.
    pub fn add_game_object_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid, position)
    }

    /// C++ `Map::RemoveGameObjectFromMoveList` (`Map.cpp:1204-1213`) seam.
    pub fn remove_game_object_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, guid)
    }

    /// C++ `Map::AddDynamicObjectToMoveList` (`Map.cpp:1215-1226`) seam.
    pub fn add_dynamic_object_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(
            MapObjectMoveListFamilyLikeCpp::DynamicObject,
            guid,
            position,
        )
    }

    /// C++ `Map::RemoveDynamicObjectFromMoveList` (`Map.cpp:1228-1237`) seam.
    pub fn remove_dynamic_object_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject, guid)
    }

    /// C++ `Map::AddAreaTriggerToMoveList` (`Map.h:566-579`, `Map.cpp:1163-1237`) seam.
    pub fn add_area_trigger_to_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        self.add_to_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, guid, position)
    }

    /// C++ `Map::RemoveAreaTriggerFromMoveList` (`Map.h:566-579`, `Map.cpp:1163-1237`) seam.
    pub fn remove_area_trigger_from_move_list_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        self.remove_from_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, guid)
    }

    pub fn move_all_creatures_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature)
    }

    pub fn move_all_game_objects_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject)
    }

    pub fn move_all_dynamic_objects_in_move_list_like_cpp(
        &mut self,
    ) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::DynamicObject)
    }

    pub fn move_all_area_triggers_in_move_list_like_cpp(&mut self) -> MoveListDrainSummaryLikeCpp {
        self.drain_move_list_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger)
    }

    /// C++ `Map::UnloadAll` clears only `_creaturesToMove` and
    /// `_gameObjectsToMove` before calling `UnloadGrid(grid, true)`
    /// (`Map.cpp:1646-1651`). It does not drain or relocate any move-list, and
    /// it does not clear AreaTrigger/DynamicObject delayed moves in that branch.
    ///
    /// Rust has no broader `UnloadAll` entry point in this seam yet; callers
    /// modeling that exact C++ pre-loop cleanup may invoke this helper before
    /// repeatedly calling `unload_grid_at(..., true)`.
    pub fn clear_unload_all_delayed_moves_like_cpp(&mut self) {
        self.creatures_to_move.clear();
        self.gameobjects_to_move.clear();
        self.creature_move_states.clear();
        self.gameobject_move_states.clear();
    }

    pub fn pending_cell_move_like_cpp(
        &self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> Option<PendingCellMoveLikeCpp> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.get(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_states.get(&guid),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.get(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_states.get(&guid),
        }
        .copied()
    }

    pub fn move_list_len_like_cpp(&self, family: MapObjectMoveListFamilyLikeCpp) -> usize {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creatures_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobjects_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_objects_to_move.len(),
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_triggers_to_move.len(),
        }
    }

    fn add_to_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
        position: Position,
    ) -> AddObjectToMoveListOutcomeLikeCpp {
        if self.move_list_locked_like_cpp(family) {
            return AddObjectToMoveListOutcomeLikeCpp::LockedIgnored;
        }
        let Some(record) = self.map_object_record(guid) else {
            return AddObjectToMoveListOutcomeLikeCpp::MissingOrStale;
        };
        let actual = record.kind();
        if !move_list_family_accepts_kind_like_cpp(family, actual) {
            return AddObjectToMoveListOutcomeLikeCpp::WrongKind { actual };
        }

        let pending = PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: position,
        };
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => {
                let existed = self.creature_move_states.insert(guid, pending).is_some();
                if !existed {
                    self.creatures_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                let existed = self.gameobject_move_states.insert(guid, pending).is_some();
                if !existed {
                    self.gameobjects_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                let existed = self
                    .dynamic_object_move_states
                    .insert(guid, pending)
                    .is_some();
                if !existed {
                    self.dynamic_objects_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                let existed = self
                    .area_trigger_move_states
                    .insert(guid, pending)
                    .is_some();
                if !existed {
                    self.area_triggers_to_move.push(guid);
                }
                if existed {
                    AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
                } else {
                    AddObjectToMoveListOutcomeLikeCpp::Queued
                }
            }
        }
    }

    fn remove_from_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> RemoveObjectFromMoveListOutcomeLikeCpp {
        if self.move_list_locked_like_cpp(family) {
            return RemoveObjectFromMoveListOutcomeLikeCpp::LockedIgnored;
        }
        let Some(record) = self.map_object_record(guid) else {
            return RemoveObjectFromMoveListOutcomeLikeCpp::MissingOrStale;
        };
        let actual = record.kind();
        if !move_list_family_accepts_kind_like_cpp(family, actual) {
            return RemoveObjectFromMoveListOutcomeLikeCpp::WrongKind { actual };
        }
        let state = match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.get_mut(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                self.gameobject_move_states.get_mut(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.get_mut(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                self.area_trigger_move_states.get_mut(&guid)
            }
        };
        let Some(pending) = state else {
            return RemoveObjectFromMoveListOutcomeLikeCpp::NotQueued;
        };
        if pending.state == MapObjectCellMoveStateLikeCpp::Active {
            pending.state = MapObjectCellMoveStateLikeCpp::Inactive;
            RemoveObjectFromMoveListOutcomeLikeCpp::MarkedInactive
        } else {
            RemoveObjectFromMoveListOutcomeLikeCpp::AlreadyInactive
        }
    }

    fn move_list_locked_like_cpp(&self, family: MapObjectMoveListFamilyLikeCpp) -> bool {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_lock,
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_lock,
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_object_move_lock,
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_lock,
        }
    }

    fn set_move_list_lock_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        locked: bool,
    ) {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::DynamicObject => self.dynamic_object_move_lock = locked,
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => self.area_trigger_move_lock = locked,
        }
    }

    fn take_move_list_queue_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
    ) -> Vec<ObjectGuid> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => std::mem::take(&mut self.creatures_to_move),
            MapObjectMoveListFamilyLikeCpp::GameObject => {
                std::mem::take(&mut self.gameobjects_to_move)
            }
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                std::mem::take(&mut self.dynamic_objects_to_move)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                std::mem::take(&mut self.area_triggers_to_move)
            }
        }
    }

    fn remove_pending_move_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
        guid: ObjectGuid,
    ) -> Option<PendingCellMoveLikeCpp> {
        match family {
            MapObjectMoveListFamilyLikeCpp::Creature => self.creature_move_states.remove(&guid),
            MapObjectMoveListFamilyLikeCpp::GameObject => self.gameobject_move_states.remove(&guid),
            MapObjectMoveListFamilyLikeCpp::DynamicObject => {
                self.dynamic_object_move_states.remove(&guid)
            }
            MapObjectMoveListFamilyLikeCpp::AreaTrigger => {
                self.area_trigger_move_states.remove(&guid)
            }
        }
    }

    fn drain_move_list_like_cpp(
        &mut self,
        family: MapObjectMoveListFamilyLikeCpp,
    ) -> MoveListDrainSummaryLikeCpp {
        let mut summary = MoveListDrainSummaryLikeCpp {
            family: Some(family),
            ..Default::default()
        };
        if self.move_list_locked_like_cpp(family) {
            summary.locked_ignored = 1;
            return summary;
        }

        self.set_move_list_lock_like_cpp(family, true);
        let queued = self.take_move_list_queue_like_cpp(family);
        for guid in queued {
            summary.processed += 1;
            let Some(pending) = self.remove_pending_move_like_cpp(family, guid) else {
                summary.inactive_reset += 1;
                continue;
            };
            if pending.state != MapObjectCellMoveStateLikeCpp::Active {
                summary.inactive_reset += 1;
                continue;
            }

            let Some(record) = self.map_object_record(guid) else {
                summary.missing_or_stale += 1;
                continue;
            };
            let actual = record.kind();
            if !move_list_family_accepts_kind_like_cpp(family, actual) {
                summary.wrong_kind += 1;
                continue;
            }
            if !record.object().object().is_in_world() {
                summary.not_in_world += 1;
                continue;
            }

            match self.relocate_map_object_like_cpp(guid, pending.new_position) {
                Ok(outcome) if outcome.relocated => summary.relocated += 1,
                Ok(outcome) if outcome.blocked_by_unloaded_grid => {
                    summary.blocked_by_unloaded_grid += 1;
                    if matches!(
                        family,
                        MapObjectMoveListFamilyLikeCpp::Creature
                            | MapObjectMoveListFamilyLikeCpp::GameObject
                    ) {
                        summary.respawn_relocation_unsupported += 1;
                    }
                }
                Ok(_) => summary.blocked_by_unloaded_grid += 1,
                Err(MapObjectRelocationError::InvalidCoordinates { .. }) => {
                    summary.failed_invalid_position += 1;
                }
                Err(MapObjectRelocationError::ObjectNotFound { .. }) => {
                    summary.missing_or_stale += 1;
                }
                Err(MapObjectRelocationError::Record(_) | MapObjectRelocationError::Store(_)) => {
                    summary.failed_store += 1;
                }
            }
        }
        self.set_move_list_lock_like_cpp(family, false);
        summary
    }

    pub fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
        self.map_objects.get(&guid)
    }

    /// Count exact typed in-world Creature/GameObject candidates for represented
    /// C++ `GameEventMgr::RunSmartAIScripts` evidence.
    ///
    /// This intentionally reads only canonical `Map::map_objects`. Generic
    /// fallback records are ignored because C++ uses typed object stores. Transport
    /// records are also ignored even though they can expose a GameObject view; the
    /// C++ hook worker's switch has no transport branch in this slice.
    pub fn game_event_smart_ai_script_candidates_like_cpp(
        &self,
    ) -> GameEventSmartAiScriptCandidateSummaryLikeCpp {
        let mut summary = GameEventSmartAiScriptCandidateSummaryLikeCpp {
            maps_visited: 1,
            ..GameEventSmartAiScriptCandidateSummaryLikeCpp::default()
        };

        for record in self.map_objects.values() {
            match record.kind() {
                AccessorObjectKind::Creature => {
                    if record
                        .creature()
                        .is_some_and(|creature| creature.unit().world().object().is_in_world())
                    {
                        summary.in_world_creature_candidates += 1;
                        summary.creature_ai_enabled_unrepresented += 1;
                        summary.script_dispatch_unrepresented += 1;
                    }
                }
                AccessorObjectKind::GameObject => {
                    if record
                        .game_object()
                        .is_some_and(|game_object| game_object.world().object().is_in_world())
                    {
                        summary.in_world_gameobject_candidates += 1;
                        summary.script_dispatch_unrepresented += 1;
                    }
                }
                _ => {}
            }
        }

        summary
    }

    /// Represented tail metrics from C++ `Map::Update` after
    /// `sScriptMgr->OnMapUpdate(this, t_diff)` (`Map.cpp:804-815`).
    ///
    /// C++ emits `TC_METRIC_VALUE("map_creatures", GetObjectsStore().Size<Creature>())`
    /// and `TC_METRIC_VALUE("map_gameobjects", GetObjectsStore().Size<GameObject>())`.
    /// Rust reads only canonical typed `MapObjectRecord`s from `map_objects`: a
    /// record must have both the exact canonical kind and the corresponding typed
    /// body. Generic `WorldObject` records, Pet, Transport, DynamicObject,
    /// AreaTrigger, Player, etc. are intentionally excluded; no telemetry backend
    /// is invoked here.
    pub fn map_update_metrics_like_cpp(&self) -> MapUpdateMetricsSummaryLikeCpp {
        let mut summary = MapUpdateMetricsSummaryLikeCpp {
            map_id: self.map_id,
            instance_id: self.instance_id,
            ..MapUpdateMetricsSummaryLikeCpp::default()
        };

        for record in self.map_objects.values() {
            match record.kind() {
                AccessorObjectKind::Creature if record.creature().is_some() => {
                    summary.creature_count += 1;
                }
                AccessorObjectKind::GameObject if record.game_object().is_some() => {
                    summary.gameobject_count += 1;
                }
                _ => {}
            }
        }

        summary
    }

    pub fn map_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_record(guid).map(MapObjectRecord::object)
    }

    fn object_is_in_world(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid)
            .is_some_and(|object| object.object().is_in_world())
    }

    fn object_needs_notify_visibility(&self, guid: ObjectGuid) -> bool {
        self.map_object(guid).is_some_and(|object| {
            object
                .object()
                .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
        })
    }

    fn exact_cell_guids_like_cpp(&self, cell_coord: CellCoord) -> NearbyCellGuids {
        let mut nearby = NearbyCellGuids::default();
        let cell = Cell::from_cell_coord(cell_coord);
        let Some(grid) = self.get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y())) else {
            return nearby;
        };
        let Some(local_cell) = grid.get_grid_type(cell.cell_x(), cell.cell_y()) else {
            return nearby;
        };

        nearby.visited_cells = 1;
        nearby.merge_world(&local_cell.world_objects);
        nearby.merge_grid(&local_cell.grid_objects);
        nearby
    }

    pub fn map_object_by_kind(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<&WorldObject> {
        let record = self.map_object_record(guid)?;
        allowed.contains(&record.kind()).then_some(record.object())
    }

    pub fn get_creature(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Creature])
    }

    pub fn get_typed_creature(&self, guid: ObjectGuid) -> Option<&Creature> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature()
    }

    pub fn get_typed_creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        let record = self.map_objects.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Creature {
            return None;
        }
        record.creature_mut()
    }

    pub fn get_pet(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Pet])
    }

    pub fn get_typed_pet(&self, guid: ObjectGuid) -> Option<&Pet> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet()
    }

    pub fn get_typed_pet_mut(&mut self, guid: ObjectGuid) -> Option<&mut Pet> {
        let record = self.map_objects.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Pet {
            return None;
        }
        record.pet_mut()
    }

    pub fn get_game_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        )
    }

    pub fn get_typed_game_object(&self, guid: ObjectGuid) -> Option<&GameObject> {
        let record = self.map_object_record(guid)?;
        if !matches!(
            record.kind(),
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport
        ) {
            return None;
        }
        record.game_object()
    }

    pub fn get_typed_game_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut GameObject> {
        let record = self.map_objects.get_mut(&guid)?;
        if !matches!(
            record.kind(),
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport
        ) {
            return None;
        }
        record.game_object_mut()
    }

    pub fn get_typed_player(&self, guid: ObjectGuid) -> Option<&Player> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player()
    }

    pub fn typed_player_counts_like_cpp(&self) -> (u32, u32) {
        let mut total = 0u32;
        let mut non_game_masters = 0u32;
        for record in self.map_objects.values() {
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

    pub fn get_typed_player_mut(&mut self, guid: ObjectGuid) -> Option<&mut Player> {
        let record = self.map_objects.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Player {
            return None;
        }
        record.player_mut()
    }

    pub fn get_typed_corpse(&self, guid: ObjectGuid) -> Option<&Corpse> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse()
    }

    pub fn get_typed_corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        let record = self.map_objects.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::Corpse {
            return None;
        }
        record.corpse_mut()
    }

    pub fn get_typed_dynamic_object(&self, guid: ObjectGuid) -> Option<&DynamicObject> {
        let record = self.map_object_record(guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object()
    }

    pub fn get_typed_dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        let record = self.map_objects.get_mut(&guid)?;
        if record.kind() != AccessorObjectKind::DynamicObject {
            return None;
        }
        record.dynamic_object_mut()
    }

    fn combat_unit_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<CombatUnitSnapshotLikeCpp<'_>> {
        if let Some(player) = self.get_typed_player(guid) {
            return Some(CombatUnitSnapshotLikeCpp {
                guid,
                unit: player.unit(),
                game_master_player: player.is_game_master_like_cpp(),
            });
        }
        self.get_typed_creature(guid)
            .map(|creature| CombatUnitSnapshotLikeCpp {
                guid,
                unit: creature.unit(),
                game_master_player: false,
            })
    }

    fn combat_begin_context_like_cpp(
        &self,
        owner: CombatUnitSnapshotLikeCpp<'_>,
        target: CombatUnitSnapshotLikeCpp<'_>,
    ) -> CombatBeginContextLikeCpp {
        let owner_world = owner.unit.world();
        let target_world = target.unit.world();
        CombatBeginContextLikeCpp {
            same_unit: owner.guid == target.guid,
            attacker_in_world: owner_world.object().is_in_world(),
            victim_in_world: target_world.object().is_in_world(),
            attacker_alive: owner.unit.is_alive(),
            victim_alive: target.unit.is_alive(),
            same_map: owner_world.is_in_map(target_world),
            same_phase: owner_world.in_same_phase(target_world),
            attacker_unit_state: owner.unit.unit_state(),
            victim_unit_state: target.unit.unit_state(),
            attacker_combat_disallowed: owner.unit.subsystems().combat.combat_disallowed,
            victim_combat_disallowed: target.unit.subsystems().combat.combat_disallowed,
            relation_represented: false,
            attacker_is_friendly_to_victim: false,
            victim_is_friendly_to_attacker: false,
            attacker_or_owner_player_is_game_master: owner.game_master_player,
            victim_or_owner_player_is_game_master: target.game_master_player,
        }
    }

    pub fn typed_combat_unit_guids_like_cpp(&self) -> Vec<ObjectGuid> {
        self.map_objects
            .iter()
            .filter_map(|(guid, record)| {
                matches!(
                    record.kind(),
                    AccessorObjectKind::Player | AccessorObjectKind::Creature
                )
                .then_some(*guid)
            })
            .collect()
    }

    pub fn revalidate_all_combat_refs_like_cpp(&mut self) -> Vec<(ObjectGuid, ObjectGuid)> {
        let owner_guids = self.typed_combat_unit_guids_like_cpp();
        let mut invalid = Vec::new();

        for owner_guid in owner_guids {
            let Some(owner) = self.combat_unit_snapshot_like_cpp(owner_guid) else {
                continue;
            };
            let refs: Vec<_> = owner
                .unit
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .chain(owner.unit.subsystems().combat.pvp_refs.keys())
                .copied()
                .collect();

            for target_guid in refs {
                let Some(target) = self.combat_unit_snapshot_like_cpp(target_guid) else {
                    invalid.push((owner_guid, target_guid));
                    continue;
                };
                if !CombatSubsystem::can_begin_combat_like_cpp(
                    self.combat_begin_context_like_cpp(owner, target),
                ) {
                    invalid.push((owner_guid, target_guid));
                }
            }
        }

        for (owner_guid, target_guid) in &invalid {
            if let Some(owner) = self.get_typed_player_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            } else if let Some(owner) = self.get_typed_creature_mut(*owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*target_guid);
            }

            if let Some(target) = self.get_typed_player_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            } else if let Some(target) = self.get_typed_creature_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            }
        }

        invalid
    }

    /// Tick C++ timed PvP combat references for every canonical combat unit
    /// and remove the reciprocal reference when one side expires.
    pub fn update_all_pvp_combat_refs_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Vec<(ObjectGuid, ObjectGuid)> {
        let owner_guids = self.typed_combat_unit_guids_like_cpp();
        let mut expired = Vec::new();

        for owner_guid in owner_guids {
            let targets = if let Some(owner) = self.get_typed_player_mut(owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .update_pvp_combat(diff_ms)
            } else if let Some(owner) = self.get_typed_creature_mut(owner_guid) {
                owner
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .update_pvp_combat(diff_ms)
            } else {
                Vec::new()
            };
            expired.extend(
                targets
                    .into_iter()
                    .map(|target_guid| (owner_guid, target_guid)),
            );
        }

        for (owner_guid, target_guid) in &expired {
            if let Some(target) = self.get_typed_player_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            } else if let Some(target) = self.get_typed_creature_mut(*target_guid) {
                target
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(*owner_guid);
            }
        }

        expired
    }

    pub fn get_transport(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Transport])
    }

    /// Return the typed canonical transport that currently owns a passenger.
    ///
    /// C++ `WorldObject::GetTransGUID` is backed by the object's transport
    /// movement state. The canonical Rust transport runtime owns passenger
    /// membership on `Transport`, so spell destination resolution uses this
    /// map-local lookup instead of guessing from a generic transport object.
    pub fn get_typed_transport_for_passenger_like_cpp(
        &self,
        passenger_guid: ObjectGuid,
    ) -> Option<&wow_entities::Transport> {
        self.map_objects.values().find_map(|record| {
            let transport = record.transport()?;
            (transport.passengers().contains(&passenger_guid)
                || transport.static_passengers().contains(&passenger_guid))
            .then_some(transport)
        })
    }

    pub fn get_typed_transport_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Transport> {
        self.map_objects.get(&guid)?.transport()
    }

    pub fn get_dynamic_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::DynamicObject])
    }

    pub fn get_area_trigger(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::AreaTrigger])
    }

    pub fn get_corpse(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Corpse])
    }

    pub fn get_scene_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::SceneObject])
    }

    pub fn get_conversation(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Conversation])
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

    pub fn mark_active_cell(&mut self, cell: CellCoord) {
        assert!(cell.is_coord_valid());
        self.active_cells.insert(cell);
    }

    pub fn unmark_active_cell(&mut self, cell: CellCoord) {
        self.active_cells.remove(&cell);
    }

    pub fn get_ngrid(&self, coord: GridCoord) -> Option<&NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref()
    }

    pub fn get_ngrid_mut(&mut self, coord: GridCoord) -> Option<&mut NGrid> {
        let index = grid_index(coord)?;
        self.grids[index].as_deref_mut()
    }

    pub fn set_ngrid(&mut self, coord: GridCoord, grid: Option<NGrid>) {
        let index = checked_grid_index(coord);
        self.grids[index] = grid.map(Box::new);
    }

    pub fn is_grid_loaded(&self, coord: GridCoord) -> bool {
        self.get_ngrid(coord)
            .is_some_and(NGrid::grid_object_data_loaded)
    }

    pub fn loaded_grid_coords_like_cpp(&self) -> Vec<GridCoord> {
        self.grids
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.as_ref()
                    .filter(|grid| grid.grid_object_data_loaded())
                    .map(|_| {
                        GridCoord::new(
                            (index as u32) / MAX_NUMBER_OF_GRIDS,
                            (index as u32) % MAX_NUMBER_OF_GRIDS,
                        )
                    })
            })
            .collect()
    }

    pub fn ensure_grid_created(&mut self, coord: GridCoord) -> bool {
        let index = checked_grid_index(coord);
        if self.grids[index].is_some() {
            return false;
        }

        let mut grid = NGrid::from_coords(
            coord.x_coord as i32,
            coord.y_coord as i32,
            self.grid_expiry_ms,
            self.grid_unload,
        );
        grid.set_state(GridStateKind::Idle);
        self.grids[index] = Some(Box::new(grid));

        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.load_map_and_vmap(terrain_x, terrain_y);
        true
    }

    pub fn ensure_grid_loaded(&mut self, cell: &Cell) -> bool {
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.ensure_grid_created(coord);
        let index = checked_grid_index(coord);
        {
            let grid = self.grids[index].as_mut().expect("grid was just created");
            if grid.grid_object_data_loaded() {
                return false;
            }

            grid.set_grid_object_data_loaded(true);
            self.lifecycle.load_grid_objects(grid, cell);
        }
        self.activate_registered_corpses_for_grid_like_cpp(coord);
        true
    }

    pub fn load_loaded_grid_area_trigger_records_like_cpp<L>(
        &mut self,
        coord: GridCoord,
        spawn_store: &SpawnStore,
        mut load_record: L,
    ) -> LoadedGridAreaTriggerRecordsSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(grid) = self.get_ngrid(coord) else {
            return LoadedGridAreaTriggerRecordsSummaryLikeCpp {
                grid_not_loaded: true,
                ..Default::default()
            };
        };
        if !grid.grid_object_data_loaded() {
            return LoadedGridAreaTriggerRecordsSummaryLikeCpp {
                grid_not_loaded: true,
                ..Default::default()
            };
        }

        let mut spawn_ids = Vec::new();
        for x in 0..MAX_NUMBER_OF_CELLS {
            for y in 0..MAX_NUMBER_OF_CELLS {
                let Some(cell) = grid.get_grid_type(x, y) else {
                    continue;
                };
                if let Some(cell_guids) = spawn_store.cell_object_guids(
                    self.map_id,
                    self.spawn_mode,
                    cell.cell_coord().get_id(),
                ) {
                    spawn_ids.extend(cell_guids.area_triggers.iter().copied());
                }
            }
        }

        let spawn_filter = self.spawn_grid_load_state_like_cpp(spawn_store);
        let mut plans = Vec::new();
        let mut summary = LoadedGridAreaTriggerRecordsSummaryLikeCpp::default();
        for spawn_id in spawn_ids {
            if self
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .is_some()
            {
                summary.skipped_already_loaded += 1;
                continue;
            }
            if !spawn_filter.should_be_spawned_on_grid_load(SpawnObjectType::AreaTrigger, spawn_id)
            {
                summary.skipped_should_not_spawn += 1;
                continue;
            }
            let Some(spawn_data) = spawn_store.spawn_data(SpawnObjectType::AreaTrigger, spawn_id)
            else {
                summary.stale_index_entries += 1;
                continue;
            };
            if spawn_data.map_id != self.map_id {
                summary.stale_index_entries += 1;
                continue;
            }
            if !spawn_data.spawn_difficulties.contains(&self.spawn_mode) {
                summary.skipped_difficulty_mismatch += 1;
                continue;
            }
            summary.metadata_entries += 1;
            plans.push(spawn_id);
        }
        drop(spawn_filter);

        for spawn_id in plans {
            let Some(records) = load_record(self, SpawnObjectType::AreaTrigger, spawn_id) else {
                summary.load_record_missing += 1;
                continue;
            };
            for pre_add_record in records.pre_add_records {
                if self
                    .add_map_object_record_to_map_like_cpp(pre_add_record)
                    .is_ok()
                {
                    summary.pre_add_records_added += 1;
                } else {
                    summary.add_to_map_errors += 1;
                }
            }
            let primary_record = records.primary_record;
            let loaded_grid_primary_record = primary_record.clone();
            match self.add_map_object_record_to_map_like_cpp(primary_record) {
                Ok(_outcome) => summary
                    .loaded_grid_primary_records
                    .push(loaded_grid_primary_record),
                Err(_error) => summary.add_to_map_errors += 1,
            }
        }

        summary
    }

    pub fn ensure_grid_loaded_for_active_object(
        &mut self,
        cell: &Cell,
        kind: ActiveObjectKind,
    ) -> bool {
        let loaded_now = self.ensure_grid_loaded(cell);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.mark_active_cell(cell.cell_coord());

        if matches!(kind, ActiveObjectKind::Player) {
            // Use `ensure_grid_loaded_for_player_phase` when phase-shift state
            // is available; this entry point only has the object kind.
        }

        let active_expiry_ms = (self.grid_expiry_ms as f32 * 0.1) as i64;
        let grid = self.get_ngrid_mut(coord).expect("grid was just loaded");
        if grid.state() != GridStateKind::Active {
            grid.info_mut().reset_time_tracker(active_expiry_ms);
            grid.set_state(GridStateKind::Active);
        }

        loaded_now
    }

    pub fn ensure_grid_loaded_for_player_phase<Filter>(
        &mut self,
        cell: &Cell,
        phase_shift: &PhaseShift,
        loader: &mut ObjectGridLoader<'_, Filter>,
    ) -> bool
    where
        Filter: GridSpawnLoadFilter,
    {
        let loaded_now = self.ensure_grid_loaded(cell);
        let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
        self.mark_active_cell(cell.cell_coord());

        let active_expiry_ms = (self.grid_expiry_ms as f32 * 0.1) as i64;
        let index = checked_grid_index(coord);
        let grid = self.grids[index].as_mut().expect("grid was just loaded");
        self.personal_phase_tracker
            .load_grid(phase_shift, grid, loader);

        if grid.state() != GridStateKind::Active {
            grid.info_mut().reset_time_tracker(active_expiry_ms);
            grid.set_state(GridStateKind::Active);
        }

        loaded_now
    }

    pub fn load_grid(&mut self, x: f32, y: f32) -> bool {
        self.ensure_grid_loaded(&Cell::from_world(x, y))
    }

    pub fn load_grid_for_active_object(&mut self, x: f32, y: f32, kind: ActiveObjectKind) -> bool {
        self.ensure_grid_loaded_for_active_object(&Cell::from_world(x, y), kind)
    }

    pub fn reset_grid_expiry(&self, grid: &mut NGrid, factor: f32) {
        grid.info_mut()
            .reset_time_tracker((self.grid_expiry_ms as f32 * factor) as i64);
    }

    pub fn active_objects_near_grid(&self, grid: &NGrid) -> bool {
        if active_cells_near_grid(&self.active_cells, self.visible_distance, grid) {
            return true;
        }

        let active_non_player_cells: HashSet<_> = self
            .active_non_players_like_cpp
            .iter()
            .filter_map(|guid| {
                let record = self.map_object_record(*guid)?;
                record.object().object().is_in_world().then(|| {
                    compute_cell_coord(record.object().position().x, record.object().position().y)
                })
            })
            .collect();
        active_cells_near_grid(&active_non_player_cells, self.visible_distance, grid)
    }

    pub fn unload_grid_at(&mut self, coord: GridCoord, unload_all: bool) -> bool {
        let index = checked_grid_index(coord);
        let Some(mut grid) = self.grids[index].take() else {
            return false;
        };

        if !self.can_unload_grid(&grid, unload_all) {
            self.grids[index] = Some(grid);
            return false;
        }

        self.run_unload_lifecycle(&mut grid, unload_all);
        true
    }

    pub fn update_loaded_grid_states_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> GridStatesUpdateSummaryLikeCpp {
        // C++ `Map::DelayedUpdate` increments the GridRefManager iterator before
        // invoking the grid-state update because that update may unload/delete
        // the current grid (`Map.cpp:2536-2542`). Rust snapshots loaded grid
        // coordinates first and then re-checks each slot, never recreating a
        // grid that disappeared earlier in the same delayed-update pass.
        let loaded_grid_coords: Vec<GridCoord> = self
            .grids
            .iter()
            .enumerate()
            .filter_map(|(index, grid)| {
                grid.as_ref().map(|_| {
                    GridCoord::new(
                        (index as u32) / MAX_NUMBER_OF_GRIDS,
                        (index as u32) % MAX_NUMBER_OF_GRIDS,
                    )
                })
            })
            .collect();

        let mut summary = GridStatesUpdateSummaryLikeCpp {
            diff_ms,
            visited: loaded_grid_coords.len(),
            ..GridStatesUpdateSummaryLikeCpp::default()
        };

        for coord in loaded_grid_coords {
            let Some(previous_state) = self.get_ngrid(coord).map(NGrid::state) else {
                summary.missing_after_snapshot += 1;
                continue;
            };

            if matches!(previous_state, GridStateKind::Invalid) {
                summary.skipped_invalid += 1;
            }

            let unloaded = self.update_grid_state_at(coord, diff_ms);
            summary.updated += 1;

            if unloaded {
                summary.unloaded += 1;
                if matches!(previous_state, GridStateKind::Removal) {
                    summary.removal_unloaded += 1;
                }
                continue;
            }

            let Some(next_state) = self.get_ngrid(coord).map(NGrid::state) else {
                summary.missing_after_snapshot += 1;
                continue;
            };

            match (previous_state, next_state) {
                (GridStateKind::Active, GridStateKind::Idle) => summary.active_to_idle += 1,
                (GridStateKind::Idle, GridStateKind::Removal) => summary.idle_to_removal += 1,
                (GridStateKind::Removal, GridStateKind::Removal) => {
                    summary.removal_deferred_or_reset += 1;
                }
                _ => {}
            }
        }

        summary
    }

    pub fn update_grid_state_at(&mut self, coord: GridCoord, diff_ms: u32) -> bool {
        let index = checked_grid_index(coord);
        let Some(mut grid) = self.grids[index].take() else {
            return false;
        };

        self.grid_state_unloaded = false;
        update_grid_state(self, &mut grid, diff_ms);
        if self.grid_state_unloaded {
            self.grid_state_unloaded = false;
            true
        } else {
            self.grids[index] = Some(grid);
            false
        }
    }

    fn can_unload_grid(&self, grid: &NGrid, unload_all: bool) -> bool {
        unload_all
            || (grid.world_creature_count_in_ngrid() == 0 && !self.active_objects_near_grid(grid))
    }

    fn run_unload_lifecycle(&mut self, grid: &mut NGrid, unload_all: bool) {
        // C++ `Map::UnloadGrid` drains Creature/GameObject/AreaTrigger move lists
        // only in the `!unloadAll` branch, before and after the evacuator
        // (`Map.cpp:1579-1596`). `UnloadGrid(..., true)` does not drain or
        // relocate move-lists; `Map::UnloadAll` only clears Creature/GameObject
        // delayed moves before entering that loop (`Map.cpp:1646-1651`). Rust
        // still keeps the rest of this unload lifecycle represented: no
        // DynamicObject drain in this path, no full visibility/fanout/scripts/DB.
        if !unload_all {
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
            self.lifecycle.evacuate_grid(grid);
            self.drain_grid_unload_actions_like_cpp();
            self.move_all_creatures_in_move_list_like_cpp();
            self.move_all_game_objects_in_move_list_like_cpp();
            self.move_all_area_triggers_in_move_list_like_cpp();
        }

        self.lifecycle.clean_grid(grid);
        self.drain_grid_unload_actions_like_cpp();
        self.personal_phase_tracker.unload_grid(grid);
        self.lifecycle.unload_grid_objects(grid);
        self.drain_grid_unload_actions_like_cpp();

        let coord = GridCoord::new(grid.x() as u32, grid.y() as u32);
        self.deactivate_registered_corpses_for_grid_like_cpp(coord);
        let (terrain_x, terrain_y) = terrain_grid_coords(coord);
        self.terrain.unload_map(terrain_x, terrain_y);
    }

    fn drain_grid_unload_actions_like_cpp(&mut self) -> Vec<GridUnloadApplyOutcome> {
        let actions = self.lifecycle.take_unload_actions_like_cpp();
        if actions.is_empty() {
            return Vec::new();
        }

        apply_grid_unload_actions(self, actions)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectStoreError {
    InvalidRecord(ObjectAccessorError),
    WrongMap {
        guid: ObjectGuid,
        expected_map_id: u32,
        expected_instance_id: u32,
        actual_map_id: u32,
        actual_instance_id: u32,
    },
}

impl From<ObjectAccessorError> for MapObjectStoreError {
    fn from(error: ObjectAccessorError) -> Self {
        Self::InvalidRecord(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectCollisionEnableOutcomeLikeCpp {
    pub requested_enable: bool,
    pub represented_model_present: bool,
    pub previous_collision_enabled: Option<bool>,
    pub new_collision_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureZoneScriptCreateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback: bool,
    pub script_dispatch_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectZoneScriptCreateOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback_boundary: bool,
    pub script_dispatch_represented: bool,
    pub object_store_present_before_callback: bool,
    pub spawn_index_present_before_callback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectZoneScriptRemoveOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback_boundary: bool,
    pub script_dispatch_represented: bool,
    pub model_remove_pending_before_callback: bool,
    pub spawn_index_present_before_callback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectAddToOwnerOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub owner_guid: ObjectGuid,
    pub owner_found_as_unit_like: bool,
    pub gameobject_found: bool,
    pub owner_guid_before: ObjectGuid,
    pub owner_guid_after: ObjectGuid,
    pub gameobject_owner_empty_before: bool,
    pub registered_owned_gameobject: bool,
    pub owner_guid_set: bool,
    pub cooldown_start_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectAddToOwnerSlotOutcomeLikeCpp {
    pub add_owner: GameObjectAddToOwnerOutcomeLikeCpp,
    pub slot: usize,
    pub slot_previous_guid: ObjectGuid,
    pub slot_set: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub slot: usize,
    pub spell_id: u32,
    pub owner_found_as_unit_like: bool,
    pub slot_guid_before: ObjectGuid,
    pub slot_had_guid: bool,
    pub gameobject_found: bool,
    pub recast_spell_id_cleared: bool,
    pub unit_pointer_owner_match: bool,
    pub remove_from_owner: Option<GameObjectRemoveFromOwnerOutcomeLikeCpp>,
    pub respawn_time_cleared: bool,
    pub delete_outcome: Option<GameObjectDeleteOutcomeLikeCpp>,
    pub slot_cleared: bool,
    pub cooldown_event_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameObjectSummonObjectForOwnerSlotStatusLikeCpp {
    MissingOwner,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapOrOwnerFailed,
    CreatedAddedAndSlotted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub slot: usize,
    pub spell_id: u32,
    pub template_entry: u32,
    pub status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub add_owner_slot: Option<GameObjectAddToOwnerSlotOutcomeLikeCpp>,
    pub respawn_time_secs: Option<i32>,
    pub caster_faction: Option<u32>,
    pub caster_level: Option<u32>,
    pub phase_inherit_represented: bool,
    pub execute_log_represented: bool,
    pub cooldown_event_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObjectSummonGameObjectStatusLikeCpp {
    MissingSummoner,
    SummonerNotInWorld,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapFailed,
    CreatedAddedToMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldObjectSummonGameObjectOutcomeLikeCpp {
    pub summoner_guid: ObjectGuid,
    pub template_entry: u32,
    pub summon_type: GameObjectSummonTypeLikeCpp,
    pub status: WorldObjectSummonGameObjectStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub add_owner: Option<GameObjectAddToOwnerOutcomeLikeCpp>,
    pub respawn_time_secs: i64,
    pub phase_inherit_represented: bool,
    pub spawned_by_default_forced_false: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
    pub position: Position,
    pub close_point_fallback_used: bool,
    pub normalized_map_coords: bool,
    pub collision_los_adjustment_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffectSummonObjectWildStatusLikeCpp {
    MissingCaster,
    LowGuidUnavailable,
    CreateFailed,
    AddToMapFailed,
    CreatedAddedToMap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellEffectSummonObjectWildOutcomeLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub template_entry: u32,
    pub status: SpellEffectSummonObjectWildStatusLikeCpp,
    pub guid: Option<ObjectGuid>,
    pub low_guid: Option<i64>,
    pub create_error: Option<GameObjectLifecycleError>,
    pub add_to_map: Option<AddToMapOutcome>,
    pub respawn_time_secs: Option<i32>,
    pub phase_inherit_represented: bool,
    pub execute_log_represented: bool,
    pub owner_linked: bool,
    pub flagdrop_type: bool,
    pub flagdrop_player_branch_reached: bool,
    pub flagdrop_battleground_update_represented: bool,
    pub linked_trap_guid: Option<ObjectGuid>,
    pub linked_trap_side_effect_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitRemoveGameObjectsBySpellOutcomeLikeCpp {
    pub owner_guid: ObjectGuid,
    pub spell_id: u32,
    pub delete_requested: bool,
    pub owner_found_as_unit_like: bool,
    pub owned_entries_before: usize,
    pub matched_entries: usize,
    pub owner_guid_cleared: usize,
    pub respawn_time_cleared: usize,
    pub owner_list_entries_removed: usize,
    pub delete_outcomes: usize,
    pub object_slot_cleanup_represented: bool,
    pub aura_cleanup_represented: bool,
    pub cooldown_event_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectRemoveFromOwnerOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub owner_guid_before: ObjectGuid,
    pub owner_guid_after: ObjectGuid,
    pub owner_found_as_unit_like: bool,
    pub cleared_owner: bool,
    pub spell_id: u32,
    pub unit_side_effects_represented: bool,
    pub unit_owned_gameobject_list_removed: bool,
    pub unit_object_slot_cleared: bool,
    pub aura_cleanup_represented: bool,
    pub aura_cleanup_removed_count: usize,
    pub cooldown_event_represented: bool,
    pub creature_ai_callback_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectRemoveLinkedTrapOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub linked_trap_guid: Option<ObjectGuid>,
    pub owner_present_before_linked_trap_remove: bool,
    pub linked_trap_removed: bool,
    pub linked_trap_remove_queued: bool,
    pub linked_trap_missing_or_self: bool,
    pub linked_trap_cycle_guarded: bool,
    pub despawn_or_unsummon_scheduler_represented: bool,
    pub object_accessor_fanout_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureZoneScriptRemoveOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub represented_callback: bool,
    pub script_dispatch_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ActiveNonPlayerRespawnLocationLikeCpp {
    spawn_id: SpawnId,
    position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveNonPlayerMutationStatusLikeCpp {
    Mutated,
    MissingRecord,
    PlayerUnsupported,
    NotActiveObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveNonPlayerUnloadLockOutcomeLikeCpp {
    pub spawn_id: SpawnId,
    pub respawn_grid: Option<GridCoord>,
    pub respawn_grid_missing: bool,
    pub invalid_respawn_position: bool,
    pub lock_incremented: bool,
    pub lock_decremented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveNonPlayerMutationOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub status: ActiveNonPlayerMutationStatusLikeCpp,
    pub inserted_in_active_set: bool,
    pub removed_from_active_set: bool,
    pub spawn_id_zero_or_unsupported: bool,
    pub unload_lock: Option<ActiveNonPlayerUnloadLockOutcomeLikeCpp>,
}

pub type AddToActiveOutcomeLikeCpp = ActiveNonPlayerMutationOutcomeLikeCpp;
pub type RemoveFromActiveOutcomeLikeCpp = ActiveNonPlayerMutationOutcomeLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddToMapPostAddToWorldOutcomeLikeCpp {
    pub initialize_object_represented: bool,
    pub pending_move_state_cleared: bool,
    pub no_pending_move_state: bool,
    pub add_to_active_represented: bool,
    pub add_to_active_skipped_runtime_gap: bool,
    pub add_to_active: Option<AddToActiveOutcomeLikeCpp>,
    pub set_is_new_object_true: bool,
    pub update_object_visibility_on_create_represented: bool,
    pub update_object_visibility_on_create_runtime_gap: bool,
    pub set_is_new_object_false: bool,
    pub final_is_new_object: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddToMapOutcome {
    pub guid: ObjectGuid,
    pub cell: CellCoord,
    pub grid: GridCoord,
    pub inserted: bool,
    pub already_in_world: bool,
    pub grid_created: bool,
    pub grid_loaded: bool,
    pub inserted_into_cell: bool,
    pub gameobject_model_insert: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub gameobject_collision_enable: Option<GameObjectCollisionEnableOutcomeLikeCpp>,
    pub gameobject_zone_script_create: Option<GameObjectZoneScriptCreateOutcomeLikeCpp>,
    pub gameobject_store_inserted_before_add_to_world: Option<bool>,
    pub gameobject_spawn_indexed_before_add_to_world: Option<bool>,
    pub creature_store_inserted_before_add_to_world: Option<bool>,
    pub creature_spawn_indexed_before_add_to_world: Option<bool>,
    pub creature_unit_add_to_world: Option<UnitAddToWorldOutcomeLikeCpp>,
    pub creature_search_formation: Option<CreatureSearchFormationOutcomeLikeCpp>,
    pub creature_aim_initialize: Option<CreatureAimInitializeOutcomeLikeCpp>,
    pub creature_vehicle_reset: Option<VehicleKitAddToWorldResetOutcomeLikeCpp>,
    pub creature_vehicle_install: Option<VehicleKitInstallOutcomeLikeCpp>,
    pub creature_zone_script_create: Option<CreatureZoneScriptCreateOutcomeLikeCpp>,
    pub add_to_map_tail: Option<AddToMapPostAddToWorldOutcomeLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddToMapError {
    InvalidCoordinates { guid: ObjectGuid, x: f32, y: f32 },
    Store(MapObjectStoreError),
}

impl From<MapObjectStoreError> for AddToMapError {
    fn from(error: MapObjectStoreError) -> Self {
        Self::Store(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub cxx_in_world: bool,
    pub update_object_visibility_on_destroy_represented: bool,
    pub update_object_visibility_on_destroy_runtime_gap: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveFromMapOutcome {
    pub guid: ObjectGuid,
    pub cell: CellCoord,
    pub grid: GridCoord,
    pub was_in_world: bool,
    pub cxx_in_world: bool,
    pub was_active: bool,
    pub remove_from_active: Option<RemoveFromActiveOutcomeLikeCpp>,
    pub removed_from_cell: bool,
    pub delete_from_world: bool,
    pub dynamic_object_caster_viewpoint: Option<DynamicObjectCasterViewpointOutcomeLikeCpp>,
    pub dynamic_object_remove_cleanup: Option<DynamicObjectRemoveCleanupOutcomeLikeCpp>,
    pub gameobject_zone_script_remove: Option<GameObjectZoneScriptRemoveOutcomeLikeCpp>,
    pub gameobject_remove_from_owner: Option<GameObjectRemoveFromOwnerOutcomeLikeCpp>,
    pub gameobject_model_remove: Option<DynamicMapTreeModelMutationOutcomeLikeCpp>,
    pub gameobject_linked_trap_remove: Option<GameObjectRemoveLinkedTrapOutcomeLikeCpp>,
    pub creature_zone_script_remove: Option<CreatureZoneScriptRemoveOutcomeLikeCpp>,
    pub creature_vehicle_remove: Option<VehicleKitRemoveOutcomeLikeCpp>,
    pub player_viewpoint_cleanup: Option<PlayerRemoveFromWorldViewpointCleanupOutcomeLikeCpp>,
    pub creature_unit_remove_from_world: Option<UnitRemoveFromWorldOutcomeLikeCpp>,
    pub creature_remove_formation: Option<CreatureRemoveFormationOutcomeLikeCpp>,
    pub personal_phase_unregister: PersonalPhaseUnregisterTrackedObjectOutcomeLikeCpp,
    pub visibility_on_destroy: RemoveFromMapVisibilityOnDestroyOutcomeLikeCpp,
    pub object: Option<WorldObject>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureRemoveFormationOutcomeLikeCpp {
    pub guid: ObjectGuid,
    pub spawn_id: SpawnId,
    pub leader_spawn_id: Option<SpawnId>,
    pub had_group: bool,
    pub removed_member: bool,
    pub removed_group: bool,
    pub remaining_members: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicObjectRemoveCleanupOutcomeLikeCpp {
    pub had_aura: bool,
    pub removed_aura_pending_delete: bool,
    pub unbound_caster: Option<ObjectGuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveFromMapError {
    ObjectNotFound { guid: ObjectGuid },
    ResetMap(MapBindingError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapObjectRelocationOutcome {
    pub guid: ObjectGuid,
    pub old_cell: CellCoord,
    pub new_cell: CellCoord,
    pub old_grid: GridCoord,
    pub new_grid: GridCoord,
    pub moved_between_cells: bool,
    pub loaded_grid: bool,
    pub created_grid: bool,
    pub relocated: bool,
    pub blocked_by_unloaded_grid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapObjectRelocationError {
    ObjectNotFound { guid: ObjectGuid },
    InvalidCoordinates { guid: ObjectGuid, x: f32, y: f32 },
    Record(ObjectAccessorError),
    Store(MapObjectStoreError),
}

#[derive(Debug, Clone, Default)]
pub struct NearbyCellGuids {
    pub world: WorldObjectGuids,
    pub grid: GridObjectGuids,
    pub visited_cells: usize,
}

impl PartialEq for NearbyCellGuids {
    fn eq(&self, other: &Self) -> bool {
        self.visited_cells == other.visited_cells
            && self.world.players == other.world.players
            && self.world.creatures == other.world.creatures
            && self.world.corpses == other.world.corpses
            && self.world.dynamic_objects == other.world.dynamic_objects
            && self.grid.gameobjects == other.grid.gameobjects
            && self.grid.creatures == other.grid.creatures
            && self.grid.dynamic_objects == other.grid.dynamic_objects
            && self.grid.corpses == other.grid.corpses
            && self.grid.area_triggers == other.grid.area_triggers
            && self.grid.scene_objects == other.grid.scene_objects
            && self.grid.conversations == other.grid.conversations
    }
}

impl Eq for NearbyCellGuids {}

impl NearbyCellGuids {
    pub fn is_empty(&self) -> bool {
        self.world.is_empty() && self.grid.is_empty()
    }

    pub fn len(&self) -> usize {
        self.world.len() + self.grid.len()
    }

    pub fn all_guids(&self) -> HashSet<ObjectGuid> {
        let mut guids = HashSet::with_capacity(self.len());
        guids.extend(self.world.players.iter().copied());
        guids.extend(self.world.creatures.iter().copied());
        guids.extend(self.world.corpses.iter().copied());
        guids.extend(self.world.dynamic_objects.iter().copied());
        guids.extend(self.grid.gameobjects.iter().copied());
        guids.extend(self.grid.creatures.iter().copied());
        guids.extend(self.grid.dynamic_objects.iter().copied());
        guids.extend(self.grid.corpses.iter().copied());
        guids.extend(self.grid.area_triggers.iter().copied());
        guids.extend(self.grid.scene_objects.iter().copied());
        guids.extend(self.grid.conversations.iter().copied());
        guids
    }

    fn merge_world(&mut self, other: &WorldObjectGuids) {
        self.world.players.extend(other.players.iter().copied());
        self.world.creatures.extend(other.creatures.iter().copied());
        self.world.corpses.extend(other.corpses.iter().copied());
        self.world
            .dynamic_objects
            .extend(other.dynamic_objects.iter().copied());
    }

    fn merge_grid(&mut self, other: &GridObjectGuids) {
        self.grid
            .gameobjects
            .extend(other.gameobjects.iter().copied());
        self.grid.creatures.extend(other.creatures.iter().copied());
        self.grid
            .dynamic_objects
            .extend(other.dynamic_objects.iter().copied());
        self.grid.corpses.extend(other.corpses.iter().copied());
        self.grid
            .area_triggers
            .extend(other.area_triggers.iter().copied());
        self.grid
            .scene_objects
            .extend(other.scene_objects.iter().copied());
        self.grid
            .conversations
            .extend(other.conversations.iter().copied());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearbyCellVisitCenter {
    pub guid: ObjectGuid,
    pub activation_radius: f32,
}

#[derive(Debug, Clone, Default)]
pub struct NearbyCellVisitPlan {
    pub marked_cells: Vec<CellCoord>,
    pub nearby: NearbyCellGuids,
    pub skipped_missing_centers: Vec<ObjectGuid>,
    pub skipped_invalid_position_centers: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerRelocationVisibilityPlan {
    pub visible_guids: HashSet<ObjectGuid>,
    pub out_of_range_guids: HashSet<ObjectGuid>,
    pub reciprocal_player_updates: HashSet<ObjectGuid>,
    pub ai_relocation_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl PlayerRelocationVisibilityPlan {
    pub fn from_nearby_like_cpp(
        player_guid: ObjectGuid,
        previous_client_guids: impl IntoIterator<Item = ObjectGuid>,
        nearby: &NearbyCellGuids,
        relocated_for_ai: bool,
        player_seers_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let player_seers_needing_notify: HashSet<_> =
            player_seers_needing_notify.into_iter().collect();
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let visible_guids = nearby.all_guids();
        let mut out_of_range_guids: HashSet<_> = previous_client_guids.into_iter().collect();
        out_of_range_guids.remove(&player_guid);

        for guid in &visible_guids {
            out_of_range_guids.remove(guid);
        }

        let mut reciprocal_player_updates = HashSet::new();
        for guid in &nearby.world.players {
            if *guid != player_guid && !player_seers_needing_notify.contains(guid) {
                reciprocal_player_updates.insert(*guid);
            }
        }

        for guid in &out_of_range_guids {
            if guid.is_player() && !player_seers_needing_notify.contains(guid) {
                reciprocal_player_updates.insert(*guid);
            }
        }

        let ai_relocation_checks = if relocated_for_ai {
            nearby_creature_guids_excluding(nearby, player_guid)
                .into_iter()
                .filter(|guid| !creatures_needing_notify.contains(guid))
                .map(|guid| (guid, player_guid))
                .collect()
        } else {
            Vec::new()
        };

        Self {
            visible_guids,
            out_of_range_guids,
            reciprocal_player_updates,
            ai_relocation_checks,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CreatureRelocationVisibilityPlan {
    pub player_visibility_updates: HashSet<ObjectGuid>,
    pub ai_relocation_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl CreatureRelocationVisibilityPlan {
    pub fn from_nearby_like_cpp(
        creature_guid: ObjectGuid,
        source_creature_alive: bool,
        nearby: &NearbyCellGuids,
        player_seers_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let player_seers_needing_notify: HashSet<_> =
            player_seers_needing_notify.into_iter().collect();
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let mut player_visibility_updates = HashSet::new();
        let mut ai_relocation_checks = Vec::new();

        for player in &nearby.world.players {
            if !player_seers_needing_notify.contains(player) {
                player_visibility_updates.insert(*player);
            }
            ai_relocation_checks.push((creature_guid, *player));
        }

        if source_creature_alive {
            for creature in nearby_creature_guids_excluding(nearby, creature_guid) {
                ai_relocation_checks.push((creature_guid, creature));
                if !creatures_needing_notify.contains(&creature) {
                    ai_relocation_checks.push((creature, creature_guid));
                }
            }
        }

        Self {
            player_visibility_updates,
            ai_relocation_checks,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationPlan {
    pub creature_relocations: Vec<ObjectGuid>,
    pub player_relocations: Vec<ObjectGuid>,
    pub skipped_invalid_viewpoints: Vec<ObjectGuid>,
}

impl DelayedUnitRelocationPlan {
    pub fn from_nearby_like_cpp(
        nearby: &NearbyCellGuids,
        creatures_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        player_viewpoints_needing_notify: impl IntoIterator<Item = ObjectGuid>,
        invalid_non_self_viewpoints: impl IntoIterator<Item = ObjectGuid>,
    ) -> Self {
        let creatures_needing_notify: HashSet<_> = creatures_needing_notify.into_iter().collect();
        let player_viewpoints_needing_notify: HashSet<_> =
            player_viewpoints_needing_notify.into_iter().collect();
        let invalid_non_self_viewpoints: HashSet<_> =
            invalid_non_self_viewpoints.into_iter().collect();

        let mut creature_relocations: Vec<_> = nearby
            .world
            .creatures
            .iter()
            .chain(nearby.grid.creatures.iter())
            .copied()
            .filter(|guid| creatures_needing_notify.contains(guid))
            .collect();
        creature_relocations.sort();
        creature_relocations.dedup();

        let mut player_relocations = Vec::new();
        let mut skipped_invalid_viewpoints = Vec::new();
        let mut players: Vec<_> = nearby.world.players.iter().copied().collect();
        players.sort();
        for player in players {
            if !player_viewpoints_needing_notify.contains(&player) {
                continue;
            }

            if invalid_non_self_viewpoints.contains(&player) {
                skipped_invalid_viewpoints.push(player);
            } else {
                player_relocations.push(player);
            }
        }

        Self {
            creature_relocations,
            player_relocations,
            skipped_invalid_viewpoints,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationForCellsPlan {
    pub cell_plans: Vec<DelayedUnitRelocationCellPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedUnitRelocationCellPlan {
    pub cell_coord: CellCoord,
    pub plan: DelayedUnitRelocationPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedPlayerRelocationContext {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: ObjectGuid,
    pub previous_client_guids: Vec<ObjectGuid>,
    pub relocated_for_ai: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedCreatureRelocationContext {
    pub creature_guid: ObjectGuid,
    pub source_creature_alive: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DelayedUnitRelocationVisibilityPlans {
    pub creature_plans: Vec<CreatureDelayedRelocationVisibilityPlan>,
    pub player_plans: Vec<PlayerDelayedRelocationVisibilityPlan>,
    pub skipped_missing_sources: Vec<ObjectGuid>,
    pub skipped_invalid_source_positions: Vec<ObjectGuid>,
    pub missing_player_contexts: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureDelayedRelocationVisibilityPlan {
    pub creature_guid: ObjectGuid,
    pub cell_coord: CellCoord,
    pub nearby: NearbyCellGuids,
    pub visibility_plan: CreatureRelocationVisibilityPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDelayedRelocationVisibilityPlan {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: ObjectGuid,
    pub cell_coord: CellCoord,
    pub nearby: NearbyCellGuids,
    pub visibility_plan: PlayerRelocationVisibilityPlan,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AIRelocationPlan {
    pub creature_unit_checks: Vec<(ObjectGuid, ObjectGuid)>,
}

impl AIRelocationPlan {
    pub fn from_nearby_like_cpp(
        unit_guid: ObjectGuid,
        unit_is_creature: bool,
        nearby: &NearbyCellGuids,
    ) -> Self {
        let nearby_creatures = nearby_creature_guids_excluding(nearby, unit_guid);
        let mut creature_unit_checks = Vec::with_capacity(if unit_is_creature {
            nearby_creatures.len() * 2
        } else {
            nearby_creatures.len()
        });

        for creature in nearby_creatures {
            creature_unit_checks.push((creature, unit_guid));
            if unit_is_creature {
                creature_unit_checks.push((unit_guid, creature));
            }
        }

        Self {
            creature_unit_checks,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectUpdatePlan {
    pub diff_ms: u32,
    pub update_guids: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapUpdatePlayerSources {
    pub player_guid: ObjectGuid,
    pub viewpoint_guid: Option<ObjectGuid>,
    pub far_combat_unit_guids: Vec<ObjectGuid>,
    pub far_aura_caster_guids: Vec<ObjectGuid>,
    pub far_summon_guids: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapUpdateVisitPlan {
    pub diff_ms: u32,
    pub session_update_players: Vec<ObjectGuid>,
    pub player_update_guids: Vec<ObjectGuid>,
    pub nearby_visit_centers: Vec<ObjectGuid>,
    pub transport_update_guids: Vec<ObjectGuid>,
    pub process_relocation_notifies: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelocationNotifyProcessPlan {
    pub diff_ms: u32,
    pub delayed_relocation_cells: Vec<CellCoord>,
    pub reset_notify_cells: Vec<CellCoord>,
    pub reset_timer_grids: Vec<GridCoord>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProcessRelocationNotifiesOutcome {
    pub process_plan: RelocationNotifyProcessPlan,
    pub delayed_plan: DelayedUnitRelocationForCellsPlan,
    pub visibility_plans: DelayedUnitRelocationVisibilityPlans,
    pub reset_outcome: ResetNotifyFlagsOutcome,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResetNotifyFlagsOutcome {
    pub reset_player_guids: Vec<ObjectGuid>,
    pub reset_creature_guids: Vec<ObjectGuid>,
    pub missing_guids: Vec<ObjectGuid>,
}

/// C++ `MapObjectCellMoveState` (`MapObject.h:28-33`) represented for
/// map-owned delayed cell/grid move-list state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectCellMoveStateLikeCpp {
    None,
    Active,
    Inactive,
}

pub type MapObjectCellMoveState = MapObjectCellMoveStateLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapObjectMoveListFamilyLikeCpp {
    Creature,
    GameObject,
    DynamicObject,
    AreaTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingCellMoveLikeCpp {
    pub state: MapObjectCellMoveStateLikeCpp,
    pub new_position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddObjectToMoveListOutcomeLikeCpp {
    Queued,
    UpdatedExisting,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveObjectFromMoveListOutcomeLikeCpp {
    MarkedInactive,
    AlreadyInactive,
    NotQueued,
    LockedIgnored,
    MissingOrStale,
    WrongKind { actual: AccessorObjectKind },
}

/// Drain summary for C++ `Map::MoveAll*InMoveList` (`Map.cpp:1239-1416`).
/// This is a map-owned seam only: it does not claim UpdatePositionData,
/// visibility fanout, AfterRelocation, respawn relocation, Pet::Remove,
/// dynamic tree, scripts/AI, ObjectAccessor, or session packet runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MoveListDrainSummaryLikeCpp {
    pub family: Option<MapObjectMoveListFamilyLikeCpp>,
    pub processed: usize,
    pub relocated: usize,
    pub inactive_reset: usize,
    pub not_in_world: usize,
    pub missing_or_stale: usize,
    pub wrong_kind: usize,
    pub blocked_by_unloaded_grid: usize,
    pub remove_list_queued: usize,
    pub pet_remove_requested: usize,
    pub respawn_relocation_unsupported: usize,
    pub failed_invalid_position: usize,
    pub failed_store: usize,
    pub locked_ignored: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapObjectMoveListEntry {
    pub guid: ObjectGuid,
    pub kind: AccessorObjectKind,
    pub move_state: MapObjectCellMoveState,
    pub new_position: Position,
    pub respawn_position: Option<Position>,
    pub is_pet: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapObjectMoveListPlan {
    pub relocated: Vec<ObjectGuid>,
    pub respawn_relocated: Vec<ObjectGuid>,
    pub remove_from_world: Vec<ObjectGuid>,
    pub pet_removed: Vec<ObjectGuid>,
    pub blocked_unloaded_grid: Vec<ObjectGuid>,
    pub reset_inactive_or_none: Vec<ObjectGuid>,
    pub skipped_not_in_world: Vec<ObjectGuid>,
    pub skipped_other_map_or_missing: Vec<ObjectGuid>,
    pub skipped_kind_mismatch: Vec<ObjectGuid>,
    pub failed_invalid_position: Vec<ObjectGuid>,
    pub failed_store: Vec<ObjectGuid>,
    pub unsupported_kind: Vec<ObjectGuid>,
}

fn is_active_object_like_cpp(kind: AccessorObjectKind, object: &WorldObject) -> bool {
    kind == AccessorObjectKind::Player || object.is_active()
}

fn remove_from_map_in_world_eligible_type_like_cpp(kind: AccessorObjectKind) -> bool {
    matches!(
        kind,
        AccessorObjectKind::Player
            | AccessorObjectKind::Creature
            | AccessorObjectKind::Pet
            | AccessorObjectKind::GameObject
            | AccessorObjectKind::Transport
    )
}

fn move_list_family_accepts_kind_like_cpp(
    family: MapObjectMoveListFamilyLikeCpp,
    kind: AccessorObjectKind,
) -> bool {
    match family {
        MapObjectMoveListFamilyLikeCpp::Creature => {
            matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
        }
        MapObjectMoveListFamilyLikeCpp::GameObject => {
            matches!(
                kind,
                AccessorObjectKind::GameObject | AccessorObjectKind::Transport
            )
        }
        MapObjectMoveListFamilyLikeCpp::DynamicObject => kind == AccessorObjectKind::DynamicObject,
        MapObjectMoveListFamilyLikeCpp::AreaTrigger => kind == AccessorObjectKind::AreaTrigger,
    }
}

fn push_in_world_guids<Terrain, Lifecycle>(
    map: &Map<Terrain, Lifecycle>,
    target: &mut Vec<ObjectGuid>,
    guids: impl IntoIterator<Item = ObjectGuid>,
) where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    target.extend(
        guids
            .into_iter()
            .filter(|guid| map.object_is_in_world(*guid)),
    );
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

fn nearby_creature_guids_excluding(
    nearby: &NearbyCellGuids,
    excluded: ObjectGuid,
) -> Vec<ObjectGuid> {
    let mut nearby_creatures: Vec<_> = nearby
        .world
        .creatures
        .iter()
        .chain(nearby.grid.creatures.iter())
        .copied()
        .filter(|guid| *guid != excluded)
        .collect();
    nearby_creatures.sort();
    nearby_creatures.dedup();
    nearby_creatures
}

fn remove_list_grid_kind_like_cpp(kind: AccessorObjectKind) -> Option<GridObjectKind> {
    match kind {
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => Some(GridObjectKind::Creature),
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            Some(GridObjectKind::GameObject)
        }
        AccessorObjectKind::DynamicObject => Some(GridObjectKind::DynamicObject),
        AccessorObjectKind::AreaTrigger => Some(GridObjectKind::AreaTrigger),
        AccessorObjectKind::Corpse => Some(GridObjectKind::Corpse),
        AccessorObjectKind::SceneObject => Some(GridObjectKind::SceneObject),
        AccessorObjectKind::Conversation => Some(GridObjectKind::Conversation),
        AccessorObjectKind::Player => None,
    }
}

fn switch_list_unit_kind_like_cpp(kind: AccessorObjectKind) -> bool {
    matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
}

fn set_record_temp_world_object_like_cpp(record: &mut MapObjectRecord, on: bool) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.set_temp_world_object_like_cpp(on);
            }
        }
        AccessorObjectKind::Pet => {
            if let Some(pet) = record.pet_mut() {
                pet.creature_mut().set_temp_world_object_like_cpp(on);
            }
        }
        _ => {}
    }
}

fn map_record_is_world_object_like_cpp(record: &MapObjectRecord) -> bool {
    if record.object().is_world_object() {
        return true;
    }
    if let Some(creature) = record.creature() {
        return creature.is_temp_world_object();
    }
    if let Some(pet) = record.pet() {
        return pet.creature().is_temp_world_object();
    }
    false
}

impl SwitchGridContainersOutcomeLikeCpp {
    const fn executed() -> Self {
        Self {
            executed: true,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn missing_or_stale() -> Self {
        Self {
            executed: false,
            missing_or_stale: true,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn unsupported_kind() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: true,
            permanent_world_object: false,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn permanent_world_object() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: true,
            invalid_or_unloaded_grid: false,
        }
    }

    const fn invalid_or_unloaded_grid() -> Self {
        Self {
            executed: false,
            missing_or_stale: false,
            unsupported_kind: false,
            permanent_world_object: false,
            invalid_or_unloaded_grid: true,
        }
    }
}

fn cleanup_map_object_record_before_delete_like_cpp(
    record: &mut MapObjectRecord,
    kind: AccessorObjectKind,
    creature_second_cleanup: bool,
) -> usize {
    match kind {
        AccessorObjectKind::Creature => record.creature_mut().map_or(0, |creature| {
            if !creature_second_cleanup {
                creature.set_destroyed_object(true);
            }
            creature.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Pet => record.pet_mut().map_or(0, |pet| {
            if !creature_second_cleanup {
                pet.creature_mut().set_destroyed_object(true);
            }
            pet.creature_mut().cleanup_before_delete();
            1
        }),
        AccessorObjectKind::GameObject => record.game_object_mut().map_or(0, |game_object| {
            game_object.set_destroyed_object(true);
            game_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Transport => record.transport_mut().map_or(0, |transport| {
            transport.game_object_mut().set_destroyed_object(true);
            let _removed_static_passengers = transport.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::DynamicObject => {
            record.dynamic_object_mut().map_or(0, |dynamic_object| {
                dynamic_object.set_destroyed_object(true);
                dynamic_object.cleanup_before_delete();
                1
            })
        }
        AccessorObjectKind::AreaTrigger => record.area_trigger_mut().map_or(0, |area_trigger| {
            area_trigger.set_destroyed_object(true);
            area_trigger.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Corpse => record.corpse_mut().map_or(0, |corpse| {
            corpse.set_destroyed_object(true);
            corpse.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::SceneObject => record.scene_object_mut().map_or(0, |scene_object| {
            scene_object.set_destroyed_object(true);
            scene_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Conversation => record.conversation_mut().map_or(0, |conversation| {
            conversation.set_destroyed_object(true);
            conversation.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Player => {
            // No typed represented `CleanupsBeforeDelete` exists for Player in this
            // bounded map remove-list seam. Preserve at least the base
            // `WorldObject::SetDestroyedObject(true)` mutation and report no
            // represented cleanup.
            record.object_mut().object_mut().set_destroyed_object(true);
            0
        }
    }
}

fn insert_object_guid_in_cell_like_cpp(
    cell: &mut Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) {
    match kind {
        AccessorObjectKind::Player => {
            cell.world_objects.players.insert(guid);
        }
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                cell.world_objects.creatures.insert(guid);
            } else {
                cell.grid_objects.creatures.insert(guid);
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            cell.grid_objects.gameobjects.insert(guid);
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                cell.world_objects.dynamic_objects.insert(guid);
            } else {
                cell.grid_objects.dynamic_objects.insert(guid);
            }
        }
        AccessorObjectKind::AreaTrigger => {
            cell.grid_objects.area_triggers.insert(guid);
        }
        AccessorObjectKind::Corpse => {
            if is_world_object {
                cell.world_objects.corpses.insert(guid);
            } else {
                cell.grid_objects.corpses.insert(guid);
            }
        }
        AccessorObjectKind::SceneObject => {
            cell.grid_objects.scene_objects.insert(guid);
        }
        AccessorObjectKind::Conversation => {
            cell.grid_objects.conversations.insert(guid);
        }
    }
}

fn remove_object_guid_from_cell_like_cpp<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    grid: GridCoord,
    cell: &Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) -> bool
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    let Some(ngrid) = map.get_ngrid_mut(grid) else {
        return false;
    };
    let Some(local_cell) = ngrid.get_grid_type_mut(cell.cell_x(), cell.cell_y()) else {
        return false;
    };

    match kind {
        AccessorObjectKind::Player => local_cell.world_objects.players.remove(&guid),
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                local_cell.world_objects.creatures.remove(&guid)
            } else {
                local_cell.grid_objects.creatures.remove(&guid)
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            local_cell.grid_objects.gameobjects.remove(&guid)
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                local_cell.world_objects.dynamic_objects.remove(&guid)
            } else {
                local_cell.grid_objects.dynamic_objects.remove(&guid)
            }
        }
        AccessorObjectKind::AreaTrigger => local_cell.grid_objects.area_triggers.remove(&guid),
        AccessorObjectKind::Corpse => {
            if is_world_object {
                local_cell.world_objects.corpses.remove(&guid)
            } else {
                local_cell.grid_objects.corpses.remove(&guid)
            }
        }
        AccessorObjectKind::SceneObject => local_cell.grid_objects.scene_objects.remove(&guid),
        AccessorObjectKind::Conversation => local_cell.grid_objects.conversations.remove(&guid),
    }
}

/// Invalidates async claims before a terminal typed object lifetime is dropped.
///
/// C++ destroys `Creature::loot` / `GameObject::loot` together with the typed
/// object. Rust leases can retain the shared authority after that drop, so the
/// backing allocation must become permanently detached first.
fn detach_typed_loot_authority_like_cpp(record: &mut MapObjectRecord) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        AccessorObjectKind::GameObject => {
            if let Some(game_object) = record.game_object_mut() {
                game_object.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        _ => {}
    }
}

/// A same-GUID whole-entity refresh may replace the record while deliberately
/// retaining the exact object lifetime. Do not detach the shared authority in
/// that case; only a distinct backing allocation is displaced terminally.
fn typed_loot_authorities_share_storage_like_cpp(
    previous: &MapObjectRecord,
    replacement: &MapObjectRecord,
) -> bool {
    match (previous.kind(), replacement.kind()) {
        (AccessorObjectKind::Creature, AccessorObjectKind::Creature) => previous
            .creature()
            .zip(replacement.creature())
            .is_some_and(|(previous, replacement)| {
                previous
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(replacement.loot_authority_like_cpp())
            }),
        (AccessorObjectKind::GameObject, AccessorObjectKind::GameObject) => previous
            .game_object()
            .zip(replacement.game_object())
            .is_some_and(|(previous, replacement)| {
                previous
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(replacement.loot_authority_like_cpp())
            }),
        _ => false,
    }
}

impl<Terrain, Lifecycle> GridUnloadEntityStore for Map<Terrain, Lifecycle> {
    fn creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut Creature> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::creature_mut)
    }

    fn game_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut GameObject> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
    }

    fn dynamic_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut DynamicObject> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::dynamic_object_mut)
    }

    fn corpse_mut(&mut self, guid: ObjectGuid) -> Option<&mut Corpse> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::corpse_mut)
    }

    fn area_trigger_mut(&mut self, guid: ObjectGuid) -> Option<&mut AreaTrigger> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::area_trigger_mut)
    }

    fn scene_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut SceneObject> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::scene_object_mut)
    }

    fn conversation_mut(&mut self, guid: ObjectGuid) -> Option<&mut Conversation> {
        self.map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::conversation_mut)
    }
}

impl<Terrain, Lifecycle> ObjectAccessorMapSource for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
        self.map_objects.get(&guid)
    }
}

impl<Terrain, Lifecycle> WorldObjectEnvironment for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader + MapWorldObjectEnvironment,
    Lifecycle: GridLifecycle,
{
    fn map_id(&self) -> u32 {
        self.map_id
    }

    fn instance_id(&self) -> u32 {
        self.instance_id
    }

    fn visibility_range(&self) -> f32 {
        self.visible_distance
    }

    fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
        self.terrain.line_of_sight(query)
    }

    fn map_height(
        &self,
        object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        self.terrain.map_height(object, x, y, z, query)
    }

    fn floor_z(&self, object: &WorldObject, position: Position, max_search_dist: f32) -> f32 {
        self.terrain.floor_z(object, position, max_search_dist)
    }
}

impl<Terrain, Lifecycle> MapGridHost for Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    fn active_objects_near_grid(&self, grid: &NGrid) -> bool {
        Map::active_objects_near_grid(self, grid)
    }

    fn stop_grid_objects(&mut self, grid: &NGrid) {
        self.lifecycle.stop_grid_objects(grid);
        self.drain_grid_unload_actions_like_cpp();
    }

    fn reset_grid_expiry(&mut self, grid: &mut NGrid, factor: f32) {
        Map::reset_grid_expiry(self, grid, factor);
    }

    fn unload_grid(&mut self, grid: &mut NGrid, unload_all: bool) -> bool {
        if !self.can_unload_grid(grid, unload_all) {
            return false;
        }

        self.run_unload_lifecycle(grid, unload_all);
        self.grid_state_unloaded = true;
        true
    }
}

fn grid_index(coord: GridCoord) -> Option<usize> {
    coord
        .is_coord_valid()
        .then_some((coord.x_coord * MAX_NUMBER_OF_GRIDS + coord.y_coord) as usize)
}

fn checked_grid_index(coord: GridCoord) -> usize {
    grid_index(coord).expect("grid coordinates must be within MAX_NUMBER_OF_GRIDS")
}

fn terrain_grid_coords(coord: GridCoord) -> (u32, u32) {
    (
        (MAX_NUMBER_OF_GRIDS - 1) - coord.x_coord,
        (MAX_NUMBER_OF_GRIDS - 1) - coord.y_coord,
    )
}

fn active_cells_near_grid(
    active_cells: &HashSet<CellCoord>,
    visible_distance: f32,
    grid: &NGrid,
) -> bool {
    let mut cell_min = CellCoord::new(
        grid.x() as u32 * MAX_NUMBER_OF_CELLS,
        grid.y() as u32 * MAX_NUMBER_OF_CELLS,
    );
    let mut cell_max = CellCoord::new(
        cell_min.x_coord + MAX_NUMBER_OF_CELLS,
        cell_min.y_coord + MAX_NUMBER_OF_CELLS,
    );
    let cell_range = (visible_distance / SIZE_OF_GRID_CELL).ceil() as u32 + 1;

    cell_min.dec_x(cell_range);
    cell_min.dec_y(cell_range);
    cell_max.inc_x(cell_range);
    cell_max.inc_y(cell_range);

    active_cells.iter().any(|cell| {
        cell_min.x_coord <= cell.x_coord
            && cell.x_coord <= cell_max.x_coord
            && cell_min.y_coord <= cell.y_coord
            && cell.y_coord <= cell_max.y_coord
    })
}

fn pool_member_kind_to_spawn_object_type_like_cpp(
    kind: PoolMemberKindLikeCpp,
) -> Option<SpawnObjectType> {
    match kind {
        PoolMemberKindLikeCpp::Creature => Some(SpawnObjectType::Creature),
        PoolMemberKindLikeCpp::GameObject => Some(SpawnObjectType::GameObject),
        PoolMemberKindLikeCpp::Pool => None,
    }
}

pub fn is_grid_id_loaded<Terrain, Lifecycle>(map: &Map<Terrain, Lifecycle>, grid_id: u32) -> bool
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    map.is_grid_loaded(GridCoord::new(
        grid_id % MAX_NUMBER_OF_GRIDS,
        grid_id / MAX_NUMBER_OF_GRIDS,
    ))
}

pub fn cell_from_grid_center(coord: GridCoord) -> Cell {
    let cell = CellCoord::new(
        coord.x_coord * MAX_NUMBER_OF_CELLS,
        coord.y_coord * MAX_NUMBER_OF_CELLS,
    );
    Cell::from_cell_coord(cell)
}

pub fn cell_from_world(x: f32, y: f32) -> Cell {
    Cell::from_cell_coord(compute_cell_coord(x, y))
}

pub const fn total_cell_count() -> u32 {
    TOTAL_NUMBER_OF_CELLS_PER_MAP * TOTAL_NUMBER_OF_CELLS_PER_MAP
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
