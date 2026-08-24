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
use wow_database::{
    CharStatements, CharacterDatabase, DatabaseError, PreparedStatement, StatementDef,
};
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
use wow_recastdetour::{
    CENTER_GRID_ID_LIKE_CPP, DetourNavMeshQueryError, DetourOwnerCapabilitiesLikeCpp,
    DetourPathOptions, DetourPathType, DetourPointPath, DetourPolyPath, DetourQueryFilterError,
    MAX_NUMBER_OF_GRIDS_LIKE_CPP, MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData,
    MMapManager as DetourMMapManager, MMapManagerError, PathQueryFilterContext,
    SIZE_OF_GRIDS_LIKE_CPP, ThreadUnsafeMapData, create_path_query_filter_like_cpp,
};

use crate::phasing::personal::MultiPersonalPhaseTracker;

/// Size of a grid cell in yards (64x64 yards like TrinityCore).
pub const GRID_SIZE: f32 = 64.0;

/// Visibility radius in yards (how far a player can see).
pub const VISIBILITY_RADIUS: f32 = 100.0;

/// C++ `BASE_ATTACK_TIME` (`UnitDefines.h:30`). Creature base/ranged attack time
/// is clamped to this when the template value is 0 (`ObjectMgr.cpp:1100-1104`); a
/// 0 attack time crashes the 3.4.3 client's swing-timer math on the first tick.
const BASE_ATTACK_TIME_LIKE_CPP: u32 = 2_000;

const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

const MAX_NUMBER_OF_CELLS_LIKE_CPP: i32 = 8;
const TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP: i32 =
    MAX_NUMBER_OF_GRIDS_LIKE_CPP * MAX_NUMBER_OF_CELLS_LIKE_CPP;
const SIZE_OF_GRID_CELL_LIKE_CPP: f32 =
    SIZE_OF_GRIDS_LIKE_CPP / MAX_NUMBER_OF_CELLS_LIKE_CPP as f32;
const CENTER_GRID_CELL_ID_LIKE_CPP: i32 = TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP / 2;
const CENTER_GRID_CELL_OFFSET_LIKE_CPP: f32 = SIZE_OF_GRID_CELL_LIKE_CPP / 2.0;

/// Default time before a grid unloads if no players are nearby (5 minutes).
pub const DEFAULT_GRID_UNLOAD_TIME: Duration = Duration::from_secs(300);

/// TrinityCore `TerrainInfo::GetMinHeight` fallback when no terrain grid is loaded.
///
/// Real grid-backed min-height data belongs to the terrain/map-data port; exposing
/// the fallback here lets movement preserve the C++ under-map branch without
/// inventing terrain values.
pub const DEFAULT_MIN_HEIGHT_LIKE_CPP: f32 = -500.0;

const MAP_MAGIC_LIKE_CPP: &[u8; 4] = b"MAPS";
const MAP_AREA_MAGIC_LIKE_CPP: &[u8; 4] = b"AREA";
const MAP_VERSION_MAGIC_LIKE_CPP: u32 = 10;
const MAP_FILE_HEADER_SIZE_LIKE_CPP: usize = 44;
const MAP_AREA_HEADER_SIZE_LIKE_CPP: usize = 8;
const MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP: u16 = 0x0001;
const MAP_AREA_CELLS_PER_GRID_LIKE_CPP: usize = 16;
const TERRAIN_GRID_COUNT_LIKE_CPP: usize =
    MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize * MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize;
const SMOOTH_PATH_STEP_SIZE_LIKE_CPP: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistedRespawnRowLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: u64,
    pub respawn_time: i64,
    pub map_id: u16,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PersistedRespawnLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub invalid_type: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyRespawnQueueReloadReportLikeCpp {
    pub rows: usize,
    pub timers_loaded: usize,
    pub creature_queued: usize,
    pub gameobject_loaded: usize,
    pub rejected_zero_spawn_id: usize,
    pub rejected_unsupported_type: usize,
    pub rejected_existing_later: usize,
    pub missing_creature_runtime: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyRespawnTimeAddOutcomeLikeCpp {
    Inserted,
    ReplacedExisting,
    RejectedZeroSpawnId,
    RejectedUnsupportedType,
    RejectedExistingSoonerOrEqual,
}

fn spawn_object_type_raw_like_cpp(object_type: SpawnObjectType) -> u16 {
    u16::from(object_type as u8)
}

pub fn respawn_time_from_instant_like_cpp(respawn_at: Instant, now: Instant, now_secs: i64) -> i64 {
    let delay_secs = respawn_at
        .checked_duration_since(now)
        .map(|delay| i64::try_from(delay.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    now_secs.saturating_add(delay_secs)
}

pub fn instant_from_respawn_time_like_cpp(
    respawn_time: i64,
    now: Instant,
    now_secs: i64,
) -> Instant {
    let delay_secs = respawn_time.saturating_sub(now_secs);
    if delay_secs <= 0 {
        return now;
    }

    let requested_delay_secs = u64::try_from(delay_secs).unwrap_or(u64::MAX);
    if let Some(deadline) = now.checked_add(Duration::from_secs(requested_delay_secs)) {
        return deadline;
    }

    // C++ uses `time_t::max()` as a never-respawn sentinel for some bosses.
    // `Instant` has a platform-specific upper bound; saturate to its farthest
    // representable future point instead of turning overflow into "ready now".
    let mut low = 0_u64;
    let mut high = requested_delay_secs;
    while low < high {
        let span = high - low;
        let midpoint = low + span / 2 + span % 2;
        if now.checked_add(Duration::from_secs(midpoint)).is_some() {
            low = midpoint;
        } else {
            high = midpoint - 1;
        }
    }
    now.checked_add(Duration::from_secs(low)).unwrap_or(now)
}

pub fn respawn_replace_statement_like_cpp(row: &PersistedRespawnRowLikeCpp) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::REP_RESPAWN);
    // C++ `Map::SaveRespawnInfoDB`: type, spawnId, respawnTime, mapId, instanceId.
    stmt.set_u16(0, spawn_object_type_raw_like_cpp(row.object_type));
    stmt.set_u64(1, row.spawn_id);
    stmt.set_i64(2, row.respawn_time);
    stmt.set_u16(3, row.map_id);
    stmt.set_u32(4, row.instance_id);
    stmt
}

pub fn respawn_delete_statement_like_cpp(
    object_type: SpawnObjectType,
    spawn_id: u64,
    map_id: u16,
    instance_id: u32,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::for_statement(CharStatements::DEL_RESPAWN);
    // C++ `Map::DeleteRespawnInfoFromDB`: type, spawnId, mapId, instanceId.
    stmt.set_u16(0, spawn_object_type_raw_like_cpp(object_type));
    stmt.set_u64(1, spawn_id);
    stmt.set_u16(2, map_id);
    stmt.set_u32(3, instance_id);
    stmt
}

pub async fn load_persisted_respawn_rows_for_map_like_cpp(
    character_db: &CharacterDatabase,
    map_id: u16,
    instance_id: u32,
) -> Result<
    (
        Vec<PersistedRespawnRowLikeCpp>,
        PersistedRespawnLoadReportLikeCpp,
    ),
    DatabaseError,
> {
    let mut stmt = character_db.prepare(CharStatements::SEL_RESPAWNS);
    stmt.set_u16(0, map_id);
    stmt.set_u32(1, instance_id);
    let mut result = character_db.query(&stmt).await?;
    let mut rows = Vec::new();
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    if result.is_empty() {
        return Ok((rows, report));
    }

    loop {
        report.rows += 1;
        let object_type_raw = result
            .try_read::<u16>(0)
            .or_else(|| result.try_read::<u8>(0).map(u16::from))
            .unwrap_or(u16::MAX);
        let Some(object_type) = u8::try_from(object_type_raw)
            .ok()
            .and_then(SpawnObjectType::from_raw)
        else {
            report.invalid_type += 1;
            if !result.next_row() {
                break;
            }
            continue;
        };

        rows.push(PersistedRespawnRowLikeCpp {
            object_type,
            spawn_id: result
                .try_read::<u64>(1)
                .or_else(|| result.try_read::<i64>(1).map(|value| value as u64))
                .unwrap_or(0),
            respawn_time: result.try_read::<i64>(2).unwrap_or(0),
            map_id,
            instance_id,
        });
        report.loaded += 1;

        if !result.next_row() {
            break;
        }
    }

    Ok((rows, report))
}

pub async fn execute_respawn_replace_like_cpp(
    character_db: &CharacterDatabase,
    row: &PersistedRespawnRowLikeCpp,
) -> Result<u64, DatabaseError> {
    character_db
        .execute(&respawn_replace_statement_like_cpp(row))
        .await
}

pub async fn execute_respawn_delete_like_cpp(
    character_db: &CharacterDatabase,
    object_type: SpawnObjectType,
    spawn_id: u64,
    map_id: u16,
    instance_id: u32,
) -> Result<u64, DatabaseError> {
    character_db
        .execute(&respawn_delete_statement_like_cpp(
            object_type,
            spawn_id,
            map_id,
            instance_id,
        ))
        .await
}

fn point_path_limit_for_distance_like_cpp(distance: f32) -> usize {
    let point_limit = if distance.is_sign_negative() {
        0
    } else {
        (distance / SMOOTH_PATH_STEP_SIZE_LIKE_CPP) as usize
    };
    point_limit.min(MAX_POINT_PATH_LENGTH_LIKE_CPP)
}

pub fn terrain_grid_coords_for_wow_position_like_cpp(x: f32, y: f32) -> (i32, i32) {
    let center_grid_offset = SIZE_OF_GRIDS_LIKE_CPP / 2.0;
    let x_offset = (x - center_grid_offset) / SIZE_OF_GRIDS_LIKE_CPP;
    let y_offset = (y - center_grid_offset) / SIZE_OF_GRIDS_LIKE_CPP;
    let grid_x = (x_offset + CENTER_GRID_ID_LIKE_CPP as f32 + 0.5) as i32;
    let grid_y = (y_offset + CENTER_GRID_ID_LIKE_CPP as f32 + 0.5) as i32;

    (
        (MAX_NUMBER_OF_GRIDS_LIKE_CPP - 1) - grid_x,
        (MAX_NUMBER_OF_GRIDS_LIKE_CPP - 1) - grid_y,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CellCoordLikeCpp {
    x: i32,
    y: i32,
}

fn compute_cell_coord_like_cpp(x: f32, y: f32) -> CellCoordLikeCpp {
    let x_offset = (f64::from(x) - f64::from(CENTER_GRID_CELL_OFFSET_LIKE_CPP))
        / f64::from(SIZE_OF_GRID_CELL_LIKE_CPP);
    let y_offset = (f64::from(y) - f64::from(CENTER_GRID_CELL_OFFSET_LIKE_CPP))
        / f64::from(SIZE_OF_GRID_CELL_LIKE_CPP);
    let x_coord = (x_offset + f64::from(CENTER_GRID_CELL_ID_LIKE_CPP) + 0.5) as i32;
    let y_coord = (y_offset + f64::from(CENTER_GRID_CELL_ID_LIKE_CPP) + 0.5) as i32;

    CellCoordLikeCpp {
        x: x_coord.clamp(0, TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP - 1),
        y: y_coord.clamp(0, TOTAL_NUMBER_OF_CELLS_PER_MAP_LIKE_CPP - 1),
    }
}

fn calculate_cell_area_like_cpp(
    position: Position,
    radius: f32,
) -> (CellCoordLikeCpp, CellCoordLikeCpp) {
    if radius <= 0.0 {
        let center = compute_cell_coord_like_cpp(position.x, position.y);
        return (center, center);
    }

    (
        compute_cell_coord_like_cpp(position.x - radius, position.y - radius),
        compute_cell_coord_like_cpp(position.x + radius, position.y + radius),
    )
}

fn cell_area_contains_position_like_cpp(
    low: CellCoordLikeCpp,
    high: CellCoordLikeCpp,
    position: Position,
) -> Option<CellCoordLikeCpp> {
    let coord = compute_cell_coord_like_cpp(position.x, position.y);
    (coord.x >= low.x && coord.x <= high.x && coord.y >= low.y && coord.y <= high.y)
        .then_some(coord)
}

pub fn terrain_map_id_for_phase_shift_like_cpp(
    phase_shift: &PhaseShift,
    map_id: u32,
    x: f32,
    y: f32,
    mut has_child_terrain_grid_file: impl FnMut(u32, i32, i32) -> bool,
) -> u32 {
    match phase_shift.visible_map_id_count_like_cpp() {
        0 => map_id,
        1 => phase_shift
            .visible_map_ids_like_cpp()
            .next()
            .unwrap_or(map_id),
        _ => {
            let (grid_x, grid_y) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
            phase_shift
                .visible_map_ids_like_cpp()
                .find(|visible_map_id| has_child_terrain_grid_file(*visible_map_id, grid_x, grid_y))
                .unwrap_or(map_id)
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerrainGridFilesLikeCpp {
    map_id: u32,
    grid_file_exists: Vec<bool>,
    child_terrain: Vec<TerrainGridFilesLikeCpp>,
}

impl TerrainGridFilesLikeCpp {
    pub fn load_root_like_cpp(
        data_dir: impl AsRef<Path>,
        map_id: u32,
        parent_child_map_data: &HashMap<u32, Vec<u32>>,
    ) -> io::Result<Self> {
        Self::load_impl_like_cpp(data_dir.as_ref(), map_id, parent_child_map_data)
    }

    fn load_impl_like_cpp(
        data_dir: &Path,
        map_id: u32,
        parent_child_map_data: &HashMap<u32, Vec<u32>>,
    ) -> io::Result<Self> {
        let grid_file_exists = discover_grid_map_files_like_cpp(data_dir, map_id)?;
        let mut child_terrain = Vec::new();
        if let Some(child_map_ids) = parent_child_map_data.get(&map_id) {
            for child_map_id in child_map_ids {
                child_terrain.push(Self::load_impl_like_cpp(
                    data_dir,
                    *child_map_id,
                    parent_child_map_data,
                )?);
            }
        }

        Ok(Self {
            map_id,
            grid_file_exists,
            child_terrain,
        })
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn has_grid_file_like_cpp(&self, gx: i32, gy: i32) -> bool {
        terrain_grid_bitset_index_like_cpp(gx, gy)
            .and_then(|idx| self.grid_file_exists.get(idx).copied())
            .unwrap_or(false)
    }

    pub fn has_child_terrain_grid_file_like_cpp(&self, map_id: u32, gx: i32, gy: i32) -> bool {
        self.child_terrain
            .iter()
            .find(|child_terrain| child_terrain.map_id == map_id)
            .is_some_and(|child_terrain| child_terrain.has_grid_file_like_cpp(gx, gy))
    }

    pub fn terrain_map_id_for_phase_shift_like_cpp(
        &self,
        phase_shift: &PhaseShift,
        source_map_id: u32,
        x: f32,
        y: f32,
    ) -> u32 {
        terrain_map_id_for_phase_shift_like_cpp(
            phase_shift,
            source_map_id,
            x,
            y,
            |map_id, gx, gy| self.has_child_terrain_grid_file_like_cpp(map_id, gx, gy),
        )
    }
}

#[derive(Debug)]
pub struct TerrainGridFileIndexLikeCpp {
    data_dir: PathBuf,
    parent_child_map_data: HashMap<u32, Vec<u32>>,
    parent_map_ids: HashMap<u32, u32>,
    terrain_maps: HashMap<u32, TerrainGridFilesLikeCpp>,
}

impl TerrainGridFileIndexLikeCpp {
    pub fn new(
        data_dir: impl AsRef<Path>,
        parent_child_map_data: impl IntoIterator<Item = (u32, Vec<u32>)>,
    ) -> Self {
        let parent_child_map_data: HashMap<u32, Vec<u32>> =
            parent_child_map_data.into_iter().collect();
        let mut parent_map_ids = HashMap::new();
        for (parent_map_id, child_map_ids) in &parent_child_map_data {
            for child_map_id in child_map_ids {
                parent_map_ids.insert(*child_map_id, *parent_map_id);
            }
        }

        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            parent_child_map_data,
            parent_map_ids,
            terrain_maps: HashMap::new(),
        }
    }

    pub fn root_map_id_like_cpp(&self, map_id: u32) -> u32 {
        let mut root_map_id = map_id;
        while let Some(parent_map_id) = self.parent_map_ids.get(&root_map_id).copied() {
            root_map_id = parent_map_id;
        }
        root_map_id
    }

    pub fn terrain_for_map_like_cpp(
        &mut self,
        map_id: u32,
    ) -> io::Result<&TerrainGridFilesLikeCpp> {
        let root_map_id = self.root_map_id_like_cpp(map_id);
        if !self.terrain_maps.contains_key(&root_map_id) {
            let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(
                &self.data_dir,
                root_map_id,
                &self.parent_child_map_data,
            )?;
            self.terrain_maps.insert(root_map_id, terrain);
        }

        Ok(self
            .terrain_maps
            .get(&root_map_id)
            .expect("terrain root inserted"))
    }

    pub fn terrain_map_id_for_phase_shift_like_cpp(
        &mut self,
        phase_shift: &PhaseShift,
        source_map_id: u32,
        x: f32,
        y: f32,
    ) -> u32 {
        if phase_shift.visible_map_id_count_like_cpp() == 0 {
            return source_map_id;
        }

        self.terrain_for_map_like_cpp(source_map_id)
            .map(|terrain| {
                terrain.terrain_map_id_for_phase_shift_like_cpp(phase_shift, source_map_id, x, y)
            })
            .unwrap_or(source_map_id)
    }
}

fn discover_grid_map_files_like_cpp(data_dir: &Path, map_id: u32) -> io::Result<Vec<bool>> {
    let tile_list_name = data_dir.join("maps").join(format!("{map_id:04}.tilelist"));
    if let Ok(mut tile_list) = File::open(tile_list_name) {
        let mut map_magic = [0_u8; 4];
        let mut version_magic = [0_u8; 4];
        let mut build = [0_u8; 4];
        let mut tiles_data = vec![0_u8; TERRAIN_GRID_COUNT_LIKE_CPP];
        if tile_list.read_exact(&mut map_magic).is_ok()
            && map_magic == *MAP_MAGIC_LIKE_CPP
            && tile_list.read_exact(&mut version_magic).is_ok()
            && u32::from_le_bytes(version_magic) == MAP_VERSION_MAGIC_LIKE_CPP
            && tile_list.read_exact(&mut build).is_ok()
            && tile_list.read_exact(&mut tiles_data).is_ok()
        {
            return Ok(terrain_grid_bitset_from_cpp_string_like_cpp(&tiles_data));
        }
    }

    let mut grid_file_exists = vec![false; TERRAIN_GRID_COUNT_LIKE_CPP];
    for gx in 0..MAX_NUMBER_OF_GRIDS_LIKE_CPP {
        for gy in 0..MAX_NUMBER_OF_GRIDS_LIKE_CPP {
            let idx = terrain_grid_bitset_index_like_cpp(gx, gy).expect("valid terrain grid index");
            grid_file_exists[idx] = exist_map_like_cpp(data_dir, map_id, gx, gy);
        }
    }
    Ok(grid_file_exists)
}

fn terrain_grid_bitset_index_like_cpp(gx: i32, gy: i32) -> Option<usize> {
    if !(0..MAX_NUMBER_OF_GRIDS_LIKE_CPP).contains(&gx)
        || !(0..MAX_NUMBER_OF_GRIDS_LIKE_CPP).contains(&gy)
    {
        return None;
    }

    Some(gx as usize * MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize + gy as usize)
}

fn terrain_grid_bitset_from_cpp_string_like_cpp(tiles_data: &[u8]) -> Vec<bool> {
    let mut grid_file_exists = vec![false; TERRAIN_GRID_COUNT_LIKE_CPP];
    for (idx, exists) in grid_file_exists.iter_mut().enumerate() {
        let string_idx = TERRAIN_GRID_COUNT_LIKE_CPP - 1 - idx;
        *exists = tiles_data.get(string_idx).copied() == Some(b'1');
    }
    grid_file_exists
}

fn exist_map_like_cpp(data_dir: &Path, map_id: u32, gx: i32, gy: i32) -> bool {
    let file_name = data_dir
        .join("maps")
        .join(format!("{map_id:04}_{gx:02}_{gy:02}.map"));
    let Ok(mut file) = File::open(file_name) else {
        return false;
    };

    let mut header = [0_u8; MAP_FILE_HEADER_SIZE_LIKE_CPP];
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    header[..4] == MAP_MAGIC_LIKE_CPP[..]
        && u32::from_le_bytes([header[4], header[5], header[6], header[7]])
            == MAP_VERSION_MAGIC_LIKE_CPP
}

pub fn terrain_grid_area_id_for_position_like_cpp(
    data_dir: impl AsRef<Path>,
    map_id: u32,
    x: f32,
    y: f32,
) -> io::Result<Option<u32>> {
    // `terrain_grid_coords_for_wow_position_like_cpp` already includes the
    // axis reversal performed by C++ `Map::EnsureGridCreated` before it asks
    // TerrainMgr for the extracted map tile (`Map.cpp:338-343`).
    let (gx, gy) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
    let file_name = data_dir
        .as_ref()
        .join("maps")
        .join(format!("{map_id:04}_{gx:02}_{gy:02}.map"));
    let mut file = match File::open(&file_name) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut header = [0_u8; MAP_FILE_HEADER_SIZE_LIKE_CPP];
    file.read_exact(&mut header)?;
    if header[..4] != MAP_MAGIC_LIKE_CPP[..]
        || u32::from_le_bytes([header[4], header[5], header[6], header[7]])
            != MAP_VERSION_MAGIC_LIKE_CPP
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid C++ terrain map header in {}", file_name.display()),
        ));
    }

    let area_map_offset = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    if area_map_offset == 0 {
        return Ok(None);
    }

    file.seek(SeekFrom::Start(u64::from(area_map_offset)))?;
    let mut area_header = [0_u8; MAP_AREA_HEADER_SIZE_LIKE_CPP];
    file.read_exact(&mut area_header)?;
    if area_header[..4] != MAP_AREA_MAGIC_LIKE_CPP[..] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid C++ terrain area header in {}", file_name.display()),
        ));
    }

    let flags = u16::from_le_bytes([area_header[4], area_header[5]]);
    let grid_area = u16::from_le_bytes([area_header[6], area_header[7]]);
    if flags & MAP_AREA_HEADER_FLAG_NO_AREA_LIKE_CPP != 0 {
        return Ok(Some(u32::from(grid_area)));
    }

    let mut area_map = [0_u16; MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP];
    let mut area_map_bytes = [0_u8;
        MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * std::mem::size_of::<u16>()];
    file.read_exact(&mut area_map_bytes)?;
    for (idx, chunk) in area_map_bytes.chunks_exact(2).enumerate() {
        area_map[idx] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }

    let x = MAP_AREA_CELLS_PER_GRID_LIKE_CPP as f32
        * (CENTER_GRID_ID_LIKE_CPP as f32 - x / SIZE_OF_GRIDS_LIKE_CPP);
    let y = MAP_AREA_CELLS_PER_GRID_LIKE_CPP as f32
        * (CENTER_GRID_ID_LIKE_CPP as f32 - y / SIZE_OF_GRIDS_LIKE_CPP);
    let lx = (x as i32 & 15) as usize;
    let ly = (y as i32 & 15) as usize;
    Ok(Some(u32::from(
        area_map[lx * MAP_AREA_CELLS_PER_GRID_LIKE_CPP + ly],
    )))
}

pub fn zone_and_area_for_position_like_cpp(
    data_dir: impl AsRef<Path>,
    map_id: u32,
    x: f32,
    y: f32,
    area_store: Option<&wow_data::AreaTableStore>,
    map_area_id_fallback: impl FnOnce(u32) -> u32,
) -> io::Result<(u32, u32)> {
    let area_id = terrain_grid_area_id_for_position_like_cpp(data_dir, map_id, x, y)?
        .filter(|area_id| *area_id != 0)
        .unwrap_or_else(|| map_area_id_fallback(map_id));

    let zone_id = area_store
        .and_then(|store| store.get(area_id))
        .filter(|area| area.parent_area_id != 0 && area.is_subzone_like_cpp())
        .map(|area| u32::from(area.parent_area_id))
        .unwrap_or(area_id);

    Ok((zone_id, area_id))
}

fn position_to_i32_tuple(position: Position) -> (i32, i32, i32) {
    (position.x as i32, position.y as i32, position.z as i32)
}

fn position_from_detour_point_like_cpp(point: [f32; 3]) -> Position {
    Position::new(point[0], point[1], point[2], 0.0)
}

fn position_to_wow_point_like_cpp(position: Position) -> [f32; 3] {
    [position.x, position.y, position.z]
}

#[derive(Debug, PartialEq)]
pub enum WorldDetourPathError {
    Filter(DetourQueryFilterError),
    Query(DetourNavMeshQueryError),
    MMap(String),
}

impl From<DetourQueryFilterError> for WorldDetourPathError {
    fn from(value: DetourQueryFilterError) -> Self {
        Self::Filter(value)
    }
}

impl From<DetourNavMeshQueryError> for WorldDetourPathError {
    fn from(value: DetourNavMeshQueryError) -> Self {
        Self::Query(value)
    }
}

impl From<MMapManagerError> for WorldDetourPathError {
    fn from(value: MMapManagerError) -> Self {
        Self::MMap(value.to_string())
    }
}

#[derive(Debug)]
pub struct WorldMMapPathfinderLikeCpp {
    data_dir: PathBuf,
    mmap_manager: DetourMMapManager,
    terrain_grid_file_index: TerrainGridFileIndexLikeCpp,
}

impl WorldMMapPathfinderLikeCpp {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        Self {
            terrain_grid_file_index: TerrainGridFileIndexLikeCpp::new(&data_dir, []),
            data_dir,
            mmap_manager: DetourMMapManager::new(),
        }
    }

    pub fn new_with_parent_map_data_like_cpp(
        data_dir: impl AsRef<Path>,
        parent_child_map_data: impl IntoIterator<Item = (u32, Vec<u32>)>,
    ) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        let parent_child_map_data: Vec<(u32, Vec<u32>)> =
            parent_child_map_data.into_iter().collect();
        let mut mmap_manager = DetourMMapManager::new();
        mmap_manager.initialize_thread_unsafe(parent_child_map_data.iter().cloned().map(
            |(map_id, child_map_ids)| ThreadUnsafeMapData {
                map_id,
                child_map_ids,
            },
        ));
        Self {
            terrain_grid_file_index: TerrainGridFileIndexLikeCpp::new(
                &data_dir,
                parent_child_map_data,
            ),
            data_dir,
            mmap_manager,
        }
    }

    pub fn calculate_creature_path_like_cpp(
        &mut self,
        creature: &WorldCreature,
        destination: Position,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
        filter_context: PathQueryFilterContext,
        force_destination: bool,
    ) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
        let creature_position = creature.position();
        let owner = creature.detour_owner_capabilities_like_cpp();
        self.calculate_path_from_positions_like_cpp(
            creature_position,
            destination,
            mesh_map_id,
            instance_map_id,
            instance_id,
            filter_context,
            owner,
            &[],
            force_destination,
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
        )
    }

    pub fn calculate_path_from_positions_like_cpp(
        &mut self,
        start: Position,
        destination: Position,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
        filter_context: PathQueryFilterContext,
        owner: DetourOwnerCapabilitiesLikeCpp,
        previous_poly_refs: &[u64],
        force_destination: bool,
        point_path_limit: usize,
    ) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
        let context = self
            .mmap_manager
            .load_pathfinding_context_for_wow_position_like_cpp(
                &self.data_dir,
                mesh_map_id,
                instance_map_id,
                instance_id,
                start.x,
                start.y,
            )?;

        if !context.map_data_available
            || !context.instance_query_available
            || !context.tile_available
        {
            return Ok(None);
        }

        // `PathGenerator::CalculatePath` requires `HaveTile(start)` *and*
        // `HaveTile(dest)` (`PathGenerator.cpp:80-81`). C++ satisfies both
        // because `TerrainInfo::LoadMMap` pushes a grid's `.mmtile` into
        // `MMapManager` as the grid loads (`TerrainMgr.cpp:174-184,237-247`),
        // whereas RustyCore has no grid-driven mmtile load and resolves tiles on
        // demand from the path request. Without also demand-loading the
        // destination tile, any destination in a neighbouring tile would report
        // "no navmesh" and silently degrade to a straight line.
        if !self
            .mmap_manager
            .load_pathfinding_context_for_wow_position_like_cpp(
                &self.data_dir,
                mesh_map_id,
                instance_map_id,
                instance_id,
                destination.x,
                destination.y,
            )?
            .tile_available
        {
            return Ok(None);
        }

        let Some(mmap_data) = self.mmap_manager.get_mmap_data(mesh_map_id) else {
            return Ok(None);
        };
        let filter = create_path_query_filter_like_cpp(filter_context)?;
        mmap_data
            .calculate_path_for_instance_with_previous_path_like_cpp(
                instance_map_id,
                instance_id,
                &filter,
                position_to_wow_point_like_cpp(start),
                position_to_wow_point_like_cpp(destination),
                DetourPathOptions {
                    point_path_limit,
                    force_destination,
                    owner,
                    ..DetourPathOptions::default()
                },
                previous_poly_refs,
            )
            .map_err(WorldDetourPathError::from)
    }

    pub fn resolve_mesh_map_id_for_path_request_like_cpp(
        &mut self,
        request: &WorldMMapPathRequestLikeCpp,
    ) -> u32 {
        self.terrain_grid_file_index
            .terrain_map_id_for_phase_shift_like_cpp(
                &request.phase_shift,
                request.mesh_map_id,
                request.start.x,
                request.start.y,
            )
    }

    pub fn mmap_manager(&self) -> &DetourMMapManager {
        &self.mmap_manager
    }
}

/// Everything a creature path query needs, assembled by the movement bridge at
/// the moment of the query.
///
/// C++ constructs a fresh `PathGenerator` per query and therefore runs
/// `CreateFilter` — and reads the generator's retained `_pathPolyRefs` — *after*
/// whatever state transition triggered the movement. Capturing those inputs in
/// the caller before the bridge runs would sample them one step too early: a
/// creature entering evade would path without `NAV_GROUND_STEEP`, and a chase
/// that just switched victim would reuse the previous victim's corridor.
#[derive(Debug, Clone, PartialEq)]
pub struct CreaturePathQueryLikeCpp {
    pub start: Position,
    pub destination: Position,
    pub point_path_limit: usize,
    /// C++ `CalculatePath(x, y, z, forceDest)`.
    pub force_destination: bool,
    /// C++ `PathGenerator::CreateFilter` + `UpdateFilter`, sampled now.
    pub filter_context: PathQueryFilterContext,
    /// C++ `BuildPolyPath`'s owner reads, sampled now.
    pub owner: DetourOwnerCapabilitiesLikeCpp,
    /// The corridor this generator still holds, after any reset this tick.
    pub previous_poly_refs: Vec<u64>,
}

/// Live snapshot of a chase victim, taken by the tick driver before the creature
/// is borrowed mutably. C++ `ChaseMovementGenerator` holds a live `Unit*`; the
/// Rust runtime has no object accessor inside the creature step, so the caller
/// supplies the same facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseTargetSnapshotLikeCpp {
    pub guid: ObjectGuid,
    pub position: Position,
    pub combat_reach: f32,
    pub in_world: bool,
    /// C++ `Unit::isInAccessiblePlaceFor(Creature const*)` branches on the
    /// victim's `IsInWater()`: in water it asks the chaser's `CanEnterWater()`,
    /// otherwise `CanWalk() || CanFly()`.
    ///
    /// `None` means the runtime cannot answer it for this victim — creature
    /// entities carry no liquid state and the terrain layer exposes heights
    /// only. See `chase_unit_snapshot_like_cpp` for how that is degraded.
    pub in_water: Option<bool>,
}

/// C++ `NOMINAL_MELEE_RANGE` (`ObjectDefines.h:44`).
const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;

/// C++ `Position::GetAbsoluteAngle`: the world bearing from `from` to `to`.
fn absolute_angle_like_cpp(from: Position, to: Position) -> f32 {
    wow_movement::normalize_orientation_like_cpp((to.y - from.y).atan2(to.x - from.x))
}

/// What one chase tick produced for the caller to publish.
#[derive(Debug, Clone, PartialEq)]
pub enum ChaseTickOutcomeLikeCpp {
    /// Nothing to send this tick.
    Idle,
    /// A superseded spline was stopped; publish `SMSG_ON_MONSTER_MOVE` stop.
    Stopped(MoveSplineStopResult),
    /// A new chase spline was launched.
    Launched(Position, MoveSpline),
}

#[derive(Debug, Clone)]
pub struct WorldMMapPathRequestLikeCpp {
    pub start: Position,
    pub destination: Position,
    pub mesh_map_id: u32,
    pub instance_map_id: u32,
    pub instance_id: u32,
    pub filter_context: PathQueryFilterContext,
    /// C++ reads these off `_source` inside `BuildPolyPath`; see
    /// `WorldCreature::detour_owner_capabilities_like_cpp`.
    pub owner: DetourOwnerCapabilitiesLikeCpp,
    /// The corridor this owner's generator still holds, i.e. C++
    /// `PathGenerator::_pathPolyRefs` on a `PathGenerator` that outlived the
    /// previous update (`PathGenerator.cpp:291-413`). Empty means "freshly
    /// constructed generator", which is what `MoveSplineInit::MoveTo` does.
    pub previous_poly_refs: Vec<u64>,
    pub force_destination: bool,
    pub point_path_limit: usize,
    pub phase_shift: PhaseShift,
}

#[derive(Debug)]
pub struct WorldMMapPathfinderWorkerLikeCpp {
    request_tx: mpsc::Sender<WorldMMapPathfinderMessageLikeCpp>,
}

#[derive(Debug)]
struct WorldMMapPathfinderMessageLikeCpp {
    request: WorldMMapPathRequestLikeCpp,
    response_tx: mpsc::Sender<Result<Option<DetourPolyPath>, WorldDetourPathError>>,
}

impl WorldMMapPathfinderWorkerLikeCpp {
    pub fn spawn(data_dir: impl AsRef<Path>) -> Self {
        Self::spawn_with_pathfinder_factory(data_dir, WorldMMapPathfinderLikeCpp::new)
    }

    pub fn spawn_with_parent_map_data_like_cpp(
        data_dir: impl AsRef<Path>,
        parent_child_map_data: Vec<(u32, Vec<u32>)>,
    ) -> Self {
        Self::spawn_with_pathfinder_factory(data_dir, move |data_dir| {
            WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
                data_dir,
                parent_child_map_data,
            )
        })
    }

    fn spawn_with_pathfinder_factory(
        data_dir: impl AsRef<Path>,
        pathfinder_factory: impl FnOnce(PathBuf) -> WorldMMapPathfinderLikeCpp + Send + 'static,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<WorldMMapPathfinderMessageLikeCpp>();
        let data_dir = data_dir.as_ref().to_path_buf();
        thread::Builder::new()
            .name("world-mmap-pathfinder-like-cpp".to_string())
            .spawn(move || {
                let mut pathfinder = pathfinder_factory(data_dir);
                while let Ok(message) = request_rx.recv() {
                    let request = message.request;
                    let mesh_map_id =
                        pathfinder.resolve_mesh_map_id_for_path_request_like_cpp(&request);
                    let result = pathfinder.calculate_path_from_positions_like_cpp(
                        request.start,
                        request.destination,
                        mesh_map_id,
                        request.instance_map_id,
                        request.instance_id,
                        request.filter_context,
                        request.owner,
                        &request.previous_poly_refs,
                        request.force_destination,
                        request.point_path_limit,
                    );
                    let _ = message.response_tx.send(result);
                }
            })
            .expect("spawn mmap pathfinder worker");

        Self { request_tx }
    }

    pub fn calculate_path_like_cpp(
        &self,
        request: WorldMMapPathRequestLikeCpp,
    ) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
        let (response_tx, response_rx) = mpsc::channel();
        self.request_tx
            .send(WorldMMapPathfinderMessageLikeCpp {
                request,
                response_tx,
            })
            .map_err(|error| WorldDetourPathError::MMap(error.to_string()))?;
        response_rx
            .recv()
            .map_err(|error| WorldDetourPathError::MMap(error.to_string()))?
    }
}

pub fn path_type_from_detour_like_cpp(path_type: DetourPathType) -> PathType {
    PathType::from_bits_retain(path_type.bits())
}

fn random_path_result_from_path_type_like_cpp(path_type: PathType) -> RandomPathResult {
    if path_type.contains(PathType::NOPATH) {
        RandomPathResult::NoPath
    } else if path_type.contains(PathType::SHORTCUT) {
        RandomPathResult::Shortcut
    } else if path_type.intersects(PathType::FARFROMPOLY) {
        RandomPathResult::FarFromPoly
    } else {
        RandomPathResult::Success
    }
}

/// C++ `PathGenerator::CalculatePath` (`PathGenerator.cpp:79-86`) does **not**
/// report a failure when the map carries no usable navmesh for this query
/// (`!_navMesh || !_navMeshQuery || !HaveTile(start) || !HaveTile(dest)`): it
/// calls `BuildShortcut()` and returns `true` with
/// `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`. Because that type carries
/// neither `PATHFIND_NOPATH` nor `PATHFIND_SHORTCUT`, callers such as
/// `RandomMovementGenerator<Creature>::SetRandomLocation`
/// (`RandomMovementGenerator.cpp:145-153`) still launch the two-point path
/// instead of standing still.
///
/// `BuildShortcut` (`PathGenerator.cpp:630-646`) is exactly "current position →
/// requested destination, then `NormalizePath()`", so the caller's normalizer
/// still runs over both points.
pub fn detour_path_without_navmesh_like_cpp(
    start: Position,
    destination: Position,
) -> DetourPolyPath {
    DetourPolyPath {
        poly_refs: Vec::new(),
        point_path: DetourPointPath {
            points: vec![
                position_to_wow_point_like_cpp(start),
                position_to_wow_point_like_cpp(destination),
            ],
            actual_end: position_to_wow_point_like_cpp(destination),
            path_type: DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    }
}

pub fn path_generator_from_detour_like_cpp(
    start: Position,
    destination: Position,
    detour_path: &DetourPolyPath,
    force_destination: bool,
) -> PathGenerator {
    path_generator_from_detour_with_normalizer_like_cpp(
        start,
        destination,
        detour_path,
        force_destination,
        |point| point,
    )
}

pub fn path_generator_from_detour_with_normalizer_like_cpp(
    start: Position,
    destination: Position,
    detour_path: &DetourPolyPath,
    force_destination: bool,
    mut normalize_position: impl FnMut(Position) -> Position,
) -> PathGenerator {
    let mut path = PathGenerator::new();
    let actual_end = normalize_position(position_from_detour_point_like_cpp(
        detour_path.point_path.actual_end,
    ));
    path.apply_detour_path_like_cpp(
        start,
        destination,
        actual_end,
        detour_path
            .point_path
            .points
            .iter()
            .copied()
            .map(position_from_detour_point_like_cpp)
            .map(normalize_position),
        &detour_path.poly_refs,
        path_type_from_detour_like_cpp(detour_path.point_path.path_type),
        force_destination,
    );
    path
}

pub fn calculate_creature_detour_path_like_cpp(
    creature: &WorldCreature,
    destination: Position,
    mmap_data: Option<&MMapData>,
    instance_map_id: u32,
    instance_id: u32,
    filter_context: PathQueryFilterContext,
    force_destination: bool,
) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
    let Some(mmap_data) = mmap_data else {
        return Ok(None);
    };

    let filter = create_path_query_filter_like_cpp(filter_context)?;
    mmap_data
        .calculate_path_for_instance_like_cpp(
            instance_map_id,
            instance_id,
            &filter,
            position_to_wow_point_like_cpp(creature.position()),
            position_to_wow_point_like_cpp(destination),
            DetourPathOptions {
                point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                force_destination,
                owner: creature.detour_owner_capabilities_like_cpp(),
                ..DetourPathOptions::default()
            },
        )
        .map_err(WorldDetourPathError::from)
}

/// Coordinate of a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i16,
    pub y: i16,
}

impl GridCoord {
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub fn personal_phase_grid_id_like_cpp(&self) -> u16 {
        (i32::from(self.x) * MAX_NUMBER_OF_GRIDS_LIKE_CPP + i32::from(self.y)) as u16
    }

    /// Get surrounding coordinates in a 3x3 area (including self).
    pub fn surrounding(&self) -> Vec<GridCoord> {
        let mut coords = Vec::with_capacity(9);
        for dx in -1..=1 {
            for dy in -1..=1 {
                coords.push(GridCoord::new(self.x + dx, self.y + dy));
            }
        }
        coords
    }

    /// Check if another coordinate is within a given range.
    pub fn distance_squared(&self, other: &GridCoord) -> i32 {
        let dx = (self.x - other.x) as i32;
        let dy = (self.y - other.y) as i32;
        dx * dx + dy * dy
    }
}

/// Runtime selector proxy for an active generator whose concrete lifecycle
/// still lives in `wow_entities::MotionSubsystem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeRepresentedActiveKeyLikeCpp {
    kind: RuntimeMovementGeneratorType,
    mode: RuntimeMovementGeneratorMode,
    priority: RuntimeMovementGeneratorPriority,
    base_unit_state: u32,
}

#[derive(Debug)]
struct RuntimeRepresentedActiveGeneratorLikeCpp {
    state: RuntimeMovementGeneratorState,
    kind: RuntimeMovementGeneratorType,
}

impl RuntimeRepresentedActiveGeneratorLikeCpp {
    fn from_represented(generator: MovementGeneratorRef) -> Option<Self> {
        let kind = RuntimeMovementGeneratorType::from_trinity_id(generator.kind.trinity_id())?;
        let mode = match generator.mode {
            wow_entities::MovementGeneratorMode::Default => RuntimeMovementGeneratorMode::Default,
            wow_entities::MovementGeneratorMode::Override => RuntimeMovementGeneratorMode::Override,
        };
        let priority = match generator.priority {
            wow_entities::MovementGeneratorPriority::None => RuntimeMovementGeneratorPriority::None,
            wow_entities::MovementGeneratorPriority::Normal => {
                RuntimeMovementGeneratorPriority::Normal
            }
            wow_entities::MovementGeneratorPriority::Highest => {
                RuntimeMovementGeneratorPriority::Highest
            }
        };
        Some(Self {
            state: RuntimeMovementGeneratorState {
                mode,
                priority,
                flags: RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING,
                base_unit_state: generator.base_unit_state,
            },
            kind,
        })
    }

    const fn key(&self) -> RuntimeRepresentedActiveKeyLikeCpp {
        RuntimeRepresentedActiveKeyLikeCpp {
            kind: self.kind,
            mode: self.state.mode,
            priority: self.state.priority,
            base_unit_state: self.state.base_unit_state,
        }
    }
}

impl RuntimeMovementGenerator for RuntimeRepresentedActiveGeneratorLikeCpp {
    fn state(&self) -> &RuntimeMovementGeneratorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut RuntimeMovementGeneratorState {
        &mut self.state
    }

    fn kind(&self) -> RuntimeMovementGeneratorType {
        self.kind
    }

    fn initialize(&mut self) {
        self.state.flags.remove(
            RuntimeMovementGeneratorFlags::INITIALIZATION_PENDING
                | RuntimeMovementGeneratorFlags::DEACTIVATED,
        );
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::INITIALIZED);
    }

    fn reset(&mut self) {
        self.initialize();
    }

    fn update(&mut self, _diff_ms: u32) -> bool {
        !self
            .state
            .flags
            .contains(RuntimeMovementGeneratorFlags::FINALIZED)
    }

    fn deactivate(&mut self) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::DEACTIVATED);
    }

    fn finalize(&mut self, _active: bool, _movement_inform: bool) {
        self.state
            .flags
            .insert(RuntimeMovementGeneratorFlags::FINALIZED);
    }
}

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
    clock_started_at: Instant,
}

#[derive(Debug, Clone, Copy)]
struct ActiveTauntLikeCpp {
    caster: ObjectGuid,
    /// `None` represents C++/DB2's permanent duration sentinel `-1`.
    due_at_ms: Option<u64>,
    spell_id: u32,
    effect_mask: u32,
    slot: u8,
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
            clock_started_at: self.clock_started_at,
        }
    }
}

/// A grid cell containing creatures and player references.
#[derive(Debug)]
pub struct Grid {
    pub coord: GridCoord,
    pub creatures: HashMap<ObjectGuid, WorldCreature>,
    pub player_guids: HashSet<ObjectGuid>,
    pub last_player_time: Instant,
    pub loaded: bool,
}

impl Grid {
    pub fn new(x: i16, y: i16) -> Self {
        Self {
            coord: GridCoord::new(x, y),
            creatures: HashMap::new(),
            player_guids: HashSet::new(),
            last_player_time: Instant::now(),
            loaded: true,
        }
    }

    pub fn add_creature(&mut self, creature: WorldCreature) -> bool {
        if self.creatures.contains_key(&creature.guid()) {
            warn!(
                "Creature {:?} already exists in grid {:?}",
                creature.guid(),
                self.coord
            );
            return false;
        }
        self.creatures.insert(creature.guid(), creature);
        true
    }

    pub fn remove_creature(&mut self, guid: ObjectGuid) -> bool {
        self.creatures.remove(&guid).is_some()
    }

    pub fn get_creature(&self, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.creatures.get(&guid)
    }

    pub fn get_creature_mut(&mut self, guid: ObjectGuid) -> Option<&mut WorldCreature> {
        self.creatures.get_mut(&guid)
    }

    pub fn player_enter(&mut self, guid: ObjectGuid) {
        self.player_guids.insert(guid);
        self.last_player_time = Instant::now();
    }

    pub fn player_leave(&mut self, guid: ObjectGuid) {
        self.player_guids.remove(&guid);
    }

    pub fn should_unload(&self, timeout: Duration) -> bool {
        self.player_guids.is_empty() && self.last_player_time.elapsed() > timeout
    }

    pub fn creature_count(&self) -> usize {
        self.creatures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.creatures.is_empty() && self.player_guids.is_empty()
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

/// Who owns the creature/combat tick for a given map at runtime.
///
/// Test/local default is `Session`: each logged-in session drives its own
/// creature and combat ticks. Production startup flips this to `GlobalLegacy`
/// by default so a global map clock drives creature runtime like C++ and
/// session-level creature ticks are skipped to avoid double resolution.
///
/// The owner lives on the shared [`MapManager`] so all sessions on the same
/// map read the same value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RuntimeTickOwner {
    /// Per-session tick for isolated tests and explicit local diagnostics.
    #[default]
    Session,
    /// Global legacy-manager tick used by production startup by default.
    GlobalLegacy,
}

/// Initial session-local packet seam produced by `run_creatures_tick` /
/// `run_combat_tick`.
///
/// **This is a seam initial, NOT the final fanout design.** It holds only the
/// raw bytes that today are sent via `send_tx`. The real global fanout (Slice
/// 5+) will need per-destination routing, not a flat byte list.
///
/// If side effects other than packet bytes are discovered during extraction
/// they must be modelled separately, NOT smuggled through this flush path.
pub struct RuntimeOutput {
    /// Packets to be flushed to the session channel in order.
    pub packets: Vec<Vec<u8>>,
}

impl RuntimeOutput {
    pub fn new() -> Self {
        Self {
            packets: Vec::new(),
        }
    }
}

impl Default for RuntimeOutput {
    fn default() -> Self {
        Self::new()
    }
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

/// Candidate-routing rule for a [`RuntimeEvent`].
///
/// Each variant maps to one of the C++ `MessageDistDeliverer` distribution
/// modes.  The final HaveAtClient / phase gate is applied by each session
/// (`SendIfVisibleLikeCpp`, Slice 4A.1b) — do NOT duplicate visibility or
/// phase logic here.
#[derive(Debug, Clone, PartialEq)]
pub enum RecipientRule {
    /// Broadcast to all sessions whose visible range overlaps the source
    /// position.  Mirrors C++ `MessageDistDeliverer` with a range constraint.
    NearbyVisible {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
    },
    /// Same visibility fanout as `NearbyVisible`, but committed combat
    /// transitions are queued on each session's durable FIFO rail.
    NearbyVisibleDurable {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
    },
    /// One creature spell cast whose START plus viewer-selected basic/full GO
    /// frames must be published and consumed as a single durable unit.
    ///
    /// `RuntimeEvent::packet_bytes` carries START and the two GO fields carry
    /// the basic/full alternatives. Routing and all companion payloads are
    /// deliberately coupled here until `RuntimeEvent` grows a first-class
    /// packet-batch payload; independent events would allow a session drain
    /// between the two committed frames.
    NearbyVisibleDurableSpellCast {
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
        required_3d: bool,
        basic_go_packet_bytes: Vec<u8>,
        full_go_packet_bytes: Vec<u8>,
    },
    /// Broadcast to every session on the map regardless of distance.
    /// Mirrors C++ map-wide broadcast.
    MapBroadcastVisible { map_id: u16, instance_id: u32 },
    /// Send to exactly one player session identified by GUID.
    ExplicitPlayer(ObjectGuid),
    /// Send only to the session that owns the source entity (self-delivery).
    SelfOnly,
}

/// A single routing-annotated packet produced during a tick.
///
/// `packet_bytes` is the already-serialised wire payload.  The routing
/// decision (who receives it) is encoded in `recipients`.
#[derive(Debug, Clone)]
pub struct RuntimeEvent {
    pub source_guid: ObjectGuid,
    pub recipients: RecipientRule,
    pub packet_bytes: Vec<u8>,
}

/// An ordered list of [`RuntimeEvent`]s produced by a single tick pass,
/// ready to be consumed by a routing layer (Slice 4A.1b+).
#[derive(Debug, Clone, Default)]
pub struct RuntimePlan {
    pub events: Vec<RuntimeEvent>,
}

/// Which C++ `Unit::Set*AnimKitId` setter to mirror.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureAnimKitSlotLikeCpp {
    Ai,
    Movement,
    Melee,
}

impl RuntimeOutput {
    /// Convert this `RuntimeOutput` into a [`RuntimePlan`] where every packet
    /// is addressed to the owning session only (`RecipientRule::SelfOnly`).
    ///
    /// This is the minimal bridge used by a single-session tick caller that
    /// still handles its own delivery.  Packet order is preserved exactly.
    /// `RuntimeOutput` itself is consumed (not cloned); the Slice 3 flush path
    /// (`flush_runtime_output`) is left untouched.
    pub fn into_owning_session_plan(self, source_guid: ObjectGuid) -> RuntimePlan {
        let events = self
            .packets
            .into_iter()
            .map(|packet_bytes| RuntimeEvent {
                source_guid,
                recipients: RecipientRule::SelfOnly,
                packet_bytes,
            })
            .collect();
        RuntimePlan { events }
    }
}

/// Live, file-backed terrain height for the legacy world runtime.
///
/// One [`GridMapTerrain`] per map id (created lazily, shared across that map's
/// instances), mirroring C++ `TerrainInfo` ownership. Rooted at the server's
/// `DataDir`; tiles load on first query. This is the seam that lets the live
/// spawn/respawn path ground-snap creatures with real `.map` heights.
#[derive(Debug)]
pub struct LiveTerrainHeights {
    data_dir: PathBuf,
    static_vmap_los: Option<SharedStaticVMapLineOfSightProvider>,
    per_map: Mutex<HashMap<u32, Arc<GridMapTerrain>>>,
}

impl LiveTerrainHeights {
    #[must_use]
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self::new_with_optional_static_vmap_line_of_sight(data_dir, None)
    }

    /// Build the live terrain cache with a static VMAP LOS provider wired into
    /// every lazily-created map terrain.
    ///
    /// C++ startup creates/configures one `VMapManager2` and each map's
    /// `TerrainInfo`/`Map::isInLineOfSight` consults it for static geometry. This
    /// keeps the Rust live cache usable by a real provider once the VMAP
    /// `StaticMapTree` parser owns model geometry; without a provider, LOS
    /// remains C++'s disabled/missing-tree clear fallback.
    #[must_use]
    pub fn new_with_static_vmap_line_of_sight(
        data_dir: impl AsRef<Path>,
        provider: SharedStaticVMapLineOfSightProvider,
    ) -> Self {
        Self::new_with_optional_static_vmap_line_of_sight(data_dir, Some(provider))
    }

    #[must_use]
    fn new_with_optional_static_vmap_line_of_sight(
        data_dir: impl AsRef<Path>,
        static_vmap_los: Option<SharedStaticVMapLineOfSightProvider>,
    ) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_path_buf(),
            static_vmap_los,
            per_map: Mutex::new(HashMap::new()),
        }
    }

    fn terrain_for_map(&self, map_id: u32) -> Arc<GridMapTerrain> {
        let mut per_map = self.per_map.lock().expect("live terrain cache poisoned");
        Arc::clone(per_map.entry(map_id).or_insert_with(|| {
            let terrain = GridMapTerrain::new(map_id, &self.data_dir);
            let terrain = match &self.static_vmap_los {
                Some(provider) => terrain.with_static_vmap_line_of_sight(Arc::clone(provider)),
                None => terrain,
            };
            Arc::new(terrain)
        }))
    }

    /// C++ `Map::GetHeight` (no VMap/GO-floor): the raw `.map` ground at `(x, y)`,
    /// accepted only when the probe `z` is at/above it. The caller supplies the
    /// already-offset probe `z` (matching `WorldObject::GetMapHeight`).
    #[must_use]
    pub fn static_height_like_cpp(&self, map_id: u32, x: f32, y: f32, z: f32) -> f32 {
        self.terrain_for_map(map_id).static_height(x, y, z)
    }

    /// Raw `.map` surface height at `(x, y)`, without the C++ probe-Z acceptance
    /// gate. This is intentionally not a general `Map::GetHeight` replacement;
    /// creature path normalization uses it only as a guard for Rust MMap points
    /// that arrived below the client-visible terrain surface.
    #[must_use]
    pub fn grid_height_like_cpp(&self, map_id: u32, x: f32, y: f32) -> f32 {
        self.terrain_for_map(map_id).grid_height(x, y)
    }

    /// C++ `WorldObject::IsWithinLOSInMap` static-VMAP portion for two objects
    /// already known to belong to the same legacy map instance.
    #[must_use]
    pub fn is_within_los_like_cpp(
        &self,
        map_id: u32,
        source: &wow_entities::WorldObject,
        target: &wow_entities::WorldObject,
    ) -> bool {
        self.terrain_for_map(map_id).line_of_sight(
            wow_entities::LineOfSightQuery::to_object_like_cpp(
                source,
                target,
                wow_entities::LineOfSightOptions::default(),
            ),
        )
    }
}

/// Ground-snap a freshly respawned creature, like C++ `Creature::Respawn`'s
/// `UpdateAllowedPositionZ` + `SetHomePosition` (`Creature.cpp:461`).
///
/// No-op (Z untouched) when terrain has no tile/ground under the position, so the
/// no-terrain runtime path is byte-identical to before this wiring. Flyers keep
/// their altitude (raise-only); grounded creatures sit on `ground + hover`.
pub fn snap_respawn_creature_to_ground_like_cpp(
    world_creature: &mut WorldCreature,
    map_id: u16,
    terrain: &LiveTerrainHeights,
) {
    let creature = &world_creature.creature;
    let pos = creature.unit().world().position();
    // C++ `Unit::GetHoverOffset()`: hover height only while the HOVER flag is set.
    let hover_offset = if creature
        .movement_flags_like_cpp()
        .contains(MovementFlag::HOVER)
    {
        creature.unit().data().hover_height
    } else {
        0.0
    };
    let caps = AllowedPositionZCaps {
        // Transport passengers recompute position from the transport elsewhere;
        // the legacy respawn path does not model on-transport spawns.
        on_transport: false,
        can_fly: creature.can_fly_like_cpp(),
        can_swim: creature.can_swim_like_cpp(),
        hover_offset,
    };

    // C++ `GetMapHeight` offsets the probe by `Z_OFFSET_FIND_HEIGHT` before the
    // terrain lookup.
    let probe_z = pos.z + Z_OFFSET_FIND_HEIGHT;
    let ground = terrain.static_height_like_cpp(u32::from(map_id), pos.x, pos.y, probe_z);
    let new_z = allowed_position_z_from_ground_like_cpp(true, ground, pos.z, caps);
    if new_z != pos.z {
        let snapped = Position::new(pos.x, pos.y, new_z, pos.orientation);
        world_creature
            .creature
            .unit_mut()
            .world_mut()
            .relocate(snapped);
        world_creature.creature.set_ai_home_position(snapped);
    }
}

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

/// Read the runtime tick owner from a shared manager under one poison policy.
///
/// The owner decided who ticks creatures, and the two readers disagreed about a
/// poisoned lock. The session read it as `mm.read().ok()` falling back to
/// [`RuntimeTickOwner::Session`], while every tick body reads through the poison
/// with `unwrap_or_else(|poisoned| poisoned.into_inner())`. A poisoned legacy
/// lock therefore told the session "you own the tick" and the global loop "you
/// own the tick" at the same time, and the creature resolved twice (#28).
///
/// One function, one policy: read through the poison, exactly as the tick
/// bodies do. A poisoned lock means some thread panicked mid-mutation, which is
/// a reason to disconnect a session — not a reason to silently hand ownership
/// back to it.
///
/// Returning a `Copy` value is the other half of the guarantee. The owner cannot
/// be read while holding a guard, so no caller can acquire the canonical map
/// lock underneath a legacy one just to find out who owns the tick.
#[must_use]
pub fn shared_runtime_tick_owner_like_cpp(manager: &SharedMapManager) -> RuntimeTickOwner {
    manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .tick_owner()
}

/// Convert world X coordinate to grid X coordinate.
/// Uses floor() to handle negative coordinates correctly.
pub fn world_to_grid_x(world_x: f32) -> i16 {
    (world_x / GRID_SIZE).floor() as i16
}

/// Convert world Y coordinate to grid Y coordinate.
/// Uses floor() to handle negative coordinates correctly.
pub fn world_to_grid_y(world_y: f32) -> i16 {
    (world_y / GRID_SIZE).floor() as i16
}

/// Convert world coordinates to grid coordinates (x, y).
/// Convenience function that returns both coordinates at once.
pub fn world_to_grid_coords(world_x: f32, world_y: f32) -> (i16, i16) {
    (world_to_grid_x(world_x), world_to_grid_y(world_y))
}

/// Convert grid coordinate to world coordinate (center of grid).
pub fn grid_to_world(grid: i16) -> f32 {
    (grid as f32 * GRID_SIZE) + (GRID_SIZE / 2.0)
}

/// Get the world coordinates of a grid's corner.
pub fn grid_corner(grid_x: i16, grid_y: i16) -> (f32, f32) {
    (grid_x as f32 * GRID_SIZE, grid_y as f32 * GRID_SIZE)
}

/// A creature waiting to respawn after its corpse despawned.
///
/// Owned by `MapInstance::respawn_queue`; processed by `tick_creatures_sync`.
/// C++ refs: `Creature::RemoveCorpse` / `AllLootRemovedFromCorpse` schedule a
/// map-owned `RespawnInfo`, and `Map::ProcessRespawns` later calls
/// `DoRespawn(SPAWN_TYPE_CREATURE, spawnId, gridId)`.
#[derive(Debug)]
pub struct PendingRespawn {
    /// When to respawn.
    pub respawn_at: Instant,
    /// C++ `RespawnInfo::spawnId` / `Creature::m_spawnId`, separate from the live ObjectGuid low counter.
    pub spawn_id: u64,
    /// Whether `spawn_id` is a real DB spawn identity rather than the
    /// queue-only GUID-low fallback used for dynamic creatures.
    pub persistent_spawn: bool,
    /// Home position (spawn point).
    pub home_pos: wow_core::Position,
    /// Full create data retained until the represented loader converges on
    /// C++ `Creature::LoadFromDB(spawnId, map, true, true)`.
    pub create_data: CreatureCreateData,
    /// AI fields needed to rebuild the canonical creature runtime.
    pub max_hp: u32,
    pub level: u8,
    pub min_dmg: u32,
    pub max_dmg: u32,
    /// Live totals used by C++ `SpellCastLogData::Initialize`.
    pub combat_log_stats: CreatureCombatLogStatsLikeCpp,
    /// DB-backed source proofs captured independently of the live aura markers.
    /// The live markers are revoked during death cleanup; the respawn rail may
    /// restore only proofs that crossed the authoritative loaded-grid bridge.
    pub spell_hit_aura_source_authority_like_cpp: bool,
    pub spell_cast_log_aura_source_authority_like_cpp: bool,
    pub aggro_radius: f32,
    pub wander_distance: f32,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub ai_name: String,
    pub script_name: String,
    pub string_id: Option<String>,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub default_movement_type: MovementGeneratorType,
    pub waypoint_path_id: u32,
    pub npc_flags: u32,
    pub unit_flags: u32,
    pub map_id: u16,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub respawn_delay_secs: u32,
    pub selected_equipment_id: u8,
    pub original_equipment_id: i8,
    pub boss_id: Option<u32>,
    pub dungeon_encounter_id: u32,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    /// Already-resolved DB phase shift from the creature that despawned.
    ///
    /// The global runtime has no `WorldSession` phase stores, so respawn must
    /// reuse the resolved phase state captured at despawn time instead of
    /// recalculating it through session-local helpers.
    pub phase_shift: PhaseShift,
}

/// Build a map-owned respawn entry from the represented runtime creature.
///
/// Mirrors the data captured by the session-local corpse despawn path; this
/// helper exists so the future global lifecycle driver and the legacy session
/// path do not drift.
pub fn pending_respawn_from_world_creature_like_cpp(
    creature: &WorldCreature,
    respawn_at: Instant,
    map_id: u16,
) -> PendingRespawn {
    let persistent_spawn = creature.creature.spawn_id() != 0;
    let spawn_id = match creature.creature.spawn_id() {
        0 => creature.guid().low_value().max(0) as u64,
        spawn_id => spawn_id,
    };
    PendingRespawn {
        respawn_at,
        spawn_id,
        persistent_spawn,
        home_pos: creature.home_position(),
        create_data: CreatureCreateData {
            guid: creature.guid(),
            entry: creature.entry(),
            display_id: creature.display_id(),
            native_display_id: creature.display_id(),
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: creature.creature.unit().data().bounding_radius,
            combat_reach: creature.creature.unit().data().combat_reach,
            health: creature.max_hp() as i64,
            max_health: creature.max_hp() as i64,
            level: creature.level(),
            faction_template: creature.faction() as i32,
            npc_flags: creature.npc_flags_mask_like_cpp(),
            unit_flags: creature.unit_flags(),
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: WorldCreature::health_aura_state_like_cpp(
                creature.max_hp() as u64,
                creature.max_hp() as u64,
                creature.max_hp() > 0,
            ),
            damage_school: creature.creature.melee_damage_school_like_cpp(),
            scale: 1.0,
            unit_class: creature.create_data.unit_class,
            display_power: creature.create_data.display_power,
            power: creature.create_data.power,
            max_power: creature.create_data.max_power,
            base_mana: creature.create_data.base_mana,
            virtual_items: [
                (
                    creature.creature.unit().data().virtual_items[0].item_id,
                    creature.creature.unit().data().virtual_items[0].item_appearance_mod_id,
                    creature.creature.unit().data().virtual_items[0].item_visual,
                ),
                (
                    creature.creature.unit().data().virtual_items[1].item_id,
                    creature.creature.unit().data().virtual_items[1].item_appearance_mod_id,
                    creature.creature.unit().data().virtual_items[1].item_visual,
                ),
                (
                    creature.creature.unit().data().virtual_items[2].item_id,
                    creature.creature.unit().data().virtual_items[2].item_appearance_mod_id,
                    creature.creature.unit().data().virtual_items[2].item_visual,
                ),
            ],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: creature.creature.movement_flags_like_cpp().bits(),
            vehicle_id: creature
                .creature
                .lifecycle_metadata()
                .vehicle_id
                .unwrap_or(0),
            play_hover_anim: false,
            hover_height: creature.creature.unit().data().hover_height,
            mount_display_id: creature.creature.unit().data().mount_display_id,
            stand_state: creature.creature.unit().data().stand_state,
            vis_flags: creature.creature.unit().data().vis_flags,
            anim_tier: creature.creature.unit().data().anim_tier,
            emote_state: creature.creature.unit().emote_state_like_cpp() as i32,
            sheathe_state: creature.creature.unit().data().sheathe_state,
            pvp_flags: creature.creature.unit().data().pvp_flags,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: creature.creature.unit().ai_anim_kit_id_like_cpp(),
            movement_anim_kit_id: creature.creature.unit().movement_anim_kit_id_like_cpp(),
            melee_anim_kit_id: creature.creature.unit().melee_anim_kit_id_like_cpp(),
        },
        max_hp: creature.max_hp(),
        level: creature.level(),
        min_dmg: creature.min_dmg(),
        max_dmg: creature.max_dmg(),
        combat_log_stats: creature.creature.combat_log_stats_like_cpp(),
        spell_hit_aura_source_authority_like_cpp: creature
            .respawn_spell_hit_aura_source_authority_like_cpp,
        spell_cast_log_aura_source_authority_like_cpp: creature
            .respawn_spell_cast_log_aura_source_authority_like_cpp,
        aggro_radius: creature.creature.ai_ownership().aggro_radius,
        wander_distance: creature.creature.ai_ownership().wander_radius.max(0.0),
        flags_extra: creature.creature.lifecycle_metadata().flags_extra,
        static_flags: creature.creature.lifecycle_metadata().static_flags,
        ai_name: creature.creature.lifecycle_metadata().ai_name.clone(),
        script_name: creature.creature.lifecycle_metadata().script_name.clone(),
        string_id: creature.creature.lifecycle_metadata().string_id.clone(),
        addon: creature.creature.lifecycle_metadata().addon.clone(),
        ground_movement_type: creature.creature.ground_movement_type_like_cpp(),
        swim_allowed: creature.creature.swim_allowed_like_cpp(),
        flight_movement_type: creature.creature.flight_movement_type_like_cpp(),
        rooted: creature.creature.is_template_rooted_like_cpp(),
        chase_movement_type: creature.creature.chase_movement_type_like_cpp(),
        random_movement_type: creature.creature.random_movement_type_like_cpp(),
        interaction_pause_timer_ms: creature.creature.interaction_pause_timer_ms_like_cpp(),
        default_movement_type: creature.creature.default_movement_type(),
        waypoint_path_id: creature.creature.waypoint_path_id_like_cpp(),
        npc_flags: creature.npc_flags(),
        unit_flags: creature.unit_flags(),
        map_id,
        loot_id: creature.loot_id(),
        skin_loot_id: creature.skin_loot_id(),
        gold_min: creature.gold_min(),
        gold_max: creature.gold_max(),
        respawn_delay_secs: creature
            .creature
            .ai_ownership()
            .respawn_time_secs
            .min(u64::from(u32::MAX)) as u32,
        selected_equipment_id: creature.creature.equipment_id(),
        original_equipment_id: creature.creature.original_equipment_id(),
        boss_id: creature.boss_id(),
        dungeon_encounter_id: creature.dungeon_encounter_id(),
        phase_use_flags: creature.creature.ai_ownership().phase_use_flags,
        phase_id: creature.creature.ai_ownership().phase_id,
        phase_group_id: creature.creature.ai_ownership().phase_group_id,
        terrain_swap_map: creature.creature.ai_ownership().terrain_swap_map,
        phase_shift: creature.phase_shift().clone(),
    }
}

pub fn pending_respawn_create_position_like_cpp(respawn: &PendingRespawn) -> Position {
    let mut position = respawn.home_pos;
    let create_flags =
        wow_constants::movement::MovementFlag::from_bits_retain(respawn.create_data.movement_flags);
    let addon_sets_hover = respawn.addon.is_some()
        && respawn.ground_movement_type == wow_constants::CreatureGroundMovementType::Hover as u8;
    if create_flags.contains(wow_constants::movement::MovementFlag::HOVER) || addon_sets_hover {
        position.z += respawn.create_data.hover_height;
    }
    position
}

/// Recreate a represented world creature from a map-owned respawn entry.
///
/// This is the session-free equivalent of `WorldSession::register_world_creature`
/// for the global lifecycle driver. It intentionally uses the already-resolved
/// phase shift stored in [`PendingRespawn`].
pub fn world_creature_from_pending_respawn_like_cpp(
    respawn: &PendingRespawn,
    instance_id: u32,
) -> WorldCreature {
    let create_data = &respawn.create_data;
    let position = pending_respawn_create_position_like_cpp(respawn);
    let guid = create_data.guid;
    let entry = create_data.entry;
    let hp = create_data.health.max(1) as u32;
    let level = create_data.level;
    let display_id = create_data.display_id;
    let faction = create_data.faction_template.max(0) as u32;
    let npc_flags = create_data.npc_flags as u32;
    let npc_flags2 = (create_data.npc_flags >> 32) as u32;
    let unit_flags = create_data.unit_flags;
    let unit_flags2 = create_data.unit_flags2;
    let unit_flags3 = create_data.unit_flags3;
    let damage_school = create_data.damage_school;

    let mut creature = Creature::new(false);
    creature.set_spawn_id(if respawn.persistent_spawn {
        respawn.spawn_id
    } else {
        0
    });
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(entry);
    let _ = creature
        .unit_mut()
        .world_mut()
        .set_map(u32::from(respawn.map_id), instance_id);
    creature.unit_mut().world_mut().relocate(position);
    *creature.unit_mut().world_mut().phase_shift_mut() = respawn.phase_shift.clone();
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(u64::from(hp));
    creature.unit_mut().set_health(u64::from(hp));
    creature.unit_mut().set_class(create_data.unit_class);
    let power_type = power_type_from_u8_like_cpp(create_data.display_power);
    creature.set_power_type(power_type);
    creature
        .unit_mut()
        .set_create_mana_like_cpp(create_data.base_mana);
    creature
        .unit_mut()
        .replace_create_power_arrays_like_cpp(create_data.power, create_data.max_power);
    creature.set_combat_log_stats_like_cpp(respawn.combat_log_stats);
    creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.set_unit_flags2_runtime_like_cpp(unit_flags2);
    creature.set_unit_flags3_runtime_like_cpp(unit_flags3);
    creature.set_melee_damage_school_like_cpp(damage_school);
    creature
        .unit_mut()
        .set_native_display_id_like_cpp(create_data.native_display_id);
    creature.unit_mut().set_display_scales_like_cpp(
        create_data.display_scale,
        create_data.native_x_display_scale,
    );
    creature
        .unit_mut()
        .set_bounding_radius(create_data.bounding_radius);
    creature
        .unit_mut()
        .set_combat_reach(create_data.combat_reach);
    creature
        .unit_mut()
        .set_hover_height_like_cpp(create_data.hover_height);
    creature.set_flags_extra_runtime_like_cpp(respawn.flags_extra);
    creature.set_static_flags_runtime_like_cpp(respawn.static_flags);
    creature.set_ai_identity_names_runtime_like_cpp(
        respawn.ai_name.clone(),
        respawn.script_name.clone(),
    );
    creature.set_spawn_string_id_runtime_like_cpp(respawn.string_id.clone());
    creature.set_ground_movement_type_runtime_like_cpp(respawn.ground_movement_type);
    creature.set_swim_allowed_runtime_like_cpp(respawn.swim_allowed);
    creature.set_flight_movement_type_runtime_like_cpp(respawn.flight_movement_type);
    creature.set_template_rooted_like_cpp(respawn.rooted);
    creature.set_chase_movement_type_runtime_like_cpp(respawn.chase_movement_type);
    creature.set_random_movement_type_runtime_like_cpp(respawn.random_movement_type);
    creature.set_interaction_pause_timer_ms_runtime_like_cpp(respawn.interaction_pause_timer_ms);
    creature.set_default_movement_type_runtime_like_cpp(respawn.default_movement_type);
    creature.set_equipment_id_like_cpp(respawn.selected_equipment_id);
    creature.set_original_equipment_id_like_cpp(respawn.original_equipment_id);
    if respawn.waypoint_path_id != 0 {
        creature.load_path_like_cpp(respawn.waypoint_path_id);
    }
    creature.apply_creatures_addon_lifecycle_like_cpp(respawn.addon.as_ref());
    let effective_waypoint_path_id = creature.waypoint_path_id_like_cpp();
    if effective_waypoint_path_id != 0 {
        creature.load_path_like_cpp(effective_waypoint_path_id);
    }
    creature.configure_ai_runtime(
        respawn.home_pos,
        respawn.aggro_radius,
        respawn.wander_distance.max(0.0),
        30,
    );
    creature.ai_ownership_mut().respawn_time_secs = u64::from(respawn.respawn_delay_secs);
    creature.set_respawn_delay(respawn.respawn_delay_secs);
    creature.ai_ownership_mut().min_damage = respawn.min_dmg;
    creature.ai_ownership_mut().max_damage = respawn.max_dmg;
    creature.ai_ownership_mut().loot_id = respawn.loot_id;
    creature.ai_ownership_mut().skin_loot_id = respawn.skin_loot_id;
    creature.ai_ownership_mut().gold_min = respawn.gold_min;
    creature.ai_ownership_mut().gold_max = respawn.gold_max;
    creature.ai_ownership_mut().boss_id = respawn.boss_id;
    creature.ai_ownership_mut().dungeon_encounter_id = respawn.dungeon_encounter_id;
    creature.ai_ownership_mut().phase_use_flags = respawn.phase_use_flags;
    creature.ai_ownership_mut().phase_id = respawn.phase_id;
    creature.ai_ownership_mut().phase_group_id = respawn.phase_group_id;
    creature.ai_ownership_mut().terrain_swap_map = respawn.terrain_swap_map;
    creature.clear_data_changes();

    let mut world_creature = WorldCreature::from_canonical(creature, respawn.create_data.clone());
    world_creature.restore_respawn_aura_source_authority_like_cpp(
        respawn.spell_hit_aura_source_authority_like_cpp,
        respawn.spell_cast_log_aura_source_authority_like_cpp,
    );
    world_creature
}

#[cfg(test)]
#[path = "../map_manager_tests.rs"]
mod tests;
