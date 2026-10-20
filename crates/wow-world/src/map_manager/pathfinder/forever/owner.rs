// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation package consumer.
//! Verified package admission and bounded operations on the existing MMap manager.

use std::{collections::HashSet, path::Path};

use forever_navigation::{
    mmap::{self, GlobalView, TileView},
    package_v23::{self, Coverage, ReadBatch, Summary},
    tilebuilder::{self, Coordinate},
};
use wow_recastdetour::{
    CHECKED_GROUND_MAX_ABS_COORDINATE, CHECKED_GROUND_MAX_BV_NODES,
    CHECKED_GROUND_MAX_DETAIL_TRIANGLES, CHECKED_GROUND_MAX_LINKS, CHECKED_GROUND_MAX_POLYGONS,
    CHECKED_GROUND_MAX_VERTICES, CHECKED_NATIVE_MAX_TILE_DIMENSION, CHECKED_NATIVE_MAX_TILE_SLOTS,
    CheckedDetour7GroundPayload, CheckedDetourNavMeshParams, CheckedGroundLimits,
    CheckedNativeParamsLimits, DetourCorridorQueryError, DetourNavMeshParams, MMapManager,
    ThreadUnsafeMapData,
};

use super::types::{
    FOREVER_NAVIGATION_QUERY_NODE_BUDGET, ForeverNavigationCorridor, ForeverNavigationError,
    ForeverNavigationLimits, ForeverNavigationQueryRequest, ForeverNavigationStats,
    ForeverNavigationWorkerAction, ForeverNavigationWorkerResponse,
    MAX_FOREVER_NAVIGATION_PATH_POLYS,
};

const V23_BUILD: u32 = 70_450;
const V23_NAVIGATION_COMPONENT: u8 = 1 << 5;
const TILE_HEADER_BYTES: usize = 20;

#[derive(Debug)]
pub(in crate::map_manager::pathfinder) struct ForeverNavigationOwner {
    root: std::path::PathBuf,
    expected: package_v23::Expected,
    limits: ForeverNavigationLimits,
    summary: Summary,
    coverage: Coverage,
    params: DetourNavMeshParams,
    poisoned: bool,
    #[cfg(feature = "test-fixtures")]
    failure_plan: Option<super::types::ForeverNavigationTestFailurePlan>,
}

struct CheckedTile<'a> {
    coordinate: Coordinate,
    manager_grid_x: i32,
    manager_grid_y: i32,
    artifact_bytes: u64,
    payload: CheckedDetour7GroundPayload<'a>,
}

impl ForeverNavigationOwner {
    pub(crate) fn open(
        root: impl AsRef<Path>,
        expected: package_v23::Expected,
        limits: ForeverNavigationLimits,
        mmap_manager: &mut MMapManager,
    ) -> Result<Self, ForeverNavigationError> {
        validate_expected(expected)?;
        let root = root.as_ref().to_path_buf();
        let initial = package_v23::read_batch(&root, expected, &[], read_limits(limits))?;
        let context = expected
            .context
            .ok_or(ForeverNavigationError::InvalidExpected { field: "context" })?;
        let summary = *initial.summary();
        let coverage = initial.coverage().clone();
        if summary.context != context
            || coverage.context != context
            || coverage.identity.selected_map != expected.selected_map
            || coverage.identity.build != V23_BUILD
            || coverage.identity.locale != expected.locale.name()
            || coverage.identity.plan_sha256 != expected.plan_sha256
            || coverage.tiles.is_empty()
        {
            return Err(ForeverNavigationError::InvalidExpected {
                field: "verified_package_identity",
            });
        }
        validate_coverage_coordinates(&coverage)?;

        let mmap_limits = mmap_limits(limits);
        let global = GlobalView::parse(initial.global_bytes(), expected.selected_map, mmap_limits)?;
        if initial.global_bytes().len() > limits.max_loaded_bytes() {
            return Err(ForeverNavigationError::LoadedByteLimit {
                actual: initial.global_bytes().len() as u64,
                maximum: limits.max_loaded_bytes(),
            });
        }
        let global_params = global.params();
        validate_global_params(global_params)?;
        let params = DetourNavMeshParams {
            origin: global_params.origin,
            tile_width: global_params.tile_width,
            tile_height: global_params.tile_height,
            max_tiles: global_params.max_tiles,
            max_polys: global_params.max_polys,
        };
        let checked_limits = CheckedNativeParamsLimits::new(
            CHECKED_NATIVE_MAX_TILE_SLOTS,
            CHECKED_NATIVE_MAX_TILE_DIMENSION,
            CHECKED_GROUND_MAX_ABS_COORDINATE,
        )?;
        let checked_params = CheckedDetourNavMeshParams::new(params, checked_limits)?;

        // The selected map is the only map admitted to this worker. No file
        // fallback or parent-map resolution is enabled for this owner.
        mmap_manager.initialize_thread_unsafe([ThreadUnsafeMapData {
            map_id: expected.selected_map,
            child_map_ids: Vec::new(),
        }]);
        mmap_manager.admit_checked_nav_mesh_params(expected.selected_map, checked_params)?;

        Ok(Self {
            root,
            expected,
            limits,
            summary,
            coverage,
            params,
            poisoned: false,
            #[cfg(feature = "test-fixtures")]
            failure_plan: None,
        })
    }

    pub(crate) fn handle_action(
        &mut self,
        mmap_manager: &mut MMapManager,
        action: ForeverNavigationWorkerAction,
    ) -> Result<ForeverNavigationWorkerResponse, ForeverNavigationError> {
        if self.poisoned && !matches!(&action, ForeverNavigationWorkerAction::Stats) {
            return Err(ForeverNavigationError::Poisoned);
        }

        match action {
            ForeverNavigationWorkerAction::PrepareTiles { map_id, tiles } => {
                self.require_map(map_id)?;
                self.prepare_tiles(mmap_manager, &tiles)?;
                Ok(ForeverNavigationWorkerResponse::Unit)
            }
            ForeverNavigationWorkerAction::UnloadTiles { map_id, tiles } => {
                self.require_map(map_id)?;
                self.unload_tiles(mmap_manager, &tiles)?;
                Ok(ForeverNavigationWorkerResponse::Unit)
            }
            ForeverNavigationWorkerAction::PrepareInstance {
                map_id,
                instance_map_id,
                instance_id,
            } => {
                self.require_map(map_id)?;
                self.prepare_instance(mmap_manager, instance_map_id, instance_id)?;
                Ok(ForeverNavigationWorkerResponse::Unit)
            }
            ForeverNavigationWorkerAction::UnloadInstance {
                map_id,
                instance_map_id,
                instance_id,
            } => {
                self.require_map(map_id)?;
                self.unload_instance(mmap_manager, instance_map_id, instance_id)?;
                Ok(ForeverNavigationWorkerResponse::Unit)
            }
            ForeverNavigationWorkerAction::Query(request) => {
                let corridor = self.query(mmap_manager, request)?;
                Ok(ForeverNavigationWorkerResponse::Corridor(corridor))
            }
            ForeverNavigationWorkerAction::Stats => Ok(ForeverNavigationWorkerResponse::Stats(
                self.stats(mmap_manager)?,
            )),
            #[cfg(feature = "test-fixtures")]
            ForeverNavigationWorkerAction::SetFailurePlan(plan) => {
                self.failure_plan = Some(plan);
                Ok(ForeverNavigationWorkerResponse::Unit)
            }
        }
    }

    fn require_map(&self, map_id: u32) -> Result<(), ForeverNavigationError> {
        if map_id != self.expected.selected_map {
            return Err(ForeverNavigationError::WrongMap {
                actual: map_id,
                expected: self.expected.selected_map,
            });
        }
        Ok(())
    }

    fn prepare_tiles(
        &mut self,
        mmap_manager: &mut MMapManager,
        tiles: &[Coordinate],
    ) -> Result<(), ForeverNavigationError> {
        self.validate_tile_batch(tiles)?;
        let batch = self.read_verified_batch(tiles)?;
        let checked_tiles = self.parse_checked_tiles(&batch)?;
        let existing_keys = self.loaded_tile_keys(mmap_manager)?;
        let current_stats = self.stats(mmap_manager)?;
        let new_tiles: Vec<_> = checked_tiles
            .iter()
            .filter(|tile| {
                !existing_keys
                    .contains(&pack_wow_grid_key(tile.manager_grid_x, tile.manager_grid_y))
            })
            .collect();

        let resulting_count = current_stats
            .loaded_tiles
            .checked_add(new_tiles.len())
            .ok_or(ForeverNavigationError::LoadedTileLimit {
                actual: usize::MAX,
                maximum: self.limits.max_loaded_tiles(),
            })?;
        let maximum_slots = usize::try_from(self.params.max_tiles).unwrap_or(0);
        if resulting_count > self.limits.max_loaded_tiles() || resulting_count > maximum_slots {
            return Err(ForeverNavigationError::LoadedTileLimit {
                actual: resulting_count,
                maximum: self.limits.max_loaded_tiles().min(maximum_slots),
            });
        }
        let new_bytes = new_tiles
            .iter()
            .try_fold(0_u64, |total, tile| total.checked_add(tile.artifact_bytes));
        let resulting_bytes = new_bytes
            .and_then(|bytes| current_stats.loaded_artifact_bytes.checked_add(bytes))
            .ok_or(ForeverNavigationError::LoadedByteLimit {
                actual: u64::MAX,
                maximum: self.limits.max_loaded_bytes(),
            })?;
        if resulting_bytes > self.limits.max_loaded_bytes() as u64 {
            return Err(ForeverNavigationError::LoadedByteLimit {
                actual: resulting_bytes,
                maximum: self.limits.max_loaded_bytes(),
            });
        }

        self.admit_tiles_transactionally(mmap_manager, &checked_tiles, &existing_keys)
    }

    fn validate_tile_batch(&self, tiles: &[Coordinate]) -> Result<(), ForeverNavigationError> {
        if tiles.len() > self.limits.max_tiles_per_batch() {
            return Err(ForeverNavigationError::BatchTooLarge {
                actual: tiles.len(),
                maximum: self.limits.max_tiles_per_batch(),
            });
        }
        let mut seen = HashSet::with_capacity(tiles.len());
        for &tile in tiles {
            if !seen.insert((tile.row, tile.column)) {
                return Err(ForeverNavigationError::DuplicateTile(tile));
            }
            if coverage_tile(&self.coverage, tile).is_none() {
                return Err(ForeverNavigationError::TileOutsideCoverage(tile));
            }
        }
        Ok(())
    }

    fn read_verified_batch(
        &self,
        tiles: &[Coordinate],
    ) -> Result<ReadBatch, ForeverNavigationError> {
        let batch =
            package_v23::read_batch(&self.root, self.expected, tiles, read_limits(self.limits))?;
        if batch.summary() != &self.summary || batch.coverage() != &self.coverage {
            return Err(ForeverNavigationError::PackageChanged);
        }
        Ok(batch)
    }

    fn parse_checked_tiles<'a>(
        &self,
        batch: &'a ReadBatch,
    ) -> Result<Vec<CheckedTile<'a>>, ForeverNavigationError> {
        let mut parsed = Vec::with_capacity(batch.tiles().len());
        let mmap_limits = mmap_limits(self.limits);
        let payload_limit = self
            .limits
            .max_file_bytes()
            .saturating_sub(TILE_HEADER_BYTES);
        let ground_limits = CheckedGroundLimits::new(
            payload_limit,
            CHECKED_GROUND_MAX_POLYGONS,
            CHECKED_GROUND_MAX_VERTICES,
            CHECKED_GROUND_MAX_LINKS,
            CHECKED_GROUND_MAX_DETAIL_TRIANGLES,
            CHECKED_GROUND_MAX_BV_NODES,
            CHECKED_GROUND_MAX_ABS_COORDINATE,
        )?;

        for requested in batch.tiles() {
            let coordinate = requested.tile();
            let coverage = coverage_tile(&self.coverage, coordinate)
                .ok_or(ForeverNavigationError::TileOutsideCoverage(coordinate))?;
            let Some(bytes) = requested.bytes() else {
                return Err(ForeverNavigationError::DeclaredEmptyTile(coordinate));
            };
            if !coverage.written || bytes.len() as u64 != coverage.bytes {
                return Err(ForeverNavigationError::PackageChanged);
            }

            // Container framing is only a first parse. The payload then passes
            // the SDK's bounded structural/index checks before native add.
            let container = TileView::parse(bytes, mmap_limits)?;
            let payload =
                CheckedDetour7GroundPayload::parse_with_limits(container.payload(), ground_limits)?;
            let expected_xy = native_tile_coordinates(coordinate, self.params);
            let metadata = payload.metadata();
            let actual_xy = (metadata.tile_x, metadata.tile_y);
            if actual_xy != expected_xy {
                return Err(ForeverNavigationError::TilePlacementMismatch {
                    tile: coordinate,
                    expected: expected_xy,
                    actual: actual_xy,
                });
            }
            let expected_bounds = tile_horizontal_bounds(coordinate);
            let actual_bounds = header_horizontal_bounds(payload.bytes()).ok_or(
                ForeverNavigationError::TileBoundsHeaderUnavailable(coordinate),
            )?;
            if !same_f32_bits(&actual_bounds, &expected_bounds) {
                return Err(ForeverNavigationError::TileBoundsMismatch {
                    tile: coordinate,
                    expected: expected_bounds,
                    actual: actual_bounds,
                });
            }

            parsed.push(CheckedTile {
                coordinate,
                // The manager's WoW grid key stores row in its x slot and
                // column in its y slot. Detour header x/y were checked above
                // using the producer's native column/row axes.
                manager_grid_x: i32::from(coordinate.row),
                manager_grid_y: i32::from(coordinate.column),
                artifact_bytes: coverage.bytes,
                payload,
            });
        }
        Ok(parsed)
    }

    fn admit_tiles_transactionally(
        &mut self,
        mmap_manager: &mut MMapManager,
        tiles: &[CheckedTile<'_>],
        existing_keys: &HashSet<u32>,
    ) -> Result<(), ForeverNavigationError> {
        let mut added = Vec::with_capacity(tiles.len());
        #[cfg(feature = "test-fixtures")]
        let failure_plan = self.failure_plan.take();

        #[cfg(feature = "test-fixtures")]
        if failure_plan.is_some_and(|plan| plan.fail_after_native_adds == 0) {
            return Err(ForeverNavigationError::TestInjectedNativeFailure);
        }

        for tile in tiles {
            let packed_key = pack_wow_grid_key(tile.manager_grid_x, tile.manager_grid_y);
            if existing_keys.contains(&packed_key) {
                continue;
            }
            #[cfg(feature = "test-fixtures")]
            let fail_rollback = failure_plan.is_some_and(|plan| plan.fail_rollback);
            #[cfg(not(feature = "test-fixtures"))]
            let fail_rollback = false;
            match mmap_manager.admit_checked_ground_tile(
                self.expected.selected_map,
                tile.manager_grid_x,
                tile.manager_grid_y,
                &tile.payload,
            ) {
                Ok(true) => added.push((tile.manager_grid_x, tile.manager_grid_y, tile.coordinate)),
                Ok(false) => {
                    self.poisoned = true;
                    let add_error = ForeverNavigationError::NativeStateDiverged {
                        packed_tile_key: packed_key,
                    };
                    return Err(self.rollback_new_tiles(
                        mmap_manager,
                        &added,
                        add_error,
                        fail_rollback,
                    ));
                }
                Err(error) => {
                    let add_error = ForeverNavigationError::NativeAdmission(error);
                    return Err(self.rollback_new_tiles(
                        mmap_manager,
                        &added,
                        add_error,
                        fail_rollback,
                    ));
                }
            }

            #[cfg(feature = "test-fixtures")]
            if failure_plan.is_some_and(|plan| added.len() >= plan.fail_after_native_adds) {
                let add_error = ForeverNavigationError::TestInjectedNativeFailure;
                return Err(self.rollback_new_tiles(
                    mmap_manager,
                    &added,
                    add_error,
                    fail_rollback,
                ));
            }
        }
        Ok(())
    }

    fn rollback_new_tiles(
        &mut self,
        mmap_manager: &mut MMapManager,
        added: &[(i32, i32, Coordinate)],
        add_error: ForeverNavigationError,
        fail_rollback: bool,
    ) -> ForeverNavigationError {
        #[cfg(feature = "test-fixtures")]
        if fail_rollback && !added.is_empty() {
            self.poisoned = true;
            return ForeverNavigationError::RollbackFailed {
                add: add_error.to_string(),
                rollback: "test-injected rollback failure".to_string(),
            };
        }
        #[cfg(not(feature = "test-fixtures"))]
        let _ = fail_rollback;
        self.rollback_new_tiles_impl(mmap_manager, added, add_error)
    }

    fn rollback_new_tiles_impl(
        &mut self,
        mmap_manager: &mut MMapManager,
        added: &[(i32, i32, Coordinate)],
        add_error: ForeverNavigationError,
    ) -> ForeverNavigationError {
        for &(x, y, coordinate) in added.iter().rev() {
            match mmap_manager.unload_map_tile(self.expected.selected_map, x, y) {
                Ok(true) => {}
                Ok(false) => {
                    self.poisoned = true;
                    return ForeverNavigationError::RollbackFailed {
                        add: add_error.to_string(),
                        rollback: format!("new tile {coordinate:?} was absent during rollback"),
                    };
                }
                Err(error) => {
                    self.poisoned = true;
                    return ForeverNavigationError::RollbackFailed {
                        add: add_error.to_string(),
                        rollback: error.to_string(),
                    };
                }
            }
        }
        add_error
    }

    fn unload_tiles(
        &mut self,
        mmap_manager: &mut MMapManager,
        tiles: &[Coordinate],
    ) -> Result<(), ForeverNavigationError> {
        if tiles.len() > self.limits.max_tiles_per_batch() {
            return Err(ForeverNavigationError::BatchTooLarge {
                actual: tiles.len(),
                maximum: self.limits.max_tiles_per_batch(),
            });
        }
        let mut seen = HashSet::with_capacity(tiles.len());
        for &coordinate in tiles {
            if !seen.insert((coordinate.row, coordinate.column)) {
                return Err(ForeverNavigationError::DuplicateTile(coordinate));
            }
            if coverage_tile(&self.coverage, coordinate).is_none() {
                return Err(ForeverNavigationError::TileOutsideCoverage(coordinate));
            }
        }
        let existing_keys = self.loaded_tile_keys(mmap_manager)?;
        for &coordinate in tiles {
            let row = i32::from(coordinate.row);
            let column = i32::from(coordinate.column);
            let packed_key = pack_wow_grid_key(row, column);
            if !existing_keys.contains(&packed_key) {
                continue;
            }
            match mmap_manager.unload_map_tile(self.expected.selected_map, row, column) {
                Ok(true) => {}
                Ok(false) => {
                    self.poisoned = true;
                    return Err(ForeverNavigationError::NativeStateDiverged {
                        packed_tile_key: packed_key,
                    });
                }
                Err(error) => {
                    self.poisoned = true;
                    return Err(ForeverNavigationError::NativeUnloadFailed(
                        error.to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn prepare_instance(
        &self,
        mmap_manager: &mut MMapManager,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<(), ForeverNavigationError> {
        let data = mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .ok_or(ForeverNavigationError::NotConfigured)?;
        let already_loaded = data.has_nav_mesh_query(instance_map_id, instance_id);
        let query_count = data.nav_mesh_query_count();
        if !already_loaded && query_count >= self.limits.max_instance_queries() {
            return Err(ForeverNavigationError::InstanceQueryLimit {
                actual: query_count + 1,
                maximum: self.limits.max_instance_queries(),
            });
        }
        mmap_manager.admit_instance_query(
            self.expected.selected_map,
            instance_map_id,
            instance_id,
            FOREVER_NAVIGATION_QUERY_NODE_BUDGET,
        )?;
        Ok(())
    }

    fn unload_instance(
        &self,
        mmap_manager: &mut MMapManager,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<(), ForeverNavigationError> {
        if mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .is_none()
        {
            return Err(ForeverNavigationError::NotConfigured);
        }
        mmap_manager.unload_map_instance(self.expected.selected_map, instance_map_id, instance_id);
        Ok(())
    }

    fn query(
        &mut self,
        mmap_manager: &mut MMapManager,
        request: ForeverNavigationQueryRequest,
    ) -> Result<ForeverNavigationCorridor, ForeverNavigationError> {
        self.require_map(request.map_id)?;
        if request.phase_shift != wow_entities::PhaseShift::default() {
            return Err(ForeverNavigationError::UnsupportedPhase);
        }
        validate_query_position(request.start, "start")?;
        validate_query_position(request.destination, "destination")?;
        validate_half_extents(request.half_extents)?;
        if request.max_path == 0 || request.max_path > MAX_FOREVER_NAVIGATION_PATH_POLYS {
            return Err(ForeverNavigationError::NativeCorridor(
                DetourCorridorQueryError::InvalidPathCapacity {
                    actual: request.max_path,
                    maximum: MAX_FOREVER_NAVIGATION_PATH_POLYS,
                },
            ));
        }
        self.require_prepared_endpoint(mmap_manager, request.start, "start")?;
        self.require_prepared_endpoint(mmap_manager, request.destination, "destination")?;
        let filter = wow_recastdetour::create_path_query_filter_like_cpp(request.filter_context)?;

        self.prepare_instance(mmap_manager, request.instance_map_id, request.instance_id)?;
        let data = mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .ok_or(ForeverNavigationError::NotConfigured)?;
        let corridor = data.find_detour_corridor_for_instance(
            request.instance_map_id,
            request.instance_id,
            &filter,
            [request.start.y, request.start.z, request.start.x],
            [
                request.destination.y,
                request.destination.z,
                request.destination.x,
            ],
            request.half_extents,
            request.max_path,
        )?;
        let Some(path) = corridor.path else {
            return Err(ForeverNavigationError::NoNearestPolygon {
                start: corridor.start,
                destination: corridor.end,
            });
        };
        Ok(ForeverNavigationCorridor {
            start: corridor.start,
            destination: corridor.end,
            path,
        })
    }

    fn require_prepared_endpoint(
        &mut self,
        mmap_manager: &MMapManager,
        position: wow_core::Position,
        endpoint: &'static str,
    ) -> Result<(), ForeverNavigationError> {
        let (row, column) = crate::map_manager::terrain_grid_coords_for_wow_position_like_cpp(
            position.x, position.y,
        );
        if !(0..64).contains(&column) || !(0..64).contains(&row) {
            return Err(ForeverNavigationError::PositionOutsideMap { endpoint });
        }
        let tile = Coordinate {
            row: row as u8,
            column: column as u8,
        };
        let fact = coverage_tile(&self.coverage, tile)
            .ok_or(ForeverNavigationError::TileOutsideCoverage(tile))?;
        if !fact.written {
            return Err(ForeverNavigationError::DeclaredEmptyTile(tile));
        }
        let packed_key = pack_wow_grid_key(row, column);
        let loaded = mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .ok_or(ForeverNavigationError::NotConfigured)?
            .loaded_tile_refs
            .contains_key(&packed_key);
        if !loaded {
            return Err(ForeverNavigationError::TileNotPrepared(tile));
        }
        Ok(())
    }

    fn stats(
        &mut self,
        mmap_manager: &MMapManager,
    ) -> Result<ForeverNavigationStats, ForeverNavigationError> {
        let data = mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .ok_or(ForeverNavigationError::NotConfigured)?;
        let loaded_keys: Vec<u32> = data.loaded_tile_refs.keys().copied().collect();
        let query_count = data.nav_mesh_query_count();
        let loaded_tiles = loaded_keys.len();
        if loaded_tiles > self.limits.max_loaded_tiles() {
            return Err(ForeverNavigationError::LoadedTileLimit {
                actual: loaded_tiles,
                maximum: self.limits.max_loaded_tiles(),
            });
        }
        if query_count > self.limits.max_instance_queries() {
            return Err(ForeverNavigationError::InstanceQueryLimit {
                actual: query_count,
                maximum: self.limits.max_instance_queries(),
            });
        }
        let mut loaded_tile_artifact_bytes = 0_u64;
        for packed_key in loaded_keys {
            let row = ((packed_key >> 16) & 0xffff) as i32;
            let column = (packed_key & 0xffff) as i32;
            if !(0..64).contains(&row) || !(0..64).contains(&column) {
                self.poisoned = true;
                return Err(ForeverNavigationError::NativeStateDiverged {
                    packed_tile_key: packed_key,
                });
            }
            let tile = Coordinate {
                row: row as u8,
                column: column as u8,
            };
            let Some(fact) = coverage_tile(&self.coverage, tile) else {
                self.poisoned = true;
                return Err(ForeverNavigationError::NativeStateDiverged {
                    packed_tile_key: packed_key,
                });
            };
            if !fact.written {
                self.poisoned = true;
                return Err(ForeverNavigationError::NativeStateDiverged {
                    packed_tile_key: packed_key,
                });
            }
            loaded_tile_artifact_bytes = loaded_tile_artifact_bytes.checked_add(fact.bytes).ok_or(
                ForeverNavigationError::LoadedByteLimit {
                    actual: u64::MAX,
                    maximum: self.limits.max_loaded_bytes(),
                },
            )?;
        }
        let global_artifact_bytes = self.coverage.global_bytes;
        let loaded_artifact_bytes = global_artifact_bytes
            .checked_add(loaded_tile_artifact_bytes)
            .ok_or(ForeverNavigationError::LoadedByteLimit {
                actual: u64::MAX,
                maximum: self.limits.max_loaded_bytes(),
            })?;
        if loaded_artifact_bytes > self.limits.max_loaded_bytes() as u64 {
            return Err(ForeverNavigationError::LoadedByteLimit {
                actual: loaded_artifact_bytes,
                maximum: self.limits.max_loaded_bytes(),
            });
        }
        Ok(ForeverNavigationStats {
            map_id: self.expected.selected_map,
            loaded_tiles,
            loaded_tile_artifact_bytes,
            global_artifact_bytes,
            loaded_artifact_bytes,
            instance_queries: query_count,
            poisoned: self.poisoned,
        })
    }

    fn loaded_tile_keys(
        &mut self,
        mmap_manager: &MMapManager,
    ) -> Result<HashSet<u32>, ForeverNavigationError> {
        let data = mmap_manager
            .get_mmap_data(self.expected.selected_map)
            .ok_or(ForeverNavigationError::NotConfigured)?;
        Ok(data.loaded_tile_refs.keys().copied().collect())
    }

    #[cfg(feature = "test-fixtures")]
    pub(super) fn set_failure_plan(
        &mut self,
        plan: super::types::ForeverNavigationTestFailurePlan,
    ) {
        self.failure_plan = Some(plan);
    }
}

fn validate_expected(expected: package_v23::Expected) -> Result<(), ForeverNavigationError> {
    let Some(context) = expected.context else {
        return Err(ForeverNavigationError::InvalidExpected { field: "context" });
    };
    if context.build != V23_BUILD {
        return Err(ForeverNavigationError::InvalidExpected { field: "build" });
    }
    if context.locale != expected.locale {
        return Err(ForeverNavigationError::InvalidExpected { field: "locale" });
    }
    if context.plan_sha256 != expected.plan_sha256 {
        return Err(ForeverNavigationError::InvalidExpected {
            field: "plan_sha256",
        });
    }
    if context.components != V23_NAVIGATION_COMPONENT {
        return Err(ForeverNavigationError::InvalidExpected {
            field: "components",
        });
    }
    Ok(())
}

fn validate_coverage_coordinates(coverage: &Coverage) -> Result<(), ForeverNavigationError> {
    let mut seen = HashSet::with_capacity(coverage.tiles.len());
    for tile in &coverage.tiles {
        if tile.tile.row >= 64
            || tile.tile.column >= 64
            || !seen.insert((tile.tile.row, tile.tile.column))
        {
            return Err(ForeverNavigationError::InvalidExpected {
                field: "coverage_inventory",
            });
        }
    }
    Ok(())
}

fn validate_global_params(params: mmap::Params) -> Result<(), ForeverNavigationError> {
    if params.origin[1] != f32::MIN_POSITIVE
        || params.tile_width != tilebuilder::GRID_SIZE
        || params.tile_height != tilebuilder::GRID_SIZE
        || !(1..=4096).contains(&params.max_tiles)
        || params.max_polys != i32::MIN
        || !grid_origin_is_canonical(params.origin[0])
        || !grid_origin_is_canonical(params.origin[2])
    {
        return Err(ForeverNavigationError::NonCanonicalGlobalParams);
    }
    Ok(())
}

fn grid_origin_is_canonical(origin: f32) -> bool {
    (0..64).any(|index| {
        let maximum = (32 - index) as f32 * tilebuilder::GRID_SIZE;
        origin.to_bits() == (maximum - tilebuilder::GRID_SIZE).to_bits()
    })
}

fn native_tile_coordinates(tile: Coordinate, params: DetourNavMeshParams) -> (i32, i32) {
    let [min_x, max_x, min_z, max_z] = tile_horizontal_bounds(tile);
    let center_x = (min_x + max_x) / 2.0;
    let center_z = (min_z + max_z) / 2.0;
    (
        ((center_x - params.origin[0]) / params.tile_width) as i32,
        ((center_z - params.origin[2]) / params.tile_height) as i32,
    )
}

fn tile_horizontal_bounds(tile: Coordinate) -> [f32; 4] {
    let max_x = (32 - i32::from(tile.column)) as f32 * tilebuilder::GRID_SIZE;
    let min_x = max_x - tilebuilder::GRID_SIZE;
    let max_z = (32 - i32::from(tile.row)) as f32 * tilebuilder::GRID_SIZE;
    let min_z = max_z - tilebuilder::GRID_SIZE;
    [min_x, max_x, min_z, max_z]
}

fn header_horizontal_bounds(payload: &[u8]) -> Option<[f32; 4]> {
    Some([
        read_header_f32(payload, 72)?,
        read_header_f32(payload, 84)?,
        read_header_f32(payload, 80)?,
        read_header_f32(payload, 92)?,
    ])
}

fn same_f32_bits(left: &[f32; 4], right: &[f32; 4]) -> bool {
    left.iter()
        .zip(right)
        .all(|(actual, expected)| actual.to_bits() == expected.to_bits())
}

fn read_header_f32(payload: &[u8], offset: usize) -> Option<f32> {
    let bytes: [u8; 4] = payload.get(offset..offset + 4)?.try_into().ok()?;
    Some(f32::from_le_bytes(bytes))
}

fn coverage_tile(coverage: &Coverage, tile: Coordinate) -> Option<&package_v23::TileOutput> {
    coverage.tiles.iter().find(|fact| fact.tile == tile)
}

fn mmap_limits(limits: ForeverNavigationLimits) -> mmap::Limits {
    mmap::Limits {
        records: mmap::Limits::default().records,
        file_bytes: limits.max_file_bytes(),
        heap_bytes: limits.max_file_bytes(),
    }
}

fn read_limits(limits: ForeverNavigationLimits) -> package_v23::ReadLimits {
    package_v23::ReadLimits {
        max_requested_tiles: limits.max_tiles_per_batch(),
        max_batch_bytes: limits.max_batch_bytes(),
        max_file_bytes: limits.max_file_bytes(),
    }
}

fn validate_query_position(
    position: wow_core::Position,
    endpoint: &'static str,
) -> Result<(), ForeverNavigationError> {
    for (axis, value) in [position.x, position.y, position.z].into_iter().enumerate() {
        if !value.is_finite() || value.abs() > CHECKED_GROUND_MAX_ABS_COORDINATE {
            return Err(ForeverNavigationError::InvalidPosition { endpoint, axis });
        }
    }
    Ok(())
}

fn validate_half_extents(extents: [f32; 3]) -> Result<(), ForeverNavigationError> {
    for (axis, value) in extents.into_iter().enumerate() {
        if !value.is_finite() || value < 0.0 || value > CHECKED_GROUND_MAX_ABS_COORDINATE {
            return Err(ForeverNavigationError::NativeCorridor(
                DetourCorridorQueryError::InvalidHalfExtent { axis, value },
            ));
        }
    }
    Ok(())
}

const fn pack_wow_grid_key(row: i32, column: i32) -> u32 {
    ((row as u32) << 16) | (column as u32 & 0xffff)
}

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;
