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
mod corpse_load;
mod movement;
mod respawn;
mod runtime;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

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
use wow_map::map::MapWorldObjectEnvironment;
use wow_map::{GridMapTerrain, SharedStaticVMapLineOfSightProvider, SpawnObjectType};
use wow_movement::generators::CreatureRandomMovementType as MovementCreatureRandomMovementType;
use wow_movement::{
    ChaseMovementGenerator, HomeMovementGenerator, MotionMaster, MoveSpline, MoveSplineFlag,
    MoveSplineInit, MoveSplineLaunchInput, MoveSplineStopInput, MoveSplineStopResult,
    MovementGenerator as RuntimeMovementGenerator,
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
///
/// #1263 F6-8A: the persistent creature runtime state that C++ keeps on the
/// object now lives in the canonical `wow_entities::Creature`
/// (`Creature::runtime_like_cpp`). This bridge keeps only the canonical entity,
/// the immutable packet projection `CreatureCreateData` and the scheduling
/// seam named below; the accessor methods read and write that same canonical
/// storage, so the legacy scheduling and publication bridges keep working
/// unchanged until F6-8D2/D3 and E delete them.
///
/// #1263 F6-8D1: the combat, swing, clock, RNG and `Unit::i_motionMaster`
/// operations moved onto canonical `Creature` ownership
/// (`wow_entities::creature::phase_ops`/`phase_motion`) and the runtime motion
/// master advance counter moved into `CreatureRuntimeLikeCpp`. What remains
/// here is the publication/provenance seam named below, plus the delegating
/// entry points the legacy scheduler still calls.
#[derive(Debug, Clone)]
pub struct WorldCreature {
    /// Canonical creature entity. Runtime/AI ownership lives here.
    pub creature: Creature,
    /// Packet-create bridge retained for update-object construction.
    pub create_data: CreatureCreateData,
    // #1263 F6-8D3a-2: the reached-home publication flag moved to the
    // canonical runtime state (`CreatureRuntimeLikeCpp`); the respawn-aura
    // provenance below stays on this bridge with the lifecycle seam.
    /// DB-backed aura-source proofs that may be re-accredited only after the
    /// respawn rail reapplies the captured creature/template addon source.
    /// These are provenance, not the live AuraSubsystem markers: ordinary aura
    /// mutations still revoke the live markers permanently for that lifetime.
    respawn_spell_hit_aura_source_authority_like_cpp: bool,
    respawn_spell_cast_log_aura_source_authority_like_cpp: bool,
}

/// An instance of a map (e.g., Eastern Kingdoms instance 0).
#[derive(Debug)]
pub struct MapInstance {
    pub map_id: u16,
    pub instance_id: u32,
    pub grids: HashMap<GridCoord, Grid>,
    pub grid_unload_timeout: Duration,
    /// C++ `Map::_creatureRespawnTimesBySpawnId` and
    /// `_gameObjectRespawnTimesBySpawnId`, represented as DB-persistable rows.
    pub persisted_respawn_times: HashMap<(SpawnObjectType, u64), PersistedRespawnRowLikeCpp>,
    /// Creatures waiting to respawn; drained by `tick_creatures_sync`.
    /// C++ ref: `Map::_respawnTimes` (Map.h:748).
    pub respawn_queue: Vec<PendingRespawn>,
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
    free_instance_ids: Vec<bool>,
    next_instance_id: u32,
    tick_owner: RuntimeTickOwner,
    /// Shared, file-backed terrain height (DataDir). `None` until wired at server
    /// startup; while absent, height-dependent paths fall back to their prior
    /// no-terrain behaviour.
    terrain: Option<Arc<LiveTerrainHeights>>,
}

/// Shared reference type for the MapManager.
pub type SharedMapManager = Arc<RwLock<MapManager>>;

/// #1263 C2 capture surface: presence and shape of the legacy runtime map table
/// observed for one exact `(map_id, instance_id)` key.
///
/// `observed == false` means the caller could not take the legacy manager guard
/// without blocking, so no statement about presence is made. It is never a
/// silent `false` presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyMapPresenceCaptureLikeCpp {
    pub observed: bool,
    pub present: bool,
    pub instance_id_is_zero: bool,
    pub map_instance_count: usize,
    /// `observed` | `legacy_map_manager_absent` | `legacy_map_manager_guard_busy`
    /// | `unobserved`.
    pub reason: &'static str,
}

impl LegacyMapPresenceCaptureLikeCpp {
    /// The "not observed" value used when the guard is busy or absent.
    pub const fn unobserved_like_cpp() -> Self {
        Self {
            observed: false,
            present: false,
            instance_id_is_zero: false,
            map_instance_count: 0,
            reason: "unobserved",
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/map_manager_tests/corpse_load.rs"]
mod corpse_load_tests;
#[cfg(test)]
#[path = "../../unit_tests/map_manager_tests.rs"]
mod tests;

mod grid;
mod pathfinder;
mod pending_respawn;
mod runtime_state;
mod terrain;

pub use corpse_load::{
    LoadedMapCorpseRowLikeCpp, MapCorpseLoadOutcomeLikeCpp,
    materialize_loaded_map_corpses_like_cpp, parse_corpse_items_like_cpp,
};
pub use grid::*;
pub use movement::{queries::CreatureMovementQueriesLikeCpp, view::CreatureMovementLikeCpp};
pub use pathfinder::*;
pub use pending_respawn::*;
pub use runtime_state::*;
pub use terrain::*;

use grid::{
    calculate_cell_area_like_cpp, cell_area_contains_position_like_cpp, position_to_i32_tuple,
};

use pending_respawn::spawn_object_type_raw_like_cpp;

use runtime_state::{
    BASE_ATTACK_TIME_LIKE_CPP, NOMINAL_MELEE_RANGE_LIKE_CPP, absolute_angle_like_cpp,
    power_type_from_u8_like_cpp,
};
