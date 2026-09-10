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

use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use tracing::{debug, info, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::{
    CreatureRandomMovementType as ConstantsCreatureRandomMovementType, PowerType, UnitDynFlags,
    UnitFlags2, UnitMoveType, UnitStandStateType, UnitState, WeaponAttackType,
};
use wow_core::{ObjectGuid, Position};
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
use wow_packet::packets::update::CreatureCreateData;
use wow_persistence::{RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp};
use wow_recastdetour::{
    CENTER_GRID_ID_LIKE_CPP, DetourNavMeshQueryError, DetourOwnerCapabilitiesLikeCpp,
    DetourPathOptions, DetourPathType, DetourPointPath, DetourPolyPath, DetourQueryFilterError,
    MAX_NUMBER_OF_GRIDS_LIKE_CPP, MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData,
    MMapManager as DetourMMapManager, MMapManagerError, PathQueryFilterContext,
    SIZE_OF_GRIDS_LIKE_CPP, ThreadUnsafeMapData, create_path_query_filter_like_cpp,
};

use crate::phasing::personal::MultiPersonalPhaseTracker;

/// A creature stored in the global map system.
#[derive(Debug)]
pub struct WorldCreature {
    /// Canonical creature entity. Runtime/AI ownership lives here.
    pub creature: Creature,
    /// Packet-create bridge retained for update-object construction.
    pub create_data: CreatureCreateData,
    /// Active movement spline for the represented world tick.
    ///
    /// This is the first runtime bridge toward C++ `Unit::movespline`; the full
    /// `MoveSplineInit`/`MotionMaster` port still owns generalized launch/stop.
    active_move_spline: Option<MoveSpline>,
    active_random_generator: Option<RandomMovementGenerator>,
    /// Corridor kept by the random generator's `PathGenerator`, which C++
    /// allocates once per generator lifetime and only drops in `DoInitialize`
    /// (`RandomMovementGenerator.cpp:95,140-143`). Reusing it is what makes
    /// `BuildPolyPath`'s subpath/suffix branches reachable
    /// (`PathGenerator.cpp:291-413`).
    active_random_path_poly_refs: Vec<u64>,
    /// Selected home generator, kept so its C++ flags survive ticks.
    active_home_generator: Option<HomeMovementGenerator>,
    /// Selected chase generator, kept so its C++ state survives ticks.
    active_chase_generator: Option<ChaseMovementGenerator>,
    /// Corridor held by the chase generator's `PathGenerator`, which C++ keeps
    /// alive across updates (`ChaseMovementGenerator.cpp:174-175`).
    active_chase_path_poly_refs: Vec<u64>,
    active_waypoint_generator: Option<WaypointMovementGenerator>,
    active_waypoint_random_at_path_end: Option<WaypointRandomAtPathEnd>,
    /// C++ `Unit::i_motionMaster`: the persistent priority stack that selects
    /// which concrete runtime generator may advance this frame.
    runtime_motion_master: MotionMaster,
    runtime_chase_target: Option<ObjectGuid>,
    runtime_represented_active: Option<RuntimeRepresentedActiveKeyLikeCpp>,
    /// Caller-owned delayed `AssistDelayEvent` payload: victim, assistant
    /// GUIDs, and map-local due time.
    pending_assistance_like_cpp: Vec<(ObjectGuid, Vec<ObjectGuid>, u64)>,
    /// C++ `m_AlreadyCallAssistance`, reset when combat stops.
    assistance_called_like_cpp: bool,
    /// Active `SPELL_AURA_MOD_TAUNT`s in application order: caster and expiry.
    active_taunts_like_cpp: Vec<ActiveTauntLikeCpp>,
    /// C++ `CombatAI::_events` due times for the eight template spell slots.
    /// `None` means that slot is not scheduled for the current engagement.
    creature_spell_due_at_ms_like_cpp: [Option<u64>; wow_entities::MAX_CREATURE_SPELLS],
    /// C++ initializes and resets `CombatAI::_events` once per AI lifecycle.
    /// The legacy map owner keeps that lifecycle bit beside the due times so
    /// multiple player sessions cannot independently schedule the same cast.
    creature_spell_schedule_initialized_like_cpp: bool,
    /// Monotonic engagement token carried by deferred session commands. It
    /// invalidates a queued cast after evade/death/reset even when the same
    /// creature later attacks the same player again.
    creature_spell_engagement_epoch_like_cpp: u64,
    /// Set by reached-home finalization until the global movement owner
    /// publishes the restored health values update.
    home_health_restored_pending_like_cpp: bool,
    runtime_motion_master_ticks: u64,
    /// False after the creature-spell slice reaches a C++ RNG decision whose
    /// exact number/order of draws is unknown. The marker prevents later spell
    /// casts from claiming exact RNG authority, but it must not disable the
    /// pre-existing best-effort melee and movement runtimes.
    runtime_rng_authority_complete_like_cpp: bool,
    /// DB-backed aura-source proofs that may be re-accredited only after the
    /// respawn rail reapplies the captured creature/template addon source.
    /// These are provenance, not the live AuraSubsystem markers: ordinary aura
    /// mutations still revoke the live markers permanently for that lifetime.
    respawn_spell_hit_aura_source_authority_like_cpp: bool,
    respawn_spell_cast_log_aura_source_authority_like_cpp: bool,
    runtime_rng_like_cpp: StdRng,
    /// C++ `Unit::Update(p_time)` advances every creature-local deadline from
    /// the `Map::Update(t_diff)` value. This logical clock is advanced only by
    /// the owning creature tick; scheduler delay or time spent between phases
    /// cannot independently move spline, combat, spell, assistance or corpse
    /// state.
    runtime_elapsed_ms_like_cpp: u64,
}

impl Clone for WorldCreature {
    fn clone(&self) -> Self {
        let creature = self.creature.clone();
        Self {
            runtime_motion_master: Self::new_runtime_motion_master_like_cpp(&creature),
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: self.pending_assistance_like_cpp.clone(),
            assistance_called_like_cpp: self.assistance_called_like_cpp,
            active_taunts_like_cpp: self.active_taunts_like_cpp.clone(),
            creature_spell_due_at_ms_like_cpp: self.creature_spell_due_at_ms_like_cpp,
            creature_spell_schedule_initialized_like_cpp: self
                .creature_spell_schedule_initialized_like_cpp,
            creature_spell_engagement_epoch_like_cpp: self.creature_spell_engagement_epoch_like_cpp,
            home_health_restored_pending_like_cpp: self.home_health_restored_pending_like_cpp,
            runtime_motion_master_ticks: self.runtime_motion_master_ticks,
            runtime_rng_authority_complete_like_cpp: self.runtime_rng_authority_complete_like_cpp,
            respawn_spell_hit_aura_source_authority_like_cpp: self
                .respawn_spell_hit_aura_source_authority_like_cpp,
            respawn_spell_cast_log_aura_source_authority_like_cpp: self
                .respawn_spell_cast_log_aura_source_authority_like_cpp,
            creature,
            create_data: self.create_data.clone(),
            active_move_spline: self.active_move_spline.clone(),
            active_random_generator: self.active_random_generator.clone(),
            active_random_path_poly_refs: self.active_random_path_poly_refs.clone(),
            active_home_generator: self.active_home_generator.clone(),
            active_chase_generator: self.active_chase_generator,
            active_chase_path_poly_refs: self.active_chase_path_poly_refs.clone(),
            active_waypoint_generator: self.active_waypoint_generator.clone(),
            active_waypoint_random_at_path_end: self.active_waypoint_random_at_path_end,
            runtime_rng_like_cpp: self.runtime_rng_like_cpp.clone(),
            runtime_elapsed_ms_like_cpp: self.runtime_elapsed_ms_like_cpp,
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
    pub personal_phases: MultiPersonalPhaseTracker,
    personal_phase_objects_to_remove: HashSet<ObjectGuid>,
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

#[cfg(test)]
#[path = "../map_manager_tests.rs"]
mod tests;

mod grid;
mod pathfinder;
mod pending_respawn;
mod runtime_state;
mod terrain;

pub use grid::*;
pub use pathfinder::*;
pub use pending_respawn::*;
pub use runtime_state::*;
pub use terrain::*;

use grid::{
    calculate_cell_area_like_cpp, cell_area_contains_position_like_cpp, position_to_i32_tuple,
};

use pending_respawn::spawn_object_type_raw_like_cpp;

use runtime_state::{
    ActiveTauntLikeCpp, BASE_ATTACK_TIME_LIKE_CPP, NOMINAL_MELEE_RANGE_LIKE_CPP,
    RuntimeRepresentedActiveGeneratorLikeCpp, RuntimeRepresentedActiveKeyLikeCpp,
    absolute_angle_like_cpp, power_type_from_u8_like_cpp,
};
