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
    let mut stmt = PreparedStatement::new(CharStatements::REP_RESPAWN.sql());
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
    let mut stmt = PreparedStatement::new(CharStatements::DEL_RESPAWN.sql());
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

impl WorldCreature {
    fn runtime_default_generator_like_cpp(
        creature: &Creature,
    ) -> Box<dyn RuntimeMovementGenerator> {
        match creature.default_movement_type() {
            MovementGeneratorType::Idle => Box::new(IdleMovementGenerator::new()),
            MovementGeneratorType::Random => Box::new(RandomMovementGenerator::new(
                creature.ai_ownership().wander_radius,
                None,
            )),
            MovementGeneratorType::Waypoint => {
                Box::new(WaypointMovementGenerator::from_db_path_id(
                    creature.waypoint_path_id_like_cpp(),
                    true,
                ))
            }
        }
    }

    fn new_runtime_motion_master_like_cpp(creature: &Creature) -> MotionMaster {
        let mut motion_master =
            MotionMaster::new(Self::runtime_default_generator_like_cpp(creature));
        if creature.ai_state() == CreatureAiState::InCombat
            && let Some(target) = creature.ai_ownership().combat_target
        {
            motion_master.add(
                Box::new(ChaseMovementGenerator::new(target, None, None)),
                RuntimeMovementSlot::Active,
            );
        }
        motion_master
    }

    pub fn new(
        guid: ObjectGuid,
        entry: u32,
        pos: Position,
        hp: u32,
        level: u8,
        min_dmg: u32,
        max_dmg: u32,
        aggro_radius: f32,
        display_id: u32,
        faction: u32,
        npc_flags: u32,
        unit_flags: u32,
    ) -> Self {
        let (min_dmg, max_dmg) = if min_dmg == 0 {
            let base = (level as u32) * 3 + 5;
            (base, base + base / 2)
        } else {
            (min_dmg, max_dmg)
        };

        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(entry);
        creature.set_ai_position(pos);
        creature.set_ai_home_position(pos);
        creature.unit_mut().set_level(level);
        creature.unit_mut().set_max_health(u64::from(hp));
        creature.unit_mut().set_health(u64::from(hp));
        creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
        creature.unit_mut().set_weapon_damage(
            WeaponAttackType::BaseAttack,
            min_dmg as f32,
            max_dmg as f32,
        );
        {
            let ai = creature.ai_ownership_mut();
            ai.aggro_radius = aggro_radius;
            // C++ `Creature::Creature` initializes `m_wanderDistance` to 0.0f and only
            // random movement spawns get a positive distance from CreatureData.
            ai.wander_radius = 0.0;
            ai.respawn_time_secs = 30;
            ai.min_damage = min_dmg;
            ai.max_damage = max_dmg;
        }

        let create_data = CreatureCreateData {
            guid,
            entry,
            display_id,
            native_display_id: display_id,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.5,
            health: hp as i64,
            max_health: hp as i64,
            level,
            faction_template: faction as i32,
            npc_flags: npc_flags as u64,
            unit_flags,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: Self::health_aura_state_like_cpp(hp as u64, hp as u64, hp > 0),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };

        Self::from_canonical(creature, create_data)
    }

    pub fn from_canonical(mut creature: Creature, mut create_data: CreatureCreateData) -> Self {
        // This generic bridge carries no proof that every aura source was
        // hydrated. Preserve fail-closed semantics even if a caller passes a
        // clone that previously crossed a more authoritative boundary.
        creature
            .unit_mut()
            .subsystems_mut()
            .auras
            .invalidate_spell_hit_aura_authority_like_cpp();
        let ai = creature.ai_ownership();
        create_data.npc_flags = (u64::from(ai.npc_flags2) << 32) | u64::from(ai.npc_flags);
        create_data.unit_flags = ai.unit_flags;
        create_data.unit_flags2 = ai.unit_flags2;
        create_data.unit_flags3 = ai.unit_flags3;
        create_data.damage_school = creature.melee_damage_school_like_cpp();
        create_data.ai_anim_kit_id = creature.unit().ai_anim_kit_id_like_cpp();
        create_data.movement_anim_kit_id = creature.unit().movement_anim_kit_id_like_cpp();
        create_data.melee_anim_kit_id = creature.unit().melee_anim_kit_id_like_cpp();
        let _ = creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .add_to_world_like_cpp();
        let runtime_motion_master = Self::new_runtime_motion_master_like_cpp(&creature);
        Self {
            creature,
            create_data,
            active_move_spline: None,
            active_random_generator: None,
            active_random_path_poly_refs: Vec::new(),
            active_home_generator: None,
            active_chase_generator: None,
            active_chase_path_poly_refs: Vec::new(),
            active_waypoint_generator: None,
            active_waypoint_random_at_path_end: None,
            runtime_motion_master,
            runtime_chase_target: None,
            runtime_represented_active: None,
            pending_assistance_like_cpp: Vec::new(),
            assistance_called_like_cpp: false,
            active_taunts_like_cpp: Vec::new(),
            creature_spell_due_at_ms_like_cpp: [None; wow_entities::MAX_CREATURE_SPELLS],
            creature_spell_schedule_initialized_like_cpp: false,
            creature_spell_engagement_epoch_like_cpp: 0,
            home_health_restored_pending_like_cpp: false,
            runtime_motion_master_ticks: 0,
            runtime_rng_authority_complete_like_cpp: true,
            respawn_spell_hit_aura_source_authority_like_cpp: false,
            respawn_spell_cast_log_aura_source_authority_like_cpp: false,
            runtime_rng_like_cpp: StdRng::from_entropy(),
            clock_started_at: Instant::now(),
        }
    }

    /// C++ `Unit::Update` health-derived `UNIT_FIELD_AURASTATE` bits.
    ///
    /// Mirrors `Unit.cpp:469-476` `ModifyAuraState` calls for an alive unit:
    /// the WOUNDED_* and WOUND_HEALTH_* / HEALTHY_75 states are pure functions
    /// of the health percentage. AURA_STATE values are 1-based flag indices, so
    /// the wire bit is `1 << (state - 1)`. A full-HP creature yields `0x00D00000`.
    /// Shipping 0 here (the bit 0x100000 = AURA_STATE_WOUND_HEALTH_20_80 in
    /// particular) crashes the 3.4.3 client on a per-frame unit tick.
    pub fn health_aura_state_like_cpp(current_health: u64, max_health: u64, alive: bool) -> u32 {
        if !alive || max_health == 0 {
            return 0;
        }
        // C++ HealthBelowPct(p): health < max * p / 100; HealthAbovePct(p): health > max * p / 100.
        let below = |p: u64| current_health.saturating_mul(100) < max_health.saturating_mul(p);
        let above = |p: u64| current_health.saturating_mul(100) > max_health.saturating_mul(p);
        let mut state: u32 = 0;
        let mut set = |flag_index: u32, apply: bool| {
            if apply {
                state |= 1 << (flag_index - 1);
            }
        };
        set(2, below(20)); // AURA_STATE_WOUNDED_20_PERCENT
        set(6, below(25)); // AURA_STATE_WOUNDED_25_PERCENT
        set(13, below(35)); // AURA_STATE_WOUNDED_35_PERCENT
        set(21, below(20) || above(80)); // AURA_STATE_WOUND_HEALTH_20_80
        set(23, above(75)); // AURA_STATE_HEALTHY_75_PERCENT
        set(24, below(35) || above(80)); // AURA_STATE_WOUND_HEALTH_35_80
        state
    }

    pub fn create_data_from_canonical_like_cpp(creature: &Creature) -> CreatureCreateData {
        let unit = creature.unit();
        let data = unit.data();
        let object = unit.world().object();
        let npc_flags = unit.npc_flags_like_cpp();
        let attack_speed = unit.base_attack_speed();
        let speed_rate = unit.speed_rate();
        let vehicle_id = unit
            .subsystems()
            .vehicle
            .kit
            .as_ref()
            .map(|kit| kit.kit_id())
            .unwrap_or(0);

        CreatureCreateData {
            guid: creature.guid(),
            entry: creature.entry(),
            display_id: data.display_id.max(0) as u32,
            native_display_id: data.native_display_id.max(0) as u32,
            display_scale: data.display_scale,
            native_x_display_scale: data.native_display_scale,
            bounding_radius: data.bounding_radius,
            combat_reach: data.combat_reach,
            health: creature.current_health().min(i64::MAX as u64) as i64,
            max_health: creature.max_health().min(i64::MAX as u64) as i64,
            level: creature.level(),
            faction_template: data.faction_template,
            npc_flags: (u64::from(npc_flags[1]) << 32) | u64::from(npc_flags[0]),
            unit_flags: data.flags,
            unit_flags2: data.flags2,
            unit_flags3: data.flags3,
            aura_state: Self::health_aura_state_like_cpp(
                creature.current_health(),
                creature.max_health(),
                creature.current_health() > 0,
            ),
            damage_school: creature.melee_damage_school_like_cpp(),
            scale: object.scale(),
            unit_class: data.class_id,
            display_power: data.display_power,
            power: data.power,
            max_power: data.max_power,
            base_mana: data.base_mana,
            virtual_items: [
                (
                    data.virtual_items[0].item_id,
                    data.virtual_items[0].item_appearance_mod_id,
                    data.virtual_items[0].item_visual,
                ),
                (
                    data.virtual_items[1].item_id,
                    data.virtual_items[1].item_appearance_mod_id,
                    data.virtual_items[1].item_visual,
                ),
                (
                    data.virtual_items[2].item_id,
                    data.virtual_items[2].item_appearance_mod_id,
                    data.virtual_items[2].item_visual,
                ),
            ],
            // C++ guarantees UNIT_FIELD_BASEATTACKTIME is never 0: ObjectMgr.cpp:1100-1104
            // clamps creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME
            // (2000) at load. The 3.4.3 client divides by this on the first post-spawn unit
            // tick (swing-timer/attack-rate math), so a 0 here crashes the client a few
            // seconds after the create burst. Defense-in-depth clamp mirroring C++.
            base_attack_time: match attack_speed[WeaponAttackType::BaseAttack as usize] {
                0 => BASE_ATTACK_TIME_LIKE_CPP,
                t => t,
            },
            ranged_attack_time: match attack_speed[WeaponAttackType::RangedAttack as usize] {
                0 => BASE_ATTACK_TIME_LIKE_CPP,
                t => t,
            },
            movement_flags: creature.movement_flags_like_cpp().bits(),
            vehicle_id,
            play_hover_anim: false,
            hover_height: data.hover_height,
            mount_display_id: data.mount_display_id,
            stand_state: data.stand_state,
            vis_flags: data.vis_flags,
            anim_tier: data.anim_tier,
            emote_state: unit.emote_state_like_cpp() as i32,
            sheathe_state: data.sheathe_state,
            pvp_flags: data.pvp_flags,
            current_area_id: 0,
            speed_walk_rate: speed_rate[UnitMoveType::Walk as usize],
            speed_run_rate: speed_rate[UnitMoveType::Run as usize],
            ai_anim_kit_id: unit.ai_anim_kit_id_like_cpp(),
            movement_anim_kit_id: unit.movement_anim_kit_id_like_cpp(),
            melee_anim_kit_id: unit.melee_anim_kit_id_like_cpp(),
        }
    }

    pub fn from_loaded_grid_canonical_like_cpp(
        creature: Creature,
        mut waypoint_path_resolver: impl FnMut(u32) -> Option<WaypointPath>,
    ) -> Self {
        let create_data = Self::create_data_from_canonical_like_cpp(&creature);
        let mut world_creature = Self::from_canonical(creature, create_data);
        // The loaded-grid lifecycle receives a Creature only after the
        // DB-backed creature_addon/template_addon store has resolved and the
        // selected addon has been applied to its canonical AuraSubsystem.
        world_creature.restore_respawn_aura_source_authority_like_cpp(true, true);
        match world_creature.creature.default_movement_type() {
            wow_entities::MovementGeneratorType::Random => {
                world_creature.initialize_default_random_movement_like_cpp();
            }
            wow_entities::MovementGeneratorType::Waypoint => {
                world_creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(
                    |path_id| waypoint_path_resolver(path_id),
                );
            }
            wow_entities::MovementGeneratorType::Idle => {}
        }
        world_creature
    }

    fn restore_respawn_aura_source_authority_like_cpp(
        &mut self,
        spell_hit: bool,
        spell_cast_log: bool,
    ) {
        self.respawn_spell_hit_aura_source_authority_like_cpp = spell_hit;
        self.respawn_spell_cast_log_aura_source_authority_like_cpp = spell_cast_log;
        let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
        auras.set_spell_hit_aura_authority_inert_like_cpp(spell_hit);
        auras.set_spell_cast_log_aura_authority_inert_like_cpp(spell_cast_log);
    }

    pub fn active_waypoint_generator_like_cpp(&self) -> Option<&WaypointMovementGenerator> {
        self.active_waypoint_generator.as_ref()
    }

    pub fn active_waypoint_random_at_path_end_like_cpp(&self) -> Option<WaypointRandomAtPathEnd> {
        self.active_waypoint_random_at_path_end
    }

    pub fn visibility_range_like_cpp(&self) -> f32 {
        self.creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp()
            .unwrap_or(VISIBILITY_RADIUS)
    }

    pub(crate) fn now_ms(&self) -> u64 {
        self.clock_started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }

    #[cfg(test)]
    pub(crate) fn backdate_runtime_clock_for_test(&mut self, elapsed: Duration) {
        self.clock_started_at = Instant::now() - elapsed;
    }

    pub fn guid(&self) -> ObjectGuid {
        self.creature.ai_guid()
    }

    pub fn entry(&self) -> u32 {
        self.creature.ai_entry()
    }

    pub fn position(&self) -> Position {
        self.creature.ai_position()
    }

    pub fn map_id(&self) -> u32 {
        self.creature.unit().world().map_id()
    }

    pub fn instance_id(&self) -> u32 {
        self.creature.unit().world().instance_id()
    }

    pub fn phase_shift(&self) -> &PhaseShift {
        self.creature.unit().world().phase_shift()
    }

    pub fn home_position(&self) -> Position {
        self.creature.ai_home_position()
    }

    /// C++ interaction handlers call `PauseMovement(timer)` and then
    /// `SetHomePosition(GetPosition())` for gossip/vendor/quest interactions.
    pub fn pause_interaction_movement_like_cpp(&mut self) -> bool {
        let pause_timer = self.creature.interaction_pause_timer_ms_like_cpp();
        if pause_timer == 0 {
            return false;
        }

        let current_position = self.position();
        let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
        motion.pause_current_movement_like_cpp(pause_timer, MovementSlot::Default, true);
        self.creature.set_ai_home_position(current_position);
        true
    }

    pub fn is_alive(&self) -> bool {
        self.creature.ai_is_alive()
    }

    pub fn current_hp(&self) -> u32 {
        self.creature.ai_current_health().min(u64::from(u32::MAX)) as u32
    }

    pub fn max_hp(&self) -> u32 {
        self.creature.ai_max_health().min(u64::from(u32::MAX)) as u32
    }

    pub fn level(&self) -> u8 {
        self.creature.ai_level()
    }

    pub fn npc_flags(&self) -> u32 {
        self.creature.ai_ownership().npc_flags
    }

    pub fn npc_flags2(&self) -> u32 {
        self.creature.ai_ownership().npc_flags2
    }

    pub fn unit_flags2_like_cpp(&self) -> UnitFlags2 {
        self.creature.unit().unit_flags2_like_cpp()
    }

    pub fn trainer_class_like_cpp(&self) -> u8 {
        self.creature.trainer_class_like_cpp()
    }

    pub fn npc_flags_mask_like_cpp(&self) -> u64 {
        (u64::from(self.npc_flags2()) << 32) | u64::from(self.npc_flags())
    }

    pub fn unit_flags(&self) -> u32 {
        self.creature.ai_ownership().unit_flags
    }

    pub fn display_id(&self) -> u32 {
        self.creature.ai_ownership().display_id
    }

    pub fn faction(&self) -> u32 {
        self.creature.ai_ownership().faction
    }

    pub fn min_dmg(&self) -> u32 {
        self.creature.ai_ownership().min_damage
    }

    pub fn max_dmg(&self) -> u32 {
        self.creature.ai_ownership().max_damage
    }

    pub fn loot_id(&self) -> u32 {
        self.creature.ai_ownership().loot_id
    }

    pub fn skin_loot_id(&self) -> u32 {
        self.creature.ai_ownership().skin_loot_id
    }

    pub fn gold_min(&self) -> u32 {
        self.creature.ai_ownership().gold_min
    }

    pub fn gold_max(&self) -> u32 {
        self.creature.ai_ownership().gold_max
    }

    pub fn boss_id(&self) -> Option<u32> {
        self.creature.ai_ownership().boss_id
    }

    pub fn dungeon_encounter_id(&self) -> u32 {
        self.creature.ai_ownership().dungeon_encounter_id
    }

    pub fn state(&self) -> CreatureAiState {
        self.creature.ai_state()
    }

    pub fn move_target(&self) -> Option<Position> {
        self.creature.ai_ownership().move_target
    }

    pub fn active_move_spline_like_cpp(&self) -> Option<&MoveSpline> {
        self.active_move_spline.as_ref()
    }

    pub fn spline_id(&self) -> u32 {
        self.creature.ai_ownership().spline_id
    }

    pub fn corpse_despawn_at(&self) -> Option<Instant> {
        self.creature
            .ai_ownership()
            .corpse_despawn_at_ms
            .map(|ms| self.clock_started_at + Duration::from_millis(ms))
    }

    pub fn corpse_delay_secs_like_cpp(&self) -> u32 {
        self.creature.corpse_delay()
    }

    pub fn respawn_at_from_death_like_cpp(&self) -> Instant {
        self.respawn_at_from_death_at_game_time_like_cpp(Instant::now(), game_time_secs_like_cpp())
    }

    pub fn respawn_at_from_death_at_game_time_like_cpp(
        &self,
        now: Instant,
        game_time_secs: i64,
    ) -> Instant {
        let death_at = self
            .creature
            .ai_ownership()
            .death_time_ms
            .map(|ms| self.clock_started_at + Duration::from_millis(ms))
            .unwrap_or(now);
        let compatibility_corpse_delay = self
            .creature
            .respawn_compatibility_mode()
            .then_some(u64::from(self.creature.corpse_delay()))
            .unwrap_or(0);
        let death_based = death_at
            + Duration::from_secs(
                self.creature
                    .ai_ownership()
                    .respawn_time_secs
                    .saturating_add(compatibility_corpse_delay),
            );
        let stored_based =
            instant_from_respawn_time_like_cpp(self.creature.respawn_time(), now, game_time_secs);
        death_based.max(stored_based)
    }

    pub fn ignore_corpse_decay_ratio_like_cpp(&self) -> bool {
        self.creature.ignore_corpse_decay_ratio()
    }

    pub fn set_corpse_despawn_at(&mut self, when: Option<Instant>) {
        let now_ms = self.now_ms();
        let at_ms = when.map(|instant| {
            if instant <= self.clock_started_at {
                0
            } else if instant <= Instant::now() {
                now_ms
            } else {
                now_ms.saturating_add(
                    instant
                        .duration_since(Instant::now())
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                )
            }
        });
        self.creature.set_ai_corpse_despawn_at(at_ms);
    }

    pub fn enter_combat(&mut self, attacker: ObjectGuid) {
        // `enter_combat` is also used when threat selection changes the current
        // victim. C++ only resets/schedules `CombatAI::_events` for a new
        // engagement, not every victim switch.
        if self.creature.ai_state() != CreatureAiState::InCombat {
            self.reset_creature_spell_schedule_like_cpp();
        }
        self.creature.enter_ai_combat(attacker);
        self.sync_runtime_motion_master_like_cpp();
        debug!(
            "Creature {:?} entered combat with {:?}",
            self.guid(),
            attacker
        );
    }

    pub fn schedule_assistance_like_cpp(
        &mut self,
        victim: ObjectGuid,
        assistants: Vec<ObjectGuid>,
        delay_ms: u32,
    ) -> bool {
        if assistants.is_empty() {
            return false;
        }
        self.pending_assistance_like_cpp.push((
            victim,
            assistants,
            self.now_ms().saturating_add(u64::from(delay_ms)),
        ));
        true
    }

    pub fn set_no_call_assistance_like_cpp(&mut self) {
        self.assistance_called_like_cpp = true;
    }

    pub fn take_assistance_call_like_cpp(&mut self) -> Option<ObjectGuid> {
        if self.assistance_called_like_cpp
            || self
                .creature
                .unit()
                .subsystems()
                .control
                .charmer_or_owner_guid()
                .is_some()
        {
            return None;
        }
        let victim = self.creature.ai_ownership().combat_target?;
        self.assistance_called_like_cpp = true;
        Some(victim)
    }

    pub fn take_due_assistance_like_cpp(&mut self) -> Vec<(ObjectGuid, Vec<ObjectGuid>)> {
        let now_ms = self.now_ms();
        let mut due = Vec::new();
        self.pending_assistance_like_cpp
            .retain(|(victim, assistants, due_at_ms)| {
                if now_ms >= *due_at_ms {
                    due.push((*victim, assistants.clone()));
                    false
                } else {
                    true
                }
            });
        due
    }

    pub fn apply_taunt_aura_like_cpp(
        &mut self,
        caster: ObjectGuid,
        spell_id: u32,
        effect_mask: u32,
        duration_ms: i32,
    ) -> Option<u8> {
        let due_at_ms =
            (duration_ms >= 0).then(|| self.now_ms().saturating_add(duration_ms as u64));
        let replaced: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .copied()
            .filter(|active| active.caster == caster && active.spell_id == spell_id)
            .collect();
        self.active_taunts_like_cpp
            .retain(|active| active.caster != caster || active.spell_id != spell_id);
        let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
        auras.remove_auras_due_to_spell_like_cpp(spell_id, caster, effect_mask);
        for active in replaced {
            auras.clear_visible(active.slot);
        }
        if !auras.add_self_cast_addon_aura_application_like_cpp(spell_id, caster, effect_mask, 0) {
            return None;
        }
        let slot = auras.visible_auras.iter().find_map(|(slot, aura)| {
            (aura.spell_id == spell_id && aura.caster_guid == caster).then_some(*slot)
        })?;
        auras.register_applied_aura_type_like_cpp(
            wow_entities::AppliedAuraRef::new(spell_id, caster, slot, effect_mask),
            wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
        );
        self.active_taunts_like_cpp.push(ActiveTauntLikeCpp {
            caster,
            due_at_ms,
            spell_id,
            effect_mask,
            slot,
        });
        self.refresh_active_taunt_states_like_cpp();
        Some(slot)
    }

    pub fn expire_taunt_auras_if_due_like_cpp(&mut self) -> Vec<u8> {
        let now_ms = self.now_ms();
        if !self.active_taunts_like_cpp.iter().any(|active| {
            active
                .due_at_ms
                .is_some_and(|due_at_ms| now_ms >= due_at_ms)
        }) {
            return Vec::new();
        }
        let expired: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .copied()
            .filter(|active| {
                active
                    .due_at_ms
                    .is_some_and(|due_at_ms| now_ms >= due_at_ms)
            })
            .collect();
        self.active_taunts_like_cpp
            .retain(|active| active.due_at_ms.is_none_or(|due_at_ms| now_ms < due_at_ms));
        for active in &expired {
            let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
            auras.remove_auras_due_to_spell_like_cpp(
                active.spell_id,
                active.caster,
                active.effect_mask,
            );
            auras.clear_visible(active.slot);
        }
        self.refresh_active_taunt_states_like_cpp();
        expired.into_iter().map(|active| active.slot).collect()
    }

    fn refresh_active_taunt_states_like_cpp(&mut self) {
        let active_casters: Vec<_> = self
            .active_taunts_like_cpp
            .iter()
            .map(|active| active.caster)
            .collect();
        let combat = &mut self.creature.unit_mut().subsystems_mut().combat;
        for guid in combat.sorted_threat_guids() {
            combat.set_threat_taunt_state(guid, wow_entities::ThreatTauntState::None);
        }
        for (priority, caster) in active_casters.into_iter().enumerate() {
            combat.set_threat_taunt_state(
                caster,
                wow_entities::ThreatTauntState::Taunt(priority as u32 + 1),
            );
        }
        // C++ `ThreatManager::TauntUpdate` finishes with
        // `EvaluateSuppressed(true)`. The runtime tick owns the target aura
        // snapshots needed for `ShouldBeSuppressed`, so retain that event
        // until the next selection pass.
        combat.request_taunt_suppression_reevaluation_like_cpp();
    }

    pub fn reset_combat(&mut self) -> Vec<u8> {
        let active_taunts = std::mem::take(&mut self.active_taunts_like_cpp);
        for active in &active_taunts {
            let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
            auras.remove_auras_due_to_spell_like_cpp(
                active.spell_id,
                active.caster,
                active.effect_mask,
            );
            auras.clear_visible(active.slot);
        }
        // C++ `AssistDelayEvent` is owned by the caller, not by an assistant's
        // combat state. Preserve represented pending requests across this
        // assistant's independent combat/evade reset; execution revalidates
        // `CanAssistTo` when the delay expires. A real `Unit::AttackStop`
        // resets `m_AlreadyCallAssistance` for the next engagement.
        self.assistance_called_like_cpp = false;
        self.reset_creature_spell_schedule_like_cpp();
        self.creature.reset_ai_combat(self.now_ms());
        self.sync_runtime_motion_master_like_cpp();
        active_taunts
            .into_iter()
            .map(|active| active.slot)
            .collect()
    }

    pub(crate) fn sync_runtime_motion_master_like_cpp(&mut self) {
        let expected_default = match self.creature.default_movement_type() {
            MovementGeneratorType::Idle => RuntimeMovementGeneratorType::Idle,
            MovementGeneratorType::Random => RuntimeMovementGeneratorType::Random,
            MovementGeneratorType::Waypoint => RuntimeMovementGeneratorType::Waypoint,
        };
        if self
            .runtime_motion_master
            .current_kind_for_slot(RuntimeMovementSlot::Default)
            != Some(expected_default)
        {
            self.runtime_motion_master.add(
                Self::runtime_default_generator_like_cpp(&self.creature),
                RuntimeMovementSlot::Default,
            );
        }

        let expected_chase_target = self.creature.ai_ownership().combat_target.filter(|_| {
            self.creature.ai_state() == CreatureAiState::InCombat && self.creature.is_alive()
        });
        if self.runtime_chase_target != expected_chase_target {
            self.runtime_motion_master.remove_kind(
                RuntimeMovementGeneratorType::Chase,
                RuntimeMovementSlot::Active,
            );
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .remove_generator_kind(MovementGeneratorKind::Chase, MovementSlot::Active);
            if let Some(target) = expected_chase_target {
                self.runtime_motion_master.add(
                    Box::new(ChaseMovementGenerator::new(target, None, None)),
                    RuntimeMovementSlot::Active,
                );
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .move_chase_like_cpp(target);
            }
            self.runtime_chase_target = expected_chase_target;
        }

        // The represented subsystem already owns concrete Point/Distract/
        // Charge/etc. lifecycle. Mirror its selected active entry into the
        // runtime selector so adding normal-priority chase cannot incorrectly
        // interrupt a higher-priority generator. C++ keeps both entries in the
        // MotionMaster multiset and selects by mode/priority.
        let expected_represented_active = {
            let motion = &self.creature.unit().subsystems().motion;
            (motion.current_slot() == MovementSlot::Active)
                .then(|| motion.current_movement_generator())
                .filter(|generator| generator.kind != MovementGeneratorKind::Chase)
                .and_then(RuntimeRepresentedActiveGeneratorLikeCpp::from_represented)
        };
        let expected_key = expected_represented_active
            .as_ref()
            .map(RuntimeRepresentedActiveGeneratorLikeCpp::key);
        let runtime_proxy_missing = expected_key.is_some_and(|key| {
            !self
                .runtime_motion_master
                .has_generator_kind(key.kind, RuntimeMovementSlot::Active)
        });
        if self.runtime_represented_active != expected_key || runtime_proxy_missing {
            if let Some(previous) = self.runtime_represented_active {
                self.runtime_motion_master
                    .remove_kind(previous.kind, RuntimeMovementSlot::Active);
            }
            if let Some(generator) = expected_represented_active {
                self.runtime_motion_master
                    .add(Box::new(generator), RuntimeMovementSlot::Active);
            }
            self.runtime_represented_active = expected_key;
        }
    }

    fn finalize_runtime_represented_generator_like_cpp(
        &mut self,
        mut generator: MovementGeneratorRef,
    ) {
        match generator.kind {
            MovementGeneratorKind::Point => {
                let finalize = generator.finalize_point_like_cpp(true, true);
                if finalize.clear_roaming_move {
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                if let Some(inform) = finalize.inform {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Rotate => {
                if let Some(inform) = generator.finalize_rotate_like_cpp(true, true).inform {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            MovementGeneratorKind::Distract => {
                let finalize = generator.finalize_distract_like_cpp(true, true);
                if finalize.set_home_orientation {
                    let current = self.position();
                    let home = self.home_position();
                    self.creature.set_ai_position(Position::new(
                        current.x,
                        current.y,
                        current.z,
                        home.orientation,
                    ));
                }
            }
            MovementGeneratorKind::Effect => {
                if let Some(inform) = generator.finalize_generic_like_cpp(true) {
                    self.creature
                        .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
                }
            }
            _ => {}
        }
    }

    fn tick_runtime_represented_motion_like_cpp(&mut self, diff_ms: u32) {
        let unit = self.creature.unit();
        let active_spline = self.active_move_spline.as_ref();
        let context = MotionMasterUpdateContext {
            diff_ms,
            can_move: !unit.has_unit_state(UnitState::NOT_MOVE.bits()),
            owner_exists: true,
            owner_is_standing: unit.is_stand_state_like_cpp(),
            spline_finalized: active_spline.is_none_or(MoveSpline::finalized),
            spline_cyclic: active_spline.is_some_and(MoveSpline::is_cyclic),
            current_orientation: self.position().orientation,
        };
        let outcome = self
            .creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .update_motion_master_like_cpp(context);
        if let MotionMasterUpdateOutcome::Updated {
            popped: Some(generator),
            ..
        } = outcome
        {
            self.finalize_runtime_represented_generator_like_cpp(generator);
        }
    }

    /// Advances the represented active lifecycle and the runtime selector once
    /// for this creature's frame, then returns the selected generator.
    pub fn tick_runtime_motion_master_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.sync_runtime_motion_master_like_cpp();
        self.tick_runtime_represented_motion_like_cpp(diff_ms);
        self.sync_runtime_motion_master_like_cpp();
        self.runtime_motion_master.update(diff_ms);
        self.runtime_motion_master_ticks = self.runtime_motion_master_ticks.saturating_add(1);
        self.runtime_motion_master.current_kind()
    }

    pub fn runtime_motion_master_current_kind_like_cpp(
        &self,
    ) -> Option<RuntimeMovementGeneratorType> {
        self.runtime_motion_master.current_kind()
    }

    pub const fn runtime_motion_master_ticks_like_cpp(&self) -> u64 {
        self.runtime_motion_master_ticks
    }

    pub fn take_damage(&mut self, damage: u32) -> bool {
        self.creature.take_ai_damage(damage, self.now_ms())
    }

    pub fn take_damage_before_death_state_like_cpp(&mut self, damage: u32) -> bool {
        self.creature
            .apply_ai_damage_before_death_state_like_cpp(damage, self.now_ms())
    }

    pub fn take_damage_before_death_state_at_game_time_like_cpp(
        &mut self,
        damage: u32,
        game_time_secs: i64,
    ) -> bool {
        let local_elapsed_ms = self.now_ms();
        self.creature
            .apply_ai_damage_before_death_state_at_game_time_like_cpp(
                damage,
                local_elapsed_ms,
                game_time_secs,
            )
    }

    pub fn complete_death_state_after_kill_hooks_like_cpp(&mut self) {
        self.complete_death_state_after_kill_hooks_at_game_time_like_cpp(game_time_secs_like_cpp());
    }

    pub fn complete_death_state_after_kill_hooks_at_game_time_like_cpp(
        &mut self,
        game_time_secs: i64,
    ) {
        let local_elapsed_ms = self.now_ms();
        self.creature
            .complete_ai_death_state_after_kill_hooks_like_cpp(local_elapsed_ms, game_time_secs);
    }

    pub fn all_loot_removed_from_corpse_like_cpp(
        &mut self,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        self.all_loot_removed_from_corpse_at_game_time_like_cpp(
            Instant::now(),
            game_time_secs_like_cpp(),
            decay_rate,
            is_fully_skinned,
        )
    }

    pub fn all_loot_removed_from_corpse_at_game_time_like_cpp(
        &mut self,
        now: Instant,
        game_time_secs: i64,
        decay_rate: f32,
        is_fully_skinned: bool,
    ) -> bool {
        let plan = self.creature.all_loot_removed_from_corpse(
            game_time_secs,
            decay_rate,
            is_fully_skinned,
        );
        if plan.is_empty() {
            return false;
        }

        // C++ stores `m_corpseRemoveTime` in the absolute GameTime domain;
        // the legacy AI mirror is elapsed milliseconds from `clock_started_at`.
        let corpse_remove_at = instant_from_respawn_time_like_cpp(
            self.creature.corpse_remove_time(),
            now,
            game_time_secs,
        );
        let corpse_remove_time_ms = corpse_remove_at
            .checked_duration_since(self.clock_started_at)
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0);
        self.creature
            .set_ai_corpse_despawn_at(Some(corpse_remove_time_ms));
        true
    }

    pub fn apply_corpse_loot_flags_after_death_state_like_cpp(
        &mut self,
        lootable: bool,
        can_skin: bool,
    ) {
        self.creature
            .apply_corpse_loot_flags_after_death_state_like_cpp(lootable, can_skin);
    }

    pub fn remove_lootable_dynamic_flag_like_cpp(&mut self) {
        let object = self.creature.unit_mut().world_mut().object_mut();
        object.remove_dynamic_flag(UnitDynFlags::Lootable as u32);
        object.force_dynamic_flags_update_like_cpp();
    }

    pub fn force_dynamic_flags_update_like_cpp(&mut self) {
        self.creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .force_dynamic_flags_update_like_cpp();
    }

    pub fn has_lootable_dynamic_flag_like_cpp(&self) -> bool {
        self.creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    }

    pub fn die(&mut self) {
        self.creature.mark_ai_dead(self.now_ms());
    }

    pub fn can_wander(&self) -> bool {
        self.creature.can_ai_wander()
    }

    pub fn try_aggro(&mut self, player_guid: ObjectGuid, player_pos: &Position) -> bool {
        self.creature.try_ai_aggro(player_guid, player_pos)
    }

    pub fn try_aggro_with_target_combat_reach_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        player_pos: &Position,
        player_combat_reach: f32,
    ) -> bool {
        self.creature
            .try_ai_aggro_with_target_combat_reach_like_cpp(
                player_guid,
                player_pos,
                player_combat_reach,
            )
    }

    pub fn should_respawn(&self) -> bool {
        self.creature.should_ai_respawn(self.now_ms())
    }

    pub fn respawn(&mut self) {
        self.creature.respawn_ai(self.now_ms());
    }

    fn set_movement_flags_like_cpp(&mut self, movement_flags: MovementFlag) {
        self.creature
            .set_movement_flags_runtime_like_cpp(movement_flags);
        self.create_data.movement_flags = movement_flags.bits();
    }

    fn apply_launch_movement_flags_like_cpp(&mut self, movement_flags: MovementFlag) {
        // C++ `MoveSplineInit::Launch` writes Unit::m_movementInfo before
        // initializing/sending the spline. Keep the create bridge in lockstep.
        self.set_movement_flags_like_cpp(movement_flags);
    }

    fn disable_spline_movement_like_cpp(&mut self) {
        // C++ `Unit::DisableSpline` and `MoveSplineInit::Stop` remove FORWARD.
        let mut movement_flags = self.creature.movement_flags_like_cpp();
        movement_flags.remove(MovementFlag::FORWARD);
        self.set_movement_flags_like_cpp(movement_flags);
    }

    pub fn movement_finished(&self) -> bool {
        if let Some(spline) = &self.active_move_spline {
            return spline.finalized();
        }
        self.creature
            .ai_ownership()
            .move_target
            .map(|_| {
                self.now_ms()
                    .saturating_sub(self.creature.ai_ownership().move_start_ms)
                    >= u64::from(self.creature.ai_ownership().move_duration_ms)
            })
            .unwrap_or(true)
    }

    pub fn interpolated_position(&self) -> Position {
        let Some(dst) = self.creature.ai_ownership().move_target else {
            return self.position();
        };
        let elapsed =
            self.now_ms()
                .saturating_sub(self.creature.ai_ownership().move_start_ms) as f32;
        let total = self.creature.ai_ownership().move_duration_ms as f32;
        if total <= 0.0 {
            return dst;
        }
        let src = self.position();
        let t = (elapsed / total).min(1.0);
        Position::new(
            src.x + (dst.x - src.x) * t,
            src.y + (dst.y - src.y) * t,
            src.z + (dst.z - src.z) * t,
            dst.orientation,
        )
    }

    pub fn begin_move(&mut self, dst: Position) {
        let dist = self.position().distance(&dst);
        let walk_speed = 2.5f32;
        let duration_ms = ((dist / walk_speed) * 1000.0) as u32;
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = Some(dst);
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = duration_ms.max(500);
        ai.spline_id = ai.spline_id.saturating_add(1);
    }

    fn launch_move_spline_init_like_cpp(
        &mut self,
        init: &mut MoveSplineInit,
        dst: Position,
    ) -> Option<(Position, MoveSpline)> {
        // C++ `MoveSplineInit::MoveSplineInit(Unit*)` snapshots `CanSwim()` into
        // every new spline before the generator customizes it
        // (`MoveSplineInit.cpp:198-207`).
        if self.creature.can_swim_like_cpp() {
            init.args.flags.insert(MoveSplineFlag::CAN_SWIM);
        } else {
            init.args.flags.remove(MoveSplineFlag::CAN_SWIM);
        }
        let spline_id = init.args.spline_id;
        let active_spline_position = self
            .active_move_spline
            .as_ref()
            .filter(|spline| !spline.finalized() && !spline.on_transport)
            .and_then(MoveSpline::compute_position);

        let now_ms = self.now_ms();
        let mut spline = self
            .active_move_spline
            .take()
            .unwrap_or_else(MoveSpline::new);
        let launch = init
            .launch(
                &mut spline,
                MoveSplineLaunchInput {
                    current_position: self.position(),
                    active_spline_position,
                    movement_flags: self.creature.movement_flags_like_cpp(),
                    selected_speed: if init.args.walk {
                        self.walk_speed_like_cpp()
                    } else {
                        self.run_speed_like_cpp()
                    },
                    run_speed: self.run_speed_like_cpp(),
                    assistance_speed_factor: 1.0,
                    on_transport: false,
                },
            )
            .ok()?;
        let duration_ms = launch.duration_ms.max(1) as u32;
        {
            let ai = self.creature.ai_ownership_mut();
            ai.move_target = Some(dst);
            ai.move_start_ms = now_ms;
            ai.move_duration_ms = duration_ms;
            ai.spline_id = spline_id;
        }
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .launch_spline(
                spline_id,
                duration_ms,
                position_to_i32_tuple(dst),
                false,
                false,
                None,
            );
        self.creature
            .unit_mut()
            .add_unit_state(UnitState::ROAMING_MOVE.bits());
        self.apply_launch_movement_flags_like_cpp(launch.movement_flags);
        self.active_move_spline = Some(spline.clone());
        Some((launch.real_position, spline))
    }

    fn walk_speed_like_cpp(&self) -> f32 {
        (self.create_data.speed_walk_rate * 2.5).max(0.01)
    }

    fn run_speed_like_cpp(&self) -> f32 {
        (self.create_data.speed_run_rate * 7.0).max(0.01)
    }

    pub fn begin_move_spline_like_cpp(&mut self, dst: Position) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_to(dst);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn begin_random_move_spline_like_cpp(
        &mut self,
        dst: Position,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_walk(self.random_movement_walk_like_cpp());
        init.move_to(dst);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn begin_random_move_spline_by_path_like_cpp<I>(
        &mut self,
        path: I,
    ) -> Option<(Position, MoveSpline)>
    where
        I: IntoIterator<Item = Position>,
    {
        let points = path.into_iter().collect::<Vec<_>>();
        let dst = points.last().copied()?;
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_walk(self.random_movement_walk_like_cpp());
        init.move_by_path(points, 0);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn random_movement_walk_like_cpp(&self) -> bool {
        match self.creature.random_movement_type_like_cpp() {
            value if value == ConstantsCreatureRandomMovementType::CanRun as u8 => self
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
            value if value == ConstantsCreatureRandomMovementType::AlwaysRun as u8 => false,
            _ => true,
        }
    }

    pub fn begin_move_spline_by_path_like_cpp<I>(
        &mut self,
        path: I,
    ) -> Option<(Position, MoveSpline)>
    where
        I: IntoIterator<Item = Position>,
    {
        let points = path.into_iter().collect::<Vec<_>>();
        let dst = points.last().copied()?;
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_by_path(points, 0);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
    }

    pub fn initialize_default_waypoint_movement_like_cpp(
        &mut self,
        loaded_path: Option<WaypointPath>,
    ) -> WaypointMovementAction {
        self.creature
            .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Waypoint);
        let mut generator = WaypointMovementGenerator::from_db_path_id(0, true);
        let action = generator.initialize_like_cpp(
            true,
            self.creature.waypoint_path_id_like_cpp(),
            loaded_path,
        );
        if action == WaypointMovementAction::StopMoving {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .stop_moving();
            self.creature
                .set_ai_state(wow_entities::CreatureAiState::WalkingWaypoint);
        }
        self.active_waypoint_generator = Some(generator);
        action
    }

    pub fn initialize_default_waypoint_movement_with_path_resolver_like_cpp(
        &mut self,
        mut resolve_path: impl FnMut(u32) -> Option<WaypointPath>,
    ) -> WaypointMovementAction {
        let owner_path_id = self.creature.waypoint_path_id_like_cpp();
        let loaded_path = (owner_path_id != 0)
            .then(|| resolve_path(owner_path_id))
            .flatten();
        self.initialize_default_waypoint_movement_like_cpp(loaded_path)
    }

    pub fn initialize_default_random_movement_like_cpp(&mut self) -> bool {
        if self.creature.default_movement_type() != wow_entities::MovementGeneratorType::Random
            || !self.is_alive()
            || self.creature.ai_ownership().wander_radius <= 0.0
        {
            return false;
        }

        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .stop_moving();
        self.active_move_spline = None;
        let next_wander_steps_roll = self.runtime_rng_like_cpp.gen_range(2..=10);
        let snapshot = self.random_unit_snapshot_like_cpp(
            true,
            RandomPathResult::Success,
            0.0,
            0.0,
            next_wander_steps_roll,
            4,
            0,
        );
        let mut generator = RandomMovementGenerator::new(0.0, None);
        let _ = generator.initialize_like_cpp(true, snapshot);
        self.active_random_generator = Some(generator);
        // C++ `RandomMovementGenerator<Creature>::DoInitialize` drops the
        // generator's `PathGenerator` (`RandomMovementGenerator.cpp:95`), so the
        // next query starts from an empty corridor.
        self.active_random_path_poly_refs.clear();
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = None;
        ai.move_start_ms = now_ms;
        ai.move_duration_ms = 0;
        ai.wander_delay_ms = 0;
        ai.wander_steps_remaining = next_wander_steps_roll;
        ai.state = CreatureAiState::Idle;
        true
    }

    /// C++ `PathGenerator::CreateFilter` + `PathGenerator::UpdateFilter`
    /// (`PathGenerator.cpp:648-698`) derive the Detour query filter from the
    /// *owner*, never from a constant: `Creature::CanWalk()` adds `NAV_GROUND`,
    /// `Creature::CanEnterWater()` adds `NAV_WATER | NAV_MAGMA_SLIME`, and
    /// `Unit::IsInCombat() || Creature::IsInEvadeMode()` adds
    /// `NAV_GROUND_STEEP`.
    ///
    /// Boundary: `UpdateFilter` also ORs in
    /// `Map::GetForceEnabled/DisabledNavMeshFilterFlags()` and, while the owner
    /// `IsInWater()/IsUnderWater()`, `GetNavTerrain()` from
    /// `Map::GetLiquidStatus`. Neither map-level source exists in the Rust
    /// runtime yet, so those stay at their neutral values here.
    pub fn path_query_filter_context_like_cpp(&self) -> PathQueryFilterContext {
        // C++ `Unit::IsInCombat()` is `HasUnitFlag(UNIT_FLAG_IN_COMBAT)`, and C++
        // really does set that flag on entering combat. RustyCore's
        // `Creature::enter_ai_combat` sets the AI state and the attacking GUID
        // but not the client-visible flag, so reading the flag alone would leave
        // every chasing creature without `NAV_GROUND_STEEP`. Both signals are
        // consulted, so the filter is correct today and still correct once the
        // flag itself is maintained.
        //
        // Boundary: that missing `UNIT_FLAG_IN_COMBAT` is a separate parity
        // defect with client-visible UpdateField consequences; it is not fixed
        // here.
        let in_combat = self
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(wow_constants::unit::UnitFlags::IN_COMBAT)
            || self.creature.is_in_combat();
        PathQueryFilterContext::creature(
            self.creature.can_walk_like_cpp(),
            self.creature.can_enter_water_like_cpp(),
            in_combat,
            self.creature.is_in_evade_mode_like_cpp(),
        )
    }

    /// Owner capabilities `PathGenerator::BuildPolyPath` reads off `_source`
    /// when a position has no navmesh polygon: `Creature::CanFly()`
    /// (`Creature.h:126`), `Creature::CanSwim()` (`Creature.cpp:2912-2921`) and
    /// `Unit::IsFalling()` (`Unit.cpp:12173-12176`, movement flags **or** the
    /// active spline falling).
    pub fn detour_owner_capabilities_like_cpp(&self) -> DetourOwnerCapabilitiesLikeCpp {
        let spline_falling = self
            .active_move_spline
            .as_ref()
            .is_some_and(|spline| spline.flags().contains(MoveSplineFlag::FALLING));
        DetourOwnerCapabilitiesLikeCpp {
            can_fly: self.creature.can_fly_like_cpp(),
            can_swim: self.creature.can_swim_like_cpp(),
            is_falling: self
                .creature
                .movement_flags_like_cpp()
                .intersects(MovementFlag::FALLING | MovementFlag::FALLING_FAR)
                || spline_falling,
        }
    }

    /// Drives the home (evade-return) generator for one frame, mirroring C++
    /// `HomeMovementGenerator<Creature>` (`HomeMovementGenerator.cpp:48-157`).
    ///
    /// C++ `SetTargetLocation` launches `init.MoveTo(home)` with the defaults
    /// `generatePath = true, forceDestination = false`, so the return trip is a
    /// real navmesh path — not a teleport.
    pub fn update_runtime_home_movement_like_cpp(
        &mut self,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> ChaseTickOutcomeLikeCpp {
        let snapshot = self.home_unit_snapshot_like_cpp();
        let from_update = self.active_home_generator.is_some();
        let action = match self.active_home_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, snapshot),
            None => {
                let mut generator = HomeMovementGenerator::new();
                let action = generator.initialize_like_cpp(true, snapshot);
                self.active_home_generator = Some(generator);
                // C++ `CreatureAI::EnterEvadeMode` adds `UNIT_STATE_EVADE`
                // immediately before `MoveTargetedHome()` (`CreatureAI.cpp:237`),
                // and `HomeMovementGenerator::DoFinalize` is what clears it
                // (`HomeMovementGenerator.cpp:143`). The state is what makes the
                // creature immune to attacks and un-aggroable while it walks
                // back; without it, this now multi-tick return would let a
                // player damage and re-aggro a fully reset creature.
                //
                // Boundary: C++ sets it one step earlier, in the AI evade entry,
                // after `_EnterEvadeMode()` bookkeeping and only on the
                // no-charmer branch. This runtime has no live AI evade entry,
                // so the state is added where the return actually begins. A
                // full `CreatureAI::EnterEvadeMode` port stays with M2.5.
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::EVADE.bits());
                action
            }
        };

        match action {
            wow_movement::HomeMovementAction::Continue => ChaseTickOutcomeLikeCpp::Idle,
            // C++ `SetTargetLocation` sets `MOVEMENTGENERATOR_FLAG_INTERRUPTED`
            // and returns without launching while ROOT/STUNNED/DISTRACTED; the
            // generator stays installed and only the *next* `DoUpdate` sets
            // `INFORM_ENABLED` and finalizes (`HomeMovementGenerator.cpp:53-58,
            // 117-122`). Finalizing here in the initialize frame would skip that
            // `INFORM_ENABLED`, suppressing `JustReachedHome` and clearing evade
            // one frame early. So `Interrupted` from initialization keeps the
            // generator; only a `Finished` from an update finalizes.
            wow_movement::HomeMovementAction::Interrupted if !from_update => {
                ChaseTickOutcomeLikeCpp::Idle
            }
            wow_movement::HomeMovementAction::Interrupted
            | wow_movement::HomeMovementAction::Finished => {
                self.finish_home_movement_like_cpp();
                ChaseTickOutcomeLikeCpp::Idle
            }
            wow_movement::HomeMovementAction::Launch(plan) => {
                self.creature
                    .unit_mut()
                    .clear_unit_state(plan.clear_unit_state_mask);
                self.creature.unit_mut().add_unit_state(plan.add_unit_state);

                let destination =
                    self.normalize_path_position_z_like_cpp(plan.destination, terrain);
                let detour_path = should_try_pathfinding
                    .then(|| {
                        // Built after `UNIT_STATE_EVADE` was added above, which
                        // is what makes `UpdateFilter` include
                        // `NAV_GROUND_STEEP` — C++ sets evade before
                        // `MoveTargetedHome` constructs the path, so sampling the
                        // filter earlier would path the return without steep
                        // ground.
                        resolve_path(CreaturePathQueryLikeCpp {
                            start: self.position(),
                            destination,
                            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                            force_destination: false,
                            filter_context: self.path_query_filter_context_like_cpp(),
                            owner: self.detour_owner_capabilities_like_cpp(),
                            // C++ `MoveSplineInit::MoveTo` builds a fresh
                            // `PathGenerator`, so the home leg has no corridor to
                            // reuse.
                            previous_poly_refs: Vec::new(),
                        })
                    })
                    .flatten();

                // C++ goes through `MoveSplineInit::MoveTo(..., generatePath)`,
                // which falls back to a direct two-point spline whenever the path
                // is unusable (`MoveSplineInit.cpp:261-277`).
                let path = detour_path
                    .as_ref()
                    .map(|detour_path| {
                        self.path_generator_from_detour_for_creature_like_cpp(
                            destination,
                            detour_path,
                            false,
                            terrain,
                        )
                    })
                    .filter(|path| !path.path_type().contains(PathType::NOPATH));

                let spline_id = self.spline_id().saturating_add(1);
                let mut init = MoveSplineInit::new(spline_id);
                init.set_walk(plan.walk);
                match path {
                    Some(path) => init.move_by_path(path.path_points().to_vec(), 0),
                    None => init.move_to(destination),
                }
                init.set_facing_angle(plan.facing);

                match self.launch_move_spline_init_like_cpp(&mut init, destination) {
                    Some((from, spline)) => ChaseTickOutcomeLikeCpp::Launched(from, spline),
                    None => {
                        self.finish_home_movement_like_cpp();
                        ChaseTickOutcomeLikeCpp::Idle
                    }
                }
            }
        }
    }

    fn home_unit_snapshot_like_cpp(&self) -> wow_movement::HomeUnitSnapshot {
        wow_movement::HomeUnitSnapshot {
            owner_alive: self.creature.is_alive(),
            owner_unit_state: self.creature.unit().unit_state(),
            home_position: self.creature.ai_ownership().home_position,
            move_spline_finalized: self
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            can_swim_out_of_combat: !self.creature.is_missing_can_swim_flag_out_of_combat(),
            is_vehicle: false,
        }
    }

    /// C++ `HomeMovementGenerator<Creature>::DoFinalize` reached-home payload:
    /// clears `UNIT_STATE_ROAMING_MOVE | UNIT_STATE_EVADE` and reports
    /// `JustReachedHome` (`HomeMovementGenerator.cpp:141-157`).
    ///
    /// Boundary: the spawn-health, creature-addon and sparring-health reloads
    /// C++ performs there are respawn-owned work in this runtime and stay with
    /// the lifecycle tick.
    fn finish_home_movement_like_cpp(&mut self) {
        let snapshot = self.home_unit_snapshot_like_cpp();
        let finalize = self
            .active_home_generator
            .as_mut()
            .map(|generator| generator.finalize_like_cpp(true, true, snapshot));
        if let Some(finalize) = finalize {
            // C++ clears `UNIT_STATE_ROAMING_MOVE | UNIT_STATE_EVADE` here when
            // the generator was active (`HomeMovementGenerator.cpp:141-143`).
            self.creature
                .unit_mut()
                .clear_unit_state(finalize.clear_unit_state_mask);
            if finalize.remove_can_swim_flag {
                self.creature.restore_can_swim_flag_after_home_like_cpp();
            }
            if finalize.just_reached_home {
                // C++ `SetSpawnHealth()` precedes `AI()->JustReachedHome()`
                // (`HomeMovementGenerator.cpp:148-156`). Addon/sparring health
                // overlays remain respawn-owned in this runtime.
                self.creature.set_spawn_health_like_cpp();
                self.home_health_restored_pending_like_cpp = true;
                self.creature.record_ai_just_reached_home();
            }
        }
        self.active_home_generator = None;
        self.creature.ai_ownership_mut().move_target = None;
        self.creature.set_ai_state(CreatureAiState::Idle);
    }

    pub fn take_home_health_restored_pending_like_cpp(&mut self) -> bool {
        std::mem::take(&mut self.home_health_restored_pending_like_cpp)
    }

    /// The chase generator currently selected for this creature, kept alongside
    /// the random/waypoint ones so its C++ state (`_lastTargetPosition`,
    /// `_rangeCheckTimer`, `_movingTowards`, `_path`) survives between ticks.
    pub fn active_chase_generator_like_cpp(&self) -> Option<&ChaseMovementGenerator> {
        self.active_chase_generator.as_ref()
    }

    /// C++ `WorldObject::GetNearPoint2D` + `GetNearPoint`
    /// (`Object.cpp:3379-3441`): a point `distance_2d` beyond the combined
    /// combat reaches, at `absolute_angle` around the target, with Z snapped by
    /// the searcher's `UpdateAllowedPositionZ`.
    ///
    /// Boundary: C++ also sweeps the angle in `M_PI/8` steps until the candidate
    /// is in line of sight when `CONFIG_DETECT_POS_COLLISION` is on. VMap line of
    /// sight is still a stub here, so the first candidate is taken.
    fn near_point_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
        distance_2d: f32,
        absolute_angle: f32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        let effective_reach =
            target.combat_reach + self.creature.unit().data().combat_reach.max(0.0);
        let radius = effective_reach + distance_2d;
        let point = Position::new(
            target.position.x + radius * absolute_angle.cos(),
            target.position.y + radius * absolute_angle.sin(),
            target.position.z,
            0.0,
        );
        self.normalize_path_position_z_like_cpp(point, terrain)
    }

    fn chase_unit_snapshot_like_cpp(
        &self,
        target: ChaseTargetSnapshotLikeCpp,
    ) -> wow_movement::ChaseUnitSnapshot {
        let unit = self.creature.unit();
        let owner_combat_reach = unit.data().combat_reach.max(0.0);
        // C++ `Unit::GetMeleeRange`: reaches plus 4/3, floored at
        // `NOMINAL_MELEE_RANGE` (`Unit.cpp:664-668`).
        let owner_melee_range = (owner_combat_reach + target.combat_reach + 4.0 / 3.0)
            .max(NOMINAL_MELEE_RANGE_LIKE_CPP);
        let can_enter_water = self.creature.can_enter_water_like_cpp();
        let can_walk = self.creature.can_walk_like_cpp();
        let can_fly = self.creature.can_fly_like_cpp();
        wow_movement::ChaseUnitSnapshot {
            owner_position: self.position(),
            target_position: target.position,
            owner_combat_reach,
            target_combat_reach: target.combat_reach,
            owner_melee_range,
            owner_alive: self.creature.is_alive(),
            target_in_world: target.in_world,
            can_move: !unit.has_unit_state(UnitState::NOT_MOVE.bits()),
            movement_prevented_by_casting: unit.has_unit_state(UnitState::CASTING.bits()),
            owner_victim_is_target: self.creature.ai_ownership().combat_target == Some(target.guid),
            owner_has_chase_move: unit.has_unit_state(UnitState::CHASE_MOVE.bits()),
            owner_movespline_finalized: self
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            // C++ `IsMutualChase` needs the target's own MotionMaster; only
            // creatures chase, and the runtime has no cross-object accessor in
            // this step, so a mutual chase is never detected yet. That only ever
            // *keeps* the chase angle applied, never drops a real constraint.
            mutual_chase: false,
            // VMap line of sight is a stub; C++ `PositionOkay` requires it.
            owner_has_los: true,
            // C++ `Unit::isInAccessiblePlaceFor` picks exactly one branch from
            // the victim's real `IsInWater()`. With no liquid data for creature
            // victims, taking either branch would be a guess, and guessing
            // "not in water" is the harmful one: it makes an aquatic,
            // non-walking chaser report `CannotReachTarget` and freeze on a
            // victim C++ would let it reach. The unknown case therefore accepts
            // the union of both branches — the Detour query and its
            // `PATHFIND_NOPATH` bail-out remain the real gate.
            target_accessible: match target.in_water {
                Some(true) => can_enter_water,
                Some(false) => can_walk || can_fly,
                None => can_enter_water || can_walk || can_fly,
            },
            owner_can_fly: can_fly,
            owner_is_creature: true,
            creature_is_pet: self.creature.unit().world().object().guid().is_pet(),
            creature_chase_walk: match self.creature.chase_movement_type_like_cpp() {
                value if value == wow_constants::CreatureChaseMovementType::CanWalk as u8 => {
                    wow_movement::ChaseWalkMode::CanWalk
                }
                value if value == wow_constants::CreatureChaseMovementType::AlwaysWalk as u8 => {
                    wow_movement::ChaseWalkMode::AlwaysWalk
                }
                _ => wow_movement::ChaseWalkMode::Default,
            },
            owner_is_walking: self
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
        }
    }

    /// Advances the selected chase generator for one frame and executes its
    /// decision, mirroring C++ `ChaseMovementGenerator::Update`
    /// (`ChaseMovementGenerator.cpp:94-240`).
    ///
    /// The path query itself is delegated so the caller keeps ownership of the
    /// off-thread pathfinder, exactly as the random and waypoint arms do.
    pub fn update_runtime_chase_movement_like_cpp(
        &mut self,
        diff_ms: u32,
        target: ChaseTargetSnapshotLikeCpp,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> ChaseTickOutcomeLikeCpp {
        // C++ installs a *new* `ChaseMovementGenerator` per `MoveChase` call and
        // its `AbstractFollower` is bound to that victim for the generator's
        // whole life (`ChaseMovementGenerator.cpp:68-76`,
        // `AbstractFollower.cpp:21-31`). Reusing one across a victim switch
        // would carry the previous follower, `_lastTargetPosition`,
        // `_rangeCheckTimer`, `_movingTowards` and the arrival
        // `MovementInform` counter onto the new target, so the generator is
        // rebuilt whenever the victim differs.
        let victim_changed = self
            .active_chase_generator
            .as_ref()
            .is_none_or(|generator| generator.target() != Some(target.guid));
        if victim_changed {
            let mut generator = ChaseMovementGenerator::new(target.guid, None, None);
            generator.initialize_like_cpp();
            self.active_chase_generator = Some(generator);
            self.active_chase_path_poly_refs.clear();
        }

        let snapshot = self.chase_unit_snapshot_like_cpp(target);
        let action = match self.active_chase_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, target.in_world, diff_ms, snapshot),
            None => return ChaseTickOutcomeLikeCpp::Idle,
        };

        match action {
            wow_movement::ChaseMovementAction::Continue => ChaseTickOutcomeLikeCpp::Idle,
            // C++ chase `Update` returns false when the victim is gone or has
            // left the world (`ChaseMovementGenerator.cpp:97,101-103`), which
            // pops the generator via `MotionMaster::Update` and runs `Finalize`.
            // Clearing only the corridor would leave the generator,
            // `UNIT_STATE_CHASE_MOVE` and the in-flight spline intact, so the
            // creature keeps sliding toward the corpse and is re-selected as
            // chasing every tick.
            wow_movement::ChaseMovementAction::Finished => {
                match self.finalize_runtime_chase_movement_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            // C++ `StopMoving()` clears `UNIT_STATE_MOVING` (which contains
            // `UNIT_STATE_CHASE_MOVE`) and stops the spline.
            wow_movement::ChaseMovementAction::StopMoving
            | wow_movement::ChaseMovementAction::CannotReachTarget => {
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::CHASE_MOVE.bits());
                self.active_chase_path_poly_refs.clear();
                match self.stop_move_spline_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            wow_movement::ChaseMovementAction::StopMovingAndFaceInform(inform)
            | wow_movement::ChaseMovementAction::ClearChaseMoveAndFaceInform(inform) => {
                // C++ `SetInFront(target)` only turns the owner server-side, and
                // then reports arrival to the AI.
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::CHASE_MOVE.bits());
                let mut position = self.position();
                position.orientation = absolute_angle_like_cpp(position, target.position);
                self.creature.set_ai_position(position);
                self.creature.record_ai_movement_inform(
                    inform.movement_type.trinity_id(),
                    inform.target_counter,
                );
                self.active_chase_path_poly_refs.clear();
                match self.stop_move_spline_like_cpp() {
                    Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
            wow_movement::ChaseMovementAction::Launch(plan) => {
                if plan.direction_changed {
                    // C++ replaces the owned `PathGenerator` before
                    // `CalculatePath` when chase direction flips
                    // (`ChaseMovementGenerator.cpp:171-175`). The retained
                    // Detour corridor belongs to that old path object.
                    self.active_chase_path_poly_refs.clear();
                }

                // C++ picks the target centre when closing in without an angle
                // constraint, otherwise a point on the tolerance ring
                // (`ChaseMovementGenerator.cpp:177-191`).
                let destination = if plan.move_toward && plan.desired_relative_angle.is_none() {
                    target.position
                } else {
                    let hitbox_sum =
                        self.creature.unit().data().combat_reach.max(0.0) + target.combat_reach;
                    let absolute_angle = match plan.desired_relative_angle {
                        Some(relative) => wow_movement::normalize_orientation_like_cpp(
                            target.position.orientation + relative,
                        ),
                        None => absolute_angle_like_cpp(target.position, self.position()),
                    };
                    self.near_point_like_cpp(
                        target,
                        plan.desired_distance - hitbox_sum,
                        absolute_angle,
                        terrain,
                    )
                };

                // C++ `ChaseMovementGenerator::Update` calls
                // `CalculatePath(x, y, z, owner->CanFly())`
                // (`ChaseMovementGenerator.cpp:196`), and `_forceDestination` is
                // consumed *inside* `BuildPointPath` (`PathGenerator.cpp:603-619`)
                // — setting it afterwards on the Rust `PathGenerator` would
                // record the flag without rebuilding the clamped point path, so
                // it has to travel with the query itself.
                let mut query_failed = false;
                let detour_path = if should_try_pathfinding {
                    // Built here, not by the caller: the victim-change reset
                    // above may already have dropped the retained corridor, and
                    // reusing the previous victim's `_pathPolyRefs` would let the
                    // ~80% prefix branch steer the first spline back toward the
                    // old target.
                    let resolved = resolve_path(CreaturePathQueryLikeCpp {
                        start: self.position(),
                        destination,
                        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                        force_destination: plan.allow_flying_path,
                        filter_context: self.path_query_filter_context_like_cpp(),
                        owner: self.detour_owner_capabilities_like_cpp(),
                        previous_poly_refs: self.active_chase_path_poly_refs.clone(),
                    });
                    // The resolver already answers a missing navmesh/tile with
                    // the C++ `BuildShortcut()` path, so `None` here means the
                    // query was attempted and genuinely failed. C++ has no such
                    // case — its own failures went through `BuildShortcut()` +
                    // `PATHFIND_NOPATH` — so it must not be confused with
                    // "there is no navmesh", which is launchable.
                    query_failed = resolved.is_none();
                    resolved
                } else {
                    None
                };

                let mut path = match detour_path.as_ref() {
                    Some(detour_path) => self.path_generator_from_detour_for_creature_like_cpp(
                        destination,
                        detour_path,
                        plan.allow_flying_path,
                        terrain,
                    ),
                    None if query_failed => {
                        // Reproduce the C++ `PATHFIND_NOPATH` branch so the
                        // bail-out below stops and retries.
                        let mut path = PathGenerator::new();
                        path.apply_detour_path_like_cpp(
                            self.position(),
                            destination,
                            destination,
                            [],
                            &[],
                            PathType::NOPATH,
                            plan.allow_flying_path,
                        );
                        path
                    }
                    None => {
                        // Pathfinding is off for this map/owner: C++
                        // `CalculatePath` answers with `BuildShortcut()` and
                        // `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`, which
                        // chase launches (`PathGenerator.cpp:79-86`).
                        let mut path = PathGenerator::new();
                        path.calculate_without_navmesh_like_cpp(
                            self.position(),
                            destination,
                            plan.allow_flying_path,
                        );
                        path
                    }
                };

                // C++ bails out only on `PATHFIND_NOPATH`; SHORTCUT, INCOMPLETE,
                // SHORT and FARFROMPOLY all proceed
                // (`ChaseMovementGenerator.cpp:197-203`).
                if path.path_type().contains(PathType::NOPATH) {
                    if let Some(generator) = self.active_chase_generator.as_mut() {
                        generator.cannot_reach_target = true;
                    }
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::CHASE_MOVE.bits());
                    self.active_chase_path_poly_refs.clear();
                    return match self.stop_move_spline_like_cpp() {
                        Some(stop) => ChaseTickOutcomeLikeCpp::Stopped(stop),
                        None => ChaseTickOutcomeLikeCpp::Idle,
                    };
                }

                if plan.shorten_path {
                    // C++ shortens against the target's exact position, using
                    // line of sight from each candidate; VMap LOS is a stub, so
                    // every candidate is treated as visible.
                    path.shorten_path_until_dist_like_cpp(
                        target.position,
                        plan.desired_distance,
                        |_| true,
                    );
                }

                if let Some(generator) = self.active_chase_generator.as_mut() {
                    // C++ clears `CannotReachTarget` after a successful
                    // `CalculatePath` and enables the next arrival inform
                    // immediately before launching the spline. A failed query
                    // must preserve the previous inform lifecycle.
                    generator.confirm_path_ready_like_cpp();
                }
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::CHASE_MOVE.bits());

                let points = path.path_points().to_vec();
                let Some(dst) = points.last().copied() else {
                    return ChaseTickOutcomeLikeCpp::Idle;
                };
                let spline_id = self.spline_id().saturating_add(1);
                let mut init = MoveSplineInit::new(spline_id);
                init.set_walk(plan.walk);
                init.move_by_path(points, 0);
                // C++ `init.SetFacing(target)` is client-side target tracking.
                init.set_facing_target_with_angle(
                    target.guid,
                    absolute_angle_like_cpp(self.position(), target.position),
                );

                match self.launch_move_spline_init_like_cpp(&mut init, dst) {
                    Some((from, spline)) => {
                        if let Some(detour_path) = detour_path.as_ref() {
                            self.active_chase_path_poly_refs
                                .clone_from(&detour_path.poly_refs);
                        } else {
                            self.active_chase_path_poly_refs.clear();
                        }
                        if let Some(generator) = self.active_chase_generator.as_mut() {
                            generator.confirm_launch_like_cpp(plan);
                        }
                        ChaseTickOutcomeLikeCpp::Launched(from, spline)
                    }
                    None => ChaseTickOutcomeLikeCpp::Idle,
                }
            }
        }
    }

    /// The corridor the random generator's `PathGenerator` still holds, for
    /// callers that build its next path request.
    pub fn active_random_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_random_path_poly_refs
    }

    /// Same, for the chase generator.
    pub fn active_chase_path_poly_refs_like_cpp(&self) -> &[u64] {
        &self.active_chase_path_poly_refs
    }

    /// Retires the chase generator the way C++ `MotionMaster` does when chase
    /// `Update` returns false: `Finalize` clears `UNIT_STATE_CHASE_MOVE` and
    /// `SetCannotReachTarget(false)`, and the generator is removed so a lower
    /// slot resumes (`ChaseMovementGenerator.cpp:251-260`). The superseded
    /// spline is stopped so the creature does not keep coasting toward a victim
    /// that is gone.
    ///
    /// Boundary: this is the movement half only. C++ also clears the victim
    /// through the kill/threat path (`UpdateVictim`, evade); the combat target
    /// and engagement are not reset here — that is M2.5 — so a creature still
    /// flagged in combat may have chase re-selected next tick, but it no longer
    /// drives toward the gone target with a stale `UNIT_STATE_CHASE_MOVE`.
    pub fn finalize_runtime_chase_movement_like_cpp(&mut self) -> Option<MoveSplineStopResult> {
        self.active_chase_generator = None;
        self.active_chase_path_poly_refs.clear();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::CHASE_MOVE.bits());
        self.stop_move_spline_like_cpp()
    }

    fn allowed_position_z_caps_like_cpp(&self) -> AllowedPositionZCaps {
        let hover_offset = if self
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::HOVER)
        {
            self.creature.unit().data().hover_height
        } else {
            0.0
        };
        AllowedPositionZCaps {
            on_transport: false,
            can_fly: self.creature.can_fly_like_cpp(),
            can_swim: self.creature.can_swim_like_cpp(),
            hover_offset,
        }
    }

    fn normalize_path_position_z_like_cpp(
        &self,
        point: Position,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        let Some(terrain) = terrain else {
            return point;
        };
        let probe_z = point.z + Z_OFFSET_FIND_HEIGHT;
        let static_ground =
            terrain.static_height_like_cpp(self.map_id(), point.x, point.y, probe_z);
        // C++ GetMapHeight combines terrain and VMap before
        // UpdateAllowedPositionZ clamps the point. Rust does not yet have the
        // VMap half, so lowering a valid elevated Detour point to terrain
        // destroys bridge/platform paths. Preserve elevations; the branch
        // below still raises points that are under known terrain.
        let mut ground = if static_ground >= point.z {
            static_ground
        } else {
            INVALID_HEIGHT
        };
        if ground <= INVALID_HEIGHT {
            let grid_ground = terrain.grid_height_like_cpp(self.map_id(), point.x, point.y);
            if grid_ground > INVALID_HEIGHT
                && point.z < grid_ground
                && grid_ground - point.z <= DEFAULT_HEIGHT_SEARCH
            {
                ground = grid_ground;
            }
        }
        let z = allowed_position_z_from_ground_like_cpp(
            true,
            ground,
            point.z,
            self.allowed_position_z_caps_like_cpp(),
        );
        Position::new(point.x, point.y, z, point.orientation)
    }

    fn path_generator_from_detour_for_creature_like_cpp(
        &self,
        destination: Position,
        detour_path: &DetourPolyPath,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> PathGenerator {
        path_generator_from_detour_with_normalizer_like_cpp(
            self.position(),
            destination,
            detour_path,
            force_destination,
            |point| self.normalize_path_position_z_like_cpp(point, terrain),
        )
    }

    pub fn update_default_random_movement_with_path_resolver_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_with_path_resolver_and_terrain_like_cpp(
            diff_ms,
            should_try_pathfinding,
            None,
            resolve_path,
        )
    }

    pub fn update_default_random_movement_with_path_resolver_and_terrain_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_after_optional_spline_like_cpp(
            diff_ms,
            should_try_pathfinding,
            terrain,
            true,
            resolve_path,
        )
    }

    pub(crate) fn update_default_random_movement_after_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        self.update_default_random_movement_after_optional_spline_like_cpp(
            diff_ms,
            should_try_pathfinding,
            terrain,
            false,
            resolve_path,
        )
    }

    fn update_default_random_movement_after_optional_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        update_spline: bool,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        if self.active_random_generator.is_none() {
            if !self.initialize_default_random_movement_like_cpp() {
                if self.state() == CreatureAiState::WalkingRandom && self.movement_finished() {
                    self.finish_move();
                    self.creature.set_ai_state(CreatureAiState::Idle);
                }
                return None;
            }
        }
        if update_spline {
            self.update_move_spline_like_cpp();
        }

        let move_spline_finalized = self
            .active_move_spline
            .as_ref()
            .is_none_or(MoveSpline::finalized);
        let should_set_location = self
            .active_random_generator
            .as_ref()
            .is_some_and(|generator| generator.timer_ms().saturating_sub(diff_ms as i32) <= 0)
            && move_spline_finalized;

        let point_path_limit =
            point_path_limit_for_distance_like_cpp(RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP);
        let mut detour_path = None;
        let mut path_result = RandomPathResult::Success;
        let mut distance_roll = 0.0;
        let mut angle_roll = 0.0;
        let mut next_wander_steps_roll = 2;
        let mut pause_seconds_roll = 4;

        if should_set_location {
            distance_roll = self.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            angle_roll = self.runtime_rng_like_cpp.gen_range(0.0..=1.0);
            next_wander_steps_roll = self.runtime_rng_like_cpp.gen_range(2..=10);
            pause_seconds_roll = self.runtime_rng_like_cpp.gen_range(4..=10);
            let reference = self
                .active_random_generator
                .as_ref()
                .map(RandomMovementGenerator::reference)
                .unwrap_or_else(|| self.position());
            let destination = compute_random_destination_like_cpp(
                reference,
                self.creature.ai_ownership().wander_radius,
                distance_roll,
                angle_roll,
            )
            .destination;
            if should_try_pathfinding {
                // Built here so the retained corridor is read after a possible
                // generator (re)initialization dropped it, and the filter after
                // any state change this tick — C++ constructs its `PathGenerator`
                // at exactly this point.
                detour_path = resolve_path(CreaturePathQueryLikeCpp {
                    start: self.position(),
                    destination,
                    point_path_limit,
                    force_destination: false,
                    filter_context: self.path_query_filter_context_like_cpp(),
                    owner: self.detour_owner_capabilities_like_cpp(),
                    previous_poly_refs: self.active_random_path_poly_refs.clone(),
                });
                if let Some(path) = detour_path.as_ref() {
                    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
                    path_result = random_path_result_from_path_type_like_cpp(path_type);
                    // The generator keeps its `PathGenerator` alive, so the
                    // corridor this query produced is the one the next one may
                    // reuse (`PathGenerator.cpp:291-413`).
                    self.active_random_path_poly_refs
                        .clone_from(&path.poly_refs);
                } else {
                    path_result = RandomPathResult::Failed;
                }
            }
        }

        let snapshot = self.random_unit_snapshot_like_cpp(
            true,
            path_result,
            distance_roll,
            angle_roll,
            next_wander_steps_roll,
            pause_seconds_roll,
            0,
        );
        let action = match self.active_random_generator.as_mut() {
            Some(generator) => generator.update_like_cpp(true, diff_ms, snapshot),
            None => return None,
        };
        self.apply_random_movement_action_with_terrain_like_cpp(
            action,
            detour_path.as_ref(),
            0,
            terrain,
        )
    }

    pub fn update_default_waypoint_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> WaypointMovementAction {
        self.update_default_waypoint_movement_with_launch_like_cpp(diff_ms)
            .0
    }

    pub fn update_default_waypoint_movement_with_path_resolver_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_path_resolver_and_terrain_like_cpp(
            diff_ms,
            should_try_pathfinding,
            None,
            resolve_path,
        )
    }

    pub fn update_default_waypoint_movement_with_path_resolver_and_terrain_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            should_try_pathfinding,
            terrain,
            true,
            resolve_path,
        )
    }

    pub(crate) fn update_default_waypoint_movement_after_spline_like_cpp(
        &mut self,
        diff_ms: u32,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            should_try_pathfinding,
            terrain,
            false,
            resolve_path,
        )
    }

    fn random_unit_snapshot_like_cpp(
        &self,
        has_los_to_destination: bool,
        path_result: RandomPathResult,
        distance_roll: f32,
        angle_roll: f32,
        next_wander_steps_roll: u8,
        pause_seconds_roll: i32,
        travel_time_ms: i32,
    ) -> RandomUnitSnapshot {
        let random_type = match self.creature.random_movement_type_like_cpp() {
            value if value == ConstantsCreatureRandomMovementType::CanRun as u8 => {
                MovementCreatureRandomMovementType::CanRun
            }
            value if value == ConstantsCreatureRandomMovementType::AlwaysRun as u8 => {
                MovementCreatureRandomMovementType::AlwaysRun
            }
            _ => MovementCreatureRandomMovementType::AlwaysWalk,
        };
        RandomUnitSnapshot {
            owner_position: self.position(),
            owner_alive: self.is_alive(),
            owner_unit_state: self.creature.unit().unit_state(),
            movement_prevented_by_casting: self
                .creature
                .unit()
                .has_unit_state(UnitState::CASTING.bits()),
            move_spline_finalized: self
                .active_move_spline
                .as_ref()
                .is_none_or(MoveSpline::finalized),
            owner_wander_distance: self.creature.ai_ownership().wander_radius,
            has_los_to_destination,
            path_result,
            movement_template: random_type,
            owner_is_walking: self
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::WALKING),
            travel_time_ms,
            distance_roll,
            angle_roll,
            next_wander_steps_roll,
            pause_seconds_roll,
            ai_enabled: true,
        }
    }

    fn apply_random_movement_action_with_terrain_like_cpp(
        &mut self,
        action: RandomMovementAction,
        detour_path: Option<&DetourPolyPath>,
        planned_travel_time_ms: i32,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline)> {
        match action {
            RandomMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                self.active_move_spline = None;
                None
            }
            RandomMovementAction::Launch(launch) => {
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::ROAMING_MOVE.bits());
                let movement = self
                    .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
                        launch.destination,
                        detour_path,
                        false,
                        terrain,
                    )
                    .map(|(from, spline, _path)| (from, spline))?;
                self.creature
                    .set_ai_state(wow_entities::CreatureAiState::WalkingRandom);
                self.creature.ai_ownership_mut().wander_steps_remaining = self
                    .active_random_generator
                    .as_ref()
                    .map(RandomMovementGenerator::wander_steps)
                    .unwrap_or_default();
                if let Some(generator) = self.active_random_generator.as_mut() {
                    generator.adjust_launch_timer_for_actual_travel_time_like_cpp(
                        planned_travel_time_ms,
                        movement.1.duration_ms(),
                    );
                }
                Some(movement)
            }
            RandomMovementAction::RetryAfterLosFailure { .. }
            | RandomMovementAction::RetryAfterPathFailure { .. }
            | RandomMovementAction::Continue
            | RandomMovementAction::Finished
            | RandomMovementAction::DurationFinished => None,
        }
    }

    pub fn update_default_waypoint_movement_with_launch_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            false,
            None,
            true,
            |_| None,
        )
    }

    pub fn update_default_waypoint_movement_with_wait_roll_like_cpp(
        &mut self,
        diff_ms: u32,
        wait_time_roll_ms: Option<i32>,
    ) -> WaypointMovementAction {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            wait_time_roll_ms,
            false,
            None,
            true,
            |_| None,
        )
        .0
    }

    fn update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
        &mut self,
        diff_ms: u32,
        wait_time_roll_ms: Option<i32>,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        update_spline: bool,
        mut resolve_path: impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        if let Some(mut random) = self.active_waypoint_random_at_path_end {
            if update_spline {
                let _ = self.update_move_spline_like_cpp();
            }
            random.duration_ms = random.duration_ms.saturating_sub(diff_ms as i32);
            if random.duration_ms > 0 {
                self.active_waypoint_random_at_path_end = Some(random);
            } else {
                self.active_waypoint_random_at_path_end = None;
            }
            return (WaypointMovementAction::Continue, None);
        }

        // C++ `Unit::Update` advances `UpdateSplineMovement` before
        // `MotionMaster::Update`, so waypoint generators observe an arrived
        // `movespline` in the same tick and can launch the next segment.
        if update_spline {
            let _ = self.update_move_spline_like_cpp();
        }

        let snapshot = self.waypoint_unit_snapshot_like_cpp();
        let Some(generator) = self.active_waypoint_generator.as_mut() else {
            return (WaypointMovementAction::Continue, None);
        };
        let action = generator.update_like_cpp(true, diff_ms, snapshot, wait_time_roll_ms);
        let launch_result = self.apply_waypoint_movement_action_with_path_resolver_like_cpp(
            action,
            should_try_pathfinding,
            terrain,
            &mut resolve_path,
        );
        if matches!(
            action,
            WaypointMovementAction::Arrived(arrived)
                if arrived.timer_ms.is_none() && arrived.move_random_at_path_end.is_none()
        ) {
            let snapshot = self.waypoint_unit_snapshot_like_cpp();
            if let Some(generator) = self.active_waypoint_generator.as_mut() {
                let chained = generator.update_like_cpp(true, 0, snapshot, None);
                if chained != WaypointMovementAction::Continue {
                    let chained_launch = self
                        .apply_waypoint_movement_action_with_path_resolver_like_cpp(
                            chained,
                            should_try_pathfinding,
                            terrain,
                            &mut resolve_path,
                        );
                    return (chained, chained_launch);
                }
            }
        }
        (action, launch_result)
    }

    fn apply_waypoint_movement_action_with_path_resolver_like_cpp(
        &mut self,
        action: WaypointMovementAction,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        resolve_path: &mut impl FnMut(CreaturePathQueryLikeCpp) -> Option<DetourPolyPath>,
    ) -> Option<(Position, MoveSpline)> {
        match action {
            WaypointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            WaypointMovementAction::Arrived(arrived) => {
                if arrived.clear_roaming_move {
                    self.creature
                        .unit_mut()
                        .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                }
                self.creature.record_ai_movement_inform(
                    arrived.inform.movement_type.trinity_id(),
                    arrived.inform.node_id,
                );
                if let Some(random) = arrived.move_random_at_path_end {
                    let launch_result = self.begin_waypoint_random_at_path_end_like_cpp(random);
                    self.active_waypoint_random_at_path_end = Some(random);
                    launch_result
                } else {
                    None
                }
            }
            WaypointMovementAction::PathEnded(ended) => {
                let home = self
                    .creature
                    .ai_ownership()
                    .move_target
                    .unwrap_or_else(|| self.position());
                self.creature.set_ai_home_position(home);
                self.creature
                    .unit_mut()
                    .clear_unit_state(UnitState::ROAMING_MOVE.bits());
                self.creature
                    .set_ai_state(wow_entities::CreatureAiState::Idle);
                let _ = ended;
                None
            }
            WaypointMovementAction::Launch(launch) => {
                let detour_path = (launch.generate_path && should_try_pathfinding)
                    .then(|| {
                        resolve_path(CreaturePathQueryLikeCpp {
                            start: self.position(),
                            destination: launch.destination,
                            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
                            force_destination: false,
                            filter_context: self.path_query_filter_context_like_cpp(),
                            owner: self.detour_owner_capabilities_like_cpp(),
                            // C++ `MoveSplineInit::MoveTo` builds a fresh
                            // `PathGenerator` per waypoint leg.
                            previous_poly_refs: Vec::new(),
                        })
                    })
                    .flatten();
                self.begin_waypoint_launch_with_detour_path_like_cpp(
                    launch,
                    detour_path.as_ref(),
                    terrain,
                )
            }
            _ => None,
        }
    }

    fn waypoint_unit_snapshot_like_cpp(&self) -> WaypointUnitSnapshot {
        let unit = self.creature.unit();
        WaypointUnitSnapshot {
            owner_alive: self.creature.is_alive(),
            owner_unit_state: unit.unit_state(),
            movement_prevented_by_casting: unit.has_unit_state(UnitState::CASTING.bits()),
            move_spline_finalized: unit.subsystems().motion.spline.finalized,
            owner_is_on_transport: false,
            owner_is_formation_leader: false,
            formation_leader_move_allowed: true,
            owner_orientation: self.position().orientation,
            owner_position: self.position(),
            ai_enabled: true,
        }
    }

    fn begin_waypoint_launch_with_detour_path_like_cpp(
        &mut self,
        launch: WaypointLaunchPlan,
        detour_path: Option<&DetourPolyPath>,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        if launch.disable_transport_transform {
            init.disable_transport_path_transformations();
        }
        let path = detour_path
            .map(|detour_path| {
                self.path_generator_from_detour_for_creature_like_cpp(
                    launch.destination,
                    detour_path,
                    false,
                    terrain,
                )
            })
            .filter(|path| !path.path_type().contains(PathType::NOPATH));
        if let Some(path) = path {
            init.move_by_path(path.path_points().to_vec(), 0);
        } else {
            init.move_to(launch.destination);
        }
        if let Some(facing) = launch.facing {
            init.set_facing_angle(facing);
        }
        if let Some(walk) = launch.walk {
            init.set_walk(walk);
        }
        if let Some(velocity) = launch.velocity {
            init.set_velocity(velocity);
        }
        if let Some(animation) = launch.animation {
            match animation {
                WaypointAnimation::Ground => init.set_animation(0, 0, 0),
                WaypointAnimation::Hover => init.set_animation(2, 0, 0),
            }
        }
        self.creature
            .unit_mut()
            .add_unit_state(launch.add_unit_state);
        self.launch_move_spline_init_like_cpp(&mut init, launch.destination)
    }

    fn begin_waypoint_random_at_path_end_like_cpp(
        &mut self,
        random: WaypointRandomAtPathEnd,
    ) -> Option<(Position, MoveSpline)> {
        let dst =
            self.pick_random_destination_from_current_position_like_cpp(random.wander_distance)?;
        self.begin_move_spline_like_cpp(dst)
    }

    pub fn begin_move_spline_with_detour_path_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        self.begin_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            detour_path,
            force_destination,
            None,
        )
    }

    pub fn begin_move_spline_with_detour_path_and_terrain_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        let Some(detour_path) = detour_path else {
            return self
                .begin_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, None));
        };

        let path = self.path_generator_from_detour_for_creature_like_cpp(
            dst,
            detour_path,
            force_destination,
            terrain,
        );
        if path.path_type().contains(PathType::NOPATH) {
            return self
                .begin_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, Some(path)));
        }

        let points = path.path_points().to_vec();
        self.begin_move_spline_by_path_like_cpp(points)
            .map(|(from, spline)| (from, spline, Some(path)))
    }

    pub fn begin_random_move_spline_with_detour_path_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        self.begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            detour_path,
            force_destination,
            None,
        )
    }

    pub fn begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        let Some(detour_path) = detour_path else {
            return self
                .begin_random_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, None));
        };

        let path = self.path_generator_from_detour_for_creature_like_cpp(
            dst,
            detour_path,
            force_destination,
            terrain,
        );
        if path
            .path_type()
            .intersects(PathType::NOPATH | PathType::SHORTCUT)
        {
            return None;
        }

        let points = path.path_points().to_vec();
        self.begin_random_move_spline_by_path_like_cpp(points)
            .map(|(from, spline)| (from, spline, Some(path)))
    }

    pub fn begin_point_movement_like_cpp(
        &mut self,
        movement_id: u32,
        dst: Position,
        can_move: bool,
    ) -> Option<(Position, MoveSpline)> {
        if movement_id == EVENT_CHARGE_PREPATH {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_charge(movement_id);
        } else {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_point(movement_id);
        }

        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion.active_generators.iter_mut().find(|generator| {
                generator.kind == MovementGeneratorKind::Point
                    && generator.movement_id == movement_id
            })?;
            generator.initialize_point_like_cpp(can_move)
        };

        match action {
            PointMovementAction::LaunchSpline => self.begin_move_spline_like_cpp(dst),
            PointMovementAction::MarkRoamingMove => {
                self.creature
                    .unit_mut()
                    .add_unit_state(UnitState::ROAMING_MOVE.bits());
                None
            }
            PointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
                None
            }
            _ => None,
        }
    }

    pub fn finalize_point_movement_like_cpp(
        &mut self,
        active: bool,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Point)?;
            generator.finalize_point_like_cpp(active, movement_inform)
        };
        if finalize.clear_roaming_move {
            self.creature
                .unit_mut()
                .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        }
        if let Some(inform) = finalize.inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        finalize.inform
    }

    pub fn begin_facing_spline_like_cpp(
        &mut self,
        facing_angle: f32,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let current = self.position();
        let active_spline_position = self
            .active_move_spline
            .as_ref()
            .filter(|spline| !spline.finalized() && !spline.on_transport)
            .and_then(MoveSpline::compute_position);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_to(current);
        init.set_facing_angle(facing_angle);

        let now_ms = self.now_ms();
        let mut spline = self
            .active_move_spline
            .take()
            .unwrap_or_else(MoveSpline::new);
        let launch = init
            .launch(
                &mut spline,
                MoveSplineLaunchInput {
                    current_position: current,
                    active_spline_position,
                    movement_flags: MovementFlag::NONE,
                    selected_speed: 2.5,
                    run_speed: 2.5,
                    assistance_speed_factor: 1.0,
                    on_transport: false,
                },
            )
            .ok()?;
        let duration_ms = launch.duration_ms.max(1) as u32;
        {
            let ai = self.creature.ai_ownership_mut();
            ai.move_target = Some(current);
            ai.move_start_ms = now_ms;
            ai.move_duration_ms = duration_ms;
            ai.spline_id = spline_id;
        }
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .launch_spline(
                spline_id,
                duration_ms,
                position_to_i32_tuple(current),
                false,
                false,
                None,
            );
        self.apply_launch_movement_flags_like_cpp(launch.movement_flags);
        self.active_move_spline = Some(spline.clone());
        Some((launch.real_position, spline))
    }

    pub fn begin_distract_movement_like_cpp(
        &mut self,
        timer_ms: u32,
        orientation: f32,
    ) -> Option<(DistractMovementAction, Position, MoveSpline)> {
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .move_distract_like_cpp(timer_ms);

        let owner_is_standing = self.creature.unit().is_stand_state_like_cpp();
        let action = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)?;
            generator.initialize_distract_like_cpp(owner_is_standing)
        };
        if action.stand_up {
            self.creature
                .unit_mut()
                .set_stand_state_like_cpp(UnitStandStateType::Stand);
        }
        let (from, spline) = self.begin_facing_spline_like_cpp(orientation)?;
        Some((action, from, spline))
    }

    pub fn tick_rotate_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> Option<(RotateMovementUpdate, MoveSpline)> {
        let update = {
            let current_orientation = self.position().orientation;
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator.update_rotate_like_cpp(true, diff_ms, current_orientation)
        };
        let (_, spline) = self.begin_facing_spline_like_cpp(update.facing_angle?)?;
        Some((update, spline))
    }

    pub fn finalize_distract_movement_like_cpp(&mut self, movement_inform: bool) -> bool {
        let finalize = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let Some(generator) = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)
            else {
                return false;
            };
            generator.finalize_distract_like_cpp(movement_inform, true)
        };

        if finalize.set_home_orientation {
            let current = self.position();
            let home = self.home_position();
            self.creature.set_ai_position(Position::new(
                current.x,
                current.y,
                current.z,
                home.orientation,
            ));
        }
        finalize.set_home_orientation
    }

    pub fn finalize_rotate_movement_like_cpp(
        &mut self,
        movement_inform: bool,
    ) -> Option<PointMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Rotate)?;
            generator
                .finalize_rotate_like_cpp(movement_inform, true)
                .inform
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }

    pub fn finalize_generic_movement_like_cpp(
        &mut self,
        kind: MovementGeneratorKind,
        movement_id: u32,
        movement_inform: bool,
    ) -> Option<GenericMovementInform> {
        let inform = {
            let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == kind && generator.movement_id == movement_id)?;
            generator.finalize_generic_like_cpp(movement_inform)
        };
        if let Some(inform) = inform {
            self.creature
                .record_ai_movement_inform(inform.kind.trinity_id(), inform.movement_id);
        }
        inform
    }

    pub fn update_move_spline_like_cpp(&mut self) -> bool {
        let Some(mut spline) = self.active_move_spline.take() else {
            return self.movement_finished();
        };

        if !spline.finalized() {
            let elapsed_ms = self
                .now_ms()
                .saturating_sub(self.creature.ai_ownership().move_start_ms)
                .min(i32::MAX as u64) as i32;
            let diff_ms = elapsed_ms.saturating_sub(spline.time_passed_ms());
            if diff_ms > 0 {
                spline.update_state(diff_ms);
            }
            let progress_ms = spline.time_passed_ms().max(0) as u32;
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .set_spline_progress(progress_ms);
        }

        if let Some(pos) = spline.compute_position() {
            self.creature.set_ai_position(pos);
        }

        let finalized = spline.finalized();
        if finalized {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .finalize_spline();
            self.disable_spline_movement_like_cpp();
            self.creature
                .unit_mut()
                .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        } else {
            self.active_move_spline = Some(spline);
        }
        finalized
    }

    pub fn stop_move_spline_like_cpp(&mut self) -> Option<MoveSplineStopResult> {
        let mut spline = self.active_move_spline.take()?;
        if spline.finalized() {
            return None;
        }

        let elapsed_ms = self
            .now_ms()
            .saturating_sub(self.creature.ai_ownership().move_start_ms)
            .min(i32::MAX as u64) as i32;
        let diff_ms = elapsed_ms.saturating_sub(spline.time_passed_ms());
        if diff_ms > 0 {
            spline.update_state(diff_ms);
        }
        if spline.finalized() {
            return None;
        }

        let stop_position = spline.compute_position().unwrap_or_else(|| self.position());
        let mut init = MoveSplineInit::new(self.spline_id().saturating_add(1));
        let stop = init.stop(
            &mut spline,
            MoveSplineStopInput {
                current_position: self.position(),
                active_spline_position: Some(stop_position),
                on_transport: false,
            },
        )?;

        self.creature.set_ai_position(stop.position);
        let ai = self.creature.ai_ownership_mut();
        ai.move_target = None;
        ai.move_duration_ms = 0;
        ai.spline_id = stop.spline_id;
        let motion = &mut self.creature.unit_mut().subsystems_mut().motion;
        motion.finalize_spline();
        motion.spline.spline_id = stop.spline_id;
        self.disable_spline_movement_like_cpp();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::ROAMING_MOVE.bits());
        Some(stop)
    }

    pub fn finish_move(&mut self) {
        if let Some(dst) = self.creature.ai_ownership_mut().move_target.take() {
            self.creature.set_ai_position(dst);
        }
        self.creature.ai_ownership_mut().move_duration_ms = 0;
        self.active_move_spline = None;
        self.creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .finalize_spline();
        self.disable_spline_movement_like_cpp();
        self.creature
            .unit_mut()
            .clear_unit_state(UnitState::ROAMING_MOVE.bits());
    }

    pub fn can_swing(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::InCombat
            && self
                .now_ms()
                .saturating_sub(self.creature.ai_ownership().last_swing_ms)
                >= self.creature.ai_ownership().swing_timer_ms
    }

    pub fn record_swing(&mut self) {
        let now_ms = self.now_ms();
        let base_attack_time = if self.create_data.base_attack_time > 0 {
            self.create_data.base_attack_time as u64
        } else {
            self.creature.ai_ownership().swing_timer_ms.max(1)
        };
        let ai = self.creature.ai_ownership_mut();
        ai.last_swing_ms = now_ms;
        ai.swing_timer_ms = base_attack_time;
    }

    pub fn record_failed_swing_retry_like_cpp(&mut self) {
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.last_swing_ms = now_ms;
        ai.swing_timer_ms = 100;
    }

    pub(crate) fn creature_spell_schedule_initialized_like_cpp(&self) -> bool {
        self.creature_spell_schedule_initialized_like_cpp
    }

    pub(crate) fn mark_creature_spell_schedule_initialized_like_cpp(&mut self) {
        self.creature_spell_schedule_initialized_like_cpp = true;
    }

    pub(crate) fn reset_creature_spell_schedule_like_cpp(&mut self) {
        self.creature_spell_due_at_ms_like_cpp = [None; wow_entities::MAX_CREATURE_SPELLS];
        self.creature_spell_schedule_initialized_like_cpp = false;
        self.creature_spell_engagement_epoch_like_cpp = self
            .creature_spell_engagement_epoch_like_cpp
            .wrapping_add(1);
    }

    pub(crate) fn creature_spell_engagement_epoch_like_cpp(&self) -> u64 {
        self.creature_spell_engagement_epoch_like_cpp
    }

    pub(crate) fn runtime_rng_authority_complete_like_cpp(&self) -> bool {
        self.runtime_rng_authority_complete_like_cpp
    }

    /// Permanently tombstone exact creature-spell RNG authority for this loaded
    /// creature. C++ keeps the same generator across combat resets, so neither
    /// a new target nor a new engagement epoch can restore a provable draw
    /// position. Existing transitional melee and movement continue to consume
    /// their best-effort stream so an unrepresented spell cannot freeze normal
    /// gameplay.
    pub(crate) fn invalidate_runtime_rng_authority_like_cpp(&mut self) {
        self.runtime_rng_authority_complete_like_cpp = false;
    }

    pub(crate) fn schedule_creature_spell_slot_after_like_cpp(
        &mut self,
        slot: usize,
        delay_ms: u64,
    ) {
        let due_at_ms = self.now_ms().saturating_add(delay_ms);
        if let Some(due_at) = self.creature_spell_due_at_ms_like_cpp.get_mut(slot) {
            *due_at = Some(due_at_ms);
        }
    }

    pub(crate) fn clear_creature_spell_slot_like_cpp(&mut self, slot: usize) {
        if let Some(due_at) = self.creature_spell_due_at_ms_like_cpp.get_mut(slot) {
            *due_at = None;
        }
    }

    pub(crate) fn first_due_creature_spell_slot_like_cpp(&self) -> Option<usize> {
        let now_ms = self.now_ms();
        self.creature_spell_due_at_ms_like_cpp
            .iter()
            .enumerate()
            .filter_map(|(slot, due_at)| due_at.map(|due_at| (slot, due_at)))
            .filter(|(_, due_at)| now_ms >= *due_at)
            // C++ `CombatAI::UpdateAI` invokes `EventMap::ExecuteEvent` once
            // per update, so simultaneous events are consumed one at a time.
            .min_by_key(|(slot, due_at)| (*due_at, *slot))
            .map(|(slot, _)| slot)
    }

    #[cfg(test)]
    pub(crate) fn creature_spell_due_in_ms_for_test(&self, slot: usize) -> Option<u64> {
        self.creature_spell_due_at_ms_like_cpp
            .get(slot)
            .copied()
            .flatten()
            .map(|due_at| due_at.saturating_sub(self.now_ms()))
    }

    pub(crate) fn random_creature_spell_delay_like_cpp(
        &mut self,
        minimum_ms: u64,
        maximum_ms: u64,
    ) -> Option<u64> {
        if !self.runtime_rng_authority_complete_like_cpp {
            return None;
        }
        if minimum_ms > maximum_ms {
            self.invalidate_runtime_rng_authority_like_cpp();
            return None;
        }
        if minimum_ms == maximum_ms {
            // C++ `urand(min, max)` still invokes its process-global engine
            // when both inclusive bounds are equal. Preserve that logical
            // draw in the Creature-owned represented stream.
            let _ = self.runtime_rng_like_cpp.next_u32();
            return Some(minimum_ms);
        }
        Some(self.runtime_rng_like_cpp.gen_range(minimum_ms..=maximum_ms))
    }

    pub(crate) fn random_creature_spell_hit_roll_like_cpp(&mut self) -> Option<u32> {
        self.runtime_rng_authority_complete_like_cpp
            .then(|| self.runtime_rng_like_cpp.gen_range(0..=9_999))
    }

    pub fn roll_damage(&mut self) -> Option<u32> {
        let min_dmg = self.min_dmg();
        let max_dmg = self.max_dmg();
        if min_dmg > max_dmg {
            self.invalidate_runtime_rng_authority_like_cpp();
            return None;
        }
        if min_dmg == max_dmg {
            let _ = self.runtime_rng_like_cpp.next_u32();
            return Some(min_dmg);
        }
        Some(self.runtime_rng_like_cpp.gen_range(min_dmg..=max_dmg))
    }

    pub fn should_wander(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::Idle
            && self.creature.default_movement_type() == wow_entities::MovementGeneratorType::Random
            && self.can_wander()
            && self.creature.ai_ownership().wander_radius > 0.0
            && self
                .now_ms()
                .saturating_sub(self.creature.ai_ownership().move_start_ms)
                >= self.creature.ai_ownership().wander_delay_ms
    }

    pub fn pick_wander_destination(&mut self) -> Option<Position> {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = self.creature.ai_ownership().wander_radius.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let home = self.home_position();
        let x = home.x + angle.cos() * dist;
        let y = home.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Some(Position::new(x, y, home.z, o))
    }

    pub fn pick_random_destination_from_current_position_like_cpp(
        &mut self,
        wander_distance: f32,
    ) -> Option<Position> {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = wander_distance.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let reference = self.position();
        let x = reference.x + angle.cos() * dist;
        let y = reference.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Some(Position::new(x, y, reference.z, o))
    }

    pub fn reset_wander_timer(&mut self) -> bool {
        let now_ms = self.now_ms();
        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        true
    }

    pub fn initialize_random_wander_steps_like_cpp(&mut self) -> bool {
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        self.creature.ai_ownership_mut().wander_steps_remaining = wander_steps_remaining;
        true
    }

    pub fn record_random_movement_launch_like_cpp(&mut self) -> bool {
        if self.creature.ai_ownership().wander_steps_remaining == 0 {
            if !self.initialize_random_wander_steps_like_cpp() {
                return false;
            }
        }
        let ai = self.creature.ai_ownership_mut();
        ai.wander_steps_remaining = ai.wander_steps_remaining.saturating_sub(1);
        ai.state = CreatureAiState::WalkingRandom;
        true
    }

    pub fn schedule_after_random_movement_like_cpp(&mut self) -> bool {
        let now_ms = self.now_ms();
        if self.creature.ai_ownership().wander_steps_remaining > 0 {
            let ai = self.creature.ai_ownership_mut();
            ai.move_start_ms = now_ms;
            ai.wander_delay_ms = 0;
            return true;
        }
        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        ai.wander_steps_remaining = wander_steps_remaining;
        true
    }

    #[cfg(test)]
    pub fn seed_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
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

impl MapInstance {
    pub fn new(map_id: u16, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id,
            grids: HashMap::new(),
            grid_unload_timeout: DEFAULT_GRID_UNLOAD_TIME,
            personal_phases: MultiPersonalPhaseTracker::default(),
            personal_phase_objects_to_remove: HashSet::new(),
            persisted_respawn_times: HashMap::new(),
            respawn_queue: Vec::new(),
        }
    }

    pub fn get_or_create_grid(&mut self, x: i16, y: i16) -> &mut Grid {
        let coord = GridCoord::new(x, y);
        if !self.grids.contains_key(&coord) {
            let grid = Grid::new(x, y);
            self.grids.insert(coord, grid);
            debug!(
                "Created new grid ({}, {}) for map {} instance {}",
                x, y, self.map_id, self.instance_id
            );
        }
        self.grids.get_mut(&coord).unwrap()
    }

    pub fn get_grid(&self, x: i16, y: i16) -> Option<&Grid> {
        self.grids.get(&GridCoord::new(x, y))
    }

    pub fn get_grid_mut(&mut self, x: i16, y: i16) -> Option<&mut Grid> {
        self.grids.get_mut(&GridCoord::new(x, y))
    }

    pub fn remove_grid(&mut self, x: i16, y: i16) -> bool {
        let coord = GridCoord::new(x, y);
        let removed = self.grids.remove(&coord).is_some();
        if removed {
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
        }
        removed
    }

    pub fn add_creature(&mut self, x: i16, y: i16, creature: WorldCreature) -> bool {
        self.get_or_create_grid(x, y).add_creature(creature)
    }

    pub fn remove_creature(&mut self, x: i16, y: i16, guid: ObjectGuid) -> bool {
        if let Some(grid) = self.get_grid_mut(x, y) {
            grid.remove_creature(guid)
        } else {
            false
        }
    }

    pub fn get_creature(&self, x: i16, y: i16, guid: ObjectGuid) -> Option<&WorldCreature> {
        self.get_grid(x, y)?.get_creature(guid)
    }

    pub fn get_creature_mut(
        &mut self,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_grid_mut(x, y)?.get_creature_mut(guid)
    }

    pub fn unload_empty_grids(&mut self) {
        let to_remove: Vec<GridCoord> = self
            .grids
            .iter()
            .filter(|(_, grid)| grid.should_unload(self.grid_unload_timeout))
            .map(|(coord, _)| *coord)
            .collect();

        for coord in to_remove {
            info!(
                "Unloading grid {:?} from map {} (timeout)",
                coord, self.map_id
            );
            self.grids.remove(&coord);
            self.personal_phases
                .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
        }
    }

    pub fn creature_count(&self) -> usize {
        self.grids.values().map(|g| g.creature_count()).sum()
    }

    pub fn is_grid_loaded(&self, x: i16, y: i16) -> bool {
        self.get_grid(x, y).is_some()
    }

    pub fn min_height_like_cpp(&self, _x: f32, _y: f32) -> f32 {
        DEFAULT_MIN_HEIGHT_LIKE_CPP
    }

    pub fn load_personal_phase_grid_like_cpp(
        &mut self,
        phase_shift: &PhaseShift,
        x: i16,
        y: i16,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.get_or_create_grid(x, y);
        self.personal_phases.load_grid_like_cpp(
            phase_shift,
            GridCoord::new(x, y).personal_phase_grid_id_like_cpp(),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn update_personal_phases_for_owner_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        phase_shift: &PhaseShift,
        grid: Option<GridCoord>,
        has_personal_spawns: impl FnMut(u32) -> bool,
        load_phase: impl FnMut(ObjectGuid, u32),
    ) -> bool {
        self.personal_phases.on_owner_phase_changed_like_cpp(
            phase_owner,
            phase_shift,
            grid.map(|coord| coord.personal_phase_grid_id_like_cpp()),
            has_personal_spawns,
            load_phase,
        )
    }

    pub fn register_personal_phase_object_like_cpp(
        &mut self,
        phase_id: u32,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .register_tracked_object_like_cpp(phase_id, phase_owner, object);
    }

    pub fn unregister_personal_phase_object_like_cpp(
        &mut self,
        phase_owner: ObjectGuid,
        object: ObjectGuid,
    ) {
        self.personal_phases
            .unregister_tracked_object_like_cpp(phase_owner, object);
    }

    pub fn mark_personal_phases_for_deletion_like_cpp(&mut self, phase_owner: ObjectGuid) {
        self.personal_phases
            .mark_all_phases_for_deletion_like_cpp(phase_owner);
    }

    pub fn update_personal_phases_like_cpp(&mut self, diff: Duration) {
        let mut objects_to_remove = Vec::new();
        self.personal_phases
            .update_like_cpp(diff, |guid| objects_to_remove.push(guid));
        self.personal_phase_objects_to_remove
            .extend(objects_to_remove);
    }

    pub fn remove_personal_phase_objects_like_cpp(&mut self) -> usize {
        let objects_to_remove = std::mem::take(&mut self.personal_phase_objects_to_remove);
        let removed = objects_to_remove.len();
        for object in objects_to_remove {
            for grid in self.grids.values_mut() {
                grid.remove_creature(object);
            }
        }
        removed
    }

    pub fn queued_personal_phase_remove_count_like_cpp(&self) -> usize {
        self.personal_phase_objects_to_remove.len()
    }

    // ── Respawn queue (Slice 4A.2a) ───────────────────────────────────────────
    //
    // Mirrors `Map::_respawnTimes` (Map.h:748-750) ownership model.
    // The queue is a plain `Vec`; heap/SpawnId convergence is deferred.

    pub fn add_persisted_respawn_time_like_cpp(
        &mut self,
        row: PersistedRespawnRowLikeCpp,
    ) -> LegacyRespawnTimeAddOutcomeLikeCpp {
        if row.spawn_id == 0 {
            return LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId;
        }
        if !matches!(
            row.object_type,
            SpawnObjectType::Creature | SpawnObjectType::GameObject
        ) {
            return LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType;
        }

        let key = (row.object_type, row.spawn_id);
        if let Some(existing) = self.persisted_respawn_times.get(&key) {
            if row.respawn_time <= existing.respawn_time {
                self.persisted_respawn_times.insert(key, row);
                LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting
            } else {
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual
            }
        } else {
            self.persisted_respawn_times.insert(key, row);
            LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
        }
    }

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times
            .remove(&(object_type, spawn_id))
    }

    pub fn persisted_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64> {
        self.persisted_respawn_times
            .get(&(object_type, spawn_id))
            .map(|row| row.respawn_time)
    }

    pub fn persisted_respawn_rows_like_cpp(&self) -> Vec<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times.values().copied().collect()
    }

    /// Enqueue a creature waiting to respawn.
    /// C++ ref: `Map::_respawnTimes` insertion path (Map.cpp:2191).
    pub fn push_respawn(&mut self, respawn: PendingRespawn) {
        if let Some(existing_index) = self.respawn_queue.iter().position(|queued| {
            queued.persistent_spawn == respawn.persistent_spawn
                && queued.spawn_id == respawn.spawn_id
        }) {
            if respawn.respawn_at <= self.respawn_queue[existing_index].respawn_at {
                self.respawn_queue.remove(existing_index);
            } else {
                return;
            }
        }
        self.respawn_queue.push(respawn);
    }

    /// Drain entries whose `respawn_at <= now` in insertion order.
    ///
    /// Entries that are NOT yet ready are retained in the queue.
    /// C++ ref: `Map::ProcessRespawns` (Map.cpp:2191).
    pub fn drain_ready_respawns(&mut self, now: Instant) -> Vec<PendingRespawn> {
        let mut remaining = Vec::new();
        let mut spawn_now = Vec::new();
        for r in self.respawn_queue.drain(..) {
            if now >= r.respawn_at {
                spawn_now.push(r);
            } else {
                remaining.push(r);
            }
        }
        self.respawn_queue = remaining;
        spawn_now
    }

    /// Number of entries currently waiting to respawn.
    pub fn respawn_queue_len(&self) -> usize {
        self.respawn_queue.len()
    }

    pub fn save_pending_respawn_time_like_cpp(
        &mut self,
        respawn: &PendingRespawn,
        now: Instant,
        now_secs: i64,
    ) -> Option<PreparedStatement> {
        let row = PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: respawn.spawn_id,
            respawn_time: respawn_time_from_instant_like_cpp(respawn.respawn_at, now, now_secs),
            map_id: self.map_id,
            instance_id: self.instance_id,
        };
        match self.add_persisted_respawn_time_like_cpp(row) {
            LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
            | LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting => {
                Some(respawn_replace_statement_like_cpp(&row))
            }
            LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId
            | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType
            | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual => None,
        }
    }

    pub fn load_persisted_respawns_into_queue_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = PersistedRespawnRowLikeCpp>,
        now: Instant,
        now_secs: i64,
        mut resolve_creature: impl FnMut(&PersistedRespawnRowLikeCpp, Instant) -> Option<PendingRespawn>,
    ) -> LegacyRespawnQueueReloadReportLikeCpp {
        let mut report = LegacyRespawnQueueReloadReportLikeCpp::default();
        for row in rows {
            report.rows += 1;
            match self.add_persisted_respawn_time_like_cpp(row) {
                LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
                | LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting => {
                    report.timers_loaded += 1;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId => {
                    report.rejected_zero_spawn_id += 1;
                    continue;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType => {
                    report.rejected_unsupported_type += 1;
                    continue;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual => {
                    report.rejected_existing_later += 1;
                    continue;
                }
            }

            let respawn_at = instant_from_respawn_time_like_cpp(row.respawn_time, now, now_secs);
            match row.object_type {
                SpawnObjectType::Creature => {
                    if let Some(mut pending) = resolve_creature(&row, respawn_at) {
                        pending.respawn_at = respawn_at;
                        pending.spawn_id = row.spawn_id;
                        pending.map_id = row.map_id;
                        self.push_respawn(pending);
                        report.creature_queued += 1;
                    } else {
                        report.missing_creature_runtime += 1;
                    }
                }
                SpawnObjectType::GameObject => {
                    report.gameobject_loaded += 1;
                }
                SpawnObjectType::AreaTrigger => {
                    report.rejected_unsupported_type += 1;
                }
            }
        }
        report
    }
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

impl MapManager {
    pub fn new() -> Self {
        let mut manager = Self {
            maps: HashMap::new(),
            free_instance_ids: Vec::new(),
            next_instance_id: 1,
            tick_owner: RuntimeTickOwner::Session,
            terrain: None,
        };
        manager.init_instance_ids_from_max(0);
        manager
    }

    /// Attach the shared, file-backed terrain height store (server startup).
    pub fn set_terrain(&mut self, terrain: Arc<LiveTerrainHeights>) {
        self.terrain = Some(terrain);
    }

    /// Shared terrain height store, if wired. Cloned so callers can use it while
    /// still holding `&mut self` for the spawn/respawn mutation.
    #[must_use]
    pub fn terrain(&self) -> Option<Arc<LiveTerrainHeights>> {
        self.terrain.clone()
    }

    /// Returns the current tick owner for this map manager.
    ///
    /// Returns a `Copy` value; the caller should read this once and release the
    /// lock before performing any tick work.
    pub fn tick_owner(&self) -> RuntimeTickOwner {
        self.tick_owner
    }

    /// Sets the tick owner.
    pub fn set_tick_owner(&mut self, owner: RuntimeTickOwner) {
        self.tick_owner = owner;
    }

    /// Returns the `(map_id, instance_id)` keys of all currently active map
    /// instances held by this manager.
    ///
    /// The key type matches `self.maps: HashMap<(u16, u32), MapInstance>` exactly.
    /// Order is unspecified (hash map iteration order).
    pub fn active_map_keys(&self) -> Vec<(u16, u32)> {
        self.maps.keys().copied().collect()
    }

    pub fn init_instance_ids_from_max(&mut self, max_existing_instance_id: u32) {
        self.next_instance_id = 1;
        self.free_instance_ids = vec![true; max_existing_instance_id.saturating_add(2) as usize];
        self.free_instance_ids[0] = false;
    }

    pub fn register_instance_id(&mut self, instance_id: u32) {
        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.free_instance_ids[index] = false;

        if self.next_instance_id == instance_id {
            self.next_instance_id = self.next_instance_id.saturating_add(1);
        }
    }

    pub fn generate_instance_id(&mut self) -> Option<u32> {
        if self.next_instance_id == u32::MAX {
            return None;
        }

        let new_instance_id = self.next_instance_id;
        let index = new_instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(1), true);
        }
        self.free_instance_ids[index] = false;

        let search_start = self.next_instance_id.saturating_add(1) as usize;
        if let Some(next_free_offset) = self.free_instance_ids[search_start..]
            .iter()
            .position(|is_free| *is_free)
        {
            self.next_instance_id = (search_start + next_free_offset) as u32;
        } else {
            self.next_instance_id = self.free_instance_ids.len() as u32;
            self.free_instance_ids.push(true);
        }

        Some(new_instance_id)
    }

    pub fn free_instance_id(&mut self, instance_id: u32) {
        if instance_id == 0 {
            if self.free_instance_ids.is_empty() {
                self.init_instance_ids_from_max(0);
            } else {
                self.free_instance_ids[0] = false;
            }
            return;
        }

        let index = instance_id as usize;
        if index >= self.free_instance_ids.len() {
            self.free_instance_ids.resize(index.saturating_add(2), true);
        }

        self.next_instance_id = self.next_instance_id.min(instance_id);
        self.free_instance_ids[index] = true;
        self.free_instance_ids[0] = false;
    }

    pub fn get_or_create_map(&mut self, map_id: u16, instance_id: u32) -> &mut MapInstance {
        let key = (map_id, instance_id);
        if !self.maps.contains_key(&key) {
            let instance = MapInstance::new(map_id, instance_id);
            self.maps.insert(key, instance);
            info!(
                "Created new map instance: map_id={}, instance_id={}",
                map_id, instance_id
            );
        }
        self.maps.get_mut(&key).unwrap()
    }

    pub fn get_map(&self, map_id: u16, instance_id: u32) -> Option<&MapInstance> {
        self.maps.get(&(map_id, instance_id))
    }

    pub fn get_map_mut(&mut self, map_id: u16, instance_id: u32) -> Option<&mut MapInstance> {
        self.maps.get_mut(&(map_id, instance_id))
    }

    // Convenience methods that delegate to MapInstance

    pub fn get_grid(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> Option<&Grid> {
        self.get_map(map_id, instance_id)?.get_grid(x, y)
    }

    pub fn get_grid_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> Option<&mut Grid> {
        self.get_map_mut(map_id, instance_id)?.get_grid_mut(x, y)
    }

    pub fn get_or_create_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
    ) -> &mut Grid {
        self.get_or_create_map(map_id, instance_id)
            .get_or_create_grid(x, y)
    }

    pub fn add_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        mut creature: WorldCreature,
    ) -> bool {
        let _ = creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(u32::from(map_id), instance_id);
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        self.get_or_create_map(map_id, instance_id)
            .add_creature(x, y, creature)
    }

    pub fn remove_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> bool {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            map.remove_creature(x, y, guid)
        } else {
            false
        }
    }

    pub fn get_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        self.get_map(map_id, instance_id)?.get_creature(x, y, guid)
    }

    pub fn get_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        self.get_map_mut(map_id, instance_id)?
            .get_creature_mut(x, y, guid)
    }

    pub fn find_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&WorldCreature> {
        let map = self.get_map(map_id, instance_id)?;
        map.grids.values().find_map(|grid| grid.get_creature(guid))
    }

    pub fn find_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<&mut WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.get_creature_mut(guid))
    }

    pub fn set_creature_anim_kit_id_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
        slot: CreatureAnimKitSlotLikeCpp,
        anim_kit_id: u16,
        anim_kit_exists: impl Fn(u16) -> bool,
    ) -> Option<RuntimeEvent> {
        use wow_packet::ServerPacket;

        let creature = self.find_creature_mut(map_id, instance_id, guid)?;
        if anim_kit_id != 0 && !anim_kit_exists(anim_kit_id) {
            return None;
        }

        let changed = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_ai_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.ai_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Movement => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_movement_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.movement_anim_kit_id = anim_kit_id;
                }
                changed
            }
            CreatureAnimKitSlotLikeCpp::Melee => {
                let changed = creature
                    .creature
                    .unit_mut()
                    .set_melee_anim_kit_id_like_cpp(anim_kit_id);
                if changed {
                    creature.create_data.melee_anim_kit_id = anim_kit_id;
                }
                changed
            }
        };
        if !changed {
            return None;
        }

        let packet_bytes = match slot {
            CreatureAnimKitSlotLikeCpp::Ai => wow_packet::packets::misc::SetAiAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Movement => wow_packet::packets::misc::SetMovementAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
            CreatureAnimKitSlotLikeCpp::Melee => wow_packet::packets::misc::SetMeleeAnimKit {
                unit: guid,
                anim_kit_id,
            }
            .to_bytes(),
        };
        let source_position = creature.position();
        let range = creature.visibility_range_like_cpp();
        Some(RuntimeEvent {
            source_guid: guid,
            recipients: RecipientRule::NearbyVisible {
                source_guid: guid,
                map_id,
                instance_id,
                source_position,
                range,
                required_3d: false,
            },
            packet_bytes,
        })
    }

    pub fn creature_guids(&self, map_id: u16, instance_id: u32) -> Vec<ObjectGuid> {
        self.get_map(map_id, instance_id)
            .map(|map| {
                map.grids
                    .values()
                    .flat_map(|grid| grid.creatures.keys().copied())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find_creature_guid_by_spawn_id_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        spawn_id: u64,
    ) -> Option<ObjectGuid> {
        (spawn_id != 0).then_some(())?;
        self.creature_guids(map_id, instance_id)
            .into_iter()
            .find(|guid| {
                self.find_creature(map_id, instance_id, *guid)
                    .is_some_and(|creature| {
                        creature.is_alive() && creature.creature.spawn_id() == spawn_id
                    })
            })
    }

    pub fn active_creature_guids_for_player_update_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        player_position: Position,
        player_phase_shift: &PhaseShift,
    ) -> Vec<ObjectGuid> {
        let Some(map) = self.get_map(map_id, instance_id) else {
            return Vec::new();
        };
        let (low, high) = calculate_cell_area_like_cpp(player_position, VISIBILITY_RADIUS);
        let mut guids = Vec::new();

        for grid in map.grids.values() {
            for creature in grid.creatures.values() {
                if !creature.creature.unit().world().object().is_in_world() {
                    continue;
                }
                if !player_phase_shift.can_see(creature.phase_shift()) {
                    continue;
                }
                let Some(cell) =
                    cell_area_contains_position_like_cpp(low, high, creature.position())
                else {
                    continue;
                };
                guids.push((cell, creature.guid()));
            }
        }

        guids.sort_by_key(|(cell, guid)| (cell.x, cell.y, guid.high_value(), guid.low_value()));
        guids.into_iter().map(|(_, guid)| guid).collect()
    }

    pub fn remove_creature_any(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<WorldCreature> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.grids
            .values_mut()
            .find_map(|grid| grid.creatures.remove(&guid))
    }

    pub fn with_creature_mut<F, R>(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        guid: ObjectGuid,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut WorldCreature) -> R,
    {
        self.get_map_mut(map_id, instance_id)?
            .get_grid_mut(x, y)?
            .get_creature_mut(guid)
            .map(f)
    }

    // ── Respawn queue delegates (Slice 4A.2a) ─────────────────────────────────

    /// Enqueue a pending respawn on the given map instance.
    /// Creates the instance if it does not yet exist.
    pub fn push_respawn(&mut self, map_id: u16, instance_id: u32, respawn: PendingRespawn) {
        self.get_or_create_map(map_id, instance_id)
            .push_respawn(respawn);
    }

    /// Drain ready respawns (`respawn_at <= now`) from the given map instance.
    /// Returns an empty `Vec` if the instance does not exist.
    pub fn drain_ready_respawns(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: Instant,
    ) -> Vec<PendingRespawn> {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            map.drain_ready_respawns(now)
        } else {
            Vec::new()
        }
    }

    /// Number of entries currently in the respawn queue of the given map
    /// instance.  Returns 0 if the instance does not exist.
    pub fn respawn_queue_len(&self, map_id: u16, instance_id: u32) -> usize {
        self.get_map(map_id, instance_id)
            .map(|m| m.respawn_queue_len())
            .unwrap_or(0)
    }

    pub fn save_pending_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: &PendingRespawn,
        now: Instant,
        now_secs: i64,
    ) -> Option<PreparedStatement> {
        self.get_or_create_map(map_id, instance_id)
            .save_pending_respawn_time_like_cpp(respawn, now, now_secs)
    }

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PreparedStatement> {
        let map = self.get_map_mut(map_id, instance_id)?;
        map.remove_persisted_respawn_time_like_cpp(object_type, spawn_id)?;
        Some(respawn_delete_statement_like_cpp(
            object_type,
            spawn_id,
            map_id,
            instance_id,
        ))
    }

    pub fn persisted_respawn_time_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64> {
        self.get_map(map_id, instance_id)?
            .persisted_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn persisted_respawn_rows_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<PersistedRespawnRowLikeCpp> {
        self.get_map(map_id, instance_id)
            .map(|map| map.persisted_respawn_rows_like_cpp())
            .unwrap_or_default()
    }

    pub fn load_persisted_respawns_into_queue_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = PersistedRespawnRowLikeCpp>,
        now: Instant,
        now_secs: i64,
        mut resolve_creature: impl FnMut(&PersistedRespawnRowLikeCpp, Instant) -> Option<PendingRespawn>,
    ) -> LegacyRespawnQueueReloadReportLikeCpp {
        let mut report = LegacyRespawnQueueReloadReportLikeCpp::default();
        for row in rows {
            let row_report = self
                .get_or_create_map(row.map_id, row.instance_id)
                .load_persisted_respawns_into_queue_like_cpp([row], now, now_secs, |row, at| {
                    resolve_creature(row, at)
                });
            report.rows += row_report.rows;
            report.timers_loaded += row_report.timers_loaded;
            report.creature_queued += row_report.creature_queued;
            report.gameobject_loaded += row_report.gameobject_loaded;
            report.rejected_zero_spawn_id += row_report.rejected_zero_spawn_id;
            report.rejected_unsupported_type += row_report.rejected_unsupported_type;
            report.rejected_existing_later += row_report.rejected_existing_later;
            report.missing_creature_runtime += row_report.missing_creature_runtime;
        }
        report
    }

    pub fn player_enter_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
        _pos: Position,
    ) {
        let grid = self.get_or_create_grid(map_id, instance_id, x, y);
        grid.player_enter(player_guid);
        debug!(
            "Player {:?} entered grid ({}, {}) in map {}",
            player_guid, x, y, map_id
        );
    }

    pub fn player_leave_grid(
        &mut self,
        map_id: u16,
        instance_id: u32,
        x: i16,
        y: i16,
        player_guid: ObjectGuid,
    ) {
        if let Some(grid) = self.get_grid_mut(map_id, instance_id, x, y) {
            grid.player_leave(player_guid);
            debug!(
                "Player {:?} left grid ({}, {}) in map {}",
                player_guid, x, y, map_id
            );
        }
    }

    pub fn player_move(
        &mut self,
        map_id: u16,
        instance_id: u32,
        from: (i16, i16),
        to: (i16, i16),
        player_guid: ObjectGuid,
        pos: Position,
    ) {
        let (from_x, from_y) = from;
        let (to_x, to_y) = to;

        // Leave old grid
        self.player_leave_grid(map_id, instance_id, from_x, from_y, player_guid);

        // Enter new grid
        self.player_enter_grid(map_id, instance_id, to_x, to_y, player_guid, pos);
    }

    pub fn get_visible_creatures(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        _z: f32,
    ) -> Vec<WorldCreature> {
        self.get_visible_creatures_in_phase(map_id, instance_id, x, y, _z, VISIBILITY_RADIUS, None)
    }

    pub fn get_visible_creatures_in_phase(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        z: f32,
        visibility_range: f32,
        seer_phase_shift: Option<&PhaseShift>,
    ) -> Vec<WorldCreature> {
        let center_x = world_to_grid_x(x);
        let center_y = world_to_grid_y(y);

        let mut creatures = Vec::new();

        // Get creatures from 3x3 grid area
        for dx in -1..=1 {
            for dy in -1..=1 {
                let grid_x = center_x + dx;
                let grid_y = center_y + dy;

                if let Some(grid) = self.get_grid(map_id, instance_id, grid_x, grid_y) {
                    for creature in grid.creatures.values() {
                        if let Some(seer_phase_shift) = seer_phase_shift
                            && !seer_phase_shift.can_see(creature.phase_shift())
                        {
                            continue;
                        }

                        // C++ `CanSeeOrDetect(..., distanceCheck=true)` uses
                        // `IsWithinDist(..., is3D=false)` for visibility
                        // (`Object.cpp:1609`). Keep the legacy map path aligned
                        // with the canonical map visibility path.
                        let dist = Position::new(x, y, z, 0.0).distance_2d(&creature.position());
                        if dist <= visibility_range {
                            creatures.push(creature.clone());
                        }
                    }
                }
            }
        }

        creatures
    }

    pub fn unload_distant_grids(
        &mut self,
        map_id: u16,
        instance_id: u32,
        center_x: i16,
        center_y: i16,
        range: i16,
    ) {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            let to_remove: Vec<GridCoord> = map
                .grids
                .keys()
                .filter(|coord| {
                    let dx = (coord.x - center_x).abs();
                    let dy = (coord.y - center_y).abs();
                    dx > range || dy > range
                })
                .copied()
                .collect();

            for coord in to_remove {
                if let Some(grid) = map.grids.get(&coord) {
                    if grid.should_unload(map.grid_unload_timeout) {
                        info!("Unloading distant grid {:?} from map {}", coord, map_id);
                        map.grids.remove(&coord);
                        map.personal_phases
                            .unload_grid_like_cpp(coord.personal_phase_grid_id_like_cpp());
                    }
                }
            }
        }
    }

    pub fn is_grid_loaded(&self, map_id: u16, instance_id: u32, x: i16, y: i16) -> bool {
        self.get_map(map_id, instance_id)
            .map(|m| m.is_grid_loaded(x, y))
            .unwrap_or(false)
    }

    pub fn min_height_like_cpp(&self, map_id: u16, instance_id: u32, x: f32, y: f32) -> f32 {
        self.get_map(map_id, instance_id)
            .map(|m| m.min_height_like_cpp(x, y))
            .unwrap_or(DEFAULT_MIN_HEIGHT_LIKE_CPP)
    }

    pub fn create_grid(&mut self, map_id: u16, instance_id: u32, x: i16, y: i16) -> &mut Grid {
        self.get_or_create_grid(map_id, instance_id, x, y)
    }

    pub fn creature_count(&self) -> usize {
        self.maps.values().map(|m| m.creature_count()).sum()
    }
}

/// Shared reference type for the MapManager.
pub type SharedMapManager = Arc<RwLock<MapManager>>;

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
#[path = "map_manager_tests.rs"]
mod tests;
