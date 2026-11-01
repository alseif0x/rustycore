// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature path queries over the shared Detour mmap data
//! (C++ `PathGenerator` / `MMAP::MMapManager` callers).
//!
//! #1263 F6-8E-2: moved out of the deleted legacy `map_manager` module; every
//! item keeps its name and body.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use wow_core::Position;
use wow_entities::{Creature, PhaseShift};
use wow_movement::{PathGenerator, PathType, RandomPathResult};
use wow_recastdetour::{
    DetourNavMeshQueryError, DetourOwnerCapabilitiesLikeCpp, DetourPathOptions, DetourPathType,
    DetourPointPath, DetourPolyPath, DetourQueryFilterError, MAX_POINT_PATH_LENGTH_LIKE_CPP,
    MMapData, MMapManager as DetourMMapManager, MMapManagerError, PathQueryFilterContext,
    ThreadUnsafeMapData, create_path_query_filter_like_cpp,
};

#[cfg(all(feature = "forever-navigation", target_os = "linux"))]
mod forever;
#[cfg(all(
    feature = "forever-navigation",
    target_os = "linux",
    feature = "test-fixtures"
))]
pub use forever::ForeverNavigationTestFailurePlan;
#[cfg(all(feature = "forever-navigation", target_os = "linux"))]
#[allow(unused_imports)]
pub use forever::{
    FOREVER_NAVIGATION_QUERY_NODE_BUDGET, ForeverNavigation56Expected, ForeverNavigation57Expected,
    ForeverNavigationContext, ForeverNavigationCorridor, ForeverNavigationError,
    ForeverNavigationExpected, ForeverNavigationLimits, ForeverNavigationLocale,
    ForeverNavigationQueryRequest, ForeverNavigationStats, ForeverNavigationSummary,
    ForeverNavigationTile, MAX_FOREVER_NAVIGATION_BATCH_BYTES, MAX_FOREVER_NAVIGATION_FILE_BYTES,
    MAX_FOREVER_NAVIGATION_INSTANCE_QUERIES, MAX_FOREVER_NAVIGATION_LOADED_BYTES,
    MAX_FOREVER_NAVIGATION_PATH_POLYS, MAX_FOREVER_NAVIGATION_TILES,
    MAX_FOREVER_NAVIGATION_TILES_PER_BATCH,
};

use crate::creature_movement::CreatureMovementQueriesLikeCpp;
use crate::terrain::TerrainGridFileIndexLikeCpp;

pub(crate) const SMOOTH_PATH_STEP_SIZE_LIKE_CPP: f32 = 4.0;

pub(crate) fn point_path_limit_for_distance_like_cpp(distance: f32) -> usize {
    let point_limit = if distance.is_sign_negative() {
        0
    } else {
        (distance / SMOOTH_PATH_STEP_SIZE_LIKE_CPP) as usize
    };
    point_limit.min(MAX_POINT_PATH_LENGTH_LIKE_CPP)
}

pub(crate) fn position_from_detour_point_like_cpp(point: [f32; 3]) -> Position {
    Position::new(point[0], point[1], point[2], 0.0)
}

pub(crate) fn position_to_wow_point_like_cpp(position: Position) -> [f32; 3] {
    [position.x, position.y, position.z]
}

#[derive(Debug, PartialEq)]
pub enum WorldDetourPathError {
    Filter(DetourQueryFilterError),
    Query(DetourNavMeshQueryError),
    MMap(String),
    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    SelectedNavigationRequiresExplicitQuery,
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
    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    forever_navigation: Option<forever::ForeverNavigationOwner>,
}

impl WorldMMapPathfinderLikeCpp {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        Self {
            terrain_grid_file_index: TerrainGridFileIndexLikeCpp::new(&data_dir, []),
            data_dir,
            mmap_manager: DetourMMapManager::new(),
            #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
            forever_navigation: None,
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
            #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
            forever_navigation: None,
        }
    }

    pub fn calculate_creature_path_like_cpp(
        &mut self,
        creature: &Creature,
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
        #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
        if self.forever_navigation.is_some() {
            return Err(WorldDetourPathError::SelectedNavigationRequiresExplicitQuery);
        }

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
    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    forever_navigation_limits: Option<forever::ForeverNavigationLimits>,
}

#[derive(Debug)]
pub(super) enum WorldMMapPathfinderMessageLikeCpp {
    Legacy {
        request: WorldMMapPathRequestLikeCpp,
        response_tx: mpsc::Sender<Result<Option<DetourPolyPath>, WorldDetourPathError>>,
    },
    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    ForeverNavigation {
        action: forever::ForeverNavigationWorkerAction,
        response_tx: mpsc::Sender<
            Result<forever::ForeverNavigationWorkerResponse, forever::ForeverNavigationError>,
        >,
    },
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
                let pathfinder = pathfinder_factory(data_dir);
                run_pathfinder_worker(pathfinder, request_rx);
            })
            .expect("spawn mmap pathfinder worker");

        Self {
            request_tx,
            #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
            forever_navigation_limits: None,
        }
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn spawn_forever_navigation(
        package_root: impl AsRef<Path>,
        expected: forever::ForeverNavigationExpected,
        limits: forever::ForeverNavigationLimits,
    ) -> Result<Self, forever::ForeverNavigationError> {
        let package_root = package_root.as_ref().to_path_buf();
        let worker_root = package_root.clone();
        let (request_tx, request_rx) = mpsc::channel::<WorldMMapPathfinderMessageLikeCpp>();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::Builder::new()
            .name("world-mmap-pathfinder-like-cpp".to_string())
            .spawn(move || {
                let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&worker_root);
                let owner = match forever::ForeverNavigationOwner::open(
                    &worker_root,
                    expected,
                    limits,
                    &mut pathfinder.mmap_manager,
                ) {
                    Ok(owner) => owner,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                pathfinder.forever_navigation = Some(owner);
                if ready_tx.send(Ok(())).is_err() {
                    return;
                }
                run_pathfinder_worker(pathfinder, request_rx);
            })
            .map_err(|error| forever::ForeverNavigationError::WorkerSpawn(error.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                request_tx,
                forever_navigation_limits: Some(limits),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(forever::ForeverNavigationError::WorkerStopped),
        }
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn spawn_forever_navigation56(
        package_root: impl AsRef<Path>,
        expected: forever::ForeverNavigation56Expected,
        limits: forever::ForeverNavigationLimits,
    ) -> Result<Self, forever::ForeverNavigationError> {
        let package_root = package_root.as_ref().to_path_buf();
        let worker_root = package_root.clone();
        let (request_tx, request_rx) = mpsc::channel::<WorldMMapPathfinderMessageLikeCpp>();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::Builder::new()
            .name("world-mmap-pathfinder-like-cpp".to_string())
            .spawn(move || {
                let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&worker_root);
                let owner = match forever::ForeverNavigationOwner::open_v56(
                    &worker_root,
                    expected,
                    limits,
                    &mut pathfinder.mmap_manager,
                ) {
                    Ok(owner) => owner,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                pathfinder.forever_navigation = Some(owner);
                if ready_tx.send(Ok(())).is_err() {
                    return;
                }
                run_pathfinder_worker(pathfinder, request_rx);
            })
            .map_err(|error| forever::ForeverNavigationError::WorkerSpawn(error.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                request_tx,
                forever_navigation_limits: Some(limits),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(forever::ForeverNavigationError::WorkerStopped),
        }
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn spawn_forever_navigation57(
        package_root: impl AsRef<Path>,
        expected: forever::ForeverNavigation57Expected,
        limits: forever::ForeverNavigationLimits,
    ) -> Result<Self, forever::ForeverNavigationError> {
        let package_root = package_root.as_ref().to_path_buf();
        let worker_root = package_root.clone();
        let (request_tx, request_rx) = mpsc::channel::<WorldMMapPathfinderMessageLikeCpp>();
        let (ready_tx, ready_rx) = mpsc::channel();
        thread::Builder::new()
            .name("world-mmap-pathfinder-like-cpp".to_string())
            .spawn(move || {
                let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&worker_root);
                let owner = match forever::ForeverNavigationOwner::open_v57(
                    &worker_root,
                    expected,
                    limits,
                    &mut pathfinder.mmap_manager,
                ) {
                    Ok(owner) => owner,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                pathfinder.forever_navigation = Some(owner);
                if ready_tx.send(Ok(())).is_err() {
                    return;
                }
                run_pathfinder_worker(pathfinder, request_rx);
            })
            .map_err(|error| forever::ForeverNavigationError::WorkerSpawn(error.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                request_tx,
                forever_navigation_limits: Some(limits),
            }),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(forever::ForeverNavigationError::WorkerStopped),
        }
    }

    pub fn calculate_path_like_cpp(
        &self,
        request: WorldMMapPathRequestLikeCpp,
    ) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
        let (response_tx, response_rx) = mpsc::channel();
        self.request_tx
            .send(WorldMMapPathfinderMessageLikeCpp::Legacy {
                request,
                response_tx,
            })
            .map_err(|error| WorldDetourPathError::MMap(error.to_string()))?;
        response_rx
            .recv()
            .map_err(|error| WorldDetourPathError::MMap(error.to_string()))?
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn prepare_forever_navigation_tiles(
        &self,
        map_id: u32,
        tiles: impl IntoIterator<Item = forever::ForeverNavigationTile>,
    ) -> Result<(), forever::ForeverNavigationError> {
        let limits = self.selected_navigation_limits()?;
        let tiles = collect_forever_navigation_tiles(tiles, limits.max_tiles_per_batch())?;
        self.send_forever_navigation_action(
            forever::ForeverNavigationWorkerAction::PrepareTiles { map_id, tiles },
        )?;
        Ok(())
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn unload_forever_navigation_tiles(
        &self,
        map_id: u32,
        tiles: impl IntoIterator<Item = forever::ForeverNavigationTile>,
    ) -> Result<(), forever::ForeverNavigationError> {
        let limits = self.selected_navigation_limits()?;
        let tiles = collect_forever_navigation_tiles(tiles, limits.max_tiles_per_batch())?;
        self.send_forever_navigation_action(forever::ForeverNavigationWorkerAction::UnloadTiles {
            map_id,
            tiles,
        })?;
        Ok(())
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn prepare_forever_navigation_instance(
        &self,
        map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<(), forever::ForeverNavigationError> {
        self.send_forever_navigation_action(
            forever::ForeverNavigationWorkerAction::PrepareInstance {
                map_id,
                instance_map_id,
                instance_id,
            },
        )?;
        Ok(())
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn unload_forever_navigation_instance(
        &self,
        map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<(), forever::ForeverNavigationError> {
        self.send_forever_navigation_action(
            forever::ForeverNavigationWorkerAction::UnloadInstance {
                map_id,
                instance_map_id,
                instance_id,
            },
        )?;
        Ok(())
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn query_forever_navigation_corridor(
        &self,
        request: forever::ForeverNavigationQueryRequest,
    ) -> Result<forever::ForeverNavigationCorridor, forever::ForeverNavigationError> {
        match self.send_forever_navigation_action(forever::ForeverNavigationWorkerAction::Query(
            request,
        ))? {
            forever::ForeverNavigationWorkerResponse::Corridor(corridor) => Ok(corridor),
            _ => Err(forever::ForeverNavigationError::UnexpectedWorkerResponse),
        }
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    pub fn forever_navigation_stats(
        &self,
    ) -> Result<forever::ForeverNavigationStats, forever::ForeverNavigationError> {
        match self.send_forever_navigation_action(forever::ForeverNavigationWorkerAction::Stats)? {
            forever::ForeverNavigationWorkerResponse::Stats(stats) => Ok(stats),
            _ => Err(forever::ForeverNavigationError::UnexpectedWorkerResponse),
        }
    }

    #[cfg(all(
        feature = "forever-navigation",
        target_os = "linux",
        feature = "test-fixtures"
    ))]
    pub fn set_forever_navigation_test_failure_plan(
        &self,
        plan: forever::ForeverNavigationTestFailurePlan,
    ) -> Result<(), forever::ForeverNavigationError> {
        self.send_forever_navigation_action(
            forever::ForeverNavigationWorkerAction::SetFailurePlan(plan),
        )?;
        Ok(())
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    fn selected_navigation_limits(
        &self,
    ) -> Result<forever::ForeverNavigationLimits, forever::ForeverNavigationError> {
        self.forever_navigation_limits
            .ok_or(forever::ForeverNavigationError::NotConfigured)
    }

    #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
    fn send_forever_navigation_action(
        &self,
        action: forever::ForeverNavigationWorkerAction,
    ) -> Result<forever::ForeverNavigationWorkerResponse, forever::ForeverNavigationError> {
        self.selected_navigation_limits()?;
        let (response_tx, response_rx) = mpsc::channel();
        self.request_tx
            .send(WorldMMapPathfinderMessageLikeCpp::ForeverNavigation {
                action,
                response_tx,
            })
            .map_err(|_| forever::ForeverNavigationError::WorkerStopped)?;
        response_rx
            .recv()
            .map_err(|_| forever::ForeverNavigationError::WorkerStopped)?
    }
}

#[cfg(all(feature = "forever-navigation", target_os = "linux"))]
fn collect_forever_navigation_tiles(
    tiles: impl IntoIterator<Item = forever::ForeverNavigationTile>,
    maximum: usize,
) -> Result<Vec<forever::ForeverNavigationTile>, forever::ForeverNavigationError> {
    let mut collected = Vec::with_capacity(maximum.min(64));
    for tile in tiles {
        if collected.len() == maximum {
            return Err(forever::ForeverNavigationError::BatchTooLarge {
                actual: maximum.saturating_add(1),
                maximum,
            });
        }
        collected.push(tile);
    }
    Ok(collected)
}

fn run_pathfinder_worker(
    mut pathfinder: WorldMMapPathfinderLikeCpp,
    request_rx: mpsc::Receiver<WorldMMapPathfinderMessageLikeCpp>,
) {
    while let Ok(message) = request_rx.recv() {
        match message {
            WorldMMapPathfinderMessageLikeCpp::Legacy {
                request,
                response_tx,
            } => {
                #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
                let result = if pathfinder.forever_navigation.is_some() {
                    Err(WorldDetourPathError::SelectedNavigationRequiresExplicitQuery)
                } else {
                    pathfinder.calculate_legacy_request_like_cpp(request)
                };
                #[cfg(not(all(feature = "forever-navigation", target_os = "linux")))]
                let result = pathfinder.calculate_legacy_request_like_cpp(request);
                let _ = response_tx.send(result);
            }
            #[cfg(all(feature = "forever-navigation", target_os = "linux"))]
            WorldMMapPathfinderMessageLikeCpp::ForeverNavigation {
                action,
                response_tx,
            } => {
                let result = forever::worker::handle_action(&mut pathfinder, action);
                let _ = response_tx.send(result);
            }
        }
    }
}

impl WorldMMapPathfinderLikeCpp {
    fn calculate_legacy_request_like_cpp(
        &mut self,
        request: WorldMMapPathRequestLikeCpp,
    ) -> Result<Option<DetourPolyPath>, WorldDetourPathError> {
        let mesh_map_id = self.resolve_mesh_map_id_for_path_request_like_cpp(&request);
        self.calculate_path_from_positions_like_cpp(
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
        )
    }
}

pub fn path_type_from_detour_like_cpp(path_type: DetourPathType) -> PathType {
    PathType::from_bits_retain(path_type.bits())
}

pub(crate) fn random_path_result_from_path_type_like_cpp(path_type: PathType) -> RandomPathResult {
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
    creature: &Creature,
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
