//! Pathfinder items of mod.
//!
//! Separated from mod.rs under #711; every item keeps its name,
//! signature and body.

use super::*;

pub(super) const SMOOTH_PATH_STEP_SIZE_LIKE_CPP: f32 = 4.0;

pub(super) fn point_path_limit_for_distance_like_cpp(distance: f32) -> usize {
    let point_limit = if distance.is_sign_negative() {
        0
    } else {
        (distance / SMOOTH_PATH_STEP_SIZE_LIKE_CPP) as usize
    };
    point_limit.min(MAX_POINT_PATH_LENGTH_LIKE_CPP)
}

pub(super) fn position_from_detour_point_like_cpp(point: [f32; 3]) -> Position {
    Position::new(point[0], point[1], point[2], 0.0)
}

pub(super) fn position_to_wow_point_like_cpp(position: Position) -> [f32; 3] {
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
pub(super) struct WorldMMapPathfinderMessageLikeCpp {
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

pub(super) fn random_path_result_from_path_type_like_cpp(path_type: PathType) -> RandomPathResult {
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
