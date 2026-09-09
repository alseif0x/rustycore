//! Navmesh FFI wrapper state definitions, part 1 of 4.
//!
//! Separated from the lib.rs root under #658. Behaviour is preserved.

use super::*;

pub const MMAP_MAGIC_LIKE_CPP: u32 = 0x4d4d_4150;

pub const MMAP_VERSION_LIKE_CPP: u32 = 15;

pub const MMAP_TILE_HEADER_SIZE_LIKE_CPP: usize = 20;

pub const DT_POLYREF64_LIKE_CPP: bool = true;

pub const DT_SALT_BITS_LIKE_CPP: u32 = 12;

pub const DT_TILE_BITS_LIKE_CPP: u32 = 21;

pub const DT_POLY_BITS_LIKE_CPP: u32 = 31;

pub const DT_NAVMESH_MAGIC_LIKE_CPP: u32 = 0x444e_4156;

pub const DT_NAVMESH_VERSION_LIKE_CPP: u32 = 7;

pub const DT_NAVMESH_STATE_MAGIC_LIKE_CPP: u32 = 0x444e_4d53;

pub const DT_NAVMESH_STATE_VERSION_LIKE_CPP: u32 = 1;

pub const DT_EXT_LINK_LIKE_CPP: u16 = 0x8000;

pub const DT_NULL_LINK_LIKE_CPP: u32 = 0xffff_ffff;

pub const DT_OFFMESH_CON_BIDIR_LIKE_CPP: u32 = 1;

pub const DT_MAX_AREAS_LIKE_CPP: usize = 64;

pub const DT_TILE_FREE_DATA_LIKE_CPP: i32 = 0x01;

pub const DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP: usize = 28;

pub const DT_FAILURE_LIKE_CPP: DetourStatus = 1_u32 << 31;

pub const DT_SUCCESS_LIKE_CPP: DetourStatus = 1_u32 << 30;

pub const DT_IN_PROGRESS_LIKE_CPP: DetourStatus = 1_u32 << 29;

pub const DT_BUFFER_TOO_SMALL_LIKE_CPP: DetourStatus = 1_u32 << 4;

pub const DT_OUT_OF_MEMORY_LIKE_CPP: DetourStatus = 1_u32 << 2;

pub const DT_INVALID_PARAM_LIKE_CPP: DetourStatus = 1_u32 << 3;

pub const DT_STRAIGHTPATH_START_LIKE_CPP: u8 = 0x01;

pub const DT_STRAIGHTPATH_END_LIKE_CPP: u8 = 0x02;

pub const DT_STRAIGHTPATH_OFFMESH_CONNECTION_LIKE_CPP: u8 = 0x04;

pub const DT_STRAIGHTPATH_AREA_CROSSINGS_LIKE_CPP: i32 = 0x01;

pub const DT_STRAIGHTPATH_ALL_CROSSINGS_LIKE_CPP: i32 = 0x02;

pub const MAX_PATH_LENGTH_LIKE_CPP: usize = 74;

pub const MAX_POINT_PATH_LENGTH_LIKE_CPP: usize = 74;

pub const SMOOTH_PATH_STEP_SIZE_LIKE_CPP: f32 = 4.0;

pub const SMOOTH_PATH_SLOP_LIKE_CPP: f32 = 0.3;

pub(crate) const PATH_POLY_ACCEPT_DISTANCE_SQ_LIKE_CPP: f32 = 3.0;

pub const MAX_NUMBER_OF_GRIDS_LIKE_CPP: i32 = 64;

pub const SIZE_OF_GRIDS_LIKE_CPP: f32 = 533.3333;

pub const CENTER_GRID_ID_LIKE_CPP: i32 = MAX_NUMBER_OF_GRIDS_LIKE_CPP / 2;

pub const NAV_AREA_EMPTY_LIKE_CPP: u8 = 0;

pub const NAV_AREA_GROUND_LIKE_CPP: u8 = 11;

pub const NAV_AREA_GROUND_STEEP_LIKE_CPP: u8 = 10;

pub const NAV_AREA_WATER_LIKE_CPP: u8 = 9;

pub const NAV_AREA_MAGMA_SLIME_LIKE_CPP: u8 = 8;

pub const NAV_AREA_MAX_VALUE_LIKE_CPP: u8 = NAV_AREA_GROUND_LIKE_CPP;

pub const NAV_AREA_MIN_VALUE_LIKE_CPP: u8 = NAV_AREA_MAGMA_SLIME_LIKE_CPP;

pub const NAV_AREA_ALL_MASK_LIKE_CPP: u8 = 0x3f;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmapTileHeader {
    pub mmap_magic: u32,
    pub dt_version: u32,
    pub mmap_version: u32,
    pub size: u32,
    pub uses_liquids: bool,
    pub padding: [u8; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmapTileBlob {
    pub header: MmapTileHeader,
    pub data: Vec<u8>,
}

impl MmapTileBlob {
    pub fn parse(bytes: &[u8], expected_dt_version: u32) -> Result<Self, MmapTileBlobError> {
        let header = MmapTileHeader::parse(bytes).map_err(MmapTileBlobError::BadHeader)?;
        header
            .validate_dt_version(expected_dt_version)
            .map_err(MmapTileBlobError::BadHeader)?;

        let data_start = MMAP_TILE_HEADER_SIZE_LIKE_CPP;
        let available = bytes.len().saturating_sub(data_start);
        let declared = header.size as usize;
        if declared > available {
            return Err(MmapTileBlobError::CorruptedDataSize {
                declared,
                available,
            });
        }

        Ok(Self {
            header,
            data: bytes[data_start..data_start + declared].to_vec(),
        })
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MmapTileBlobError {
    #[error("bad mmap tile header: {0}")]
    BadHeader(MmapTileHeaderError),
    #[error("corrupted mmap tile data size: declared {declared} bytes, available {available}")]
    CorruptedDataSize { declared: usize, available: usize },
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetourNavMeshParams {
    pub origin: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
    pub max_tiles: i32,
    pub max_polys: i32,
}

#[repr(C)]
pub struct RawDetourNavMesh {
    pub(crate) _private: [u8; 0],
}

#[repr(C)]
pub struct RawDetourNavMeshQuery {
    pub(crate) _private: [u8; 0],
}

#[repr(C)]
pub struct RawDetourQueryFilter {
    pub(crate) _private: [u8; 0],
}

pub type DetourStatus = u32;

pub type DetourTileRef = u64;

pub type DetourPolyRef = u64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetourNearestPoly {
    pub poly_ref: DetourPolyRef,
    pub nearest_point: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetourStraightPathPoint {
    pub position: [f32; 3],
    pub flags: u8,
    pub poly_ref: DetourPolyRef,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetourMoveAlongSurface {
    pub result_position: [f32; 3],
    pub visited: Vec<DetourPolyRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetourRaycast {
    pub hit_t: f32,
    pub hit_normal: [f32; 3],
    pub path: Vec<DetourPolyRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetourPointPath {
    pub points: Vec<[f32; 3]>,
    pub actual_end: [f32; 3],
    pub path_type: DetourPathType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetourPolyPath {
    pub poly_refs: Vec<DetourPolyRef>,
    pub point_path: DetourPointPath,
    pub start_far_from_poly: bool,
    pub end_far_from_poly: bool,
}

/// Owner capabilities `PathGenerator::BuildPolyPath` reads off `_source` when it
/// decides whether a position with no navmesh polygon may still be walked
/// directly: `Creature::CanFly()`, `Creature::CanSwim()` and `Unit::IsFalling()`
/// (`PathGenerator.cpp:180-202`, `:222-240`).
///
/// All false reproduces "a ground creature that is not falling", which is what
/// the pathfinder assumed before these were threaded through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DetourOwnerCapabilitiesLikeCpp {
    pub can_fly: bool,
    pub can_swim: bool,
    pub is_falling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetourPathOptions {
    pub point_path_limit: usize,
    pub force_destination: bool,
    pub use_straight_path: bool,
    pub use_raycast: bool,
    pub owner: DetourOwnerCapabilitiesLikeCpp,
}

impl Default for DetourPathOptions {
    fn default() -> Self {
        Self {
            point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
            force_destination: false,
            use_straight_path: false,
            use_raycast: false,
            owner: DetourOwnerCapabilitiesLikeCpp::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviousPolyPathLikeCpp {
    Recalculate,
    PolyRefs(Vec<DetourPolyRef>),
    ShortcutNoPath,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetourSteerTarget {
    pub position: [f32; 3],
    pub flags: u8,
    pub poly_ref: DetourPolyRef,
}

#[must_use]
pub const fn wow_position_to_detour_like_cpp(position: [f32; 3]) -> [f32; 3] {
    [position[1], position[2], position[0]]
}

#[must_use]
pub const fn detour_position_to_wow_like_cpp(position: [f32; 3]) -> [f32; 3] {
    [position[2], position[0], position[1]]
}

#[derive(Debug)]
pub struct DetourNavMesh {
    pub(crate) raw: NonNull<RawDetourNavMesh>,
    pub(crate) _not_send_or_sync: PhantomData<Rc<()>>,
}

impl DetourNavMesh {
    pub fn new(params: &DetourNavMeshParams) -> Result<Self, DetourNavMeshError> {
        let raw = NonNull::new(unsafe { rustycore_dt_alloc_nav_mesh() })
            .ok_or(DetourNavMeshError::AllocationFailed)?;
        let status = unsafe { rustycore_dt_nav_mesh_init(raw.as_ptr(), params) };
        if detour_status_failed(status) {
            unsafe { rustycore_dt_free_nav_mesh(raw.as_ptr()) };
            return Err(DetourNavMeshError::InitFailed { status });
        }

        Ok(Self {
            raw,
            _not_send_or_sync: PhantomData,
        })
    }

    #[must_use]
    pub fn max_tiles(&self) -> u32 {
        unsafe { rustycore_dt_nav_mesh_get_max_tiles(self.raw.as_ptr()) }
    }

    pub fn add_tile(&mut self, tile: &MmapTileBlob) -> Result<DetourTileRef, DetourTileError> {
        let data_size =
            i32::try_from(tile.data.len()).map_err(|_| DetourTileError::TileDataTooLarge {
                size: tile.data.len(),
            })?;
        let mut tile_ref = 0;
        let status = unsafe {
            rustycore_dt_nav_mesh_add_tile_copy(
                self.raw.as_ptr(),
                tile.data.as_ptr(),
                data_size,
                DT_TILE_FREE_DATA_LIKE_CPP,
                &mut tile_ref,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourTileError::AddTileFailed { status });
        }

        Ok(tile_ref)
    }

    pub fn remove_tile(&mut self, tile_ref: DetourTileRef) -> Result<(), DetourTileError> {
        let status = unsafe { rustycore_dt_nav_mesh_remove_tile(self.raw.as_ptr(), tile_ref) };
        if detour_status_failed(status) {
            return Err(DetourTileError::RemoveTileFailed { status });
        }

        Ok(())
    }

    #[must_use]
    pub fn calc_tile_loc(&self, position: [f32; 3]) -> (i32, i32) {
        let mut tile_x = -1;
        let mut tile_y = -1;
        unsafe {
            rustycore_dt_nav_mesh_calc_tile_loc(
                self.raw.as_ptr(),
                position.as_ptr(),
                &mut tile_x,
                &mut tile_y,
            );
        }
        (tile_x, tile_y)
    }

    #[must_use]
    pub fn has_tile_at(&self, tile_x: i32, tile_y: i32, layer: i32) -> bool {
        unsafe { rustycore_dt_nav_mesh_has_tile_at(self.raw.as_ptr(), tile_x, tile_y, layer) }
    }

    #[must_use]
    pub fn have_tile_for_wow_position_like_cpp(&self, position: [f32; 3]) -> bool {
        let detour_position = wow_position_to_detour_like_cpp(position);
        let (tile_x, tile_y) = self.calc_tile_loc(detour_position);
        if tile_x < 0 || tile_y < 0 {
            return false;
        }

        self.has_tile_at(tile_x, tile_y, 0)
    }

    pub fn get_off_mesh_connection_poly_end_points(
        &self,
        prev_ref: DetourPolyRef,
        poly_ref: DetourPolyRef,
    ) -> Result<([f32; 3], [f32; 3]), DetourNavMeshError> {
        let mut start_pos = [0.0; 3];
        let mut end_pos = [0.0; 3];
        let status = unsafe {
            rustycore_dt_nav_mesh_get_off_mesh_connection_poly_end_points(
                self.raw.as_ptr(),
                prev_ref,
                poly_ref,
                start_pos.as_mut_ptr(),
                end_pos.as_mut_ptr(),
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshError::OffMeshConnectionEndpointsFailed { status });
        }

        Ok((start_pos, end_pos))
    }

    #[must_use]
    pub const fn as_raw(&self) -> *mut RawDetourNavMesh {
        self.raw.as_ptr()
    }
}

impl Drop for DetourNavMesh {
    fn drop(&mut self) {
        unsafe { rustycore_dt_free_nav_mesh(self.raw.as_ptr()) };
    }
}

#[derive(Debug)]
pub struct DetourNavMeshQuery<'mesh> {
    pub(crate) raw: NonNull<RawDetourNavMeshQuery>,
    pub(crate) _mesh_lifetime_and_thread_model: PhantomData<(&'mesh DetourNavMesh, Rc<()>)>,
}

impl<'mesh> DetourNavMeshQuery<'mesh> {
    pub fn new(
        mesh: &'mesh DetourNavMesh,
        max_nodes: i32,
    ) -> Result<Self, DetourNavMeshQueryError> {
        let raw = NonNull::new(unsafe { rustycore_dt_alloc_nav_mesh_query() })
            .ok_or(DetourNavMeshQueryError::AllocationFailed)?;
        let status =
            unsafe { rustycore_dt_nav_mesh_query_init(raw.as_ptr(), mesh.as_raw(), max_nodes) };
        if detour_status_failed(status) {
            unsafe { rustycore_dt_free_nav_mesh_query(raw.as_ptr()) };
            return Err(DetourNavMeshQueryError::InitFailed { status });
        }

        Ok(Self {
            raw,
            _mesh_lifetime_and_thread_model: PhantomData,
        })
    }

    #[must_use]
    pub const fn as_raw(&self) -> *mut RawDetourNavMeshQuery {
        self.raw.as_ptr()
    }

    pub fn find_nearest_poly(
        &self,
        center: [f32; 3],
        half_extents: [f32; 3],
        filter: &DetourQueryFilter,
    ) -> Result<DetourNearestPoly, DetourNavMeshQueryError> {
        let mut poly_ref = 0;
        let mut nearest_point = [0.0; 3];
        let status = unsafe {
            rustycore_dt_nav_mesh_query_find_nearest_poly(
                self.raw.as_ptr(),
                center.as_ptr(),
                half_extents.as_ptr(),
                filter.as_raw(),
                &mut poly_ref,
                nearest_point.as_mut_ptr(),
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::FindNearestPolyFailed { status });
        }

        Ok(DetourNearestPoly {
            poly_ref,
            nearest_point,
        })
    }

    pub fn find_path(
        &self,
        start_ref: DetourPolyRef,
        end_ref: DetourPolyRef,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        filter: &DetourQueryFilter,
        max_path: usize,
    ) -> Result<Vec<DetourPolyRef>, DetourNavMeshQueryError> {
        if max_path > i32::MAX as usize {
            return Err(DetourNavMeshQueryError::PathBufferTooLarge { max_path });
        }

        let mut path = vec![0; max_path];
        let mut path_count = 0;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_find_path(
                self.raw.as_ptr(),
                start_ref,
                end_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                filter.as_raw(),
                path.as_mut_ptr(),
                &mut path_count,
                max_path as i32,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::FindPathFailed { status });
        }

        path.truncate(path_count.max(0) as usize);
        Ok(path)
    }

    pub fn find_straight_path(
        &self,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        path: &[DetourPolyRef],
        max_straight_path: usize,
        options: i32,
    ) -> Result<Vec<DetourStraightPathPoint>, DetourNavMeshQueryError> {
        if path.len() > i32::MAX as usize {
            return Err(DetourNavMeshQueryError::PathBufferTooLarge {
                max_path: path.len(),
            });
        }
        if max_straight_path > i32::MAX as usize {
            return Err(DetourNavMeshQueryError::StraightPathBufferTooLarge { max_straight_path });
        }

        let mut positions = vec![0.0; max_straight_path.saturating_mul(3)];
        let mut flags = vec![0; max_straight_path];
        let mut refs = vec![0; max_straight_path];
        let mut count = 0;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_find_straight_path(
                self.raw.as_ptr(),
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                path.as_ptr(),
                path.len() as i32,
                positions.as_mut_ptr(),
                flags.as_mut_ptr(),
                refs.as_mut_ptr(),
                &mut count,
                max_straight_path as i32,
                options,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::FindStraightPathFailed { status });
        }

        let count = count.max(0) as usize;
        Ok((0..count)
            .map(|i| DetourStraightPathPoint {
                position: [positions[i * 3], positions[i * 3 + 1], positions[i * 3 + 2]],
                flags: flags[i],
                poly_ref: refs[i],
            })
            .collect())
    }

    pub fn closest_point_on_poly(
        &self,
        poly_ref: DetourPolyRef,
        position: [f32; 3],
    ) -> Result<([f32; 3], bool), DetourNavMeshQueryError> {
        let mut closest = [0.0; 3];
        let mut position_over_poly = false;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_closest_point_on_poly(
                self.raw.as_ptr(),
                poly_ref,
                position.as_ptr(),
                closest.as_mut_ptr(),
                &mut position_over_poly,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::ClosestPointOnPolyFailed { status });
        }

        Ok((closest, position_over_poly))
    }

    pub fn closest_point_on_poly_boundary(
        &self,
        poly_ref: DetourPolyRef,
        position: [f32; 3],
    ) -> Result<[f32; 3], DetourNavMeshQueryError> {
        let mut closest = [0.0; 3];
        let status = unsafe {
            rustycore_dt_nav_mesh_query_closest_point_on_poly_boundary(
                self.raw.as_ptr(),
                poly_ref,
                position.as_ptr(),
                closest.as_mut_ptr(),
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::ClosestPointOnPolyBoundaryFailed { status });
        }

        Ok(closest)
    }

    pub fn get_poly_height(
        &self,
        poly_ref: DetourPolyRef,
        position: [f32; 3],
    ) -> Result<f32, DetourNavMeshQueryError> {
        let mut height = 0.0;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_get_poly_height(
                self.raw.as_ptr(),
                poly_ref,
                position.as_ptr(),
                &mut height,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::GetPolyHeightFailed { status });
        }

        Ok(height)
    }

    pub fn move_along_surface(
        &self,
        start_ref: DetourPolyRef,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        filter: &DetourQueryFilter,
        max_visited_size: usize,
    ) -> Result<DetourMoveAlongSurface, DetourNavMeshQueryError> {
        if max_visited_size > i32::MAX as usize {
            return Err(DetourNavMeshQueryError::VisitedBufferTooLarge { max_visited_size });
        }

        let mut result_position = [0.0; 3];
        let mut visited = vec![0; max_visited_size];
        let mut visited_count = 0;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_move_along_surface(
                self.raw.as_ptr(),
                start_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                filter.as_raw(),
                result_position.as_mut_ptr(),
                visited.as_mut_ptr(),
                &mut visited_count,
                max_visited_size as i32,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::MoveAlongSurfaceFailed { status });
        }

        visited.truncate(visited_count.max(0) as usize);
        Ok(DetourMoveAlongSurface {
            result_position,
            visited,
        })
    }

    pub fn raycast(
        &self,
        start_ref: DetourPolyRef,
        start_pos: [f32; 3],
        end_pos: [f32; 3],
        filter: &DetourQueryFilter,
        max_path: usize,
    ) -> Result<DetourRaycast, DetourNavMeshQueryError> {
        if max_path > i32::MAX as usize {
            return Err(DetourNavMeshQueryError::PathBufferTooLarge { max_path });
        }

        let mut hit_t = 0.0;
        let mut hit_normal = [0.0; 3];
        let mut path = vec![0; max_path];
        let mut path_count = 0;
        let status = unsafe {
            rustycore_dt_nav_mesh_query_raycast(
                self.raw.as_ptr(),
                start_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                filter.as_raw(),
                &mut hit_t,
                hit_normal.as_mut_ptr(),
                path.as_mut_ptr(),
                &mut path_count,
                max_path as i32,
            )
        };
        if detour_status_failed(status) {
            return Err(DetourNavMeshQueryError::RaycastFailed { status });
        }

        path.truncate(path_count.max(0) as usize);
        Ok(DetourRaycast {
            hit_t,
            hit_normal,
            path,
        })
    }
}

impl Drop for DetourNavMeshQuery<'_> {
    fn drop(&mut self) {
        unsafe { rustycore_dt_free_nav_mesh_query(self.raw.as_ptr()) };
    }
}

#[derive(Debug)]
pub struct MMapNavMeshQuery {
    pub(crate) raw: NonNull<RawDetourNavMeshQuery>,
    pub(crate) _not_send_or_sync: PhantomData<Rc<()>>,
}

impl MMapNavMeshQuery {
    pub fn new(mesh: &DetourNavMesh, max_nodes: i32) -> Result<Self, DetourNavMeshQueryError> {
        let raw = NonNull::new(unsafe { rustycore_dt_alloc_nav_mesh_query() })
            .ok_or(DetourNavMeshQueryError::AllocationFailed)?;
        let status =
            unsafe { rustycore_dt_nav_mesh_query_init(raw.as_ptr(), mesh.as_raw(), max_nodes) };
        if detour_status_failed(status) {
            unsafe { rustycore_dt_free_nav_mesh_query(raw.as_ptr()) };
            return Err(DetourNavMeshQueryError::InitFailed { status });
        }

        Ok(Self {
            raw,
            _not_send_or_sync: PhantomData,
        })
    }

    #[must_use]
    pub const fn as_raw(&self) -> *mut RawDetourNavMeshQuery {
        self.raw.as_ptr()
    }
}

impl Drop for MMapNavMeshQuery {
    fn drop(&mut self) {
        unsafe { rustycore_dt_free_nav_mesh_query(self.raw.as_ptr()) };
    }
}

#[derive(Debug)]
pub struct DetourQueryFilter {
    pub(crate) raw: NonNull<RawDetourQueryFilter>,
    pub(crate) _not_send_or_sync: PhantomData<Rc<()>>,
}

impl DetourQueryFilter {
    pub fn new() -> Result<Self, DetourQueryFilterError> {
        let raw = NonNull::new(unsafe { rustycore_dt_alloc_query_filter() })
            .ok_or(DetourQueryFilterError::AllocationFailed)?;

        Ok(Self {
            raw,
            _not_send_or_sync: PhantomData,
        })
    }

    #[must_use]
    pub fn include_flags(&self) -> u16 {
        unsafe { rustycore_dt_query_filter_get_include_flags(self.raw.as_ptr()) }
    }

    pub fn set_include_flags(&mut self, flags: u16) {
        unsafe { rustycore_dt_query_filter_set_include_flags(self.raw.as_ptr(), flags) };
    }

    #[must_use]
    pub fn exclude_flags(&self) -> u16 {
        unsafe { rustycore_dt_query_filter_get_exclude_flags(self.raw.as_ptr()) }
    }

    pub fn set_exclude_flags(&mut self, flags: u16) {
        unsafe { rustycore_dt_query_filter_set_exclude_flags(self.raw.as_ptr(), flags) };
    }

    pub fn area_cost(&self, area: usize) -> Result<f32, DetourQueryFilterError> {
        let area = validate_area_index(area)?;
        Ok(unsafe { rustycore_dt_query_filter_get_area_cost(self.raw.as_ptr(), area) })
    }

    pub fn set_area_cost(&mut self, area: usize, cost: f32) -> Result<(), DetourQueryFilterError> {
        let area = validate_area_index(area)?;
        unsafe { rustycore_dt_query_filter_set_area_cost(self.raw.as_ptr(), area, cost) };
        Ok(())
    }

    #[must_use]
    pub const fn as_raw(&self) -> *mut RawDetourQueryFilter {
        self.raw.as_ptr()
    }
}

impl Drop for DetourQueryFilter {
    fn drop(&mut self) {
        unsafe { rustycore_dt_free_query_filter(self.raw.as_ptr()) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathQueryFilterOwner {
    Creature {
        can_walk: bool,
        can_enter_water: bool,
        in_combat: bool,
        in_evade_mode: bool,
    },
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathQueryFilterContext {
    pub owner: PathQueryFilterOwner,
    pub force_enabled_flags: NavTerrainFlag,
    pub force_disabled_flags: NavTerrainFlag,
    pub is_in_water: bool,
    pub is_under_water: bool,
    pub current_nav_terrain: NavTerrainFlag,
}

impl PathQueryFilterContext {
    #[must_use]
    pub const fn creature(
        can_walk: bool,
        can_enter_water: bool,
        in_combat: bool,
        in_evade_mode: bool,
    ) -> Self {
        Self {
            owner: PathQueryFilterOwner::Creature {
                can_walk,
                can_enter_water,
                in_combat,
                in_evade_mode,
            },
            force_enabled_flags: NavTerrainFlag::EMPTY,
            force_disabled_flags: NavTerrainFlag::EMPTY,
            is_in_water: false,
            is_under_water: false,
            current_nav_terrain: NavTerrainFlag::GROUND,
        }
    }

    #[must_use]
    pub const fn player() -> Self {
        Self {
            owner: PathQueryFilterOwner::Player,
            force_enabled_flags: NavTerrainFlag::EMPTY,
            force_disabled_flags: NavTerrainFlag::EMPTY,
            is_in_water: false,
            is_under_water: false,
            current_nav_terrain: NavTerrainFlag::GROUND,
        }
    }
}

pub fn create_path_query_filter_like_cpp(
    context: PathQueryFilterContext,
) -> Result<DetourQueryFilter, DetourQueryFilterError> {
    let mut filter = DetourQueryFilter::new()?;
    let include_flags = match context.owner {
        PathQueryFilterOwner::Creature {
            can_walk,
            can_enter_water,
            ..
        } => {
            let mut flags = NavTerrainFlag::EMPTY;
            if can_walk {
                flags |= NavTerrainFlag::GROUND;
            }
            if can_enter_water {
                flags |= NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME;
            }
            flags
        }
        PathQueryFilterOwner::Player => {
            NavTerrainFlag::GROUND | NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME
        }
    };

    filter.set_include_flags(include_flags.bits());
    filter.set_exclude_flags(NavTerrainFlag::EMPTY.bits());
    update_path_query_filter_like_cpp(&mut filter, context);
    Ok(filter)
}

pub fn update_path_query_filter_like_cpp(
    filter: &mut DetourQueryFilter,
    context: PathQueryFilterContext,
) {
    filter.set_include_flags(filter.include_flags() | context.force_enabled_flags.bits());
    filter.set_exclude_flags(filter.exclude_flags() | context.force_disabled_flags.bits());

    if context.is_in_water || context.is_under_water {
        filter.set_include_flags(filter.include_flags() | context.current_nav_terrain.bits());
    }

    if let PathQueryFilterOwner::Creature {
        in_combat,
        in_evade_mode,
        ..
    } = context.owner
    {
        if in_combat || in_evade_mode {
            filter.set_include_flags(filter.include_flags() | NavTerrainFlag::GROUND_STEEP.bits());
        }
    }
}
