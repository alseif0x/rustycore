// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Legacy map manager and its creature runtime.
//!
//! Issue #225 split the former 6,607-line `map_manager.rs` into private
//! runtime modules. The owner, the two documented runtime models and every
//! clock, writer, phase and bridge are unchanged.

mod combat;
mod player_melee;
pub use player_melee::{PlayerMeleeCreatureHit, PlayerMeleeSwing};
mod aggro;
mod creature_spell;
pub use aggro::{
    AggroAiFacts, AggroAiKind, AggroAiSelection, AggroAttackDecision, AggroAttackStart,
    AggroAttackStop, AggroCandidate, AggroDistanceFacts, AggroEffect, AggroEffectKind,
    AggroFactionTarget, AggroLeash, AggroOutcome, AggroOwnerSnapshot, AggroPolicies, AggroSettings,
    AggroThreatUpdate, AggroTurretFacts, AggroVisibility, can_attack, candidate_accessible,
    candidate_has_stealth, candidate_hostile, candidate_leash, candidate_targetable,
    candidate_visibility, select_ai, snapshot_hostile, snapshot_leash, trigger_alert,
    update_threat_victim,
};
pub(crate) use aggro::{AggroFrame, AggroLosPending, AggroMap, AggroTailProgress};
pub(crate) use creature_spell::SpellMap;
pub use creature_spell::*;
mod creature_loot;
mod creature_runtime;
pub use creature_loot::{
    CreatureLootObservation, CreatureLootReleaseOutcome, CreatureLootReleasePhase,
};
mod creature_transport;
pub(crate) use creature_transport::{LegacyCreatureTransportError, LegacyCreatureTransportSlot};
mod movement;
mod respawn;
mod runtime;

pub use self::movement::{CreatureMovementSource, CreatureMovementStep};
pub(crate) use self::movement::{
    StepGridHeightContinuation, StepPathContinuation, StepPending, StepProgress,
    StepStaticHeightContinuation,
};

use self::creature_runtime::WorldCreatureRuntime;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::map::MapWorldObjectEnvironment;
use crate::{GridMapTerrain, SharedStaticVMapLineOfSightProvider, SpawnObjectType};
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use tracing::{debug, info, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::{
    CreatureRandomMovementType as ConstantsCreatureRandomMovementType, PowerType, UnitDynFlags,
    UnitFlags2, UnitMoveType, UnitStandStateType, UnitState, WeaponAttackType,
};
use wow_core::{ObjectGuid, Position};
use wow_entities::creature_create::CreatureCreateData;
use wow_entities::{
    AllowedPositionZCaps, Creature, CreatureAddonLifecycleRecordLikeCpp, CreatureAiState,
    CreatureCombatLogStatsLikeCpp, DEFAULT_HEIGHT_SEARCH, DistractMovementAction,
    EVENT_CHARGE_PREPATH, GenericMovementInform, INVALID_HEIGHT, MotionMasterUpdateContext,
    MotionMasterUpdateOutcome, MovementGeneratorKind, MovementGeneratorRef, MovementGeneratorType,
    MovementSlot, PhaseShift, PointMovementAction, PointMovementInform, RotateMovementUpdate,
    Z_OFFSET_FIND_HEIGHT, allowed_position_z_from_ground_like_cpp, game_time_secs_like_cpp,
};
use wow_movement::generators::CreatureRandomMovementType as MovementCreatureRandomMovementType;
use wow_movement::{
    ChaseMovementGenerator, HomeMovementGenerator, IdleMovementGenerator, MotionMaster, MoveSpline,
    MoveSplineFlag, MoveSplineInit, MoveSplineLaunchInput, MoveSplineStopInput,
    MoveSplineStopResult, MovementGenerator as RuntimeMovementGenerator,
    MovementGeneratorFlags as RuntimeMovementGeneratorFlags,
    MovementGeneratorMode as RuntimeMovementGeneratorMode,
    MovementGeneratorPriority as RuntimeMovementGeneratorPriority,
    MovementGeneratorState as RuntimeMovementGeneratorState,
    MovementGeneratorType as RuntimeMovementGeneratorType, MovementSlot as RuntimeMovementSlot,
    PathGenerator, PathType, RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP, RandomMovementAction,
    RandomMovementGenerator, RandomPathResult, RandomUnitSnapshot, WaypointAnimation,
    WaypointLaunchPlan, WaypointMovementAction, WaypointMovementGenerator, WaypointPath,
    WaypointRandomAtPathEnd, WaypointUnitSnapshot, compute_random_destination_like_cpp,
};
use wow_persistence::{RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp};
use wow_recastdetour::{
    CENTER_GRID_ID_LIKE_CPP, DetourNavMeshQueryError, DetourOwnerCapabilitiesLikeCpp,
    DetourPathOptions, DetourPathType, DetourPointPath, DetourPolyPath, DetourQueryFilterError,
    MAX_NUMBER_OF_GRIDS_LIKE_CPP, MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData,
    MMapManager as DetourMMapManager, MMapManagerError, PathQueryFilterContext,
    SIZE_OF_GRIDS_LIKE_CPP, ThreadUnsafeMapData, create_path_query_filter_like_cpp,
};

/// A creature stored in the global map system.
#[derive(Debug)]
pub struct WorldCreature {
    /// Canonical creature entity. Runtime/AI ownership lives here.
    pub creature: Creature,
    /// Packet-create bridge retained for update-object construction.
    pub create_data: CreatureCreateData,
    runtime: WorldCreatureRuntime,
}

impl Clone for WorldCreature {
    fn clone(&self) -> Self {
        let creature = self.creature.clone();
        let runtime = self.runtime.clone_for_creature(&creature);
        Self {
            creature,
            create_data: self.create_data.clone(),
            runtime,
        }
    }
}

/// An instance of a map (e.g., Eastern Kingdoms instance 0).
#[derive(Debug)]
pub struct MapInstance {
    pub map_id: u16,
    pub instance_id: u32,
    pub grids: HashMap<GridCoord, Grid>,
    pub grid_unload_timeout: Duration,
    /// Temporary legacy instance of the same tagged owner held by canonical Map.
    /// Quiescent move-only transport must retire this instance before activation.
    respawn_store: crate::spawn::RespawnStoreLikeCpp,
}

// ── Slice 4A.1a: addressable routing types ────────────────────────────────────
//
// These types model *candidate* recipients for map-wide packet fanout,
// mirroring C++ `MessageDistDeliverer` routing modes.
//
// IMPORTANT: these rules select *candidate* sessions only.  The final gate
// (HaveAtClient / phase check) is applied by each session via
// `SendIfVisibleLikeCpp`, which lands in Slice 4A.1b.  Do NOT duplicate
// visibility or phase logic here.
//
// Extensions from the C++ model that are not yet needed (own_team_only,
// skipped_receiver, team-based broadcast) are omitted for now and will be
// added as variants in future sub-slices.

/// Global map manager containing all map instances.
#[derive(Debug)]
pub struct MapManager {
    maps: HashMap<(u16, u32), MapInstance>, // (map_id, instance_id) -> MapInstance
    tick_owner: RuntimeTickOwner,
    /// Shared, file-backed terrain height (DataDir). `None` until wired at server
    /// startup; while absent, height-dependent paths fall back to their prior
    /// no-terrain behaviour.
    terrain: Option<Arc<LiveTerrainHeights>>,
}

/// Shared reference type for the MapManager.
pub type SharedMapManager = Arc<RwLock<MapManager>>;

#[cfg(test)]
#[path = "../map_manager_tests.rs"]
mod tests;

mod creature_respawn_prefix;
mod grid;
mod pathfinder;
mod pending_respawn;
mod runtime_state;
mod terrain;

pub(crate) use self::grid::DEFAULT_GRID_UNLOAD_TIME;
#[cfg(test)]
pub(crate) use self::grid::GRID_SIZE;
pub use self::grid::{
    Grid, GridCoord, VISIBILITY_RADIUS, grid_corner, grid_to_world, world_to_grid_coords,
    world_to_grid_x, world_to_grid_y,
};
pub(crate) use self::pathfinder::path_type_from_detour_like_cpp;
pub use self::pathfinder::{
    CreaturePathQueryLikeCpp, WorldDetourPathError, WorldMMapPathRequestLikeCpp,
    WorldMMapPathfinderLikeCpp, WorldMMapPathfinderWorkerLikeCpp,
    calculate_creature_detour_path_like_cpp, detour_path_without_navmesh_like_cpp,
    path_generator_from_detour_like_cpp,
};
use self::pathfinder::{
    path_generator_from_detour_with_normalizer_like_cpp, point_path_limit_for_distance_like_cpp,
    random_path_result_from_path_type_like_cpp,
};
pub use self::pending_respawn::{
    LegacyRespawnQueueReloadReportLikeCpp, LegacyRespawnTimeAddOutcomeLikeCpp, PendingRespawn,
    PersistedRespawnRowLikeCpp, pending_respawn_create_position_like_cpp,
    pending_respawn_from_world_creature_like_cpp, respawn_time_from_instant_like_cpp,
    snap_respawn_creature_to_ground_like_cpp, world_creature_from_pending_respawn_like_cpp,
};
use self::pending_respawn::{
    instant_from_respawn_time_like_cpp, respawn_delete_mutation_like_cpp,
    spawn_object_type_raw_like_cpp,
};
use self::runtime_state::{
    ActiveTauntLikeCpp, BASE_ATTACK_TIME_LIKE_CPP, NOMINAL_MELEE_RANGE_LIKE_CPP,
    RuntimeRepresentedActiveGeneratorLikeCpp, RuntimeRepresentedActiveKeyLikeCpp,
    absolute_angle_like_cpp, power_type_from_u8_like_cpp,
};
pub use self::runtime_state::{
    ChaseTargetSnapshotLikeCpp, ChaseTickOutcomeLikeCpp, CreatureAnimKitSlotLikeCpp, RecipientRule,
    RuntimeEvent, RuntimeOutput, RuntimePlan, RuntimeTickOwner, shared_runtime_tick_owner_like_cpp,
};
pub use self::terrain::*;

#[cfg(test)]
pub(crate) use self::terrain::{
    MAP_AREA_CELLS_PER_GRID_LIKE_CPP, MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP,
    MAP_AREA_HEADER_SIZE_LIKE_CPP, MAP_AREA_MAGIC_LIKE_CPP, MAP_FILE_HEADER_SIZE_LIKE_CPP,
    MAP_MAGIC_LIKE_CPP, MAP_VERSION_MAGIC_LIKE_CPP, TERRAIN_GRID_COUNT_LIKE_CPP,
};

use self::grid::{
    calculate_cell_area_like_cpp, cell_area_contains_position_like_cpp, position_to_i32_tuple,
};
