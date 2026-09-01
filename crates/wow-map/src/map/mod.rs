// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map storage, update phases and object lifecycle.
//!
//! Issue #225 split the former 15,250-line `map.rs` into private runtime
//! modules. The Map owner, the two documented runtime models and every phase
//! and bridge are unchanged; this module keeps the shared types, the
//! constructors and the helpers the phases build on.

mod game_object;
mod relocation;
mod respawn;
mod scripts_weather;
mod spawn_groups;
mod storage;
mod update;
mod visibility;

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

    pub fn represented_zone_dynamic_info_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&RepresentedZoneDynamicInfoLikeCpp> {
        self.zone_dynamic_info_like_cpp.get(&zone_id)
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

    pub fn map_object_count(&self) -> usize {
        self.map_objects.len()
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

    pub fn creature_group_holder_member_count_like_cpp(&self, leader_spawn_id: SpawnId) -> usize {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .map_or(0, HashSet::len)
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

    fn viewpoint_has_invalid_position_like_cpp(&self, viewpoint_guid: ObjectGuid) -> bool {
        self.map_object(viewpoint_guid).is_none_or(|viewpoint| {
            let position = viewpoint.position();
            !is_valid_map_coord_2d(position.x, position.y)
        })
    }

    pub fn map_object_record(&self, guid: ObjectGuid) -> Option<&MapObjectRecord> {
        self.map_objects.get(&guid)
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
    /// The exact canonical Player value retained by a non-delete Map transfer.
    /// Other object families remain represented by `object` below.
    pub player: Option<Box<Player>>,
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
#[path = "../map_tests.rs"]
mod tests;
