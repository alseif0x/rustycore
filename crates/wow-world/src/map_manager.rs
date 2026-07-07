use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng, rngs::StdRng};
use tracing::{debug, info, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::{
    CreatureRandomMovementType as ConstantsCreatureRandomMovementType, UnitDynFlags, UnitMoveType,
    UnitStandStateType, UnitState, WeaponAttackType,
};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    AllowedPositionZCaps, Creature, CreatureAddonLifecycleRecordLikeCpp, CreatureAiState,
    DEFAULT_HEIGHT_SEARCH, DistractMovementAction, EVENT_CHARGE_PREPATH, GenericMovementInform,
    INVALID_HEIGHT, MovementGeneratorKind, MovementGeneratorType, MovementSlot, PhaseShift,
    PointMovementAction, PointMovementInform, RotateMovementUpdate, Z_OFFSET_FIND_HEIGHT,
    allowed_position_z_from_ground_like_cpp,
};
use wow_map::{GridMapTerrain, SharedStaticVMapLineOfSightProvider};
use wow_movement::generators::CreatureRandomMovementType as MovementCreatureRandomMovementType;
use wow_movement::{
    MoveSpline, MoveSplineInit, MoveSplineLaunchInput, MoveSplineStopInput, MoveSplineStopResult,
    PathGenerator, PathType, RANDOM_PATH_LENGTH_LIMIT_LIKE_CPP, RandomMovementAction,
    RandomMovementGenerator, RandomPathResult, RandomUnitSnapshot, WaypointAnimation,
    WaypointLaunchPlan, WaypointMovementAction, WaypointMovementGenerator, WaypointPath,
    WaypointRandomAtPathEnd, WaypointUnitSnapshot, compute_random_destination_like_cpp,
};
use wow_packet::packets::update::CreatureCreateData;
use wow_recastdetour::{
    CENTER_GRID_ID_LIKE_CPP, DetourNavMeshQueryError, DetourPathOptions, DetourPathType,
    DetourPolyPath, DetourQueryFilterError, MAX_NUMBER_OF_GRIDS_LIKE_CPP,
    MAX_POINT_PATH_LENGTH_LIKE_CPP, MMapData, MMapManager as DetourMMapManager, MMapManagerError,
    PathQueryFilterContext, SIZE_OF_GRIDS_LIKE_CPP, ThreadUnsafeMapData,
    create_path_query_filter_like_cpp,
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
        self.calculate_path_from_positions_like_cpp(
            creature_position,
            destination,
            mesh_map_id,
            instance_map_id,
            instance_id,
            filter_context,
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

        let Some(mmap_data) = self.mmap_manager.get_mmap_data(mesh_map_id) else {
            return Ok(None);
        };
        let filter = create_path_query_filter_like_cpp(filter_context)?;
        mmap_data
            .calculate_path_for_instance_like_cpp(
                instance_map_id,
                instance_id,
                &filter,
                position_to_wow_point_like_cpp(start),
                position_to_wow_point_like_cpp(destination),
                DetourPathOptions {
                    point_path_limit,
                    force_destination,
                    ..DetourPathOptions::default()
                },
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

#[derive(Debug, Clone)]
pub struct WorldMMapPathRequestLikeCpp {
    pub start: Position,
    pub destination: Position,
    pub mesh_map_id: u32,
    pub instance_map_id: u32,
    pub instance_id: u32,
    pub filter_context: PathQueryFilterContext,
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

/// A creature stored in the global map system.
#[derive(Debug, Clone)]
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
    active_waypoint_generator: Option<WaypointMovementGenerator>,
    active_waypoint_random_at_path_end: Option<WaypointRandomAtPathEnd>,
    runtime_rng_like_cpp: StdRng,
    clock_started_at: Instant,
}

impl WorldCreature {
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
        creature.set_display_id(display_id, true, None);
        creature.set_faction(faction);
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
            ai.npc_flags = npc_flags;
            ai.unit_flags = unit_flags;
            ai.display_id = display_id;
            ai.faction = faction;
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

    pub fn from_canonical(creature: Creature, mut create_data: CreatureCreateData) -> Self {
        let ai = creature.ai_ownership();
        create_data.npc_flags = (u64::from(ai.npc_flags2) << 32) | u64::from(ai.npc_flags);
        create_data.unit_flags = ai.unit_flags;
        create_data.unit_flags2 = ai.unit_flags2;
        create_data.unit_flags3 = ai.unit_flags3;
        create_data.damage_school = creature.melee_damage_school_like_cpp();
        create_data.ai_anim_kit_id = creature.unit().ai_anim_kit_id_like_cpp();
        create_data.movement_anim_kit_id = creature.unit().movement_anim_kit_id_like_cpp();
        create_data.melee_anim_kit_id = creature.unit().melee_anim_kit_id_like_cpp();
        Self {
            creature,
            create_data,
            active_move_spline: None,
            active_random_generator: None,
            active_waypoint_generator: None,
            active_waypoint_random_at_path_end: None,
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
            base_mana: data.max_power[0],
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
        self.creature.enter_ai_combat(attacker);
        debug!(
            "Creature {:?} entered combat with {:?}",
            self.guid(),
            attacker
        );
    }

    pub fn reset_combat(&mut self) {
        self.creature.reset_ai_combat(self.now_ms());
    }

    pub fn take_damage(&mut self, damage: u32) -> bool {
        self.creature.take_ai_damage(damage, self.now_ms())
    }

    pub fn take_damage_before_death_state_like_cpp(&mut self, damage: u32) -> bool {
        self.creature
            .apply_ai_damage_before_death_state_like_cpp(damage, self.now_ms())
    }

    pub fn complete_death_state_after_kill_hooks_like_cpp(&mut self) {
        self.creature
            .complete_ai_death_state_after_kill_hooks_like_cpp(self.now_ms());
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
        let mut ground = terrain.static_height_like_cpp(self.map_id(), point.x, point.y, probe_z);
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
        resolve_path: impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
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
        mut resolve_path: impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
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
        self.update_move_spline_like_cpp();

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
                detour_path = resolve_path(self.position(), destination, point_path_limit);
                if let Some(path) = detour_path.as_ref() {
                    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
                    path_result = random_path_result_from_path_type_like_cpp(path_type);
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
        resolve_path: impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
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
        resolve_path: impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        self.update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
            diff_ms,
            None,
            should_try_pathfinding,
            terrain,
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
            |_, _, _| None,
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
            |_, _, _| None,
        )
        .0
    }

    fn update_default_waypoint_movement_with_wait_roll_path_resolver_and_launch_like_cpp(
        &mut self,
        diff_ms: u32,
        wait_time_roll_ms: Option<i32>,
        should_try_pathfinding: bool,
        terrain: Option<&LiveTerrainHeights>,
        mut resolve_path: impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
    ) -> (WaypointMovementAction, Option<(Position, MoveSpline)>) {
        if let Some(mut random) = self.active_waypoint_random_at_path_end {
            let _ = self.update_move_spline_like_cpp();
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
        let _ = self.update_move_spline_like_cpp();

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
        resolve_path: &mut impl FnMut(Position, Position, usize) -> Option<DetourPolyPath>,
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
                        resolve_path(
                            self.position(),
                            launch.destination,
                            MAX_POINT_PATH_LENGTH_LIKE_CPP,
                        )
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
            self.pick_random_destination_from_current_position_like_cpp(random.wander_distance);
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

    pub fn roll_damage(&mut self) -> u32 {
        let min_dmg = self.min_dmg();
        let max_dmg = self.max_dmg();
        if min_dmg >= max_dmg {
            return min_dmg;
        }
        self.runtime_rng_like_cpp.gen_range(min_dmg..=max_dmg)
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

    pub fn pick_wander_destination(&mut self) -> Position {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = self.creature.ai_ownership().wander_radius.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let home = self.home_position();
        let x = home.x + angle.cos() * dist;
        let y = home.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Position::new(x, y, home.z, o)
    }

    pub fn pick_random_destination_from_current_position_like_cpp(
        &mut self,
        wander_distance: f32,
    ) -> Position {
        let angle = self
            .runtime_rng_like_cpp
            .gen_range(0.0..(2.0 * std::f32::consts::PI));
        let radius = wander_distance.max(0.0);
        let dist = self.runtime_rng_like_cpp.gen_range(0.0..=radius);
        let reference = self.position();
        let x = reference.x + angle.cos() * dist;
        let y = reference.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Position::new(x, y, reference.z, o)
    }

    pub fn reset_wander_timer(&mut self) {
        let now_ms = self.now_ms();
        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
    }

    pub fn initialize_random_wander_steps_like_cpp(&mut self) {
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        self.creature.ai_ownership_mut().wander_steps_remaining = wander_steps_remaining;
    }

    pub fn record_random_movement_launch_like_cpp(&mut self) {
        if self.creature.ai_ownership().wander_steps_remaining == 0 {
            self.initialize_random_wander_steps_like_cpp();
        }
        let ai = self.creature.ai_ownership_mut();
        ai.wander_steps_remaining = ai.wander_steps_remaining.saturating_sub(1);
        ai.state = CreatureAiState::WalkingRandom;
    }

    pub fn schedule_after_random_movement_like_cpp(&mut self) {
        let now_ms = self.now_ms();
        if self.creature.ai_ownership().wander_steps_remaining > 0 {
            let ai = self.creature.ai_ownership_mut();
            ai.move_start_ms = now_ms;
            ai.wander_delay_ms = 0;
            return;
        }

        let wander_delay_ms = self.runtime_rng_like_cpp.gen_range(4_000..=10_000);
        let wander_steps_remaining = self.runtime_rng_like_cpp.gen_range(2..=10);
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = wander_delay_ms;
        ai.wander_steps_remaining = wander_steps_remaining;
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

    /// Enqueue a creature waiting to respawn.
    /// C++ ref: `Map::_respawnTimes` insertion path (Map.cpp:2191).
    pub fn push_respawn(&mut self, respawn: PendingRespawn) {
        if let Some(existing_index) = self
            .respawn_queue
            .iter()
            .position(|queued| queued.spawn_id == respawn.spawn_id)
        {
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
    PendingRespawn {
        respawn_at,
        spawn_id: creature.creature.spawn_id(),
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
            unit_class: 1,
            display_power: creature.creature.unit().data().display_power,
            power: creature.creature.unit().data().power,
            max_power: creature.creature.unit().data().max_power,
            base_mana: creature.creature.unit().data().max_power[0],
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

    WorldCreature::from_canonical(creature, respawn.create_data.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wow_constants::{CreatureFlagsExtra, PhaseFlags};
    use wow_core::guid::HighGuid;
    use wow_map::map::MapWorldObjectEnvironment;

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

    fn test_creature(guid: ObjectGuid) -> WorldCreature {
        WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        )
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

    #[test]
    fn live_terrain_wires_static_vmap_los_provider_into_map_cache_like_cpp() {
        let dir = unique_temp_data_dir("live-vmap-los-provider");
        let provider = Arc::new(RecordingLiveStaticVMapLos::new(false));
        let shared_provider: SharedStaticVMapLineOfSightProvider = provider.clone();
        let terrain_cache =
            LiveTerrainHeights::new_with_static_vmap_line_of_sight(&dir, shared_provider);

        let terrain = terrain_cache.terrain_for_map(1);
        let mut source = wow_entities::WorldObject::new(
            false,
            wow_constants::TypeId::Unit,
            wow_constants::TypeMask::UNIT,
        );
        source.relocate(Position::new(10.0, 10.0, 1.0, 0.0));
        let query = wow_entities::LineOfSightQuery::to_position_like_cpp(
            &source,
            Position::new(20.0, 10.0, 1.0, 0.0),
            wow_entities::LineOfSightOptions::default(),
        );

        assert!(
            !terrain.line_of_sight(query),
            "live terrain must not bypass an installed static VMAP LOS provider"
        );
        let calls = provider
            .calls
            .lock()
            .expect("recording live vmap LOS calls poisoned");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].map_id, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn health_aura_state_like_cpp_matches_cpp_modify_aura_state() {
        // Regression for the world-entry ERROR #132 client crash: every creature
        // CREATE block must carry UNIT_FIELD_AURASTATE matching C++ Unit::Update ->
        // ModifyAuraState (Unit.cpp:469-476). A full-HP alive creature yields
        // 0x00D00000 (bits 20|22|23 = WOUND_HEALTH_20_80 | HEALTHY_75 | WOUND_HEALTH_35_80).
        // The client tests bit 0x100000 of this field on a per-frame tick; 0 crashed it.
        assert_eq!(
            WorldCreature::health_aura_state_like_cpp(100, 100, true),
            0x00D0_0000,
            "full-HP alive creature must match C++ 0x00D00000"
        );
        // Dead unit / zero max: no aura state (C++ only runs ModifyAuraState if IsAlive).
        assert_eq!(WorldCreature::health_aura_state_like_cpp(0, 100, false), 0);
        assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 0, true), 0);
        // Low health (<=20%): WOUNDED_20/25/35 + WOUND_HEALTH_20_80 + WOUND_HEALTH_35_80
        // bits set, HEALTHY_75 clear. Must include the crash bit 0x100000.
        let low = WorldCreature::health_aura_state_like_cpp(10, 100, true);
        assert_ne!(
            low & 0x0010_0000,
            0,
            "WOUND_HEALTH_20_80 (0x100000) set at low HP"
        );
        assert_eq!(low & 0x0040_0000, 0, "HEALTHY_75 clear at low HP");
        // Mid health (50%): none of the threshold states (not <35, not >75, not <20/>80).
        assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 100, true), 0);
    }

    #[test]
    fn create_data_from_canonical_clamps_zero_base_attack_time_like_cpp() {
        // Regression for the world-entry client crash: a creature CREATE block with
        // UnitData.AttackRoundBaseTime == 0 makes the 3.4.3 client divide-by-zero in its
        // swing-timer math on the first post-spawn tick (crash ~5s after the visibility
        // burst). C++ guarantees this is never 0 (ObjectMgr.cpp:1100-1104 clamps
        // creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME=2000).
        // A bare canonical Creature leaves Unit::base_attack_speed at its [0; MAX_ATTACK]
        // default, reproducing the bug; create_data_from_canonical_like_cpp must clamp it.
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9001);
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(9001);
        // Sanity: the underlying base attack speed really is the uninitialized 0 here.
        assert_eq!(
            creature.unit().base_attack_speed()[WeaponAttackType::BaseAttack as usize],
            0,
            "precondition: bare canonical creature has 0 base attack speed"
        );

        let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

        assert_eq!(
            create_data.base_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
            "0 base attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
        );
        assert_eq!(
            create_data.ranged_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
            "0 ranged attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
        );
    }

    #[test]
    fn create_data_from_canonical_preserves_nonzero_base_attack_time_like_cpp() {
        // The clamp must only replace 0; a real attack time must pass through unchanged.
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9002);
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(9002);
        creature
            .unit_mut()
            .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 1500);
        creature
            .unit_mut()
            .set_base_attack_time_like_cpp(WeaponAttackType::RangedAttack, 1800);

        let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

        assert_eq!(create_data.base_attack_time, 1500);
        assert_eq!(create_data.ranged_attack_time, 1800);
    }

    #[test]
    fn terrain_grid_area_map_decodes_cpp_area_cell_and_zone_parent() {
        let data_dir = unique_temp_data_dir("terrain-area-map");
        let map_id = 571;
        let x = 0.0;
        let y = 0.0;
        let (gx, gy) = terrain_grid_coords_for_wow_position_like_cpp(x, y);
        let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
        let area_size = (MAP_AREA_HEADER_SIZE_LIKE_CPP
            + MAP_AREA_CELLS_PER_GRID_LIKE_CPP
                * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
                * std::mem::size_of::<u16>()) as u32;

        let mut bytes = map_file_header_with_area_like_cpp(area_offset, area_size);
        bytes.extend_from_slice(MAP_AREA_MAGIC_LIKE_CPP);
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&4395_u16.to_le_bytes());
        let mut cells =
            [0_u16; MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP];
        cells[0] = 4613;
        for cell in cells {
            bytes.extend_from_slice(&cell.to_le_bytes());
        }
        fs::write(
            data_dir
                .join("maps")
                .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
            bytes,
        )
        .expect("write test map");

        let area_store = wow_data::AreaTableStore::from_entries([
            test_area_entry(4395, 0, 0),
            test_area_entry(4613, 4395, 0x4000_0000),
        ]);

        assert_eq!(
            zone_and_area_for_position_like_cpp(&data_dir, map_id, x, y, Some(&area_store), |_| {
                9999
            },)
            .expect("resolve terrain zone area"),
            (4395, 4613)
        );
    }

    #[test]
    fn terrain_zone_area_falls_back_to_map_area_when_grid_missing_like_cpp() {
        let data_dir = unique_temp_data_dir("terrain-area-fallback");
        assert_eq!(
            zone_and_area_for_position_like_cpp(&data_dir, 571, 0.0, 0.0, None, |_| 4395)
                .expect("resolve fallback terrain zone area"),
            (4395, 4395)
        );
    }

    #[test]
    fn world_creature_runtime_rng_replaces_timer_seeded_damage_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70001);
        let mut creature = test_creature(guid);
        creature.seed_runtime_rng_like_cpp(0xA141_BEEF);

        let rolls: Vec<u32> = (0..16).map(|_| creature.roll_damage()).collect();

        assert!(rolls.iter().all(|roll| (5..=10).contains(roll)));
        assert!(
            rolls.iter().any(|roll| *roll != rolls[0]),
            "damage rolls should come from owned RNG, not now_ms/spline_id: {rolls:?}"
        );
    }

    #[test]
    fn world_creature_random_movement_walk_rule_matches_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70003);
        let mut creature = test_creature(guid);

        creature.creature.set_random_movement_type_runtime_like_cpp(
            wow_constants::CreatureRandomMovementType::Walk as u8,
        );
        assert!(creature.random_movement_walk_like_cpp());

        creature.creature.set_random_movement_type_runtime_like_cpp(
            wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
        );
        assert!(!creature.random_movement_walk_like_cpp());

        creature.creature.set_random_movement_type_runtime_like_cpp(
            wow_constants::CreatureRandomMovementType::CanRun as u8,
        );
        creature
            .creature
            .set_movement_flags_runtime_like_cpp(MovementFlag::NONE);
        assert!(!creature.random_movement_walk_like_cpp());
        creature
            .creature
            .set_movement_flags_runtime_like_cpp(MovementFlag::WALKING);
        assert!(creature.random_movement_walk_like_cpp());
    }

    #[test]
    fn world_creature_random_spline_uses_walk_or_run_speed_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70004);
        let mut walker = test_creature(guid);
        walker.create_data.speed_walk_rate = 1.0;
        walker.create_data.speed_run_rate = 1.0;
        walker.creature.set_random_movement_type_runtime_like_cpp(
            wow_constants::CreatureRandomMovementType::Walk as u8,
        );
        let (_, walk_spline) = walker
            .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
            .expect("walk random spline");

        let mut runner = test_creature(guid);
        runner.create_data.speed_walk_rate = 1.0;
        runner.create_data.speed_run_rate = 1.0;
        runner.creature.set_random_movement_type_runtime_like_cpp(
            wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
        );
        let (_, run_spline) = runner
            .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
            .expect("run random spline");

        assert!((walk_spline.duration_ms() - 4_000).abs() <= 1);
        assert!((run_spline.duration_ms() - 1_429).abs() <= 1);
        assert!(
            run_spline.duration_ms() < walk_spline.duration_ms(),
            "C++ RandomMovementGenerator SetWalk(false) uses run speed"
        );
    }

    #[test]
    fn world_creature_default_random_initializes_generator_without_spline_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
        let mut creature = test_creature(guid);
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        creature.creature.ai_ownership_mut().wander_radius = 12.0;
        creature.seed_runtime_rng_like_cpp(0x7005);

        assert!(creature.initialize_default_random_movement_like_cpp());

        assert_eq!(creature.move_target(), None);
        assert!(creature.active_move_spline_like_cpp().is_none());
        assert!(
            !creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
        assert!(
            (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
            "C++ RandomMovementGenerator::DoInitialize seeds 2..10 steps but SetRandomLocation consumes the first step later"
        );
        assert_eq!(
            creature.creature.ai_ownership().wander_delay_ms,
            0,
            "C++ RandomMovementGenerator::DoInitialize resets its timer to 0 so the next update can choose a path"
        );
        assert_eq!(
            creature
                .creature
                .unit()
                .subsystems()
                .motion
                .current_movement_generator()
                .kind,
            MovementGeneratorKind::Random
        );
    }

    #[test]
    fn world_creature_random_wander_steps_pause_only_after_step_batch_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70006);
        let mut creature = test_creature(guid);
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        creature.creature.ai_ownership_mut().wander_steps_remaining = 2;

        creature.record_random_movement_launch_like_cpp();
        assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 1);
        creature.schedule_after_random_movement_like_cpp();
        assert_eq!(creature.creature.ai_ownership().wander_delay_ms, 0);

        creature.record_random_movement_launch_like_cpp();
        assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 0);
        creature.schedule_after_random_movement_like_cpp();
        assert!(
            (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
            "C++ RandomMovementGenerator pauses 4..10 seconds only after its wander step batch"
        );
        assert!(
            (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
            "C++ RandomMovementGenerator reseeds 2..10 wander steps after a pause"
        );
    }

    #[test]
    fn world_creature_interaction_pause_stops_and_updates_home_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
        let mut creature = test_creature(guid);
        let current = Position::new(14.0, 15.0, 16.0, 1.5);
        creature.creature.unit_mut().world_mut().relocate(current);
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .start_spline(42, 1_000);

        assert!(creature.pause_interaction_movement_like_cpp());

        let motion = &creature.creature.unit().subsystems().motion;
        assert!(motion.paused);
        assert!(motion.stopped);
        assert!(!motion.spline.enabled);
        assert_eq!(creature.home_position(), current);

        creature
            .creature
            .set_interaction_pause_timer_ms_runtime_like_cpp(0);
        assert!(!creature.pause_interaction_movement_like_cpp());
    }

    #[test]
    fn world_creature_wander_rng_matches_cpp_random_movement_bounds() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70002);
        let mut creature = test_creature(guid);
        creature.creature.ai_ownership_mut().wander_radius = 12.0;
        creature.seed_runtime_rng_like_cpp(0x5757);

        for _ in 0..24 {
            let dst = creature.pick_wander_destination();
            let dist = creature.home_position().distance(&dst);
            assert!(
                dist <= creature.creature.ai_ownership().wander_radius + f32::EPSILON,
                "wander destination {dst:?} was {dist} yd from home"
            );
        }

        for _ in 0..24 {
            creature.reset_wander_timer();
            assert!(
                (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
                "C++ RandomMovementGenerator pauses with urand(4, 10) seconds"
            );
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

    #[test]
    fn terrain_grid_coords_match_cpp_compute_grid_coord_reversal() {
        assert_eq!(
            terrain_grid_coords_for_wow_position_like_cpp(0.0, 0.0),
            (31, 31)
        );
        assert_eq!(
            terrain_grid_coords_for_wow_position_like_cpp(SIZE_OF_GRIDS_LIKE_CPP, 0.0),
            (30, 31)
        );
        assert_eq!(
            terrain_grid_coords_for_wow_position_like_cpp(-SIZE_OF_GRIDS_LIKE_CPP, 0.0),
            (32, 31)
        );
    }

    #[test]
    fn terrain_map_id_without_visible_maps_returns_source_map_like_cpp() {
        let phase_shift = PhaseShift::default();
        let mut called = false;

        let map_id =
            terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
                called = true;
                true
            });

        assert_eq!(map_id, 571);
        assert!(!called);
    }

    #[test]
    fn terrain_map_id_single_visible_map_returns_it_like_cpp() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        let mut called = false;

        let map_id =
            terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| {
                called = true;
                false
            });

        assert_eq!(map_id, 609);
        assert!(!called);
    }

    #[test]
    fn terrain_map_id_multiple_visible_maps_uses_child_grid_lookup_like_cpp() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(700, 1);
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        let mut checked = Vec::new();

        let map_id = terrain_map_id_for_phase_shift_like_cpp(
            &phase_shift,
            571,
            0.0,
            0.0,
            |visible_map_id, gx, gy| {
                checked.push((visible_map_id, gx, gy));
                visible_map_id == 609
            },
        );

        assert_eq!(map_id, 609);
        assert_eq!(checked, vec![(609, 31, 31)]);
    }

    #[test]
    fn terrain_map_id_multiple_visible_maps_falls_back_to_source_map_like_cpp() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        phase_shift.add_visible_map_id_like_cpp(700, 1);

        let map_id =
            terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0, |_, _, _| false);

        assert_eq!(map_id, 571);
    }

    #[test]
    fn terrain_grid_files_read_cpp_tilelist_bitset_string_order() {
        let data_dir = unique_temp_data_dir("terrain-grid-tilelist");
        let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
        fs::write(
            data_dir.join("maps").join("0609.tilelist"),
            tilelist_like_cpp([grid_idx]),
        )
        .expect("write tilelist");

        let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
            .expect("load terrain grid files");

        assert!(terrain.has_grid_file_like_cpp(31, 31));
        assert!(!terrain.has_grid_file_like_cpp(31, 30));
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn terrain_grid_files_fallback_validates_map_header_like_cpp() {
        let data_dir = unique_temp_data_dir("terrain-grid-map-header");
        fs::write(
            data_dir.join("maps").join("0609_31_31.map"),
            map_file_header_like_cpp(),
        )
        .expect("write map file");
        fs::write(
            data_dir.join("maps").join("0609_31_30.map"),
            b"not a valid map header",
        )
        .expect("write invalid map file");

        let terrain = TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 609, &HashMap::new())
            .expect("load terrain grid files");

        assert!(terrain.has_grid_file_like_cpp(31, 31));
        assert!(!terrain.has_grid_file_like_cpp(31, 30));
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn terrain_grid_files_has_child_terrain_grid_file_like_cpp() {
        let data_dir = unique_temp_data_dir("terrain-grid-child");
        let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
        fs::write(
            data_dir.join("maps").join("0571.tilelist"),
            tilelist_like_cpp([]),
        )
        .expect("write parent tilelist");
        fs::write(
            data_dir.join("maps").join("0609.tilelist"),
            tilelist_like_cpp([grid_idx]),
        )
        .expect("write child tilelist");
        let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);

        let terrain =
            TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
                .expect("load terrain grid files");

        assert!(terrain.has_child_terrain_grid_file_like_cpp(609, 31, 31));
        assert!(!terrain.has_child_terrain_grid_file_like_cpp(609, 31, 30));
        assert!(!terrain.has_child_terrain_grid_file_like_cpp(700, 31, 31));
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn terrain_grid_files_resolve_phase_shift_visible_map_like_cpp() {
        let data_dir = unique_temp_data_dir("terrain-grid-resolver");
        let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
        fs::write(
            data_dir.join("maps").join("0571.tilelist"),
            tilelist_like_cpp([]),
        )
        .expect("write parent tilelist");
        fs::write(
            data_dir.join("maps").join("0609.tilelist"),
            tilelist_like_cpp([grid_idx]),
        )
        .expect("write child tilelist");
        let parent_child_map_data = HashMap::from([(571, vec![609]), (609, Vec::new())]);
        let terrain =
            TerrainGridFilesLikeCpp::load_root_like_cpp(&data_dir, 571, &parent_child_map_data)
                .expect("load terrain grid files");
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(700, 1);
        phase_shift.add_visible_map_id_like_cpp(609, 1);

        assert_eq!(
            terrain.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
            609
        );
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn terrain_grid_file_index_resolves_root_and_visible_child_map_like_cpp() {
        let data_dir = unique_temp_data_dir("terrain-grid-index");
        let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
        fs::write(
            data_dir.join("maps").join("0571.tilelist"),
            tilelist_like_cpp([]),
        )
        .expect("write parent tilelist");
        fs::write(
            data_dir.join("maps").join("0609.tilelist"),
            tilelist_like_cpp([grid_idx]),
        )
        .expect("write child tilelist");
        let mut index =
            TerrainGridFileIndexLikeCpp::new(&data_dir, [(571, vec![609]), (609, Vec::new())]);
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(609, 1);

        assert_eq!(index.root_map_id_like_cpp(609), 571);
        assert_eq!(
            index.terrain_map_id_for_phase_shift_like_cpp(&phase_shift, 571, 0.0, 0.0),
            609
        );
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn world_mmap_pathfinder_resolves_mesh_map_from_phase_shift_like_cpp() {
        let data_dir = unique_temp_data_dir("mmap-phase-shift-mesh-map");
        let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
        fs::write(
            data_dir.join("maps").join("0571.tilelist"),
            tilelist_like_cpp([]),
        )
        .expect("write parent tilelist");
        fs::write(
            data_dir.join("maps").join("0609.tilelist"),
            tilelist_like_cpp([grid_idx]),
        )
        .expect("write child tilelist");
        let mut pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
            &data_dir,
            [(571, vec![609]), (609, Vec::new())],
        );
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        let request = WorldMMapPathRequestLikeCpp {
            start: Position::new(0.0, 0.0, 0.0, 0.0),
            destination: Position::new(20.0, 0.0, 0.0, 0.0),
            mesh_map_id: 571,
            instance_map_id: 571,
            instance_id: 42,
            filter_context: PathQueryFilterContext::creature(true, false, false, false),
            force_destination: false,
            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
            phase_shift,
        };

        assert_eq!(
            pathfinder.resolve_mesh_map_id_for_path_request_like_cpp(&request),
            609
        );
        fs::remove_dir_all(data_dir).expect("remove test dir");
    }

    #[test]
    fn test_world_to_grid_positive() {
        assert_eq!(world_to_grid_x(0.0), 0);
        assert_eq!(world_to_grid_x(63.9), 0);
        assert_eq!(world_to_grid_x(64.0), 1);
        assert_eq!(world_to_grid_x(127.9), 1);
        assert_eq!(world_to_grid_x(128.0), 2);
    }

    #[test]
    fn test_world_to_grid_negative() {
        assert_eq!(world_to_grid_x(-0.1), -1);
        assert_eq!(world_to_grid_x(-64.0), -1);
        assert_eq!(world_to_grid_x(-64.1), -2);
        assert_eq!(world_to_grid_x(-127.9), -2);
        assert_eq!(world_to_grid_x(-128.0), -2);
    }

    #[test]
    fn test_world_to_grid_coords() {
        let (x, y) = world_to_grid_coords(100.0, -50.0);
        assert_eq!(x, 1); // 100 / 64 = 1.56 -> floor = 1
        assert_eq!(y, -1); // -50 / 64 = -0.78 -> floor = -1
    }

    #[test]
    fn test_grid_round_trip() {
        let world_x = 150.5;
        let grid_x = world_to_grid_x(world_x);
        let world_center = grid_to_world(grid_x);
        // Center should be within half grid size
        assert!((world_x - world_center).abs() <= GRID_SIZE / 2.0);
    }

    #[test]
    fn test_creature_add_remove() {
        let mut grid = Grid::new(0, 0);
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );

        assert!(grid.add_creature(creature.clone()));
        assert_eq!(grid.creature_count(), 1);
        assert!(grid.get_creature(guid).is_some());

        assert!(grid.remove_creature(guid));
        assert_eq!(grid.creature_count(), 0);
        assert!(grid.get_creature(guid).is_none());
    }

    #[test]
    fn test_duplicate_creature_rejected() {
        let mut grid = Grid::new(0, 0);
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );

        assert!(grid.add_creature(creature.clone()));
        assert!(!grid.add_creature(creature)); // Duplicate should fail
    }

    #[test]
    fn test_player_enter_leave() {
        let mut grid = Grid::new(0, 0);
        let player = ObjectGuid::create_player(1, 1);

        grid.player_enter(player);
        assert!(grid.player_guids.contains(&player));

        grid.player_leave(player);
        assert!(!grid.player_guids.contains(&player));
    }

    #[test]
    fn test_should_unload() {
        let mut grid = Grid::new(0, 0);
        grid.last_player_time = Instant::now() - Duration::from_secs(400);
        assert!(grid.should_unload(Duration::from_secs(300)));
    }

    #[test]
    fn test_should_not_unload_with_player() {
        let mut grid = Grid::new(0, 0);
        let player = ObjectGuid::create_player(1, 1);
        grid.player_enter(player);
        grid.last_player_time = Instant::now() - Duration::from_secs(400);
        assert!(!grid.should_unload(Duration::from_secs(300)));
    }

    #[test]
    fn map_instance_load_personal_phase_grid_tracks_cpp_grid_id_once() {
        let owner = ObjectGuid::create_player(1, 1);
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
        phase_shift.set_personal_guid_like_cpp(owner);
        let mut map = MapInstance::new(571, 0);
        let mut loaded = Vec::new();

        assert!(map.load_personal_phase_grid_like_cpp(
            &phase_shift,
            3,
            5,
            |phase_id| phase_id == 10,
            |owner, phase_id| loaded.push((owner, phase_id)),
        ));
        assert!(map.is_grid_loaded(3, 5));
        assert_eq!(loaded, vec![(owner, 10)]);

        assert!(!map.load_personal_phase_grid_like_cpp(
            &phase_shift,
            3,
            5,
            |phase_id| phase_id == 10,
            |owner, phase_id| loaded.push((owner, phase_id)),
        ));
        assert_eq!(loaded, vec![(owner, 10)]);

        let tracker = map.personal_phases.owner_tracker_like_cpp(owner).unwrap();
        assert!(tracker.is_grid_loaded_for_phase_like_cpp(3 * 64 + 5, 10));
    }

    #[test]
    fn map_instance_unload_grid_purges_personal_phase_grid_tracking_like_cpp() {
        let owner = ObjectGuid::create_player(1, 1);
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
        phase_shift.set_personal_guid_like_cpp(owner);
        let mut map = MapInstance::new(571, 0);

        map.load_personal_phase_grid_like_cpp(&phase_shift, 3, 5, |_| true, |_, _| {});
        assert!(map.remove_grid(3, 5));
        assert!(map.personal_phases.owner_tracker_like_cpp(owner).is_none());
    }

    #[test]
    fn map_instance_update_personal_phases_queues_and_removes_expired_objects_like_cpp() {
        let owner = ObjectGuid::create_player(1, 1);
        let object = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 100);
        let mut map = MapInstance::new(571, 0);
        map.add_creature(0, 0, test_creature(object));
        map.register_personal_phase_object_like_cpp(10, owner, object);
        map.mark_personal_phases_for_deletion_like_cpp(owner);

        map.update_personal_phases_like_cpp(Duration::from_secs(60));
        assert_eq!(map.queued_personal_phase_remove_count_like_cpp(), 1);
        assert!(map.get_creature(0, 0, object).is_some());

        assert_eq!(map.remove_personal_phase_objects_like_cpp(), 1);
        assert!(map.get_creature(0, 0, object).is_none());
    }

    #[test]
    fn test_map_manager_create_map() {
        let mut manager = MapManager::new();
        let map = manager.get_or_create_map(0, 0);
        assert_eq!(map.map_id, 0);
        assert_eq!(map.instance_id, 0);
    }

    #[test]
    fn instance_id_allocator_generates_lowest_free_id_like_cpp() {
        let mut manager = MapManager::new();

        assert_eq!(manager.generate_instance_id(), Some(1));
        assert_eq!(manager.generate_instance_id(), Some(2));
        assert_eq!(manager.generate_instance_id(), Some(3));

        manager.free_instance_id(2);
        assert_eq!(manager.generate_instance_id(), Some(2));
        assert_eq!(manager.generate_instance_id(), Some(4));
    }

    #[test]
    fn instance_id_allocator_registers_loaded_ids_in_order_like_cpp() {
        let mut manager = MapManager::new();
        manager.init_instance_ids_from_max(5);

        manager.register_instance_id(1);
        manager.register_instance_id(2);
        manager.register_instance_id(4);

        assert_eq!(manager.generate_instance_id(), Some(3));
        assert_eq!(manager.generate_instance_id(), Some(5));
        assert_eq!(manager.generate_instance_id(), Some(6));
    }

    #[test]
    fn instance_id_allocator_keeps_zero_reserved_like_cpp() {
        let mut manager = MapManager::new();

        manager.free_instance_id(0);

        assert_eq!(manager.generate_instance_id(), Some(1));
    }

    #[test]
    fn test_add_creature_to_map() {
        let mut manager = MapManager::new();
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );

        assert!(manager.add_creature(0, 0, 0, 0, creature));
        assert!(manager.get_creature(0, 0, 0, 0, guid).is_some());
    }

    #[test]
    fn map_manager_uses_canonical_creature_guid_position_and_runtime() {
        let mut manager = MapManager::new();
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );

        assert!(manager.add_creature(0, 0, 0, 0, creature));
        let stored = manager
            .find_creature(0, 0, guid)
            .expect("canonical creature stored");
        assert_eq!(stored.guid(), guid);
        assert_eq!(stored.position(), Position::new(10.0, 10.0, 0.0, 0.0));
        assert_eq!(stored.current_hp(), 50);

        manager
            .find_creature_mut(0, 0, guid)
            .expect("canonical creature mutable")
            .take_damage(25);
        let stored = manager
            .find_creature(0, 0, guid)
            .expect("canonical creature stored");
        assert_eq!(stored.current_hp(), 25);
        assert_eq!(stored.creature.unit().data().health, 25);
    }

    #[test]
    fn world_creature_move_spline_bridge_advances_and_finalizes_like_cpp_unit_tick() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54321);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(0, 0)
            .expect("bind test creature to map");
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let dst = Position::new(15.0, 10.0, 0.0, 0.0);

        let (from, spline) = creature
            .begin_move_spline_like_cpp(dst)
            .expect("valid two-point spline");

        assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
        assert!(creature.active_move_spline.is_some());
        assert_eq!(creature.spline_id(), 2);
        assert!(
            creature
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::FORWARD),
            "C++ MoveSplineInit::Launch writes MOVEMENTFLAG_FORWARD to Unit::m_movementInfo"
        );
        assert!(
            MovementFlag::from_bits_retain(creature.create_data.movement_flags)
                .contains(MovementFlag::FORWARD),
            "the create bridge must mirror Unit::m_movementInfo after Launch"
        );
        assert!(
            creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        let motion_spline = &creature.creature.unit().subsystems().motion.spline;
        assert!(motion_spline.enabled);
        assert!(!motion_spline.finalized);
        assert_eq!(motion_spline.spline_id, spline.id());
        assert_eq!(motion_spline.duration_ms, spline.duration_ms() as u32);
        assert_eq!(motion_spline.final_destination, Some((15, 10, 0)));

        let duration_ms = spline.duration_ms() as u32;
        let now_ms = creature.now_ms();
        creature.creature.ai_ownership_mut().move_start_ms =
            now_ms.saturating_sub(u64::from(duration_ms / 2));
        assert!(!creature.update_move_spline_like_cpp());
        let mid = creature.position();
        assert!(mid.x > 10.0 && mid.x < 15.0, "mid position was {mid:?}");
        assert_eq!(
            creature
                .creature
                .unit()
                .subsystems()
                .motion
                .spline
                .progress_ms,
            duration_ms / 2
        );

        let now_ms = creature.now_ms();
        creature.creature.ai_ownership_mut().move_start_ms =
            now_ms.saturating_sub(u64::from(duration_ms));
        assert!(creature.update_move_spline_like_cpp());
        assert!(creature.active_move_spline.is_none());
        assert_eq!(creature.position(), dst);
        let motion_spline = &creature.creature.unit().subsystems().motion.spline;
        assert!(!motion_spline.enabled);
        assert!(motion_spline.finalized);
        assert_eq!(motion_spline.progress_ms, motion_spline.duration_ms);
        assert!(
            !creature
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::FORWARD),
            "C++ Unit::DisableSpline removes MOVEMENTFLAG_FORWARD on arrival"
        );
        assert!(
            !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
                .contains(MovementFlag::FORWARD),
            "the create bridge must mirror Unit::m_movementInfo after DisableSpline"
        );
        assert!(
            !creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
    }

    #[test]
    fn world_creature_move_spline_by_path_uses_cpp_moveby_path_bridge() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let path = [
            Position::new(10.0, 10.0, 0.0, 0.0),
            Position::new(12.0, 11.0, 0.0, 0.0),
            Position::new(15.0, 12.0, 0.0, 0.0),
        ];

        let (from, spline) = creature
            .begin_move_spline_by_path_like_cpp(path)
            .expect("valid multi-point path spline");

        assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
        assert!(creature.active_move_spline.is_some());
        assert_eq!(creature.spline_id(), 2);
        assert_eq!(creature.move_target(), Some(path[2]));
        assert_eq!(spline.final_destination(), Some(path[2]));
        assert_eq!(spline.monster_move_path_data().points, vec![path[2]]);
        assert_eq!(spline.monster_move_path_data().packed_deltas.len(), 1);
        assert!(
            creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        let motion_spline = &creature.creature.unit().subsystems().motion.spline;
        assert!(motion_spline.enabled);
        assert_eq!(motion_spline.spline_id, spline.id());
        assert_eq!(motion_spline.final_destination, Some((15, 12, 0)));
    }

    #[test]
    fn world_creature_waypoint_default_initialize_stores_generator_and_stops_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54329);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            77,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            ],
        );

        let action = creature.initialize_default_waypoint_movement_like_cpp(Some(path));

        assert_eq!(action, WaypointMovementAction::StopMoving);
        assert!(creature.creature.unit().subsystems().motion.stopped);
        let generator = creature
            .active_waypoint_generator_like_cpp()
            .expect("waypoint generator stored");
        assert_eq!(
            generator.next_move_time_ms(),
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP
        );
        assert_eq!(generator.stop_moving_calls, 1);
    }

    #[test]
    fn world_creature_waypoint_default_initialize_missing_path_does_not_stop_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54330);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );

        let action = creature.initialize_default_waypoint_movement_like_cpp(None);

        assert_eq!(action, WaypointMovementAction::MissingPath);
        assert!(!creature.creature.unit().subsystems().motion.stopped);
        assert!(creature.active_waypoint_generator_like_cpp().is_some());
    }

    #[test]
    fn world_creature_waypoint_default_initialize_resolves_owner_path_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54338);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.creature.load_path_like_cpp(90_001);
        let path = WaypointPath::new(
            90_001,
            vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
        );

        let action =
            creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
                (path_id == path.id).then_some(path.clone())
            });

        assert_eq!(action, WaypointMovementAction::StopMoving);
        assert!(
            creature.creature.unit().subsystems().motion.stopped,
            "C++ DoInitialize calls owner->StopMoving() after sWaypointMgr resolves the path"
        );
        assert_eq!(
            creature
                .active_waypoint_generator_like_cpp()
                .map(WaypointMovementGenerator::next_move_time_ms),
            Some(wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP)
        );
    }

    #[test]
    fn world_creature_waypoint_update_launches_initial_node_spline_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54331);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            77,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            ],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );

        let action = creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
        );

        match action {
            WaypointMovementAction::Launch(launch) => {
                assert_eq!(launch.node_id, 10);
                assert_eq!(launch.path_id, 77);
                assert_eq!(launch.destination, Position::new(11.0, 10.0, 0.0, 0.0));
            }
            other => panic!("expected initial waypoint launch, got {other:?}"),
        }
        assert!(creature.active_move_spline.is_some());
        assert_eq!(
            creature.move_target(),
            Some(Position::new(11.0, 10.0, 0.0, 0.0))
        );
        assert!(
            creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        assert_eq!(
            creature
                .active_waypoint_generator_like_cpp()
                .expect("waypoint generator")
                .waypoint_started
                .len(),
            1
        );
    }

    #[test]
    fn world_creature_waypoint_generate_path_uses_detour_point_path_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54348);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let destination = Position::new(30.0, 10.0, 0.0, 0.0);
        let path = WaypointPath::new(
            77,
            vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        let mut resolver_calls = 0;

        let (action, launched) = creature
            .update_default_waypoint_movement_with_path_resolver_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
                true,
                |start, destination_arg, point_path_limit| {
                    resolver_calls += 1;
                    assert_eq!(start, Position::new(10.0, 10.0, 0.0, 0.0));
                    assert_eq!(destination_arg, destination);
                    assert_eq!(point_path_limit, MAX_POINT_PATH_LENGTH_LIKE_CPP);
                    Some(DetourPolyPath {
                        poly_refs: vec![11, 22, 33],
                        point_path: wow_recastdetour::DetourPointPath {
                            points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                            actual_end: [30.0, 10.0, 0.0],
                            path_type: DetourPathType::NORMAL,
                        },
                        start_far_from_poly: false,
                        end_far_from_poly: false,
                    })
                },
            );

        assert!(matches!(action, WaypointMovementAction::Launch(_)));
        assert_eq!(resolver_calls, 1);
        let (_from, spline) = launched.expect("waypoint detour path launches");
        assert_eq!(spline.final_destination(), Some(destination));
        assert!(
            spline
                .create_object_path_points_like_cpp()
                .contains(&Position::new(20.0, 15.0, 2.0, 0.0)),
            "C++ MoveSplineInit::MoveTo(generatePath=true) switches to MovebyPath(PathGenerator::GetPath())"
        );
        assert!(
            !spline.monster_move_path_data().packed_deltas.is_empty(),
            "a generated multi-point waypoint path must not serialize as a single direct segment"
        );
    }

    #[test]
    fn world_creature_waypoint_generate_path_nopath_falls_back_direct_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54349);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let destination = Position::new(30.0, 10.0, 0.0, 0.0);
        let path = WaypointPath::new(
            77,
            vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        let mut resolver_calls = 0;

        let (_action, launched) = creature
            .update_default_waypoint_movement_with_path_resolver_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
                true,
                |_start, _destination_arg, _point_path_limit| {
                    resolver_calls += 1;
                    Some(DetourPolyPath {
                        poly_refs: Vec::new(),
                        point_path: wow_recastdetour::DetourPointPath {
                            points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                            actual_end: [30.0, 10.0, 0.0],
                            path_type: DetourPathType::NOPATH,
                        },
                        start_far_from_poly: false,
                        end_far_from_poly: false,
                    })
                },
            );

        assert_eq!(resolver_calls, 1);
        let (_from, spline) = launched.expect("waypoint direct fallback launches");
        assert_eq!(spline.final_destination(), Some(destination));
        assert!(
            spline.monster_move_path_data().packed_deltas.is_empty(),
            "C++ MoveSplineInit::MoveTo(generatePath=true) falls back to a direct path when PathGenerator reports NOPATH"
        );
    }

    #[test]
    fn world_creature_waypoint_launch_applies_land_takeoff_anim_tier_like_cpp() {
        for (move_type, expected_anim_tier) in [
            (wow_movement::WaypointMoveType::Land, 0),
            (wow_movement::WaypointMoveType::TakeOff, 2),
        ] {
            let guid = ObjectGuid::create_world_object(
                HighGuid::Creature,
                0,
                1,
                0,
                0,
                1,
                54335 + i64::from(expected_anim_tier),
            );
            let mut creature = WorldCreature::new(
                guid,
                1,
                Position::new(10.0, 10.0, 0.0, 0.0),
                50,
                2,
                5,
                10,
                20.0,
                100,
                14,
                0,
                0,
            );
            let mut path = WaypointPath::new(
                90 + expected_anim_tier as u32,
                vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
            );
            path.move_type = move_type;
            assert_eq!(
                creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
                WaypointMovementAction::StopMoving
            );

            assert!(matches!(
                creature.update_default_waypoint_movement_like_cpp(
                    wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
                ),
                WaypointMovementAction::Launch(_)
            ));

            assert_eq!(
                creature
                    .active_move_spline
                    .as_ref()
                    .and_then(MoveSpline::anim_tier)
                    .map(|anim| anim.anim_tier),
                Some(expected_anim_tier)
            );
        }
    }

    #[test]
    fn world_creature_waypoint_arrival_records_inform_and_launches_next_node_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54332);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            77,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0).with_delay(500),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            ],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));
        creature
            .active_move_spline
            .as_mut()
            .expect("initial waypoint spline")
            .finalize();
        assert!(creature.update_move_spline_like_cpp());

        let arrived = creature.update_default_waypoint_movement_like_cpp(0);

        match arrived {
            WaypointMovementAction::Arrived(arrived) => {
                assert_eq!(arrived.inform.node_id, 10);
                assert_eq!(arrived.inform.path_id, 77);
                assert_eq!(arrived.timer_ms, Some(500));
            }
            other => panic!("expected waypoint arrival, got {other:?}"),
        }
        assert!(
            !creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
                movement_id: 10,
            })
        );

        let next = creature.update_default_waypoint_movement_like_cpp(500);

        match next {
            WaypointMovementAction::Launch(launch) => {
                assert_eq!(launch.node_id, 20);
                assert_eq!(launch.path_id, 77);
                assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
            }
            other => panic!("expected next waypoint launch, got {other:?}"),
        }
        assert_eq!(
            creature.move_target(),
            Some(Position::new(12.0, 10.0, 0.0, 0.0))
        );
    }

    #[test]
    fn world_creature_waypoint_arrival_without_delay_launches_next_node_same_tick_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54333);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            88,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            ],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));
        creature
            .active_move_spline
            .as_mut()
            .expect("single waypoint spline")
            .finalize();
        assert!(creature.update_move_spline_like_cpp());

        let action = creature.update_default_waypoint_movement_like_cpp(0);

        match action {
            WaypointMovementAction::Launch(launch) => {
                assert_eq!(launch.node_id, 20);
                assert_eq!(launch.path_id, 88);
                assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
            }
            other => panic!("expected same-tick next waypoint launch, got {other:?}"),
        }
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
                movement_id: 10,
            })
        );
        assert_eq!(
            creature.move_target(),
            Some(Position::new(12.0, 10.0, 0.0, 0.0))
        );
    }

    #[test]
    fn world_creature_waypoint_tick_advances_spline_before_motionmaster_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54338);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            92,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            ],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));
        creature
            .active_move_spline
            .as_mut()
            .expect("initial waypoint spline")
            .finalize();
        assert!(
            !creature
                .creature
                .unit()
                .subsystems()
                .motion
                .spline
                .finalized,
            "the represented MotionSubsystem is stale until Unit::UpdateSplineMovement runs"
        );

        let action = creature.update_default_waypoint_movement_like_cpp(0);

        match action {
            WaypointMovementAction::Launch(launch) => {
                assert_eq!(launch.node_id, 20);
                assert_eq!(launch.path_id, 92);
                assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
            }
            other => panic!("expected next waypoint launch after spline advance, got {other:?}"),
        }
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
                movement_id: 10,
            })
        );
    }

    #[test]
    fn world_creature_waypoint_single_node_path_ends_same_tick_after_arrival_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54334);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let path = WaypointPath::new(
            89,
            vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
        );
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );
        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));
        creature
            .active_move_spline
            .as_mut()
            .expect("single waypoint spline")
            .finalize();
        assert!(creature.update_move_spline_like_cpp());

        let ended = creature.update_default_waypoint_movement_like_cpp(0);

        match ended {
            WaypointMovementAction::PathEnded(ended) => {
                assert_eq!(ended.node_id, 10);
                assert_eq!(ended.path_id, 89);
            }
            other => panic!("expected waypoint path end, got {other:?}"),
        }
        assert_eq!(
            creature.home_position(),
            Position::new(11.0, 10.0, 0.0, 0.0)
        );
    }

    #[test]
    fn world_creature_waypoint_path_end_random_handoff_launches_active_random_spline_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54337);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let mut path = WaypointPath::new(
            91,
            vec![
                wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(30, 13.0, 10.0, 0.0),
            ],
        );
        path.follow_path_backwards_from_end_to_start = true;
        creature.active_waypoint_generator = Some(WaypointMovementGenerator::from_path(
            path,
            true,
            Some(10_000),
            None,
            wow_movement::MovementWalkRunSpeedSelectionMode::Default,
            Some((1_000, 2_000)),
            Some(5.0),
            true,
            true,
        ));

        for expected_node in [10, 20, 30] {
            match creature.update_default_waypoint_movement_like_cpp(0) {
                WaypointMovementAction::Launch(launch) => assert_eq!(launch.node_id, expected_node),
                other => panic!("expected waypoint launch for node {expected_node}, got {other:?}"),
            }
            creature
                .active_move_spline
                .as_mut()
                .expect("active waypoint spline")
                .finalize();
            assert!(creature.update_move_spline_like_cpp());
        }

        let action =
            creature.update_default_waypoint_movement_with_wait_roll_like_cpp(0, Some(1_500));

        match action {
            WaypointMovementAction::Arrived(arrived) => {
                assert_eq!(arrived.inform.node_id, 30);
                assert_eq!(
                    arrived.move_random_at_path_end,
                    Some(WaypointRandomAtPathEnd {
                        wander_distance: 5.0,
                        duration_ms: 1_500,
                    })
                );
                assert_eq!(arrived.duration_after_wait_ms, Some(8_500));
            }
            other => panic!("expected endpoint random handoff arrival, got {other:?}"),
        }
        let random_target = creature
            .move_target()
            .expect("C++ MoveRandom handoff should launch an active random spline");
        assert!(creature.active_move_spline.is_some());
        assert!(
            random_target.distance_2d(&Position::new(13.0, 10.0, 0.0, 0.0)) <= 5.001,
            "C++ RandomMovementGenerator chooses a destination within _wanderDistance of its reference"
        );
        assert_eq!(
            creature.active_waypoint_random_at_path_end_like_cpp(),
            Some(WaypointRandomAtPathEnd {
                wander_distance: 5.0,
                duration_ms: 1_500,
            })
        );
        assert_eq!(
            creature
                .active_waypoint_generator_like_cpp()
                .and_then(WaypointMovementGenerator::duration_ms),
            Some(8_500)
        );

        assert_eq!(
            creature.update_default_waypoint_movement_like_cpp(100),
            WaypointMovementAction::Continue
        );
        assert!(creature.active_move_spline.is_some());
        assert_eq!(
            creature.active_waypoint_random_at_path_end_like_cpp(),
            Some(WaypointRandomAtPathEnd {
                wander_distance: 5.0,
                duration_ms: 1_400,
            })
        );
    }

    #[test]
    fn world_creature_detour_path_bridge_uses_moveby_path_or_direct_fallback_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let normal_path = DetourPolyPath {
            poly_refs: vec![11, 22],
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, 0.0], [12.0, 11.0, 0.0], [15.0, 12.0, 0.0]],
                actual_end: [15.0, 12.0, 0.0],
                path_type: DetourPathType::NORMAL,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };
        let dst = Position::new(15.0, 12.0, 0.0, 0.0);

        let (from, spline, path) = creature
            .begin_move_spline_with_detour_path_like_cpp(dst, Some(&normal_path), false)
            .expect("detour path launches");

        assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
        assert_eq!(spline.final_destination(), Some(dst));
        assert_eq!(spline.monster_move_path_data().points, vec![dst]);
        let path = path.expect("path generator");
        assert_eq!(path.path_type(), PathType::NORMAL);
        assert_eq!(path.poly_length(), 2);
        assert_eq!(
            path.path_points(),
            &[
                Position::new(10.0, 10.0, 0.0, 0.0),
                Position::new(12.0, 11.0, 0.0, 0.0),
                dst
            ]
        );

        let nopath = DetourPolyPath {
            poly_refs: Vec::new(),
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[15.0, 12.0, 0.0], [20.0, 10.0, 0.0]],
                actual_end: [20.0, 10.0, 0.0],
                path_type: DetourPathType::NOPATH,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };
        let fallback_dst = Position::new(20.0, 10.0, 0.0, 0.0);

        let (_from, fallback_spline, fallback_path) = creature
            .begin_move_spline_with_detour_path_like_cpp(fallback_dst, Some(&nopath), false)
            .expect("direct fallback launches");

        assert_eq!(fallback_spline.final_destination(), Some(fallback_dst));
        assert!(
            fallback_path
                .expect("fallback path metadata")
                .path_type()
                .contains(PathType::NOPATH)
        );
    }

    #[test]
    fn world_creature_detour_path_bridge_normalizes_points_to_terrain_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54350);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 1.75, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(1, 0)
            .expect("bind test creature to terrain map");
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
        let terrain = LiveTerrainHeights::new(&data_dir);
        assert!(
            (terrain.static_height_like_cpp(1, 10.0, 10.0, 51.75) - 2.0).abs() < 1e-3,
            "synthetic terrain tile must cover the test path"
        );
        let dst = Position::new(15.0, 12.0, 1.75, 0.0);
        let normal_path = DetourPolyPath {
            poly_refs: vec![11, 22],
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, 1.75], [12.0, 11.0, 1.75], [15.0, 12.0, 1.75]],
                actual_end: [15.0, 12.0, 1.75],
                path_type: DetourPathType::NORMAL,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };

        let (_from, spline, path) = creature
            .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
                dst,
                Some(&normal_path),
                false,
                Some(&terrain),
            )
            .expect("terrain-normalized detour path launches");

        let path = path.expect("path generator");
        assert_eq!(
            path.path_points(),
            &[
                Position::new(10.0, 10.0, 2.0, 0.0),
                Position::new(12.0, 11.0, 2.0, 0.0),
                Position::new(15.0, 12.0, 2.0, 0.0),
            ],
            "C++ PathGenerator::NormalizePath calls UpdateAllowedPositionZ for every path point"
        );
        assert_eq!(
            spline.final_destination(),
            Some(Position::new(15.0, 12.0, 2.0, 0.0))
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn world_creature_detour_path_bridge_raises_low_mmap_points_to_grid_ground_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54351);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 43.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(1, 0)
            .expect("bind test creature to terrain map");
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let data_dir = temp_dir_with_constant_tile(1, 31, 31, 50.0);
        let terrain = LiveTerrainHeights::new(&data_dir);
        assert!(
            terrain.static_height_like_cpp(1, 10.0, 10.0, 43.0 + Z_OFFSET_FIND_HEIGHT)
                <= INVALID_HEIGHT,
            "the C++ probe gate rejects this low Rust MMap point"
        );
        assert!(
            (terrain.grid_height_like_cpp(1, 10.0, 10.0) - 50.0).abs() < 1e-3,
            "synthetic terrain still has a usable raw ground height"
        );
        let dst = Position::new(15.0, 12.0, 43.0, 0.0);
        let low_mmap_path = DetourPolyPath {
            poly_refs: vec![11, 22],
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, 43.0], [12.0, 11.0, 43.0], [15.0, 12.0, 43.0]],
                actual_end: [15.0, 12.0, 43.0],
                path_type: DetourPathType::NORMAL,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };

        let (_from, spline, path) = creature
            .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
                dst,
                Some(&low_mmap_path),
                false,
                Some(&terrain),
            )
            .expect("terrain-normalized low detour path launches");

        let path = path.expect("path generator");
        assert_eq!(
            path.path_points(),
            &[
                Position::new(10.0, 10.0, 50.0, 0.0),
                Position::new(12.0, 11.0, 50.0, 0.0),
                Position::new(15.0, 12.0, 50.0, 0.0),
            ],
            "NormalizePath must not serialize underground Rust MMap points to the client"
        );
        assert_eq!(
            spline.final_destination(),
            Some(Position::new(15.0, 12.0, 50.0, 0.0))
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn world_creature_detour_path_bridge_keeps_far_below_points_without_ground_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, -5.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .unit_mut()
            .world_mut()
            .set_map(1, 0)
            .expect("bind test creature to terrain map");
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let data_dir = temp_dir_with_constant_tile(1, 31, 31, 50.0);
        let terrain = LiveTerrainHeights::new(&data_dir);
        let dst = Position::new(15.0, 12.0, -5.0, 0.0);
        let far_below_path = DetourPolyPath {
            poly_refs: vec![11, 22],
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, -5.0], [12.0, 11.0, -5.0], [15.0, 12.0, -5.0]],
                actual_end: [15.0, 12.0, -5.0],
                path_type: DetourPathType::NORMAL,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };

        let (_from, spline, path) = creature
            .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
                dst,
                Some(&far_below_path),
                false,
                Some(&terrain),
            )
            .expect("far-below detour path launches without terrain lift");

        let path = path.expect("path generator");
        assert_eq!(
            path.path_points(),
            &[
                Position::new(10.0, 10.0, -5.0, 0.0),
                Position::new(12.0, 11.0, -5.0, 0.0),
                Position::new(15.0, 12.0, -5.0, 0.0),
            ],
            "points farther than DEFAULT_HEIGHT_SEARCH below raw ground keep C++ no-ground behavior"
        );
        assert_eq!(
            spline.final_destination(),
            Some(Position::new(15.0, 12.0, -5.0, 0.0))
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn world_creature_random_detour_rejects_nopath_and_shortcut_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let dst = Position::new(20.0, 10.0, 0.0, 0.0);

        for path_type in [DetourPathType::NOPATH, DetourPathType::SHORTCUT] {
            let detour_path = DetourPolyPath {
                poly_refs: Vec::new(),
                point_path: wow_recastdetour::DetourPointPath {
                    points: vec![[10.0, 10.0, 0.0], [20.0, 10.0, 0.0]],
                    actual_end: [20.0, 10.0, 0.0],
                    path_type,
                },
                start_far_from_poly: false,
                end_far_from_poly: false,
            };

            assert!(
                creature
                    .begin_random_move_spline_with_detour_path_like_cpp(
                        dst,
                        Some(&detour_path),
                        false
                    )
                    .is_none(),
                "C++ RandomMovementGenerator retries later instead of launching {:?} paths",
                path_type
            );
            assert!(creature.active_move_spline_like_cpp().is_none());
        }
    }

    #[test]
    fn world_creature_random_missing_path_retries_instead_of_direct_fallback_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54340);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        creature.creature.ai_ownership_mut().wander_radius = 8.0;
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);

        let mut resolver_called = false;
        let movement = creature.update_default_random_movement_with_path_resolver_like_cpp(
            10,
            true,
            |_start, _destination, _point_path_limit| {
                resolver_called = true;
                None
            },
        );

        assert!(resolver_called);
        assert!(
            movement.is_none(),
            "C++ RandomMovementGenerator retries when PathGenerator cannot build a usable path"
        );
        assert!(creature.active_move_spline_like_cpp().is_none());
        assert_eq!(
            creature
                .active_random_generator
                .as_ref()
                .expect("random generator")
                .timer_ms(),
            wow_movement::RANDOM_PATH_RETRY_MS_LIKE_CPP
        );
    }

    #[test]
    fn calculate_creature_detour_path_returns_none_until_runtime_mmap_exists_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let dst = Position::new(20.0, 10.0, 0.0, 0.0);
        let filter_context = PathQueryFilterContext::creature(true, false, false, false);

        assert_eq!(
            calculate_creature_detour_path_like_cpp(
                &creature,
                dst,
                None,
                0,
                0,
                filter_context,
                false
            ),
            Ok(None)
        );

        let mmap_data = MMapData::new(wow_recastdetour::DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 16,
            max_polys: 16,
        })
        .expect("navmesh allocation");
        assert_eq!(
            calculate_creature_detour_path_like_cpp(
                &creature,
                dst,
                Some(&mmap_data),
                0,
                0,
                filter_context,
                false,
            ),
            Ok(None)
        );
    }

    #[test]
    fn world_mmap_pathfinder_falls_back_when_runtime_tile_missing_like_cpp() {
        let root = unique_test_dir("world-mmap-pathfinder-missing-tile");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();
        let params = wow_recastdetour::DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 4096,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(0.0, 0.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&root);
        let filter_context = PathQueryFilterContext::creature(true, false, false, false);

        assert_eq!(
            pathfinder.calculate_creature_path_like_cpp(
                &creature,
                Position::new(20.0, 0.0, 0.0, 0.0),
                1,
                1,
                42,
                filter_context,
                false,
            ),
            Ok(None)
        );
        assert!(
            pathfinder
                .mmap_manager()
                .get_nav_mesh_query(1, 1, 42)
                .is_some()
        );
        assert_eq!(pathfinder.mmap_manager().get_loaded_tiles_count(), 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn world_mmap_pathfinder_worker_keeps_detour_off_session_thread_like_cpp() {
        let root = unique_test_dir("world-mmap-pathfinder-worker-missing-tile");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();
        let params = wow_recastdetour::DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 4096,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let worker = WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
        let result = worker.calculate_path_like_cpp(WorldMMapPathRequestLikeCpp {
            start: Position::new(0.0, 0.0, 0.0, 0.0),
            destination: Position::new(20.0, 0.0, 0.0, 0.0),
            mesh_map_id: 1,
            instance_map_id: 1,
            instance_id: 42,
            filter_context: PathQueryFilterContext::creature(true, false, false, false),
            force_destination: false,
            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
            phase_shift: PhaseShift::default(),
        });

        assert_eq!(result, Ok(None));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn world_mmap_pathfinder_initializes_thread_unsafe_parent_map_data_like_cpp() {
        let root = unique_test_dir("world-mmap-pathfinder-parent-map-data");
        let pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
            &root,
            [(571, vec![609]), (609, Vec::new())],
        );

        assert!(!pathfinder.mmap_manager().is_thread_safe_environment());
        assert_eq!(pathfinder.mmap_manager().get_loaded_maps_count(), 2);
        assert_eq!(pathfinder.mmap_manager().parent_map_id(609), Some(571));
    }

    #[test]
    fn world_creature_begin_point_movement_uses_point_lifecycle_and_real_spline() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54323);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let dst = Position::new(14.0, 10.0, 0.0, 0.0);

        let (from, spline) = creature
            .begin_point_movement_like_cpp(42, dst, true)
            .expect("point movement starts direct spline");

        assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
        assert!(creature.active_move_spline.is_some());
        assert_eq!(creature.move_target(), Some(dst));
        assert!(
            creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        let motion = &creature.creature.unit().subsystems().motion;
        let generator = motion.current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Point);
        assert_eq!(generator.movement_id, 42);
        assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
        assert!(!generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
        assert!(motion.spline.enabled);
        assert_eq!(motion.spline.spline_id, spline.id());
        assert_eq!(motion.spline.final_destination, Some((14, 10, 0)));

        {
            let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Point)
                .expect("point generator");
            assert_eq!(
                generator.update_point_like_cpp(true, true),
                PointMovementAction::Finished
            );
        }
        assert_eq!(
            creature.finalize_point_movement_like_cpp(true, true),
            Some(PointMovementInform {
                kind: MovementGeneratorKind::Point,
                movement_id: 42,
            })
        );
        assert!(
            !creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Point.trinity_id(),
                movement_id: 42,
            })
        );
    }

    #[test]
    fn world_creature_begin_point_movement_handles_blocked_and_prepath_branches() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let dst = Position::new(14.0, 10.0, 0.0, 0.0);

        assert!(
            creature
                .begin_point_movement_like_cpp(43, dst, false)
                .is_none()
        );
        assert!(creature.active_move_spline.is_none());
        let generator = creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator();
        assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INTERRUPTED));
        assert!(creature.creature.unit().subsystems().motion.stopped);

        assert!(
            creature
                .begin_point_movement_like_cpp(EVENT_CHARGE_PREPATH, dst, true)
                .is_none()
        );
        assert!(creature.active_move_spline.is_none());
        assert!(
            creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        let generator = creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Point);
        assert_eq!(generator.movement_id, EVENT_CHARGE_PREPATH);
        assert_eq!(generator.base_unit_state, UnitState::CHARGING.bits());
    }

    #[test]
    fn world_creature_finalize_generic_movement_records_ai_inform_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        let target = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
        {
            let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
            motion.launch_generic_movement(
                MovementGeneratorKind::Effect,
                77,
                1_000,
                Some((1234, target)),
            );
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Effect)
                .expect("generic effect generator");
            generator.initialize_generic_like_cpp();
            assert!(!generator.update_generic_like_cpp(1_000, false, false));
        }

        assert_eq!(
            creature.finalize_generic_movement_like_cpp(MovementGeneratorKind::Effect, 77, true),
            Some(GenericMovementInform {
                kind: MovementGeneratorKind::Effect,
                movement_id: 77,
                arrival_spell_id: Some(1234),
                arrival_spell_target_guid: Some(target),
            })
        );
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Effect.trinity_id(),
                movement_id: 77,
            })
        );
    }

    #[test]
    fn world_creature_begin_distract_and_rotate_launch_facing_splines_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        creature
            .creature
            .unit_mut()
            .set_stand_state_like_cpp(UnitStandStateType::Sit);

        let (action, from, spline) = creature
            .begin_distract_movement_like_cpp(500, 1.25)
            .expect("distract launches facing spline");

        assert_eq!(
            action,
            DistractMovementAction {
                stand_up: true,
                launch_facing_spline: true,
            }
        );
        assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
        assert_eq!(
            creature.creature.unit().stand_state_like_cpp(),
            UnitStandStateType::Stand
        );
        assert_eq!(
            spline.facing().kind,
            wow_movement::MonsterMoveType::FacingAngle
        );
        assert!((spline.facing().angle - 1.25).abs() < 0.0001);
        assert!(spline.spline_is_facing_only);
        assert_eq!(creature.spline_id(), spline.id());
        let generator = creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Distract);
        assert!(generator.has_flag(wow_entities::MOVEMENTGENERATOR_FLAG_INITIALIZED));
        creature
            .creature
            .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 2.5));
        {
            let motion = &mut creature.creature.unit_mut().subsystems_mut().motion;
            let generator = motion
                .active_generators
                .iter_mut()
                .find(|generator| generator.kind == MovementGeneratorKind::Distract)
                .expect("distract generator");
            assert!(!generator.update_distract_like_cpp(true, 501));
        }
        assert!(creature.finalize_distract_movement_like_cpp(true));
        assert!((creature.position().orientation - 2.5).abs() < 0.0001);

        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .motion
            .clear_active();
        assert!(
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_rotate_like_cpp(8, 1_000, wow_entities::RotateDirection::Left)
        );
        let (update, spline) = creature
            .tick_rotate_movement_like_cpp(250)
            .expect("rotate tick launches facing spline");
        assert!(update.keep_running);
        let expected_rotate_angle = 2.5 + std::f32::consts::FRAC_PI_2;
        assert!(
            update
                .facing_angle
                .is_some_and(|angle| (angle - expected_rotate_angle).abs() < 0.0001)
        );
        assert_eq!(
            spline.facing().kind,
            wow_movement::MonsterMoveType::FacingAngle
        );
        assert!(
            (spline.facing().angle - expected_rotate_angle).abs() < 0.0001,
            "facing angle was {}",
            spline.facing().angle
        );
        assert!(spline.spline_is_facing_only);
        let generator = creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator();
        assert_eq!(generator.kind, MovementGeneratorKind::Rotate);
        assert_eq!(generator.duration_ms, Some(750));
        assert_eq!(
            creature.finalize_rotate_movement_like_cpp(true),
            Some(PointMovementInform {
                kind: MovementGeneratorKind::Rotate,
                movement_id: 8,
            })
        );
        assert_eq!(
            creature.creature.ai_ownership().last_movement_inform,
            Some(wow_entities::CreatureMovementInform {
                movement_type: MovementGeneratorKind::Rotate.trinity_id(),
                movement_id: 8,
            })
        );
    }

    #[test]
    fn world_creature_stop_move_spline_emits_cpp_stop_state_before_arrival() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            2,
            5,
            10,
            20.0,
            100,
            14,
            0,
            0,
        );
        creature.clock_started_at = Instant::now() - Duration::from_secs(10);
        let dst = Position::new(20.0, 10.0, 0.0, 0.0);
        let (_, spline) = creature
            .begin_move_spline_like_cpp(dst)
            .expect("valid two-point spline");
        assert!(
            creature
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::FORWARD)
        );
        let duration_ms = spline.duration_ms() as u32;
        let now_ms = creature.now_ms();
        creature.creature.ai_ownership_mut().move_start_ms =
            now_ms.saturating_sub(u64::from(duration_ms / 2));

        let stop = creature
            .stop_move_spline_like_cpp()
            .expect("active spline stops");

        assert_eq!(stop.spline_id, 3);
        assert_eq!(stop.stop_distance_tolerance, 2);
        assert!(stop.position.x > 10.0 && stop.position.x < 20.0);
        assert_eq!(creature.position(), stop.position);
        assert!(creature.active_move_spline.is_none());
        assert_eq!(creature.move_target(), None);
        assert!(
            !creature
                .creature
                .movement_flags_like_cpp()
                .contains(MovementFlag::FORWARD),
            "C++ MoveSplineInit::Stop removes MOVEMENTFLAG_FORWARD"
        );
        assert!(
            !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
                .contains(MovementFlag::FORWARD)
        );
        assert!(
            !creature
                .creature
                .unit()
                .has_unit_state(UnitState::ROAMING_MOVE.bits())
        );
        let motion_spline = &creature.creature.unit().subsystems().motion.spline;
        assert!(!motion_spline.enabled);
        assert!(motion_spline.finalized);
        assert_eq!(motion_spline.spline_id, stop.spline_id);
        assert!(creature.stop_move_spline_like_cpp().is_none());
    }

    #[test]
    fn test_visible_creatures() {
        let mut manager = MapManager::new();
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );

        manager.add_creature(0, 0, 0, 0, creature);

        // Should find creature at (10, 10)
        let visible = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
        assert!(!visible.is_empty());
        assert_eq!(visible[0].guid(), guid);

        // Should not find creature far away
        let visible = manager.get_visible_creatures(0, 0, 1000.0, 1000.0, 0.0);
        assert!(visible.is_empty());
    }

    #[test]
    fn visible_creatures_in_phase_filters_like_cpp_grid_searchers() {
        let mut manager = MapManager::new();
        let visible_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);
        let hidden_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

        let mut seer_phase = PhaseShift::default();
        seer_phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

        let mut visible_creature = WorldCreature::new(
            visible_guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );
        visible_creature
            .creature
            .unit_mut()
            .world_mut()
            .phase_shift_mut()
            .add_phase_like_cpp(20, wow_constants::PhaseFlags::empty(), 1);

        let mut hidden_creature = WorldCreature::new(
            hidden_guid,
            1,
            Position::new(11.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );
        hidden_creature
            .creature
            .unit_mut()
            .world_mut()
            .phase_shift_mut()
            .add_phase_like_cpp(30, wow_constants::PhaseFlags::empty(), 1);

        manager.add_creature(0, 0, 0, 0, visible_creature);
        manager.add_creature(0, 0, 0, 0, hidden_creature);

        let visible = manager.get_visible_creatures_in_phase(
            0,
            0,
            10.0,
            10.0,
            0.0,
            VISIBILITY_RADIUS,
            Some(&seer_phase),
        );
        let visible_guids: HashSet<ObjectGuid> = visible.iter().map(WorldCreature::guid).collect();
        assert!(visible_guids.contains(&visible_guid));
        assert!(!visible_guids.contains(&hidden_guid));

        let unfiltered = manager.get_visible_creatures(0, 0, 10.0, 10.0, 0.0);
        let unfiltered_guids: HashSet<ObjectGuid> =
            unfiltered.iter().map(WorldCreature::guid).collect();
        assert!(unfiltered_guids.contains(&visible_guid));
        assert!(unfiltered_guids.contains(&hidden_guid));
    }

    #[test]
    fn get_visible_creatures_uses_cpp_2d_sight_range() {
        let mut manager = MapManager::new();
        manager.get_or_create_map(1, 0);
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 72);
        let creature = WorldCreature::new(
            guid,
            1,
            Position::new(80.0, 0.0, 80.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0,
            0,
        );
        manager.add_creature(1, 0, 0, 0, creature);

        let visible = manager.get_visible_creatures_in_phase(1, 0, 0.0, 0.0, 0.0, 100.0, None);

        assert_eq!(
            visible.iter().map(WorldCreature::guid).collect::<Vec<_>>(),
            vec![guid],
            "C++ visibility uses horizontal distance; a vertically separated creature inside sight range must still be sent"
        );
    }

    #[test]
    fn world_creature_create_bridge_preserves_npc_flags2_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
            50,
            1,
            5,
            10,
            20.0,
            0,
            35,
            0x40,
            0,
        );
        creature
            .creature
            .set_npc_flags2_runtime_like_cpp(0x0000_0001);

        let bridged = WorldCreature::from_canonical(creature.creature, creature.create_data);

        assert_eq!(bridged.npc_flags(), 0x40);
        assert_eq!(bridged.npc_flags2(), 0x1);
        assert_eq!(bridged.npc_flags_mask_like_cpp(), 0x1_0000_0040);
        assert_eq!(bridged.create_data.npc_flags, 0x1_0000_0040);
    }

    #[test]
    fn loaded_grid_canonical_bridge_preserves_level_and_stats_like_cpp() {
        let guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 29_715, 97_932);
        let position = Position::new(5875.25, 609.063, 650.368, 1.676);
        let template = wow_entities::CreatureTemplateLifecycleRecord {
            entry: 29_715,
            original_entry: 29_715,
            difficulty_id: 0,
            name: "Quartermaster".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 2,
            unit_class: 1,
            trainer_class: 0,
            faction: 35,
            npc_flags: 0x280,
            display_id: 26_441,
            model_dimensions: Some(wow_entities::CreatureModelDimensions {
                bounding_radius: 0.389,
                combat_reach: 1.5,
            }),
            scale: 1.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
            spells: [0; wow_entities::MAX_CREATURE_SPELLS],
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: wow_constants::UnitFlags2::REGENERATE_POWER.bits(),
            unit_flags3: 0,
            flags_extra: 0,
            static_flags: [0; 8],
            creature_type: 7,
            type_flags: 0,
            movement_type: wow_entities::MovementGeneratorType::Idle,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            min_level: 75,
            max_level: 75,
            equipment_id: 0,
            original_equipment_id: 0,
        };
        let spawn = wow_entities::CreatureSpawnLifecycleRecord {
            spawn_id: 97_932,
            map_id: 571,
            instance_id: 0,
            position,
            home_position: position,
            phase_id: None,
            phase_group: None,
            terrain_swap_map: None,
            spawn_group_id: None,
            spawn_group_name: None,
            pool_id: None,
            equipment_id: Some(0),
            original_equipment_id: Some(0),
            wander_distance: 0.0,
            respawn_delay: 120,
            respawn_time: 0,
            movement_type: wow_entities::MovementGeneratorType::Idle,
            string_id: None,
            is_active: true,
            inactive_by_spawn_group: false,
            duplicate_spawn_found: false,
            add_to_map: true,
            respawn_compatibility_mode: false,
        };
        let canonical = wow_entities::Creature::load_from_db_lifecycle(
            wow_entities::CreatureLoadFromDbLifecycleRecord {
                create: wow_entities::CreatureCreateLifecycleRecord {
                    guid,
                    entry: 29_715,
                    map_id: 571,
                    instance_id: 0,
                    position,
                    dynamic: false,
                    vehicle_id: None,
                    vehicle_kit_create_input: None,
                    add_to_world_vehicle_reset_context: None,
                    template,
                    spawn: Some(spawn.clone()),
                    selected_level: 75,
                    stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                    selected_display_id: 26_441,
                    selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                        bounding_radius: 0.389,
                        combat_reach: 1.5,
                    }),
                    selected_equipment_id: 0,
                    selected_original_equipment_id: 0,
                    selected_virtual_items: [(0, 0, 0); 3],
                    corpse_delay: 60,
                    ignore_corpse_decay_ratio: false,
                    addon: None,
                },
                spawn,
            },
        );

        let bridged = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

        assert_eq!(bridged.level(), 75);
        assert_eq!(bridged.current_hp(), 4_652);
        assert_eq!(bridged.max_hp(), 4_652);
        assert_eq!(bridged.create_data.level, 75);
        assert_eq!(bridged.create_data.health, 4_652);
        assert_eq!(bridged.create_data.max_health, 4_652);
        assert_eq!(bridged.create_data.display_id, 26_441);
        assert_eq!(bridged.create_data.npc_flags, 0x280);
        assert_eq!(
            bridged.create_data.unit_flags2,
            wow_constants::UnitFlags2::REGENERATE_POWER.bits()
        );
        assert_eq!(bridged.create_data.speed_walk_rate, 1.0);
        assert_eq!(bridged.create_data.speed_run_rate, 1.14286);
    }

    #[test]
    fn loaded_grid_canonical_bridge_only_sets_vehicle_create_flag_for_real_vehicle_kit_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Vehicle, 0, 1, 571, 0, 29_715, 97_933);
        let position = Position::new(5875.25, 609.063, 650.368, 1.676);
        let template = wow_entities::CreatureTemplateLifecycleRecord {
            entry: 29_715,
            original_entry: 29_715,
            difficulty_id: 0,
            name: "Vehicle-shaped creature".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 2,
            unit_class: 1,
            trainer_class: 0,
            faction: 35,
            npc_flags: 0,
            display_id: 26_441,
            model_dimensions: Some(wow_entities::CreatureModelDimensions {
                bounding_radius: 0.389,
                combat_reach: 1.5,
            }),
            scale: 1.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
            spells: [0; wow_entities::MAX_CREATURE_SPELLS],
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            flags_extra: 0,
            static_flags: [0; 8],
            creature_type: 7,
            type_flags: 0,
            movement_type: wow_entities::MovementGeneratorType::Idle,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: wow_constants::CreatureFlightMovementType::None as u8,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            min_level: 75,
            max_level: 75,
            equipment_id: 0,
            original_equipment_id: 0,
        };
        let spawn = wow_entities::CreatureSpawnLifecycleRecord {
            spawn_id: 97_933,
            map_id: 571,
            instance_id: 0,
            position,
            home_position: position,
            phase_id: None,
            phase_group: None,
            terrain_swap_map: None,
            spawn_group_id: None,
            spawn_group_name: None,
            pool_id: None,
            equipment_id: Some(0),
            original_equipment_id: Some(0),
            wander_distance: 0.0,
            respawn_delay: 120,
            respawn_time: 0,
            movement_type: wow_entities::MovementGeneratorType::Idle,
            string_id: None,
            is_active: true,
            inactive_by_spawn_group: false,
            duplicate_spawn_found: false,
            add_to_map: true,
            respawn_compatibility_mode: false,
        };
        let canonical = wow_entities::Creature::load_from_db_lifecycle(
            wow_entities::CreatureLoadFromDbLifecycleRecord {
                create: wow_entities::CreatureCreateLifecycleRecord {
                    guid,
                    entry: 29_715,
                    map_id: 571,
                    instance_id: 0,
                    position,
                    dynamic: false,
                    vehicle_id: Some(909),
                    vehicle_kit_create_input: None,
                    add_to_world_vehicle_reset_context: None,
                    template,
                    spawn: Some(spawn.clone()),
                    selected_level: 75,
                    stats: wow_entities::CreatureLifecycleStats::new(4_652, 4_652, 0, 0),
                    selected_display_id: 26_441,
                    selected_model_dimensions: Some(wow_entities::CreatureModelDimensions {
                        bounding_radius: 0.389,
                        combat_reach: 1.5,
                    }),
                    selected_equipment_id: 0,
                    selected_original_equipment_id: 0,
                    selected_virtual_items: [(0, 0, 0); 3],
                    corpse_delay: 60,
                    ignore_corpse_decay_ratio: false,
                    addon: None,
                },
                spawn,
            },
        );

        assert_eq!(canonical.lifecycle_metadata().vehicle_id, Some(909));
        assert!(canonical.unit().subsystems().vehicle.kit.is_none());

        let bridged = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

        assert_eq!(bridged.create_data.vehicle_id, 0);
    }

    #[test]
    fn set_creature_anim_kit_id_like_cpp_mutates_state_create_data_and_returns_fanout() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 103);
        let mut manager = MapManager::new();
        manager.add_creature(
            571,
            0,
            0,
            0,
            WorldCreature::new(
                guid,
                1,
                Position::new(10.0, 20.0, 30.0, 0.0),
                50,
                1,
                5,
                10,
                20.0,
                0,
                35,
                0,
                0,
            ),
        );

        let event = manager
            .set_creature_anim_kit_id_like_cpp(
                571,
                0,
                guid,
                CreatureAnimKitSlotLikeCpp::Ai,
                77,
                |id| id == 77,
            )
            .expect("valid changed anim kit emits fanout event");

        let creature = manager.find_creature(571, 0, guid).expect("creature");
        assert_eq!(creature.creature.unit().ai_anim_kit_id_like_cpp(), 77);
        assert_eq!(
            creature.create_data.ai_anim_kit_id, 77,
            "late CREATE viewers must see the mutated anim kit state"
        );
        assert_eq!(event.source_guid, guid);
        match event.recipients {
            RecipientRule::NearbyVisible {
                source_guid,
                map_id,
                instance_id,
                source_position,
                range,
                required_3d,
            } => {
                assert_eq!(source_guid, guid);
                assert_eq!(map_id, 571);
                assert_eq!(instance_id, 0);
                assert_eq!(source_position, Position::new(10.0, 20.0, 30.0, 0.0));
                assert_eq!(range, VISIBILITY_RADIUS);
                assert!(!required_3d);
            }
            other => panic!("expected NearbyVisible, got {other:?}"),
        }
        let opcode = u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]]);
        assert_eq!(
            opcode,
            wow_constants::ServerOpcodes::SetAiAnimKit as u16,
            "C++ Unit::SetAIAnimKitId sends SMSG_SET_AI_ANIM_KIT after mutation"
        );
    }

    #[test]
    fn set_creature_anim_kit_id_like_cpp_rejects_same_and_invalid_nonzero_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 104);
        let mut manager = MapManager::new();
        manager.add_creature(
            571,
            0,
            0,
            0,
            WorldCreature::new(
                guid,
                1,
                Position::new(10.0, 20.0, 30.0, 0.0),
                50,
                1,
                5,
                10,
                20.0,
                0,
                35,
                0,
                0,
            ),
        );

        assert!(
            manager
                .set_creature_anim_kit_id_like_cpp(
                    571,
                    0,
                    guid,
                    CreatureAnimKitSlotLikeCpp::Movement,
                    88,
                    |_| false,
                )
                .is_none(),
            "C++ Unit::SetMovementAnimKitId rejects nonzero IDs missing from sAnimKitStore"
        );
        assert_eq!(
            manager
                .find_creature(571, 0, guid)
                .unwrap()
                .creature
                .unit()
                .movement_anim_kit_id_like_cpp(),
            0
        );

        assert!(
            manager
                .set_creature_anim_kit_id_like_cpp(
                    571,
                    0,
                    guid,
                    CreatureAnimKitSlotLikeCpp::Melee,
                    0,
                    |_| false,
                )
                .is_none(),
            "same ID must not emit the C++ live packet"
        );
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rustycore-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    // ── Slice 4A.1a tests ────────────────────────────────────────────────────

    /// `into_owning_session_plan` must produce one `SelfOnly` `RuntimeEvent`
    /// per packet, in the same order, with `source_guid` set on every event.
    #[test]
    fn into_owning_session_plan_preserves_packets_as_self_only() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);

        let pkt_a = vec![0x01, 0x02, 0x03];
        let pkt_b = vec![0xAA, 0xBB];
        let pkt_c = vec![0xFF];

        let mut output = RuntimeOutput::new();
        output.packets.push(pkt_a.clone());
        output.packets.push(pkt_b.clone());
        output.packets.push(pkt_c.clone());

        let plan = output.into_owning_session_plan(guid);

        assert_eq!(plan.events.len(), 3, "must produce one event per packet");

        for (i, event) in plan.events.iter().enumerate() {
            assert_eq!(
                event.source_guid, guid,
                "event[{i}] must carry the source guid"
            );
            assert_eq!(
                event.recipients,
                RecipientRule::SelfOnly,
                "event[{i}] must be SelfOnly"
            );
        }

        // Packet bytes preserved in order.
        assert_eq!(plan.events[0].packet_bytes, pkt_a);
        assert_eq!(plan.events[1].packet_bytes, pkt_b);
        assert_eq!(plan.events[2].packet_bytes, pkt_c);
    }

    /// Empty `RuntimeOutput` produces an empty `RuntimePlan`.
    #[test]
    fn into_owning_session_plan_empty_output_gives_empty_plan() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 1);
        let plan = RuntimeOutput::new().into_owning_session_plan(guid);
        assert!(plan.events.is_empty());
    }

    /// Smoke: `RecipientRule::NearbyVisible` stores all its fields correctly.
    #[test]
    fn recipient_rule_nearby_visible_stores_fields() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 7);
        let pos = Position::new(1.0, 2.0, 3.0, 0.5);

        let rule = RecipientRule::NearbyVisible {
            source_guid: guid,
            map_id: 571,
            instance_id: 0,
            source_position: pos,
            range: 100.0,
            required_3d: true,
        };

        if let RecipientRule::NearbyVisible {
            source_guid,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } = rule
        {
            assert_eq!(source_guid, guid);
            assert_eq!(map_id, 571);
            assert_eq!(instance_id, 0);
            assert_eq!(source_position.x, 1.0);
            assert_eq!(source_position.y, 2.0);
            assert_eq!(source_position.z, 3.0);
            assert!((range - 100.0).abs() < f32::EPSILON);
            assert!(required_3d);
        } else {
            panic!("expected NearbyVisible");
        }
    }

    /// Smoke: `RecipientRule::MapBroadcastVisible` stores map_id and instance_id.
    #[test]
    fn recipient_rule_map_broadcast_visible_stores_fields() {
        let rule = RecipientRule::MapBroadcastVisible {
            map_id: 0,
            instance_id: 5,
        };

        if let RecipientRule::MapBroadcastVisible {
            map_id,
            instance_id,
        } = rule
        {
            assert_eq!(map_id, 0);
            assert_eq!(instance_id, 5);
        } else {
            panic!("expected MapBroadcastVisible");
        }
    }

    /// `active_map_keys` returns the exact `(map_id, instance_id)` pairs of
    /// the maps that have been created in the manager.
    #[test]
    fn active_map_keys_returns_inserted_map_keys() {
        let mut manager = MapManager::new();

        // No maps yet.
        assert!(manager.active_map_keys().is_empty());

        // Insert two distinct maps.
        manager.get_or_create_map(0, 0);
        manager.get_or_create_map(571, 1);

        let mut keys = manager.active_map_keys();
        keys.sort_unstable(); // deterministic order for assertions

        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], (0, 0));
        assert_eq!(keys[1], (571, 1));
    }

    // ── Slice 4A.2a: respawn queue tests ──────────────────────────────────────

    fn make_pending_respawn(respawn_at: Instant) -> PendingRespawn {
        use wow_packet::packets::update::CreatureCreateData;
        static NEXT_TEST_SPAWN_ID: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let spawn_id = NEXT_TEST_SPAWN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Creature,
            0,
            1,
            0,
            0,
            1,
            spawn_id as i64,
        );
        PendingRespawn {
            respawn_at,
            spawn_id,
            home_pos: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                orientation: 0.0,
            },
            create_data: CreatureCreateData {
                guid,
                entry: 1,
                display_id: 1,
                native_display_id: 1,
                display_scale: 1.0,
                native_x_display_scale: 1.0,
                bounding_radius: 0.389,
                combat_reach: 1.5,
                health: 100,
                max_health: 100,
                level: 1,
                faction_template: 1,
                npc_flags: 0,
                unit_flags: 0,
                unit_flags2: 0,
                unit_flags3: 0,
                aura_state: WorldCreature::health_aura_state_like_cpp(100, 100, true),
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
            },
            max_hp: 100,
            level: 1,
            min_dmg: 1,
            max_dmg: 5,
            aggro_radius: 10.0,
            wander_distance: 0.0,
            flags_extra: 0,
            static_flags: [0; 8],
            ai_name: String::new(),
            script_name: String::new(),
            string_id: None,
            addon: None,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            default_movement_type: MovementGeneratorType::Idle,
            waypoint_path_id: 0,
            npc_flags: 0,
            unit_flags: 0,
            map_id: 0,
            loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            respawn_delay_secs: 30,
            selected_equipment_id: 0,
            original_equipment_id: 0,
            boss_id: None,
            dungeon_encounter_id: 0,
            phase_use_flags: 0,
            phase_id: 0,
            phase_group_id: 0,
            terrain_swap_map: -1,
            phase_shift: PhaseShift::default(),
        }
    }

    /// A newly created `MapInstance` starts with an empty respawn queue.
    #[test]
    fn respawn_queue_starts_empty_like_cpp() {
        let map = MapInstance::new(0, 0);
        assert_eq!(map.respawn_queue_len(), 0);
    }

    /// Pushing one entry increments the length to 1.
    #[test]
    fn push_respawn_increments_len_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let now = Instant::now();
        map.push_respawn(make_pending_respawn(now));
        assert_eq!(map.respawn_queue_len(), 1);
    }

    #[test]
    fn push_respawn_replaces_later_duplicate_spawn_id_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let now = Instant::now();
        let later = now + Duration::from_secs(60);
        let earlier = now + Duration::from_secs(10);

        let mut first = make_pending_respawn(later);
        first.spawn_id = 42;
        let mut replacement = make_pending_respawn(earlier);
        replacement.spawn_id = 42;

        map.push_respawn(first);
        map.push_respawn(replacement);

        assert_eq!(map.respawn_queue_len(), 1);
        let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].spawn_id, 42);
    }

    #[test]
    fn push_respawn_ignores_later_duplicate_spawn_id_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let now = Instant::now();
        let earlier = now + Duration::from_secs(10);
        let later = now + Duration::from_secs(60);

        let mut first = make_pending_respawn(earlier);
        first.spawn_id = 77;
        let mut duplicate = make_pending_respawn(later);
        duplicate.spawn_id = 77;

        map.push_respawn(first);
        map.push_respawn(duplicate);

        assert_eq!(map.respawn_queue_len(), 1);
        let ready = map.drain_ready_respawns(now + Duration::from_secs(11));
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].spawn_id, 77);
    }

    #[test]
    fn pending_respawn_rebuild_preserves_zero_wander_distance_like_cpp() {
        let mut pending = make_pending_respawn(Instant::now());
        pending.respawn_delay_secs = 45;
        pending.selected_equipment_id = 6;
        pending.original_equipment_id = -1;
        pending.string_id = Some("respawn-string".to_string());

        let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

        assert_eq!(
            creature.creature.ai_ownership().wander_radius,
            0.0,
            "C++ respawn uses CreatureData::wander_distance; idle spawns must not regain an invented wander radius"
        );
        assert_eq!(
            creature.creature.ai_ownership().respawn_time_secs,
            45,
            "C++ Creature::LoadFromDB copies CreatureData::spawntimesecs into m_respawnDelay"
        );
        assert_eq!(
            creature.creature.equipment_id(),
            6,
            "C++ LoadEquipment mutates m_equipmentId to the selected equipment template"
        );
        assert_eq!(
            creature.creature.original_equipment_id(),
            -1,
            "C++ InitEntry keeps CreatureData::equipmentId in m_originalEquipmentId before random equipment selection mutates the selected id"
        );
        assert_eq!(
            creature.creature.lifecycle_metadata().string_id.as_deref(),
            Some("respawn-string"),
            "C++ respawn reloads CreatureData::StringId through Creature::LoadFromDB"
        );
        assert!(!creature.should_wander());
    }

    #[test]
    fn pending_respawn_rebuild_preserves_random_movement_type_like_cpp() {
        let mut pending = make_pending_respawn(Instant::now());
        pending.random_movement_type = wow_constants::CreatureRandomMovementType::AlwaysRun as u8;

        let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

        assert_eq!(
            creature.creature.random_movement_type_like_cpp(),
            wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
            "C++ respawn keeps using Creature::GetMovementTemplate(); Rust respawn must preserve the captured Random movement metadata"
        );
    }

    #[test]
    fn pending_respawn_rebuild_preserves_default_movement_and_path_like_cpp() {
        let mut pending = make_pending_respawn(Instant::now());
        pending.default_movement_type = MovementGeneratorType::Waypoint;
        pending.waypoint_path_id = 9_002;

        let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

        assert_eq!(
            creature.creature.default_movement_type(),
            MovementGeneratorType::Waypoint,
            "C++ respawn reload path uses Creature::LoadFromDB/LoadCreaturesAddon and keeps the selected default motion"
        );
        assert_eq!(
            creature.creature.waypoint_path_id_like_cpp(),
            9_002,
            "C++ Creature::LoadCreaturesAddon preserves nonzero PathId for waypoint movement after respawn"
        );
    }

    /// `drain_ready_respawns` returns only entries whose `respawn_at <= now`.
    #[test]
    fn drain_returns_only_ready_entries_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let now = Instant::now();
        let past = now - Duration::from_secs(5);
        let future = now + Duration::from_secs(60);

        map.push_respawn(make_pending_respawn(past));
        map.push_respawn(make_pending_respawn(future));

        let ready = map.drain_ready_respawns(now);
        assert_eq!(ready.len(), 1);
        assert_eq!(map.respawn_queue_len(), 1);
    }

    /// Entries that are not yet ready remain in the queue after drain.
    #[test]
    fn future_entries_remain_after_drain_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let future = Instant::now() + Duration::from_secs(60);

        map.push_respawn(make_pending_respawn(future));

        let ready = map.drain_ready_respawns(Instant::now());
        assert_eq!(ready.len(), 0);
        assert_eq!(map.respawn_queue_len(), 1);
    }

    /// Ready entries are returned in insertion order.
    #[test]
    fn drain_preserves_insertion_order_like_cpp() {
        let mut map = MapInstance::new(0, 0);
        let t0 = Instant::now() - Duration::from_secs(10);
        let t1 = Instant::now() - Duration::from_secs(5);
        let t2 = Instant::now() - Duration::from_secs(1);

        // Insert in REVERSE temporal order (t2, t1, t0) — all in the past, all ready.
        // drain must return them in INSERTION order, not sorted by respawn_at, mirroring
        // the original Vec partition in run_creatures_tick (session.rs:20189-20201).
        map.push_respawn(make_pending_respawn(t2));
        map.push_respawn(make_pending_respawn(t1));
        map.push_respawn(make_pending_respawn(t0));

        let now = Instant::now();
        let ready = map.drain_ready_respawns(now);

        assert_eq!(ready.len(), 3);
        // Insertion order (t2, t1, t0), distinct from temporal order (t0, t1, t2).
        assert_eq!(ready[0].respawn_at, t2);
        assert_eq!(ready[1].respawn_at, t1);
        assert_eq!(ready[2].respawn_at, t0);
    }

    /// Queues are independent per (map_id, instance_id).
    /// Pushing to (0, 0) must not affect (571, 1).
    #[test]
    fn respawn_queues_are_isolated_by_map_and_instance_like_cpp() {
        let mut manager = MapManager::new();
        let now = Instant::now();
        let past = now - Duration::from_secs(1);

        manager.push_respawn(0, 0, make_pending_respawn(past));

        assert_eq!(manager.respawn_queue_len(0, 0), 1);
        assert_eq!(manager.respawn_queue_len(571, 1), 0);

        let ready_571 = manager.drain_ready_respawns(571, 1, now);
        assert_eq!(ready_571.len(), 0);

        let ready_0 = manager.drain_ready_respawns(0, 0, now);
        assert_eq!(ready_0.len(), 1);
    }

    /// Unique temp `maps/` dir holding one synthetic constant-height tile.
    fn temp_dir_with_constant_tile(map_id: u32, gx: i32, gy: i32, height: f32) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("rustycore_live_terrain_{}_{n}", std::process::id()));
        std::fs::create_dir_all(dir.join("maps")).expect("create temp maps dir");

        // Minimal float `.map`: fileheader(44) + MHGT header(16) + V9 + V8, all = height.
        const V9: usize = 129 * 129;
        const V8: usize = 128 * 128;
        let mut b = Vec::new();
        b.extend_from_slice(b"MAPS");
        b.extend_from_slice(&10u32.to_le_bytes()); // version
        b.extend_from_slice(&0u32.to_le_bytes()); // build
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapOffset
        b.extend_from_slice(&0u32.to_le_bytes()); // areaMapSize
        b.extend_from_slice(&44u32.to_le_bytes()); // heightMapOffset
        for _ in 0..5 {
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        b.extend_from_slice(b"MHGT");
        b.extend_from_slice(&0u32.to_le_bytes()); // flags = float
        b.extend_from_slice(&height.to_le_bytes()); // gridHeight
        b.extend_from_slice(&height.to_le_bytes()); // gridMaxHeight
        for _ in 0..(V9 + V8) {
            b.extend_from_slice(&height.to_le_bytes());
        }
        std::fs::write(
            dir.join("maps")
                .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
            &b,
        )
        .expect("write tile");
        dir
    }

    #[test]
    fn respawn_ground_snap_uses_real_terrain_like_cpp() {
        // World (0,0) → raw tile (32,32). Ground at 77.0; spawn hovering above it.
        let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
        let terrain = LiveTerrainHeights::new(&dir);

        let mut pending = make_pending_respawn(Instant::now());
        pending.home_pos.z = 80.0; // above ground; probe accepts the surface
        let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
        assert!((creature.creature.unit().world().position().z - 80.0).abs() < 1e-3);

        snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

        // Grounded, non-hovering: snapped exactly onto the surface (+0 hover).
        assert!(
            (creature.creature.unit().world().position().z - 77.0).abs() < 1e-2,
            "respawn must sit on the .map ground like Creature::Respawn/UpdateAllowedPositionZ"
        );
        // C++ SetHomePosition takes the snapped Z too.
        assert!((creature.home_position().z - 77.0).abs() < 1e-2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn respawn_ground_snap_noop_without_terrain_tile_like_cpp() {
        // Empty maps dir → no tile → GetGridHeight invalid → Z untouched.
        let dir = temp_dir_with_constant_tile(0, 10, 10, 5.0); // tile for a different grid
        let terrain = LiveTerrainHeights::new(&dir);

        let mut pending = make_pending_respawn(Instant::now());
        pending.home_pos.z = 80.0;
        let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
        snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

        assert!(
            (creature.creature.unit().world().position().z - 80.0).abs() < 1e-3,
            "no terrain under the spawn → C++ leaves Z unchanged"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn respawn_ground_snap_skips_creature_far_below_surface_like_cpp() {
        // Spawn well below ground: probe z < gridHeight - tolerance → GetStaticHeight
        // returns invalid, so C++ does NOT rescue a buried creature.
        let dir = temp_dir_with_constant_tile(0, 32, 32, 77.0);
        let terrain = LiveTerrainHeights::new(&dir);

        let mut pending = make_pending_respawn(Instant::now());
        pending.home_pos.z = 10.0; // far under the 77.0 surface
        let mut creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);
        snap_respawn_creature_to_ground_like_cpp(&mut creature, 0, &terrain);

        assert!((creature.creature.unit().world().position().z - 10.0).abs() < 1e-3);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pending_respawn_preserves_flags_extra_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);
        let mut creature = test_creature(guid);
        creature.creature.set_spawn_id(42);
        creature
            .creature
            .set_flags_extra_runtime_like_cpp(CreatureFlagsExtra::CIVILIAN.bits());
        let mut static_flags = [0; 8];
        static_flags[0] = wow_constants::creature::CreatureStaticFlags::NO_MELEE_FLEE.bits();
        creature
            .creature
            .set_static_flags_runtime_like_cpp(static_flags);
        creature
            .creature
            .set_ai_identity_names_runtime_like_cpp("SmartAI", "npc_respawn_identity");
        creature
            .creature
            .set_spawn_string_id_runtime_like_cpp(Some("respawn-string".to_string()));
        creature.creature.set_flight_movement_type_runtime_like_cpp(
            wow_constants::CreatureFlightMovementType::CanFly as u8,
        );
        creature.creature.set_ground_movement_type_runtime_like_cpp(
            wow_constants::CreatureGroundMovementType::None as u8,
        );
        creature.creature.set_swim_allowed_runtime_like_cpp(false);

        let mut pending =
            pending_respawn_from_world_creature_like_cpp(&creature, Instant::now(), 0);
        pending.create_data.hover_height = 1.5;
        pending.ground_movement_type = wow_constants::CreatureGroundMovementType::Hover as u8;
        pending.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 88_001,
            visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
            auras: vec![70_020],
            ..CreatureAddonLifecycleRecordLikeCpp::default()
        });
        assert_eq!(
            pending.spawn_id, 42,
            "creature respawn must preserve C++ RespawnInfo::spawnId from Creature::GetSpawnId"
        );
        assert_eq!(pending.flags_extra, CreatureFlagsExtra::CIVILIAN.bits());
        assert_eq!(pending.static_flags[0], static_flags[0]);
        assert_eq!(pending.ai_name, "SmartAI");
        assert_eq!(pending.script_name, "npc_respawn_identity");
        assert_eq!(pending.string_id.as_deref(), Some("respawn-string"));
        assert_eq!(
            pending.ground_movement_type,
            wow_constants::CreatureGroundMovementType::Hover as u8
        );
        assert!(!pending.swim_allowed);
        assert_eq!(
            pending.flight_movement_type,
            wow_constants::CreatureFlightMovementType::CanFly as u8
        );

        let respawned = world_creature_from_pending_respawn_like_cpp(&pending, 0);
        assert!(
            respawned.creature.is_civilian_like_cpp(),
            "map-owned respawn must keep C++ flags_extra gates"
        );
        assert_eq!(respawned.creature.lifecycle_metadata().ai_name, "SmartAI");
        assert_eq!(
            respawned.creature.lifecycle_metadata().script_name,
            "npc_respawn_identity"
        );
        assert_eq!(
            respawned.creature.lifecycle_metadata().string_id.as_deref(),
            Some("respawn-string")
        );
        assert!(respawned.creature.can_walk_like_cpp());
        assert!(!respawned.creature.can_enter_water_like_cpp());
        assert!(respawned.creature.can_fly_like_cpp());
        assert_eq!(
            respawned.position().z,
            1.5,
            "C++ Creature::Create adds GetHoverOffset() to Z when respawn reloads a hovering creature"
        );
        assert_eq!(respawned.creature.unit().data().hover_height, 1.5);
        assert_eq!(
            respawned.creature.waypoint_path_id_like_cpp(),
            88_001,
            "C++ respawn goes back through Creature::LoadFromDB and reapplies LoadCreaturesAddon PathId"
        );
        assert!(
            respawned
                .creature
                .unit()
                .unit_flags2_like_cpp()
                .contains(wow_constants::unit::UnitFlags2::LARGE_AOI),
            "C++ LoadCreaturesAddon reapplies addon visibility/AOI flags on respawn"
        );
        assert!(
            respawned
                .creature
                .unit()
                .subsystems()
                .auras
                .has_aura_spell_like_cpp(70_020),
            "C++ LoadCreaturesAddon reapplies addon auras on respawn"
        );
    }
}
