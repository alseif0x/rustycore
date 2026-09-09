//! Navmesh FFI wrapper state definitions, part 3 of 4.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

pub fn find_smooth_path_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_pos: [f32; 3],
    end_pos: [f32; 3],
    poly_path: &[DetourPolyRef],
    max_smooth_path_size: usize,
) -> Result<Vec<[f32; 3]>, DetourNavMeshQueryError> {
    if poly_path.is_empty() || max_smooth_path_size == 0 {
        return Ok(Vec::new());
    }

    let mut polys = poly_path.to_vec();
    let mut iter_pos;
    let target_pos;

    if poly_path.len() > 1 {
        iter_pos = query.closest_point_on_poly_boundary(polys[0], start_pos)?;
        target_pos = query.closest_point_on_poly_boundary(*polys.last().unwrap(), end_pos)?;
    } else {
        iter_pos = start_pos;
        target_pos = end_pos;
    }

    let mut smooth_path = Vec::with_capacity(max_smooth_path_size);
    smooth_path.push(iter_pos);

    while !polys.is_empty() && smooth_path.len() < max_smooth_path_size {
        let Some(steer) = get_steer_target_like_cpp(
            query,
            iter_pos,
            target_pos,
            SMOOTH_PATH_SLOP_LIKE_CPP,
            &polys,
        )?
        else {
            break;
        };

        let end_of_path = steer.flags & DT_STRAIGHTPATH_END_LIKE_CPP != 0;
        let offmesh_connection = steer.flags & DT_STRAIGHTPATH_OFFMESH_CONNECTION_LIKE_CPP != 0;

        let delta = [
            steer.position[0] - iter_pos[0],
            steer.position[1] - iter_pos[1],
            steer.position[2] - iter_pos[2],
        ];
        let delta_len = detour_distance(steer.position, iter_pos);
        if delta_len <= f32::EPSILON {
            break;
        }
        let len =
            if (end_of_path || offmesh_connection) && delta_len < SMOOTH_PATH_STEP_SIZE_LIKE_CPP {
                1.0
            } else {
                SMOOTH_PATH_STEP_SIZE_LIKE_CPP / delta_len
            };
        let move_target = [
            iter_pos[0] + delta[0] * len,
            iter_pos[1] + delta[1] * len,
            iter_pos[2] + delta[2] * len,
        ];

        let moved = query.move_along_surface(polys[0], iter_pos, move_target, filter, 16)?;
        polys = fixup_corridor_like_cpp(&polys, MAX_PATH_LENGTH_LIKE_CPP, &moved.visited);

        let mut result = moved.result_position;
        if let Some(first_poly) = polys.first().copied() {
            if let Ok(height) = query.get_poly_height(first_poly, result) {
                result[1] = height;
            }
        }
        result[1] += 0.5;
        iter_pos = result;

        if end_of_path && detour_in_range(iter_pos, steer.position, SMOOTH_PATH_SLOP_LIKE_CPP, 1.0)
        {
            iter_pos = target_pos;
            if smooth_path.len() < max_smooth_path_size {
                smooth_path.push(iter_pos);
            }
            break;
        }

        if offmesh_connection
            && detour_in_range(iter_pos, steer.position, SMOOTH_PATH_SLOP_LIKE_CPP, 1.0)
        {
            let mut prev_ref = 0;
            let mut poly_ref = polys[0];
            let mut npos = 0;
            while npos < polys.len() && poly_ref != steer.poly_ref {
                prev_ref = poly_ref;
                poly_ref = polys[npos];
                npos += 1;
            }

            polys.drain(0..npos);

            if let Ok((connection_start_pos, connection_end_pos)) =
                nav_mesh.get_off_mesh_connection_poly_end_points(prev_ref, poly_ref)
            {
                if smooth_path.len() < max_smooth_path_size {
                    smooth_path.push(connection_start_pos);
                }

                iter_pos = connection_end_pos;
                if let Some(first_poly) = polys.first().copied() {
                    let height = query.get_poly_height(first_poly, iter_pos)?;
                    iter_pos[1] = height;
                }
                iter_pos[1] += 0.5;
            }
        }

        if smooth_path.len() < max_smooth_path_size {
            smooth_path.push(iter_pos);
        }
    }

    if smooth_path.len() >= MAX_POINT_PATH_LENGTH_LIKE_CPP {
        return Err(DetourNavMeshQueryError::SmoothPathTooLong {
            point_count: smooth_path.len(),
        });
    }

    Ok(smooth_path)
}

pub(crate) fn add_far_from_poly_flags_like_cpp(
    path_type: &mut DetourPathType,
    start_far_from_poly: bool,
    end_far_from_poly: bool,
) {
    if start_far_from_poly {
        path_type.insert(DetourPathType::FARFROMPOLY_START);
    }
    if end_far_from_poly {
        path_type.insert(DetourPathType::FARFROMPOLY_END);
    }
}

pub(crate) fn detour_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    detour_distance_sq(left, right).sqrt()
}

pub(crate) fn detour_distance_sq(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}

pub(crate) fn detour_in_range(first: [f32; 3], second: [f32; 3], range: f32, height: f32) -> bool {
    let dx = second[0] - first[0];
    let dy = second[1] - first[1];
    let dz = second[2] - first[2];
    (dx * dx + dz * dz) < range * range && dy.abs() < height
}

pub(crate) fn detour_lerp(start: [f32; 3], end: [f32; 3], t: f32) -> [f32; 3] {
    [
        start[0] + (end[0] - start[0]) * t,
        start[1] + (end[1] - start[1]) * t,
        start[2] + (end[2] - start[2]) * t,
    ]
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DetourNavMeshError {
    #[error("Detour navmesh allocation failed")]
    AllocationFailed,
    #[error("Detour navmesh initialization failed with status 0x{status:08x}")]
    InitFailed { status: DetourStatus },
    #[error("Detour off-mesh connection endpoints lookup failed with status 0x{status:08x}")]
    OffMeshConnectionEndpointsFailed { status: DetourStatus },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DetourTileError {
    #[error("Detour tile data is too large for C++ int size: {size} bytes")]
    TileDataTooLarge { size: usize },
    #[error("Detour addTile failed with status 0x{status:08x}")]
    AddTileFailed { status: DetourStatus },
    #[error("Detour removeTile failed with status 0x{status:08x}")]
    RemoveTileFailed { status: DetourStatus },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DetourNavMeshQueryError {
    #[error("Detour navmesh query allocation failed")]
    AllocationFailed,
    #[error("Detour navmesh query initialization failed with status 0x{status:08x}")]
    InitFailed { status: DetourStatus },
    #[error("Detour findNearestPoly failed with status 0x{status:08x}")]
    FindNearestPolyFailed { status: DetourStatus },
    #[error("Detour findPath failed with status 0x{status:08x}")]
    FindPathFailed { status: DetourStatus },
    #[error("Detour findPath output buffer is too large for C++ int size: {max_path}")]
    PathBufferTooLarge { max_path: usize },
    #[error("Detour findStraightPath failed with status 0x{status:08x}")]
    FindStraightPathFailed { status: DetourStatus },
    #[error(
        "Detour findStraightPath output buffer is too large for C++ int size: {max_straight_path}"
    )]
    StraightPathBufferTooLarge { max_straight_path: usize },
    #[error("Detour closestPointOnPoly failed with status 0x{status:08x}")]
    ClosestPointOnPolyFailed { status: DetourStatus },
    #[error("Detour closestPointOnPolyBoundary failed with status 0x{status:08x}")]
    ClosestPointOnPolyBoundaryFailed { status: DetourStatus },
    #[error("Detour getPolyHeight failed with status 0x{status:08x}")]
    GetPolyHeightFailed { status: DetourStatus },
    #[error("Detour moveAlongSurface failed with status 0x{status:08x}")]
    MoveAlongSurfaceFailed { status: DetourStatus },
    #[error(
        "Detour moveAlongSurface visited buffer is too large for C++ int size: {max_visited_size}"
    )]
    VisitedBufferTooLarge { max_visited_size: usize },
    #[error("Detour raycast failed with status 0x{status:08x}")]
    RaycastFailed { status: DetourStatus },
    #[error("Detour smooth path reached C++ MAX_POINT_PATH_LENGTH: {point_count}")]
    SmoothPathTooLong { point_count: usize },
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DetourQueryFilterError {
    #[error("Detour query filter allocation failed")]
    AllocationFailed,
    #[error("Detour query filter area index is out of range: {area} >= {max}")]
    AreaIndexOutOfRange { area: usize, max: usize },
}

#[must_use]
pub const fn detour_status_failed(status: DetourStatus) -> bool {
    status & DT_FAILURE_LIKE_CPP != 0
}

pub(crate) fn validate_area_index(area: usize) -> Result<i32, DetourQueryFilterError> {
    if area >= DT_MAX_AREAS_LIKE_CPP {
        return Err(DetourQueryFilterError::AreaIndexOutOfRange {
            area,
            max: DT_MAX_AREAS_LIKE_CPP,
        });
    }

    Ok(area as i32)
}

impl DetourNavMeshParams {
    pub fn parse(bytes: &[u8]) -> Result<Self, DetourNavMeshParamsError> {
        if bytes.len() < DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP {
            return Err(DetourNavMeshParamsError::TooShort {
                actual: bytes.len(),
                expected: DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP,
            });
        }

        Ok(Self {
            origin: [read_f32(bytes, 0), read_f32(bytes, 4), read_f32(bytes, 8)],
            tile_width: read_f32(bytes, 12),
            tile_height: read_f32(bytes, 16),
            max_tiles: read_i32(bytes, 20),
            max_polys: read_i32(bytes, 24),
        })
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP] {
        let mut bytes = [0; DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP];
        bytes[0..4].copy_from_slice(&self.origin[0].to_le_bytes());
        bytes[4..8].copy_from_slice(&self.origin[1].to_le_bytes());
        bytes[8..12].copy_from_slice(&self.origin[2].to_le_bytes());
        bytes[12..16].copy_from_slice(&self.tile_width.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.tile_height.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.max_tiles.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.max_polys.to_le_bytes());
        bytes
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum DetourNavMeshParamsError {
    #[error("Detour navmesh params are too short: got {actual} bytes, expected {expected}")]
    TooShort { actual: usize, expected: usize },
}

#[derive(Debug)]
pub struct MMapData {
    pub(crate) nav_mesh_queries: HashMap<(u32, u32), MMapNavMeshQuery>,
    pub(crate) nav_mesh: DetourNavMesh,
    pub nav_mesh_params: DetourNavMeshParams,
    pub loaded_tile_refs: HashMap<u32, u64>,
}

impl MMapData {
    pub fn new(nav_mesh_params: DetourNavMeshParams) -> Result<Self, DetourNavMeshError> {
        let nav_mesh = DetourNavMesh::new(&nav_mesh_params)?;

        Ok(Self {
            nav_mesh_queries: HashMap::new(),
            nav_mesh,
            nav_mesh_params,
            loaded_tile_refs: HashMap::new(),
        })
    }

    #[must_use]
    pub const fn nav_mesh(&self) -> &DetourNavMesh {
        &self.nav_mesh
    }

    #[must_use]
    pub fn nav_mesh_query_count(&self) -> usize {
        self.nav_mesh_queries.len()
    }

    #[must_use]
    pub fn has_nav_mesh_query(&self, instance_map_id: u32, instance_id: u32) -> bool {
        self.nav_mesh_queries
            .contains_key(&(instance_map_id, instance_id))
    }

    #[must_use]
    pub fn get_nav_mesh_query(
        &self,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Option<&MMapNavMeshQuery> {
        self.nav_mesh_queries.get(&(instance_map_id, instance_id))
    }

    pub fn load_nav_mesh_query(
        &mut self,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<bool, DetourNavMeshQueryError> {
        let key = (instance_map_id, instance_id);
        if self.nav_mesh_queries.contains_key(&key) {
            return Ok(true);
        }

        let query = MMapNavMeshQuery::new(&self.nav_mesh, 1024)?;
        self.nav_mesh_queries.insert(key, query);
        Ok(true)
    }

    pub fn unload_nav_mesh_query(&mut self, instance_map_id: u32, instance_id: u32) -> bool {
        self.nav_mesh_queries
            .remove(&(instance_map_id, instance_id))
            .is_some()
    }

    pub fn load_tile(
        &mut self,
        packed_grid_pos: u32,
        tile: &MmapTileBlob,
    ) -> Result<bool, DetourTileError> {
        if self.loaded_tile_refs.contains_key(&packed_grid_pos) {
            return Ok(false);
        }

        let tile_ref = self.nav_mesh.add_tile(tile)?;
        self.loaded_tile_refs.insert(packed_grid_pos, tile_ref);
        Ok(true)
    }

    pub fn unload_tile(&mut self, packed_grid_pos: u32) -> Result<bool, DetourTileError> {
        let Some(tile_ref) = self.loaded_tile_refs.get(&packed_grid_pos).copied() else {
            return Ok(false);
        };

        self.nav_mesh.remove_tile(tile_ref)?;
        self.loaded_tile_refs.remove(&packed_grid_pos);
        Ok(true)
    }

    pub fn calculate_path_for_instance_like_cpp(
        &self,
        instance_map_id: u32,
        instance_id: u32,
        filter: &DetourQueryFilter,
        start_wow: [f32; 3],
        end_wow: [f32; 3],
        options: DetourPathOptions,
    ) -> Result<Option<DetourPolyPath>, DetourNavMeshQueryError> {
        self.calculate_path_for_instance_with_previous_path_like_cpp(
            instance_map_id,
            instance_id,
            filter,
            start_wow,
            end_wow,
            options,
            &[],
        )
    }

    /// As above, for a caller that kept the corridor from its previous query and
    /// can therefore reproduce the C++ `_pathPolyRefs` reuse.
    #[allow(clippy::too_many_arguments)]
    pub fn calculate_path_for_instance_with_previous_path_like_cpp(
        &self,
        instance_map_id: u32,
        instance_id: u32,
        filter: &DetourQueryFilter,
        start_wow: [f32; 3],
        end_wow: [f32; 3],
        options: DetourPathOptions,
        previous_poly_refs: &[DetourPolyRef],
    ) -> Result<Option<DetourPolyPath>, DetourNavMeshQueryError> {
        let Some(query) = self.get_nav_mesh_query(instance_map_id, instance_id) else {
            return Ok(None);
        };

        if !self.nav_mesh.have_tile_for_wow_position_like_cpp(start_wow)
            || !self.nav_mesh.have_tile_for_wow_position_like_cpp(end_wow)
        {
            return Ok(None);
        }

        calculate_detour_path_with_raw_query_like_cpp(
            &self.nav_mesh,
            query.as_raw(),
            filter,
            start_wow,
            end_wow,
            options,
            previous_poly_refs,
        )
        .map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadUnsafeMapData {
    pub map_id: u32,
    pub child_map_ids: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MMapPathfindingContextLoadLikeCpp {
    pub mesh_map_id: u32,
    pub instance_map_id: u32,
    pub instance_id: u32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub map_data_available: bool,
    pub instance_query_available: bool,
    pub tile_available: bool,
    pub tile_loaded: bool,
}

#[derive(Debug)]
pub struct MMapManager {
    pub(crate) loaded_mmaps: HashMap<u32, Option<MMapData>>,
    pub(crate) parent_map_data: HashMap<u32, u32>,
    pub(crate) loaded_tiles: u32,
    pub(crate) thread_safe_environment: bool,
}

impl Default for MMapManager {
    fn default() -> Self {
        Self {
            loaded_mmaps: HashMap::new(),
            parent_map_data: HashMap::new(),
            loaded_tiles: 0,
            thread_safe_environment: true,
        }
    }
}

impl MMapManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn initialize_thread_unsafe<I>(&mut self, map_data: I)
    where
        I: IntoIterator<Item = ThreadUnsafeMapData>,
    {
        self.loaded_mmaps.clear();
        self.parent_map_data.clear();
        self.loaded_tiles = 0;

        for data in map_data {
            self.loaded_mmaps.entry(data.map_id).or_insert(None);

            for child_map_id in data.child_map_ids {
                self.parent_map_data.insert(child_map_id, data.map_id);
            }
        }

        self.thread_safe_environment = false;
    }

    pub fn load_map_data(
        &mut self,
        base_path: impl AsRef<Path>,
        map_id: u32,
    ) -> Result<bool, MMapManagerError> {
        if let Some(data) = self.loaded_mmaps.get(&map_id) {
            if data.is_some() {
                return Ok(true);
            }
        } else if !self.thread_safe_environment {
            return Err(MMapManagerError::InvalidMapInThreadUnsafe { map_id });
        } else {
            self.loaded_mmaps.insert(map_id, None);
        }

        let path = map_file_path_like_cpp(base_path, map_id);
        let bytes = fs::read(&path).map_err(|source| MMapManagerError::ReadMapFile {
            path: path.clone(),
            source,
        })?;
        let nav_mesh_params =
            DetourNavMeshParams::parse(&bytes).map_err(MMapManagerError::BadMapParams)?;

        self.loaded_mmaps.insert(
            map_id,
            Some(MMapData::new(nav_mesh_params).map_err(MMapManagerError::NavMesh)?),
        );

        Ok(true)
    }

    #[must_use]
    pub fn get_mmap_data(&self, map_id: u32) -> Option<&MMapData> {
        self.loaded_mmaps.get(&map_id).and_then(Option::as_ref)
    }

    pub fn unload_map(&mut self, map_id: u32) -> bool {
        let Some(data) = self.loaded_mmaps.get_mut(&map_id) else {
            return false;
        };

        let Some(loaded) = data.take() else {
            return false;
        };

        self.loaded_tiles = self
            .loaded_tiles
            .saturating_sub(loaded.loaded_tile_refs.len() as u32);
        true
    }

    pub fn load_map(
        &mut self,
        base_path: impl AsRef<Path>,
        map_id: u32,
        x: i32,
        y: i32,
    ) -> Result<bool, MMapManagerError> {
        let base_path = base_path.as_ref();
        self.load_map_data(base_path, map_id)?;

        let packed_grid_pos = pack_tile_id_like_cpp(x, y);
        if self
            .loaded_mmaps
            .get(&map_id)
            .and_then(Option::as_ref)
            .is_some_and(|data| data.loaded_tile_refs.contains_key(&packed_grid_pos))
        {
            return Ok(false);
        }

        let tile = self.read_tile_blob_with_parent_fallback(base_path, map_id, x, y)?;
        let Some(data) = self.loaded_mmaps.get_mut(&map_id).and_then(Option::as_mut) else {
            return Ok(false);
        };

        if data
            .load_tile(packed_grid_pos, &tile)
            .map_err(MMapManagerError::Tile)?
        {
            self.loaded_tiles = self.loaded_tiles.saturating_add(1);
            return Ok(true);
        }

        Ok(false)
    }

    pub fn unload_map_tile(
        &mut self,
        map_id: u32,
        x: i32,
        y: i32,
    ) -> Result<bool, MMapManagerError> {
        let Some(data) = self.loaded_mmaps.get_mut(&map_id).and_then(Option::as_mut) else {
            return Ok(false);
        };

        if data
            .unload_tile(pack_tile_id_like_cpp(x, y))
            .map_err(MMapManagerError::Tile)?
        {
            self.loaded_tiles = self.loaded_tiles.saturating_sub(1);
            return Ok(true);
        }

        Ok(false)
    }

    pub(crate) fn read_tile_blob_with_parent_fallback(
        &self,
        base_path: &Path,
        map_id: u32,
        x: i32,
        y: i32,
    ) -> Result<MmapTileBlob, MMapManagerError> {
        let path = tile_file_path_like_cpp(base_path, map_id, x, y);
        match read_mmap_tile_blob_file(&path, DT_NAVMESH_VERSION_LIKE_CPP) {
            Ok(tile) => Ok(tile),
            Err(MmapTileFileError::ReadTileFile { .. }) => {
                let Some(parent_map_id) = self.parent_map_data.get(&map_id).copied() else {
                    return Err(MMapManagerError::ReadTileFile { path });
                };

                let parent_path = tile_file_path_like_cpp(base_path, parent_map_id, x, y);
                read_mmap_tile_blob_file(&parent_path, DT_NAVMESH_VERSION_LIKE_CPP).map_err(
                    |source| MMapManagerError::TileFile {
                        path: parent_path,
                        source,
                    },
                )
            }
            Err(source) => Err(MMapManagerError::TileFile { path, source }),
        }
    }

    pub fn load_map_instance(
        &mut self,
        base_path: impl AsRef<Path>,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Result<bool, MMapManagerError> {
        self.load_map_data(base_path, mesh_map_id)?;

        let Some(data) = self
            .loaded_mmaps
            .get_mut(&mesh_map_id)
            .and_then(Option::as_mut)
        else {
            return Ok(false);
        };

        data.load_nav_mesh_query(instance_map_id, instance_id)
            .map_err(MMapManagerError::NavMeshQuery)
    }

    pub fn load_pathfinding_context_for_wow_position_like_cpp(
        &mut self,
        base_path: impl AsRef<Path>,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
        x: f32,
        y: f32,
    ) -> Result<MMapPathfindingContextLoadLikeCpp, MMapManagerError> {
        let base_path = base_path.as_ref();
        let (tile_x, tile_y) = mmap_tile_coords_for_wow_position_like_cpp(x, y);
        let map_data_available = match self.load_map_data(base_path, mesh_map_id) {
            Ok(loaded) => loaded,
            Err(error @ MMapManagerError::InvalidMapInThreadUnsafe { .. }) => return Err(error),
            Err(_) => false,
        };

        let mut instance_query_available = false;
        let mut tile_loaded = false;
        let mut tile_available = false;

        if map_data_available {
            instance_query_available = match self.load_map_instance(
                base_path,
                mesh_map_id,
                instance_map_id,
                instance_id,
            ) {
                Ok(loaded) => loaded,
                Err(error @ MMapManagerError::InvalidMapInThreadUnsafe { .. }) => {
                    return Err(error);
                }
                Err(_) => false,
            };

            let packed_grid_pos = pack_tile_id_like_cpp(tile_x, tile_y);
            let had_tile = self
                .loaded_mmaps
                .get(&mesh_map_id)
                .and_then(Option::as_ref)
                .is_some_and(|data| data.loaded_tile_refs.contains_key(&packed_grid_pos));
            tile_loaded = match self.load_map(base_path, mesh_map_id, tile_x, tile_y) {
                Ok(loaded) => loaded,
                Err(error @ MMapManagerError::InvalidMapInThreadUnsafe { .. }) => {
                    return Err(error);
                }
                Err(_) => false,
            };
            tile_available = had_tile
                || tile_loaded
                || self
                    .loaded_mmaps
                    .get(&mesh_map_id)
                    .and_then(Option::as_ref)
                    .is_some_and(|data| data.loaded_tile_refs.contains_key(&packed_grid_pos));
        }

        Ok(MMapPathfindingContextLoadLikeCpp {
            mesh_map_id,
            instance_map_id,
            instance_id,
            tile_x,
            tile_y,
            map_data_available,
            instance_query_available,
            tile_available,
            tile_loaded,
        })
    }

    pub fn unload_map_instance(
        &mut self,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    ) -> bool {
        let Some(data) = self
            .loaded_mmaps
            .get_mut(&mesh_map_id)
            .and_then(Option::as_mut)
        else {
            return false;
        };

        data.unload_nav_mesh_query(instance_map_id, instance_id)
    }

    #[must_use]
    pub fn get_nav_mesh(&self, map_id: u32) -> Option<&DetourNavMesh> {
        self.loaded_mmaps
            .get(&map_id)
            .and_then(Option::as_ref)
            .map(MMapData::nav_mesh)
    }

    #[must_use]
    pub fn get_nav_mesh_query(
        &self,
        mesh_map_id: u32,
        instance_map_id: u32,
        instance_id: u32,
    ) -> Option<&MMapNavMeshQuery> {
        self.loaded_mmaps
            .get(&mesh_map_id)
            .and_then(Option::as_ref)
            .and_then(|data| data.get_nav_mesh_query(instance_map_id, instance_id))
    }

    #[must_use]
    pub fn get_nav_mesh_params(&self, map_id: u32) -> Option<DetourNavMeshParams> {
        self.loaded_mmaps
            .get(&map_id)
            .and_then(Option::as_ref)
            .map(|data| data.nav_mesh_params)
    }

    #[must_use]
    pub fn parent_map_id(&self, child_map_id: u32) -> Option<u32> {
        self.parent_map_data.get(&child_map_id).copied()
    }

    #[must_use]
    pub fn get_loaded_tiles_count(&self) -> u32 {
        self.loaded_tiles
    }

    #[must_use]
    pub fn get_loaded_maps_count(&self) -> u32 {
        self.loaded_mmaps.len() as u32
    }

    #[must_use]
    pub fn is_thread_safe_environment(&self) -> bool {
        self.thread_safe_environment
    }
}

#[derive(Debug, Error)]
pub enum MMapManagerError {
    #[error("invalid map id {map_id} passed after thread-unsafe initialization")]
    InvalidMapInThreadUnsafe { map_id: u32 },
    #[error("failed to read mmap file {path:?}: {source}")]
    ReadMapFile { path: PathBuf, source: io::Error },
    #[error("bad mmap params: {0}")]
    BadMapParams(DetourNavMeshParamsError),
    #[error("failed to initialize Detour navmesh: {0}")]
    NavMesh(DetourNavMeshError),
    #[error("failed to initialize Detour navmesh query: {0}")]
    NavMeshQuery(DetourNavMeshQueryError),
    #[error("failed to read mmap tile file {path:?}")]
    ReadTileFile { path: PathBuf },
    #[error("bad mmap tile file {path:?}: {source}")]
    TileFile {
        path: PathBuf,
        source: MmapTileFileError,
    },
    #[error("failed to load Detour tile: {0}")]
    Tile(DetourTileError),
}
