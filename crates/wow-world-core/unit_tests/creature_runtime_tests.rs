//! Behaviour tests for the creature runtime utilities that outlived the legacy
//! `map_manager` module: terrain, pathfinder, creature movement, respawn,
//! create projection and runtime plans.
//!
//! #1263 F6-8E-2: moved with those modules out of the deleted
//! `map_manager_tests`; the tests that exercised only the legacy
//! `MapManager`/`MapInstance`/`Grid` owner are deleted with it, and the
//! `Creature` tests run on the canonical `Creature`.

#![cfg(test)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wow_constants::movement::MovementFlag;
use wow_constants::{
    Class, CreatureFlagsExtra, DeathState, PowerType, UnitMoveType, UnitStandStateType, UnitState,
    WeaponAttackType,
};
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    Creature, CreatureAiState, DistractMovementAction, EVENT_CHARGE_PREPATH, GenericMovementInform,
    INVALID_HEIGHT, MovementGeneratorKind, MovementGeneratorType, PhaseShift, PointMovementAction,
    PointMovementInform, Z_OFFSET_FIND_HEIGHT,
};
use wow_map::SharedStaticVMapLineOfSightProvider;
use wow_map::SpawnObjectType;
use wow_map::map::MapWorldObjectEnvironment;
use wow_movement::MovementGeneratorType as RuntimeMovementGeneratorType;
use wow_movement::{
    MoveSpline, MoveSplineFlag, MovementGenerator, PathType, RandomPathResult,
    WaypointMovementAction, WaypointMovementGenerator, WaypointPath, WaypointRandomAtPathEnd,
};
use wow_persistence::{RespawnPersistenceKeyLikeCpp, RespawnPersistenceMutationLikeCpp};
use wow_recastdetour::{
    DetourOwnerCapabilitiesLikeCpp, DetourPathType, DetourPointPath, DetourPolyPath,
    MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData, PathQueryFilterContext, SIZE_OF_GRIDS_LIKE_CPP,
    create_path_query_filter_like_cpp,
};

use crate::creature_movement::*;
use crate::creature_projection::test_support::*;
use crate::creature_projection::*;
use crate::creature_respawn::*;
use crate::pathfinder::*;
use crate::runtime_plan::*;
use crate::terrain::*;

fn unique_temp_data_dir(test_name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let data_dir = std::env::temp_dir().join(format!("rustycore-{test_name}-{unique}"));
    fs::create_dir_all(data_dir.join("maps")).expect("create maps test dir");
    data_dir
}

fn map_file_header_like_cpp() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(MAP_MAGIC_LIKE_CPP);
    header.extend_from_slice(&MAP_VERSION_MAGIC_LIKE_CPP.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes());
    assert_eq!(header.len(), MAP_FILE_HEADER_SIZE_LIKE_CPP);
    header
}

fn map_file_header_with_area_like_cpp(area_offset: u32, area_size: u32) -> Vec<u8> {
    let mut header = map_file_header_like_cpp();
    header[12..16].copy_from_slice(&area_offset.to_le_bytes());
    header[16..20].copy_from_slice(&area_size.to_le_bytes());
    header
}

fn test_area_entry(id: u32, parent_area_id: u16, flags: u32) -> wow_data::AreaTableEntry {
    wow_data::AreaTableEntry {
        id,
        continent_id: 571,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    }
}

fn test_chase_target(victim: ObjectGuid, x: f32) -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    }
}

fn test_chase_corridor(poly_refs: Vec<u64>, end_x: f32) -> DetourPolyPath {
    let points = vec![
        [10.0, 10.0, 0.0],
        [(10.0 + end_x) * 0.5, 10.0, 0.0],
        [end_x, 10.0, 0.0],
    ];
    DetourPolyPath {
        poly_refs,
        point_path: DetourPointPath {
            actual_end: *points.last().expect("test path"),
            points,
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    }
}

#[derive(Debug)]
struct RecordingLiveStaticVMapLos {
    result: bool,
    calls: std::sync::Mutex<Vec<wow_map::VMapLineOfSightQuery>>,
}

impl RecordingLiveStaticVMapLos {
    fn new(result: bool) -> Self {
        Self {
            result,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl wow_map::StaticVMapLineOfSightProvider for RecordingLiveStaticVMapLos {
    fn is_in_line_of_sight(&self, query: wow_map::VMapLineOfSightQuery) -> bool {
        self.calls
            .lock()
            .expect("recording live vmap LOS calls poisoned")
            .push(query);
        self.result
    }
}

fn tilelist_like_cpp(grid_indices: impl IntoIterator<Item = usize>) -> Vec<u8> {
    let mut bitset_string = vec![b'0'; TERRAIN_GRID_COUNT_LIKE_CPP];
    for grid_idx in grid_indices {
        bitset_string[TERRAIN_GRID_COUNT_LIKE_CPP - 1 - grid_idx] = b'1';
    }

    let mut tilelist = Vec::new();
    tilelist.extend_from_slice(MAP_MAGIC_LIKE_CPP);
    tilelist.extend_from_slice(&MAP_VERSION_MAGIC_LIKE_CPP.to_le_bytes());
    tilelist.extend_from_slice(&0_u32.to_le_bytes());
    tilelist.extend_from_slice(&bitset_string);
    tilelist
}

fn unique_test_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rustycore-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}

#[path = "creature_runtime_tests/fixtures.rs"]
mod fixtures;
use fixtures::*;

#[path = "creature_runtime_tests/combat.rs"]
mod combat;
#[path = "creature_runtime_tests/creature_1.rs"]
mod creature_1;
#[path = "creature_runtime_tests/creature_2.rs"]
mod creature_2;
#[path = "creature_runtime_tests/creature_3.rs"]
mod creature_3;
#[path = "creature_runtime_tests/creature_4.rs"]
mod creature_4;
#[path = "creature_runtime_tests/creature_5.rs"]
mod creature_5;
#[path = "creature_runtime_tests/f6_8d1_phase_engine.rs"]
mod f6_8d1_phase_engine;
#[path = "creature_runtime_tests/instance.rs"]
mod instance;
#[path = "creature_runtime_tests/misc.rs"]
mod misc;
#[path = "creature_runtime_tests/movement.rs"]
mod movement;
#[path = "creature_runtime_tests/persistence.rs"]
mod persistence;
#[path = "creature_runtime_tests/spawn.rs"]
mod spawn;
#[path = "creature_runtime_tests/spell.rs"]
mod spell;
#[path = "creature_runtime_tests/visibility.rs"]
mod visibility;
