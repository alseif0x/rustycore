use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};
use wow_constants::movement::MovementFlag;
use wow_constants::{UnitStandStateType, UnitState, WeaponAttackType};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    Creature, CreatureAiState, DistractMovementAction, EVENT_CHARGE_PREPATH, GenericMovementInform,
    MovementGeneratorKind, PhaseShift, PointMovementAction, PointMovementInform,
    RotateMovementUpdate,
};
use wow_movement::{
    MoveSpline, MoveSplineInit, MoveSplineLaunchInput, MoveSplineStopInput, MoveSplineStopResult,
    PathGenerator, PathType, WaypointLaunchPlan, WaypointMovementAction, WaypointMovementGenerator,
    WaypointPath, WaypointUnitSnapshot,
};
use wow_packet::packets::update::CreatureCreateData;
use wow_recastdetour::{
    CENTER_GRID_ID_LIKE_CPP, DetourNavMeshQueryError, DetourPathOptions, DetourPathType,
    DetourPolyPath, DetourQueryFilterError, MAX_NUMBER_OF_GRIDS_LIKE_CPP, MMapData,
    MMapManager as DetourMMapManager, MMapManagerError, PathQueryFilterContext,
    SIZE_OF_GRIDS_LIKE_CPP, ThreadUnsafeMapData, create_path_query_filter_like_cpp,
};

use crate::phasing::personal::MultiPersonalPhaseTracker;

/// Size of a grid cell in yards (64x64 yards like TrinityCore).
pub const GRID_SIZE: f32 = 64.0;

/// Visibility radius in yards (how far a player can see).
pub const VISIBILITY_RADIUS: f32 = 100.0;

/// Default time before a grid unloads if no players are nearby (5 minutes).
pub const DEFAULT_GRID_UNLOAD_TIME: Duration = Duration::from_secs(300);

/// TrinityCore `TerrainInfo::GetMinHeight` fallback when no terrain grid is loaded.
///
/// Real grid-backed min-height data belongs to the terrain/map-data port; exposing
/// the fallback here lets movement preserve the C++ under-map branch without
/// inventing terrain values.
pub const DEFAULT_MIN_HEIGHT_LIKE_CPP: f32 = -500.0;

const MAP_MAGIC_LIKE_CPP: &[u8; 4] = b"MAPS";
const MAP_VERSION_MAGIC_LIKE_CPP: u32 = 10;
const MAP_FILE_HEADER_SIZE_LIKE_CPP: usize = 44;
const TERRAIN_GRID_COUNT_LIKE_CPP: usize =
    MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize * MAX_NUMBER_OF_GRIDS_LIKE_CPP as usize;

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

pub fn path_generator_from_detour_like_cpp(
    start: Position,
    destination: Position,
    detour_path: &DetourPolyPath,
    force_destination: bool,
) -> PathGenerator {
    let mut path = PathGenerator::new();
    path.apply_detour_path_like_cpp(
        start,
        destination,
        position_from_detour_point_like_cpp(detour_path.point_path.actual_end),
        detour_path
            .point_path
            .points
            .iter()
            .copied()
            .map(position_from_detour_point_like_cpp),
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
    active_waypoint_generator: Option<WaypointMovementGenerator>,
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
        let _ = creature.unit_mut().world_mut().set_map(0, 0);
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
            ai.wander_radius = 5.0;
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
            health: hp as i64,
            max_health: hp as i64,
            level,
            faction_template: faction as i32,
            npc_flags: npc_flags as u64,
            unit_flags,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 0,
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
            active_waypoint_generator: None,
            clock_started_at: Instant::now(),
        }
    }

    pub fn active_waypoint_generator_like_cpp(&self) -> Option<&WaypointMovementGenerator> {
        self.active_waypoint_generator.as_ref()
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

    pub fn spline_id(&self) -> u32 {
        self.creature.ai_ownership().spline_id
    }

    pub fn corpse_despawn_at(&self) -> Option<Instant> {
        self.creature
            .ai_ownership()
            .corpse_despawn_at_ms
            .map(|ms| self.clock_started_at + Duration::from_millis(ms))
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
        self.active_move_spline = Some(spline.clone());
        Some((launch.real_position, spline))
    }

    pub fn begin_move_spline_like_cpp(&mut self, dst: Position) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        init.set_velocity(2.5);
        init.move_to(dst);

        self.launch_move_spline_init_like_cpp(&mut init, dst)
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
        }
        self.active_waypoint_generator = Some(generator);
        action
    }

    pub fn update_default_waypoint_movement_like_cpp(
        &mut self,
        diff_ms: u32,
    ) -> WaypointMovementAction {
        let snapshot = self.waypoint_unit_snapshot_like_cpp();
        let Some(generator) = self.active_waypoint_generator.as_mut() else {
            return WaypointMovementAction::Continue;
        };
        let action = generator.update_like_cpp(true, diff_ms, snapshot, None);
        self.apply_waypoint_movement_action_like_cpp(action);
        if matches!(
            action,
            WaypointMovementAction::Arrived(arrived) if arrived.timer_ms.is_none()
        ) {
            let snapshot = self.waypoint_unit_snapshot_like_cpp();
            if let Some(generator) = self.active_waypoint_generator.as_mut() {
                let chained = generator.update_like_cpp(true, 0, snapshot, None);
                if chained != WaypointMovementAction::Continue {
                    self.apply_waypoint_movement_action_like_cpp(chained);
                    return chained;
                }
            }
        }
        action
    }

    fn apply_waypoint_movement_action_like_cpp(&mut self, action: WaypointMovementAction) {
        match action {
            WaypointMovementAction::StopMoving => {
                self.creature
                    .unit_mut()
                    .subsystems_mut()
                    .motion
                    .stop_moving();
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
                let _ = ended;
            }
            WaypointMovementAction::Launch(launch) => {
                self.begin_waypoint_launch_like_cpp(launch);
            }
            _ => {}
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

    fn begin_waypoint_launch_like_cpp(
        &mut self,
        launch: WaypointLaunchPlan,
    ) -> Option<(Position, MoveSpline)> {
        let spline_id = self.spline_id().saturating_add(1);
        let mut init = MoveSplineInit::new(spline_id);
        if launch.disable_transport_transform {
            init.disable_transport_path_transformations();
        }
        init.move_to(launch.destination);
        if let Some(facing) = launch.facing {
            init.set_facing_angle(facing);
        }
        if let Some(walk) = launch.walk {
            init.set_walk(walk);
        }
        if let Some(velocity) = launch.velocity {
            init.set_velocity(velocity);
        }
        self.creature
            .unit_mut()
            .add_unit_state(launch.add_unit_state);
        self.launch_move_spline_init_like_cpp(&mut init, launch.destination)
    }

    pub fn begin_move_spline_with_detour_path_like_cpp(
        &mut self,
        dst: Position,
        detour_path: Option<&DetourPolyPath>,
        force_destination: bool,
    ) -> Option<(Position, MoveSpline, Option<PathGenerator>)> {
        let Some(detour_path) = detour_path else {
            return self
                .begin_move_spline_like_cpp(dst)
                .map(|(from, spline)| (from, spline, None));
        };

        let path = path_generator_from_detour_like_cpp(
            self.position(),
            dst,
            detour_path,
            force_destination,
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
            if let Some(pos) = spline.compute_position() {
                self.creature.set_ai_position(pos);
            }
            let progress_ms = spline.time_passed_ms().max(0) as u32;
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .set_spline_progress(progress_ms);
        }

        let finalized = spline.finalized();
        if finalized {
            self.creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .finalize_spline();
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

    pub fn roll_damage(&self) -> u32 {
        let min_dmg = self.min_dmg();
        let max_dmg = self.max_dmg();
        if min_dmg >= max_dmg {
            return min_dmg;
        }
        let range = max_dmg - min_dmg;
        let seed = (self.now_ms() as u32).wrapping_add(self.spline_id());
        min_dmg + (seed % (range + 1))
    }

    pub fn should_wander(&self) -> bool {
        self.is_alive()
            && self.state() == CreatureAiState::Idle
            && self.can_wander()
            && self
                .now_ms()
                .saturating_sub(self.creature.ai_ownership().move_start_ms)
                >= self.creature.ai_ownership().wander_delay_ms
    }

    pub fn pick_wander_destination(&mut self) -> Position {
        let seed = self.now_ms() as f32;
        let angle = (seed * 0.001) % (2.0 * std::f32::consts::PI);
        let radius = self.creature.ai_ownership().wander_radius.max(1.0);
        let dist = (seed * 0.0001) % radius + 1.0;
        let home = self.home_position();
        let x = home.x + angle.cos() * dist;
        let y = home.y + angle.sin() * dist;
        let o = angle + std::f32::consts::PI;
        Position::new(x, y, home.z, o)
    }

    pub fn reset_wander_timer(&mut self) {
        let now_ms = self.now_ms();
        let ai = self.creature.ai_ownership_mut();
        ai.move_start_ms = now_ms;
        ai.wander_delay_ms = 5_000 + (now_ms % 10_000);
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
/// Default is `Session`: each logged-in session drives its own creature and
/// combat ticks — this is the legacy behaviour and the safe starting point.
/// `GlobalLegacy` signals that a global clock will drive the ticks instead,
/// so session-level tick calls must be skipped to avoid double resolution.
///
/// The owner lives on the shared [`MapManager`] so all sessions on the same
/// map read the same value.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RuntimeTickOwner {
    /// Per-session tick (legacy behaviour, current default).
    #[default]
    Session,
    /// Global legacy-manager tick (activated in Slice 4+).
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

/// Global map manager containing all map instances.
#[derive(Debug)]
pub struct MapManager {
    maps: HashMap<(u16, u32), MapInstance>, // (map_id, instance_id) -> MapInstance
    free_instance_ids: Vec<bool>,
    next_instance_id: u32,
    tick_owner: RuntimeTickOwner,
}

impl MapManager {
    pub fn new() -> Self {
        let mut manager = Self {
            maps: HashMap::new(),
            free_instance_ids: Vec::new(),
            next_instance_id: 1,
            tick_owner: RuntimeTickOwner::Session,
        };
        manager.init_instance_ids_from_max(0);
        manager
    }

    /// Returns the current tick owner for this map manager.
    ///
    /// Returns a `Copy` value; the caller should read this once and release the
    /// lock before performing any tick work.
    pub fn tick_owner(&self) -> RuntimeTickOwner {
        self.tick_owner
    }

    /// Sets the tick owner. Not called in Slice 3 — default remains `Session`.
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
        creature: WorldCreature,
    ) -> bool {
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
        self.get_visible_creatures_in_phase(map_id, instance_id, x, y, _z, None)
    }

    pub fn get_visible_creatures_in_phase(
        &self,
        map_id: u16,
        instance_id: u32,
        x: f32,
        y: f32,
        z: f32,
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

                        // Optional: Check actual distance for precise visibility
                        let dist =
                            Position::distance(&Position::new(x, y, z, 0.0), &creature.position());
                        if dist <= VISIBILITY_RADIUS {
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
/// C++ ref: `Map::_respawnTimes` / `Map::ProcessRespawns` (Map.h:748-750, Map.cpp:2191).
/// C# ref: `Creature::AllLootRemovedFromCorpse` → `m_respawnTime` → `Map::AddToMap`.
#[derive(Debug)]
pub struct PendingRespawn {
    /// When to respawn.
    pub respawn_at: Instant,
    /// Home position (spawn point).
    pub home_pos: wow_core::Position,
    /// Full create data — reused verbatim for the respawn CREATE packet.
    pub create_data: CreatureCreateData,
    /// AI fields needed to rebuild the canonical creature runtime.
    pub max_hp: u32,
    pub level: u8,
    pub min_dmg: u32,
    pub max_dmg: u32,
    pub aggro_radius: f32,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub ai_name: String,
    pub script_name: String,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub npc_flags: u32,
    pub unit_flags: u32,
    pub map_id: u16,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
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
        home_pos: creature.home_position(),
        create_data: CreatureCreateData {
            guid: creature.guid(),
            entry: creature.entry(),
            display_id: creature.display_id(),
            native_display_id: creature.display_id(),
            health: creature.max_hp() as i64,
            max_health: creature.max_hp() as i64,
            level: creature.level(),
            faction_template: creature.faction() as i32,
            npc_flags: creature.npc_flags_mask_like_cpp(),
            unit_flags: creature.unit_flags(),
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: creature.creature.melee_damage_school_like_cpp(),
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 0,
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
        flags_extra: creature.creature.lifecycle_metadata().flags_extra,
        static_flags: creature.creature.lifecycle_metadata().static_flags,
        ai_name: creature.creature.lifecycle_metadata().ai_name.clone(),
        script_name: creature.creature.lifecycle_metadata().script_name.clone(),
        ground_movement_type: creature.creature.ground_movement_type_like_cpp(),
        swim_allowed: creature.creature.swim_allowed_like_cpp(),
        flight_movement_type: creature.creature.flight_movement_type_like_cpp(),
        npc_flags: creature.npc_flags(),
        unit_flags: creature.unit_flags(),
        map_id,
        loot_id: creature.loot_id(),
        skin_loot_id: creature.skin_loot_id(),
        gold_min: creature.gold_min(),
        gold_max: creature.gold_max(),
        boss_id: creature.boss_id(),
        dungeon_encounter_id: creature.dungeon_encounter_id(),
        phase_use_flags: creature.creature.ai_ownership().phase_use_flags,
        phase_id: creature.creature.ai_ownership().phase_id,
        phase_group_id: creature.creature.ai_ownership().phase_group_id,
        terrain_swap_map: creature.creature.ai_ownership().terrain_swap_map,
        phase_shift: creature.phase_shift().clone(),
    }
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
    creature.unit_mut().world_mut().relocate(respawn.home_pos);
    *creature.unit_mut().world_mut().phase_shift_mut() = respawn.phase_shift.clone();
    creature.unit_mut().set_level(level);
    creature.unit_mut().set_max_health(u64::from(hp));
    creature.unit_mut().set_health(u64::from(hp));
    creature.set_ai_identity_runtime(display_id, faction, npc_flags, unit_flags);
    creature.set_npc_flags2_runtime_like_cpp(npc_flags2);
    creature.set_unit_flags2_runtime_like_cpp(unit_flags2);
    creature.set_unit_flags3_runtime_like_cpp(unit_flags3);
    creature.set_melee_damage_school_like_cpp(damage_school);
    creature.set_flags_extra_runtime_like_cpp(respawn.flags_extra);
    creature.set_static_flags_runtime_like_cpp(respawn.static_flags);
    creature.set_ai_identity_names_runtime_like_cpp(
        respawn.ai_name.clone(),
        respawn.script_name.clone(),
    );
    creature.set_ground_movement_type_runtime_like_cpp(respawn.ground_movement_type);
    creature.set_swim_allowed_runtime_like_cpp(respawn.swim_allowed);
    creature.set_flight_movement_type_runtime_like_cpp(respawn.flight_movement_type);
    creature.configure_ai_runtime(respawn.home_pos, respawn.aggro_radius, 5.0, 30);
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

        let visible =
            manager.get_visible_creatures_in_phase(0, 0, 10.0, 10.0, 0.0, Some(&seer_phase));
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
        PendingRespawn {
            respawn_at,
            home_pos: Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                orientation: 0.0,
            },
            create_data: CreatureCreateData {
                guid: ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Creature,
                    0,
                    1,
                    0,
                    0,
                    1,
                    100,
                ),
                entry: 1,
                display_id: 1,
                native_display_id: 1,
                health: 100,
                max_health: 100,
                level: 1,
                faction_template: 1,
                npc_flags: 0,
                unit_flags: 0,
                unit_flags2: 0,
                unit_flags3: 0,
                damage_school: wow_constants::spell::SpellSchools::Normal as u8,
                scale: 1.0,
                unit_class: 1,
                base_attack_time: 2000,
                ranged_attack_time: 0,
                zone_id: 0,
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
            flags_extra: 0,
            static_flags: [0; 8],
            ai_name: String::new(),
            script_name: String::new(),
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            npc_flags: 0,
            unit_flags: 0,
            map_id: 0,
            loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
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

    #[test]
    fn pending_respawn_preserves_flags_extra_like_cpp() {
        let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 42);
        let mut creature = test_creature(guid);
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
        creature.creature.set_flight_movement_type_runtime_like_cpp(
            wow_constants::CreatureFlightMovementType::CanFly as u8,
        );
        creature.creature.set_ground_movement_type_runtime_like_cpp(
            wow_constants::CreatureGroundMovementType::None as u8,
        );
        creature.creature.set_swim_allowed_runtime_like_cpp(false);

        let pending = pending_respawn_from_world_creature_like_cpp(&creature, Instant::now(), 0);
        assert_eq!(pending.flags_extra, CreatureFlagsExtra::CIVILIAN.bits());
        assert_eq!(pending.static_flags[0], static_flags[0]);
        assert_eq!(pending.ai_name, "SmartAI");
        assert_eq!(pending.script_name, "npc_respawn_identity");
        assert_eq!(
            pending.ground_movement_type,
            wow_constants::CreatureGroundMovementType::None as u8
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
        assert!(!respawned.creature.can_walk_like_cpp());
        assert!(!respawned.creature.can_enter_water_like_cpp());
        assert!(respawned.creature.can_fly_like_cpp());
    }
}
