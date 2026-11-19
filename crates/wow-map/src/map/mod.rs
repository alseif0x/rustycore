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

mod entity_world;
mod game_object;
mod lent_creature;
mod lifecycle;
mod loaded_grid;
mod object_access;
mod object_update_selection;
mod outcomes;
mod outcomes_lifecycle;
mod outcomes_update;
mod pet_combat;
mod plans;
mod relocation;
mod respawn;
mod runtime;
mod scripts_weather;
mod send_object_updates;
mod spawn_groups;
mod spawn_runtime;
mod storage;
mod update;
mod visibility;

use crate::map_rules::{
    decrement_pool_counter_like_cpp, map_record_unit_mut_like_cpp,
    player_set_viewpoint_outcome_like_cpp, remove_spawn_id_index_entry_like_cpp,
};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use rand::{Rng, SeedableRng, rngs::StdRng};

use self::entity_world::EntityWorld;
pub use self::lent_creature::{LentCreatureOriginLikeCpp, LentCreatureReturnLikeCpp};
pub use self::loaded_grid::{LoadedGridObjectFacts, LoadedGridRespawnRecordsLikeCpp};
pub(in crate::map) use self::object_access::*;
pub(in crate::map) use self::outcomes::ActiveNonPlayerRespawnLocationLikeCpp;
pub use self::outcomes::*;
pub(in crate::map) use self::outcomes_lifecycle::SwitchGridContainersOutcomeLikeCpp;
pub use self::outcomes_lifecycle::*;
pub(in crate::map) use self::outcomes_update::gameobject_type_is_transport_like_cpp;
pub use self::outcomes_update::*;
pub use self::pet_combat::MapPetCombatStopOutcomeLikeCpp;
pub use self::plans::*;
pub use self::runtime::{
    MapCommandKindLikeCpp, MapCommandLikeCpp, MapCommandOutcomeLikeCpp, MapCommandStatusLikeCpp,
};
pub(crate) use self::runtime::{
    MapRuntime, MapRuntimePlayerAttachErrorLikeCpp, MapRuntimePlayerDetachErrorLikeCpp,
    MapRuntimePlayerRelocationErrorLikeCpp,
};
pub use self::send_object_updates::{
    RepresentedAreaTriggerValuesUpdateLikeCpp, RepresentedConversationValuesUpdateLikeCpp,
    RepresentedCorpseValuesUpdateLikeCpp, RepresentedDynamicObjectValuesUpdateLikeCpp,
    RepresentedGameObjectValuesUpdateLikeCpp, RepresentedPlayerValuesUpdateLikeCpp,
    RepresentedSceneObjectValuesUpdateLikeCpp, RepresentedUnitValuesUpdateLikeCpp,
    SendObjectUpdatesSummaryLikeCpp,
};
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
    DynamicObjectType, GAMEOBJECT_TYPE_CAPTURE_POINT, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT,
    GAMEOBJECT_TYPE_NEW_FLAG, GAMEOBJECT_TYPE_NEW_FLAG_DROP, GAMEOBJECT_TYPE_TRANSPORT,
    GO_FLAG_NODESPAWN, GameObject, GameObjectCreateLifecycleRecord, GameObjectLifecycleError,
    GameObjectTemplateLifecycleRecord,
    GameObjectUpdateOutcomeLikeCpp as EntityGameObjectUpdateOutcomeLikeCpp,
    GameObjectUpdateStatusLikeCpp as EntityGameObjectUpdateStatusLikeCpp, GoState, INVALID_HEIGHT,
    LineOfSightQuery, LootState, MAX_VISIBILITY_DISTANCE, MapBindingError, MapObjectRecord,
    ObjectAccessorError, ObjectNotifyFlags, Pet, Player, SceneObject, TransportUpdateLikeCpp, Unit,
    UnitAddToWorldOutcomeLikeCpp, UnitRemoveFromWorldOutcomeLikeCpp,
    UnitSharedVisionSetWorldObjectRequestLikeCpp, UnitValuesUpdate,
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
    /// Source-of-truth remains `entity_world`; this set stores only non-player active
    /// object GUID membership produced by `Map::AddToActive`/`RemoveFromActive` seams.
    /// It is not rebuilt by sessions/ObjectAccessor scans. Rust does not yet model
    /// C++ `Map::Update`'s mutating iterator adjustment; consumers snapshot/sort GUIDs.
    active_non_players_like_cpp: HashSet<ObjectGuid>,
    personal_phase_tracker: MultiPersonalPhaseTracker,
    spawn_group_state: SpawnGroupRuntimeState,
    respawn_store: RespawnStoreLikeCpp,
    creature_respawn_queue: crate::creature_respawn_queue::CreatureRespawnQueueLikeCpp,
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
