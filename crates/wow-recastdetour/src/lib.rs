use std::{
    collections::HashMap,
    fs, io,
    marker::PhantomData,
    mem::ManuallyDrop,
    path::{Path, PathBuf},
    ptr::NonNull,
    rc::Rc,
};

use thiserror::Error;

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
const PATH_POLY_ACCEPT_DISTANCE_SQ_LIKE_CPP: f32 = 3.0;
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

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct NavTerrainFlag: u16 {
        const EMPTY = 0x00;
        const GROUND = 1 << (NAV_AREA_MAX_VALUE_LIKE_CPP - NAV_AREA_GROUND_LIKE_CPP);
        const GROUND_STEEP = 1 << (NAV_AREA_MAX_VALUE_LIKE_CPP - NAV_AREA_GROUND_STEEP_LIKE_CPP);
        const WATER = 1 << (NAV_AREA_MAX_VALUE_LIKE_CPP - NAV_AREA_WATER_LIKE_CPP);
        const MAGMA_SLIME = 1 << (NAV_AREA_MAX_VALUE_LIKE_CPP - NAV_AREA_MAGMA_SLIME_LIKE_CPP);
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DetourPathType: u8 {
        const BLANK = 0x00;
        const NORMAL = 0x01;
        const SHORTCUT = 0x02;
        const INCOMPLETE = 0x04;
        const NOPATH = 0x08;
        const NOT_USING_PATH = 0x10;
        const SHORT = 0x20;
        const FARFROMPOLY_START = 0x40;
        const FARFROMPOLY_END = 0x80;
        const FARFROMPOLY = Self::FARFROMPOLY_START.bits() | Self::FARFROMPOLY_END.bits();
    }
}

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
    _private: [u8; 0],
}

#[repr(C)]
pub struct RawDetourNavMeshQuery {
    _private: [u8; 0],
}

#[repr(C)]
pub struct RawDetourQueryFilter {
    _private: [u8; 0],
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

unsafe extern "C" {
    fn rustycore_dt_alloc_nav_mesh() -> *mut RawDetourNavMesh;
    fn rustycore_dt_free_nav_mesh(mesh: *mut RawDetourNavMesh);
    fn rustycore_dt_nav_mesh_init(
        mesh: *mut RawDetourNavMesh,
        params: *const DetourNavMeshParams,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_get_max_tiles(mesh: *const RawDetourNavMesh) -> u32;
    fn rustycore_dt_nav_mesh_calc_tile_loc(
        mesh: *const RawDetourNavMesh,
        position: *const f32,
        tile_x: *mut i32,
        tile_y: *mut i32,
    );
    fn rustycore_dt_nav_mesh_has_tile_at(
        mesh: *const RawDetourNavMesh,
        tile_x: i32,
        tile_y: i32,
        layer: i32,
    ) -> bool;
    fn rustycore_dt_nav_mesh_add_tile_copy(
        mesh: *mut RawDetourNavMesh,
        data: *const u8,
        data_size: i32,
        flags: i32,
        result: *mut DetourTileRef,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_remove_tile(
        mesh: *mut RawDetourNavMesh,
        tile_ref: DetourTileRef,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_get_off_mesh_connection_poly_end_points(
        mesh: *const RawDetourNavMesh,
        prev_ref: DetourPolyRef,
        poly_ref: DetourPolyRef,
        start_pos: *mut f32,
        end_pos: *mut f32,
    ) -> DetourStatus;
    fn rustycore_dt_alloc_nav_mesh_query() -> *mut RawDetourNavMeshQuery;
    fn rustycore_dt_free_nav_mesh_query(query: *mut RawDetourNavMeshQuery);
    fn rustycore_dt_nav_mesh_query_init(
        query: *mut RawDetourNavMeshQuery,
        mesh: *const RawDetourNavMesh,
        max_nodes: i32,
    ) -> DetourStatus;
    fn rustycore_dt_alloc_query_filter() -> *mut RawDetourQueryFilter;
    fn rustycore_dt_free_query_filter(filter: *mut RawDetourQueryFilter);
    fn rustycore_dt_query_filter_get_include_flags(filter: *const RawDetourQueryFilter) -> u16;
    fn rustycore_dt_query_filter_set_include_flags(filter: *mut RawDetourQueryFilter, flags: u16);
    fn rustycore_dt_query_filter_get_exclude_flags(filter: *const RawDetourQueryFilter) -> u16;
    fn rustycore_dt_query_filter_set_exclude_flags(filter: *mut RawDetourQueryFilter, flags: u16);
    fn rustycore_dt_query_filter_get_area_cost(
        filter: *const RawDetourQueryFilter,
        area: i32,
    ) -> f32;
    fn rustycore_dt_query_filter_set_area_cost(
        filter: *mut RawDetourQueryFilter,
        area: i32,
        cost: f32,
    );
    fn rustycore_dt_nav_mesh_query_find_nearest_poly(
        query: *const RawDetourNavMeshQuery,
        center: *const f32,
        half_extents: *const f32,
        filter: *const RawDetourQueryFilter,
        nearest_ref: *mut DetourPolyRef,
        nearest_point: *mut f32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_find_path(
        query: *const RawDetourNavMeshQuery,
        start_ref: DetourPolyRef,
        end_ref: DetourPolyRef,
        start_pos: *const f32,
        end_pos: *const f32,
        filter: *const RawDetourQueryFilter,
        path: *mut DetourPolyRef,
        path_count: *mut i32,
        max_path: i32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_find_straight_path(
        query: *const RawDetourNavMeshQuery,
        start_pos: *const f32,
        end_pos: *const f32,
        path: *const DetourPolyRef,
        path_size: i32,
        straight_path: *mut f32,
        straight_path_flags: *mut u8,
        straight_path_refs: *mut DetourPolyRef,
        straight_path_count: *mut i32,
        max_straight_path: i32,
        options: i32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_closest_point_on_poly(
        query: *const RawDetourNavMeshQuery,
        poly_ref: DetourPolyRef,
        position: *const f32,
        closest: *mut f32,
        position_over_poly: *mut bool,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_closest_point_on_poly_boundary(
        query: *const RawDetourNavMeshQuery,
        poly_ref: DetourPolyRef,
        position: *const f32,
        closest: *mut f32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_get_poly_height(
        query: *const RawDetourNavMeshQuery,
        poly_ref: DetourPolyRef,
        position: *const f32,
        height: *mut f32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_move_along_surface(
        query: *const RawDetourNavMeshQuery,
        start_ref: DetourPolyRef,
        start_pos: *const f32,
        end_pos: *const f32,
        filter: *const RawDetourQueryFilter,
        result_pos: *mut f32,
        visited: *mut DetourPolyRef,
        visited_count: *mut i32,
        max_visited_size: i32,
    ) -> DetourStatus;
    fn rustycore_dt_nav_mesh_query_raycast(
        query: *const RawDetourNavMeshQuery,
        start_ref: DetourPolyRef,
        start_pos: *const f32,
        end_pos: *const f32,
        filter: *const RawDetourQueryFilter,
        hit_t: *mut f32,
        hit_normal: *mut f32,
        path: *mut DetourPolyRef,
        path_count: *mut i32,
        max_path: i32,
    ) -> DetourStatus;
    fn rustycore_dt_free(ptr: *mut std::ffi::c_void);
    fn rustycore_dt_create_square_tile_data(
        tile_x: i32,
        tile_y: i32,
        out_data: *mut *mut u8,
        out_data_size: *mut i32,
    ) -> bool;
    // Test-only fixture builder; see `test_fixtures::obstacle_ring_tile_blob`.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[allow(clippy::too_many_arguments)]
    fn rustycore_dt_create_poly_mesh_tile_data(
        tile_x: i32,
        tile_y: i32,
        verts: *const u16,
        vert_count: i32,
        polys: *const u16,
        poly_count: i32,
        nvp: i32,
        poly_flags: *const u16,
        poly_areas: *const u8,
        bmin: *const f32,
        bmax: *const f32,
        cs: f32,
        ch: f32,
        walkable_height: f32,
        walkable_radius: f32,
        walkable_climb: f32,
        out_data: *mut *mut u8,
        out_data_size: *mut i32,
    ) -> bool;
}

#[derive(Debug)]
pub struct DetourNavMesh {
    raw: NonNull<RawDetourNavMesh>,
    _not_send_or_sync: PhantomData<Rc<()>>,
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
    raw: NonNull<RawDetourNavMeshQuery>,
    _mesh_lifetime_and_thread_model: PhantomData<(&'mesh DetourNavMesh, Rc<()>)>,
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
    raw: NonNull<RawDetourNavMeshQuery>,
    _not_send_or_sync: PhantomData<Rc<()>>,
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
    raw: NonNull<RawDetourQueryFilter>,
    _not_send_or_sync: PhantomData<Rc<()>>,
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

/// C++ `PathGenerator::GetPathPolyByPosition` (`PathGenerator.cpp:94-123`).
///
/// Scans the *current* corridor for the polygon closest to `point`, breaking
/// early once its 3D squared distance is below `1.0`, and accepts the result
/// only while that squared distance is below the literal `3.0` (an effective
/// linear radius of `sqrt(3)`, not 3 yards). The returned distance is the real
/// distance to the closest polygon even when the reference is rejected,
/// matching C++ writing `*distance` before its `minDist < 3.0f` test.
///
/// The audited legacy source and TrinityCore's `3.3.5`, `cata_classic` and
/// `master` branches all retain this squared comparison. Changing it to `9.0`
/// would be a plausible optimization, but would also change corridor selection
/// around close stacked surfaces without evidence that C++ intended a
/// three-yard linear radius, so this port preserves the observable heuristic.
pub fn get_path_poly_by_position_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    poly_path: &[DetourPolyRef],
    point: [f32; 3],
) -> (DetourPolyRef, f32) {
    if poly_path.is_empty() {
        return (0, f32::MAX);
    }

    let mut nearest_poly = 0;
    let mut min_dist_sq = f32::MAX;
    for poly_ref in poly_path.iter().copied() {
        let Ok((closest, _)) = query.closest_point_on_poly(poly_ref, point) else {
            continue;
        };
        let dist_sq = detour_distance_sq(point, closest);
        if dist_sq < min_dist_sq {
            min_dist_sq = dist_sq;
            nearest_poly = poly_ref;
        }
        if min_dist_sq < 1.0 {
            break;
        }
    }

    let distance = min_dist_sq.sqrt();
    if min_dist_sq < PATH_POLY_ACCEPT_DISTANCE_SQ_LIKE_CPP {
        (nearest_poly, distance)
    } else {
        (0, distance)
    }
}

/// C++ `PathGenerator::GetPolyByLocation` (`PathGenerator.cpp:125-158`).
///
/// It checks the current corridor first and only falls back to the expensive
/// `findNearestPoly`, first with a low search box and then with a taller one.
pub fn get_poly_by_location_with_previous_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    previous_poly_refs: &[DetourPolyRef],
    point: [f32; 3],
) -> Result<(DetourPolyRef, f32), DetourNavMeshQueryError> {
    let (from_path, path_distance) =
        get_path_poly_by_position_like_cpp(query, previous_poly_refs, point);
    if from_path != 0 {
        return Ok((from_path, path_distance));
    }

    let low = query.find_nearest_poly(point, [3.0, 5.0, 3.0], filter)?;
    if low.poly_ref != 0 {
        return Ok((low.poly_ref, detour_distance(low.nearest_point, point)));
    }

    let high = query.find_nearest_poly(point, [3.0, 50.0, 3.0], filter)?;
    if high.poly_ref != 0 {
        return Ok((high.poly_ref, detour_distance(high.nearest_point, point)));
    }

    Ok((0, f32::MAX))
}

/// [`get_poly_by_location_with_previous_path_like_cpp`] with no prior corridor,
/// i.e. the C++ behaviour on a freshly constructed `PathGenerator`.
pub fn get_poly_by_location_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    point: [f32; 3],
) -> Result<(DetourPolyRef, f32), DetourNavMeshQueryError> {
    get_poly_by_location_with_previous_path_like_cpp(query, filter, &[], point)
}

/// Internal result of C++ `PathGenerator::BuildPointPath`
/// (`PathGenerator.cpp:530-622`).
///
/// `BuildShortcut()` calls `Clear()` before installing the two direct points
/// (`PathGenerator.h:117-121`, `PathGenerator.cpp:630-645`), so point data
/// cannot be returned without its corridor-lifecycle decision.
#[derive(Debug, Clone, PartialEq)]
struct BuildPointPathOutcomeLikeCpp {
    point_path: DetourPointPath,
    cleared_poly_path: bool,
}

/// `end_point` is the (possibly clamped) `endPoint` C++ passes as an argument
/// and queries against, while `requested_end_point` is `GetEndPosition()` — the
/// destination `CalculatePath` was originally asked for. The `_forceDestination`
/// block compares against the latter (`PathGenerator.cpp:603-619`), so the two
/// must stay distinct whenever the far-from-poly branch clamped the endpoint.
#[allow(clippy::too_many_arguments)]
fn build_point_path_outcome_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    end_point: [f32; 3],
    requested_end_point: [f32; 3],
    poly_refs: &[DetourPolyRef],
    point_path_limit: usize,
    mut path_type: DetourPathType,
    force_destination: bool,
    use_straight_path: bool,
    use_raycast: bool,
) -> Result<BuildPointPathOutcomeLikeCpp, DetourNavMeshQueryError> {
    if use_raycast {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::NOPATH,
            },
            cleared_poly_path: true,
        });
    }

    let point_result = if use_straight_path {
        query
            .find_straight_path(start_point, end_point, poly_refs, point_path_limit, 0)
            .map(|points| points.into_iter().map(|point| point.position).collect())
    } else {
        find_smooth_path_like_cpp(
            nav_mesh,
            query,
            filter,
            start_point,
            end_point,
            poly_refs,
            point_path_limit,
        )
    };

    // C++ `BuildShortcut()` *assigns* `_type = PATHFIND_SHORTCUT`
    // (`PathGenerator.cpp:645`) and the failure branches then OR onto that
    // fresh value, so the incoming `NORMAL`/`INCOMPLETE`/`FARFROMPOLY_*` bits
    // are discarded rather than merged (`PathGenerator.cpp:575-591`).
    let mut points = match point_result {
        Ok(points) => points,
        Err(_) => {
            return Ok(BuildPointPathOutcomeLikeCpp {
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type: DetourPathType::SHORTCUT | DetourPathType::NOPATH,
                },
                cleared_poly_path: true,
            });
        }
    };

    if poly_refs.len() == 1 && points.len() == 1 {
        points.push(end_point);
    } else if points.len() < 2 {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::SHORTCUT | DetourPathType::NOPATH,
            },
            cleared_poly_path: true,
        });
    } else if points.len() >= point_path_limit {
        return Ok(BuildPointPathOutcomeLikeCpp {
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type: DetourPathType::SHORTCUT | DetourPathType::SHORT,
            },
            cleared_poly_path: true,
        });
    }

    // C++ `SetActualEndPosition(_pathPoints[pointCount-1])`.
    let mut actual_end = points.last().copied().unwrap_or(end_point);
    let mut cleared_poly_path = false;
    if force_destination
        && (!path_type.contains(DetourPathType::NORMAL)
            || !detour_in_range(requested_end_point, actual_end, 1.0, 1.0))
    {
        actual_end = requested_end_point;
        if detour_distance_sq(
            points.last().copied().unwrap_or(start_point),
            requested_end_point,
        ) < 0.3 * detour_distance_sq(start_point, requested_end_point)
        {
            if let Some(last) = points.last_mut() {
                *last = requested_end_point;
            }
        } else {
            // C++ `BuildShortcut()`: current position -> actual end position,
            // which the branch above has just set to the requested destination.
            points = vec![start_point, requested_end_point];
            cleared_poly_path = true;
        }
        path_type = DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH;
    }

    Ok(BuildPointPathOutcomeLikeCpp {
        point_path: DetourPointPath {
            points,
            actual_end,
            path_type,
        },
        cleared_poly_path,
    })
}

pub fn reuse_previous_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    previous_poly_refs: &[DetourPolyRef],
    start_poly: DetourPolyRef,
    end_poly: DetourPolyRef,
    end_point: [f32; 3],
    use_raycast: bool,
) -> Result<PreviousPolyPathLikeCpp, DetourNavMeshQueryError> {
    if previous_poly_refs.is_empty() {
        return Ok(PreviousPolyPathLikeCpp::Recalculate);
    }

    let Some(path_start_index) = previous_poly_refs
        .iter()
        .position(|poly| *poly == start_poly)
    else {
        return Ok(PreviousPolyPathLikeCpp::Recalculate);
    };

    let path_end_index = previous_poly_refs
        .iter()
        .enumerate()
        .skip(path_start_index + 1)
        .rev()
        .find_map(|(index, poly)| (*poly == end_poly).then_some(index));

    if let Some(path_end_index) = path_end_index {
        return Ok(PreviousPolyPathLikeCpp::PolyRefs(
            previous_poly_refs[path_start_index..=path_end_index].to_vec(),
        ));
    }

    let remaining = &previous_poly_refs[path_start_index..];
    let mut prefix_poly_length = ((remaining.len() as f32) * 0.8 + 0.5) as usize;
    prefix_poly_length = prefix_poly_length.clamp(1, remaining.len());
    let mut prefix = remaining[..prefix_poly_length].to_vec();

    let mut suffix_start_poly = *prefix.last().unwrap();
    let suffix_end_point = match query.closest_point_on_poly(suffix_start_poly, end_point) {
        Ok((closest, _)) => closest,
        Err(_) if prefix.len() > 1 => {
            prefix.pop();
            suffix_start_poly = *prefix.last().unwrap();
            match query.closest_point_on_poly(suffix_start_poly, end_point) {
                Ok((closest, _)) => closest,
                Err(_) => return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath),
            }
        }
        Err(_) => return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath),
    };

    if use_raycast {
        return Ok(PreviousPolyPathLikeCpp::ShortcutNoPath);
    }

    let max_suffix_path = MAX_PATH_LENGTH_LIKE_CPP.saturating_sub(prefix.len());
    let suffix = query
        .find_path(
            suffix_start_poly,
            end_poly,
            suffix_end_point,
            end_point,
            filter,
            max_suffix_path,
        )
        .unwrap_or_default();

    if suffix.is_empty() {
        // C++ deliberately keeps the valid prefix after an empty/failed suffix
        // so the creature can advance and recover on the next update
        // (`PathGenerator.cpp:401-412`). Its final `prefix + suffix - overlap`
        // arithmetic assumes `findPath` returned at least `suffixStartPoly`;
        // with an empty suffix there is no overlap to remove. Reproducing the
        // unconditional `- 1` discards a valid last prefix polygon and can make
        // a remote destination look like a same-poly direct path.
        if prefix.len() == 1 {
            // A singleton cannot provide a useful retained segment. Recalculate
            // rather than reproduce C++'s zero-length underflow.
            return Ok(PreviousPolyPathLikeCpp::Recalculate);
        }
        return Ok(PreviousPolyPathLikeCpp::PolyRefs(prefix));
    }

    prefix.pop();
    prefix.extend(suffix);
    Ok(PreviousPolyPathLikeCpp::PolyRefs(prefix))
}

/// C++ `PathGenerator::BuildPolyPath` (`PathGenerator.cpp:160-528`).
///
/// This produces the *polygon corridor* and `_type`. Following C++, it does
/// **not** build the point path itself: the branches that answer with
/// `BuildShortcut()` return terminal results whose `point_path.points` are
/// already populated, while the branches that fall through to
/// `BuildPointPath(startPoint, endPoint)` (`PathGenerator.cpp:287` and
/// `:527`) return an **empty** `point_path.points` so the caller runs
/// `build_point_path_outcome_like_cpp` exactly once, in the mode
/// `_useStraightPath`/`_useRaycast` selects.
///
/// `point_path.actual_end` carries the possibly clamped `endPoint`, mirroring
/// the `SetActualEndPosition(closestPointOnPoly(endPoly, endPoint))` the
/// far-from-poly branch performs (`PathGenerator.cpp:253-259`); the caller must
/// feed it back into `BuildPointPath` the way C++ passes its mutated local
/// `endPoint`.
pub fn build_straight_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    mut end_point: [f32; 3],
    owner: DetourOwnerCapabilitiesLikeCpp,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    // C++ resolves both endpoints through `GetPolyByLocation`, which consults the
    // corridor it already holds before paying for `findNearestPoly`
    // (`PathGenerator.cpp:125-158`); the distance it reports there feeds the
    // `> 7.0f` far-from-poly decision below.
    let (start_poly, dist_to_start_poly) = get_poly_by_location_with_previous_path_like_cpp(
        query,
        filter,
        previous_poly_refs,
        start_point,
    )?;
    let (end_poly, dist_to_end_poly) = get_poly_by_location_with_previous_path_like_cpp(
        query,
        filter,
        previous_poly_refs,
        end_point,
    )?;

    if start_poly == 0 || end_poly == 0 {
        // C++ runs `BuildShortcut()` first, then grants
        // `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH` when the owner `CanFly()`,
        // or `CanSwim()` with every shortcut point in liquid
        // (`PathGenerator.cpp:178-202`). Otherwise, and while `!_useRaycast`, it
        // *assigns* `_type = PATHFIND_NOPATH` (`:207`), discarding the
        // `PATHFIND_SHORTCUT` that `BuildShortcut()` had set.
        //
        // Boundary: `waterPath` needs `Map::GetLiquidStatus` for each shortcut
        // point, which this layer has no access to. A swimming owner therefore
        // still falls through to `PATHFIND_NOPATH` here, exactly as it did
        // before — the C++ `waterPath` loop can only ever *reduce* `waterPath`
        // to false, so treating "no liquid data" as "not fully submerged" never
        // grants a shortcut C++ would have withheld.
        let path_type = if owner.can_fly {
            DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
        } else {
            DetourPathType::NOPATH
        };
        return Ok(DetourPolyPath {
            poly_refs: Vec::new(),
            point_path: DetourPointPath {
                points: vec![start_point, end_point],
                actual_end: end_point,
                path_type,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        });
    }

    let start_far_from_poly = dist_to_start_poly > 7.0;
    let end_far_from_poly = dist_to_end_poly > 7.0;
    let mut path_type = DetourPathType::NORMAL;
    if start_far_from_poly || end_far_from_poly {
        // C++ picks the offending endpoint, and when it is *not* under water
        // allows a direct shortcut for a flying owner, or for one that is
        // falling towards a lower destination — the charge case
        // (`PathGenerator.cpp:221-240`).
        //
        // Boundary: C++ selects the offending endpoint
        // (`(distToStartPoly > 7.0f) ? startPos : endPos`) only to ask
        // `Map::IsUnderWater` about it and, if so, consult `CanSwim()` instead.
        // That liquid lookup is unavailable at this layer, so the "flying case"
        // arm is always taken — the same arm Rust already took unconditionally.
        // This only adds the shortcuts C++ grants inside it.
        //
        // Detour space is Y-up and `wow_position_to_detour_like_cpp` maps WoW z
        // onto index 1, so index 1 is the height C++ compares as `endPos.z <
        // startPos.z`.
        let build_shortcut = owner.can_fly || (owner.is_falling && end_point[1] < start_point[1]);
        if build_shortcut {
            let mut path_type = DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            return Ok(DetourPolyPath {
                poly_refs: Vec::new(),
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type,
                },
                start_far_from_poly,
                end_far_from_poly,
            });
        }

        if let Ok((closest, _)) = query.closest_point_on_poly(end_poly, end_point) {
            end_point = closest;
        }
        path_type = DetourPathType::INCOMPLETE;
        add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);
    }

    // C++ `BuildShortcut(); _type = PATHFIND_NOPATH;` with no
    // `AddFarFromPolyFlags` afterwards (`PathGenerator.cpp:371-374`, `:508-515`).
    let shortcut_no_path = || DetourPolyPath {
        poly_refs: Vec::new(),
        point_path: DetourPointPath {
            points: vec![start_point, end_point],
            actual_end: end_point,
            path_type: DetourPathType::NOPATH,
        },
        start_far_from_poly,
        end_far_from_poly,
    };

    let poly_refs = if start_poly == end_poly {
        // C++ `PathGenerator.cpp:271-289` treats this as a one-polygon corridor
        // and hands it straight to `BuildPointPath`.
        vec![start_poly]
    } else {
        // C++ `PathGenerator.cpp:291-413` first tries to reuse the corridor the
        // generator already holds: an exact subpath when both polygons are still
        // on it, or an ~80% prefix plus a freshly generated suffix when only the
        // start polygon is.
        match reuse_previous_poly_path_like_cpp(
            query,
            filter,
            previous_poly_refs,
            start_poly,
            end_poly,
            end_point,
            false,
        )? {
            PreviousPolyPathLikeCpp::PolyRefs(reused) if !reused.is_empty() => reused,
            PreviousPolyPathLikeCpp::ShortcutNoPath => return Ok(shortcut_no_path()),
            // C++ `Clear()`s and generates the whole corridor when the retained
            // path does not contain the current start polygon (`:414-516`).
            // `reuse_previous_poly_path_like_cpp` also selects this branch as a
            // deliberate safety repair for C++'s one-prefix/empty-suffix
            // zero-length underflow.
            PreviousPolyPathLikeCpp::Recalculate | PreviousPolyPathLikeCpp::PolyRefs(_) => {
                match query.find_path(
                    start_poly,
                    end_poly,
                    start_point,
                    end_point,
                    filter,
                    MAX_PATH_LENGTH_LIKE_CPP,
                ) {
                    Ok(path) if !path.is_empty() => path,
                    // C++ has already `Clear()`ed the previous corridor in
                    // this branch and turns either a failed status or zero
                    // length into `BuildShortcut(); PATHFIND_NOPATH`.
                    Ok(_) | Err(_) => return Ok(shortcut_no_path()),
                }
            }
        }
    };

    if poly_refs.last().copied() == Some(end_poly)
        && !path_type.contains(DetourPathType::INCOMPLETE)
    {
        path_type = DetourPathType::NORMAL;
    } else {
        path_type = DetourPathType::INCOMPLETE;
    }
    add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);

    if start_poly != end_poly && poly_refs.len() == 1 {
        // A successful Detour query may still return only `startPoly` when the
        // destination is on a disconnected island or every neighbour is
        // excluded by the filter. C++ passes the remote endpoint into
        // `FindSmoothPath`, whose singleton shortcut treats it as same-poly and
        // appends a final straight segment across the gap. Keep the valid
        // partial result, but clamp its effective endpoint to the reachable
        // boundary. Genuine same-poly paths retain their requested endpoint.
        match query.closest_point_on_poly_boundary(poly_refs[0], end_point) {
            Ok(boundary) => end_point = boundary,
            // C++ `FindSmoothPath` cannot build a usable singleton partial
            // path when the reachable boundary itself is invalid. Fall back
            // to the same cleared shortcut/NOPATH state as its failed point
            // query branches instead of retaining the remote endpoint.
            Err(_) => return Ok(shortcut_no_path()),
        }
    }

    Ok(DetourPolyPath {
        poly_refs,
        point_path: DetourPointPath {
            points: Vec::new(),
            actual_end: end_point,
            path_type,
        },
        start_far_from_poly,
        end_far_from_poly,
    })
}

pub fn build_raycast_poly_path_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_point: [f32; 3],
    mut end_point: [f32; 3],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let (start_poly, dist_to_start_poly) =
        get_poly_by_location_like_cpp(query, filter, start_point)?;
    let (end_poly, dist_to_end_poly) = get_poly_by_location_like_cpp(query, filter, end_point)?;
    let start_far_from_poly = dist_to_start_poly > 7.0;
    let end_far_from_poly = dist_to_end_poly > 7.0;

    if start_far_from_poly || end_far_from_poly {
        if let Ok((closest, _)) = query.closest_point_on_poly(end_poly, end_point) {
            end_point = closest;
        }
    }

    let raycast = match query.raycast(
        start_poly,
        start_point,
        end_point,
        filter,
        MAX_PATH_LENGTH_LIKE_CPP,
    ) {
        Ok(raycast) if !raycast.path.is_empty() => raycast,
        Ok(raycast) => {
            let mut path_type = DetourPathType::NOPATH | DetourPathType::SHORTCUT;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            return Ok(DetourPolyPath {
                poly_refs: raycast.path,
                point_path: DetourPointPath {
                    points: vec![start_point, end_point],
                    actual_end: end_point,
                    path_type,
                },
                start_far_from_poly,
                end_far_from_poly,
            });
        }
        Err(error) => {
            let mut path_type = DetourPathType::NOPATH | DetourPathType::SHORTCUT;
            add_far_from_poly_flags_like_cpp(
                &mut path_type,
                start_far_from_poly,
                end_far_from_poly,
            );
            if start_poly == 0 {
                return Ok(DetourPolyPath {
                    poly_refs: Vec::new(),
                    point_path: DetourPointPath {
                        points: vec![start_point, end_point],
                        actual_end: end_point,
                        path_type,
                    },
                    start_far_from_poly,
                    end_far_from_poly,
                });
            }
            return Err(error);
        }
    };

    let last_poly = raycast.path.last().copied().unwrap_or(start_poly);
    if raycast.hit_t != f32::MAX {
        let mut hit_t = raycast.hit_t * 0.99;
        if !hit_t.is_finite() {
            hit_t = 0.0;
        }
        let mut hit_pos = detour_lerp(start_point, end_point, hit_t);
        match query.get_poly_height(last_poly, hit_pos) {
            Ok(height) => hit_pos[1] = height,
            Err(_) => {
                if let Ok(boundary) = query.closest_point_on_poly_boundary(last_poly, hit_pos) {
                    hit_pos = boundary;
                }
            }
        }

        let mut path_type = DetourPathType::INCOMPLETE;
        add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, false);
        return Ok(DetourPolyPath {
            poly_refs: raycast.path,
            point_path: DetourPointPath {
                points: vec![start_point, hit_pos],
                actual_end: hit_pos,
                path_type,
            },
            start_far_from_poly,
            end_far_from_poly,
        });
    }

    match query.get_poly_height(last_poly, end_point) {
        Ok(height) => end_point[1] = height,
        Err(_) => {
            if let Ok(boundary) = query.closest_point_on_poly_boundary(last_poly, end_point) {
                end_point = boundary;
            }
        }
    }

    let mut path_type = if start_far_from_poly || end_far_from_poly {
        DetourPathType::INCOMPLETE
    } else {
        DetourPathType::NORMAL
    };
    add_far_from_poly_flags_like_cpp(&mut path_type, start_far_from_poly, end_far_from_poly);

    Ok(DetourPolyPath {
        poly_refs: raycast.path,
        point_path: DetourPointPath {
            points: vec![start_point, end_point],
            actual_end: end_point,
            path_type,
        },
        start_far_from_poly,
        end_far_from_poly,
    })
}

/// [`calculate_detour_path_with_previous_path_like_cpp`] on a freshly
/// constructed `PathGenerator`, i.e. with no corridor to reuse.
pub fn calculate_detour_path_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    calculate_detour_path_with_previous_path_like_cpp(
        nav_mesh,
        query,
        filter,
        start_wow,
        end_wow,
        options,
        &[],
    )
}

/// C++ `PathGenerator::CalculatePath` from `BuildPolyPath` onwards, for a
/// generator that kept its `PathGenerator` alive across updates and therefore
/// still holds `_pathPolyRefs` (`PathGenerator.cpp:291-413`).
#[allow(clippy::too_many_arguments)]
pub fn calculate_detour_path_with_previous_path_like_cpp(
    nav_mesh: &DetourNavMesh,
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let start_point = wow_position_to_detour_like_cpp(start_wow);
    let end_point = wow_position_to_detour_like_cpp(end_wow);
    let mut poly_path = if options.use_raycast {
        build_raycast_poly_path_like_cpp(query, filter, start_point, end_point)?
    } else {
        build_straight_poly_path_like_cpp(
            query,
            filter,
            start_point,
            end_point,
            options.owner,
            previous_poly_refs,
        )?
    };

    // C++ `BuildPolyPath` runs `BuildPointPath` exactly once, and only on the
    // branches that did not already answer with `BuildShortcut()`
    // (`PathGenerator.cpp:287`, `:527`). Those shortcut branches arrive here
    // with their two points already set, so re-running the point build would
    // both waste a Detour query and let the discarded pass leak
    // `PATHFIND_SHORTCUT`/`PATHFIND_SHORT` into an otherwise usable path.
    if !options.use_raycast && poly_path.point_path.points.is_empty() {
        let point_path_outcome = build_point_path_outcome_like_cpp(
            nav_mesh,
            query,
            filter,
            start_point,
            // C++ passes its own possibly clamped local `endPoint`, while
            // `GetEndPosition()` keeps the originally requested destination.
            poly_path.point_path.actual_end,
            end_point,
            &poly_path.poly_refs,
            options.point_path_limit,
            poly_path.point_path.path_type,
            options.force_destination,
            options.use_straight_path,
            false,
        )?;
        poly_path.point_path = point_path_outcome.point_path;
        if point_path_outcome.cleared_poly_path {
            poly_path.poly_refs.clear();
        }
    }

    for point in &mut poly_path.point_path.points {
        *point = detour_position_to_wow_like_cpp(*point);
    }
    poly_path.point_path.actual_end =
        detour_position_to_wow_like_cpp(poly_path.point_path.actual_end);

    Ok(poly_path)
}

fn calculate_detour_path_with_raw_query_like_cpp(
    nav_mesh: &DetourNavMesh,
    raw_query: *mut RawDetourNavMeshQuery,
    filter: &DetourQueryFilter,
    start_wow: [f32; 3],
    end_wow: [f32; 3],
    options: DetourPathOptions,
    previous_poly_refs: &[DetourPolyRef],
) -> Result<DetourPolyPath, DetourNavMeshQueryError> {
    let Some(raw) = NonNull::new(raw_query) else {
        return Err(DetourNavMeshQueryError::AllocationFailed);
    };
    let query = ManuallyDrop::new(DetourNavMeshQuery {
        raw,
        _mesh_lifetime_and_thread_model: PhantomData,
    });

    calculate_detour_path_with_previous_path_like_cpp(
        nav_mesh,
        &query,
        filter,
        start_wow,
        end_wow,
        options,
        previous_poly_refs,
    )
}

#[must_use]
pub fn fixup_corridor_like_cpp(
    path: &[DetourPolyRef],
    max_path: usize,
    visited: &[DetourPolyRef],
) -> Vec<DetourPolyRef> {
    let mut furthest_path = None;
    let mut furthest_visited = None;

    for i in (0..path.len()).rev() {
        let mut found = false;
        for j in (0..visited.len()).rev() {
            if path[i] == visited[j] {
                furthest_path = Some(i);
                furthest_visited = Some(j);
                found = true;
            }
        }
        if found {
            break;
        }
    }

    let (Some(furthest_path), Some(furthest_visited)) = (furthest_path, furthest_visited) else {
        return path.to_vec();
    };

    let req = visited.len() - furthest_visited;
    let orig = (furthest_path + 1).min(path.len());
    let mut size = path.len().saturating_sub(orig);
    if req + size > max_path {
        size = max_path.saturating_sub(req);
    }

    let mut fixed = Vec::with_capacity((req + size).min(max_path));
    fixed.extend((0..req).map(|i| visited[(visited.len() - 1) - i]));
    fixed.extend_from_slice(&path[orig..orig + size]);
    fixed
}

pub fn get_steer_target_like_cpp(
    query: &DetourNavMeshQuery<'_>,
    start_pos: [f32; 3],
    end_pos: [f32; 3],
    min_target_dist: f32,
    path: &[DetourPolyRef],
) -> Result<Option<DetourSteerTarget>, DetourNavMeshQueryError> {
    const MAX_STEER_POINTS: usize = 3;

    let steer_path = query.find_straight_path(start_pos, end_pos, path, MAX_STEER_POINTS, 0)?;
    if steer_path.is_empty() {
        return Ok(None);
    }

    let steer = steer_path.into_iter().find(|point| {
        point.flags & DT_STRAIGHTPATH_OFFMESH_CONNECTION_LIKE_CPP != 0
            || !detour_in_range(point.position, start_pos, min_target_dist, 1000.0)
    });

    Ok(steer.map(|point| {
        let mut position = point.position;
        position[1] = start_pos[1];
        DetourSteerTarget {
            position,
            flags: point.flags,
            poly_ref: point.poly_ref,
        }
    }))
}

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

fn add_far_from_poly_flags_like_cpp(
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

fn detour_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    detour_distance_sq(left, right).sqrt()
}

fn detour_distance_sq(left: [f32; 3], right: [f32; 3]) -> f32 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    let dz = left[2] - right[2];
    dx * dx + dy * dy + dz * dz
}

fn detour_in_range(first: [f32; 3], second: [f32; 3], range: f32, height: f32) -> bool {
    let dx = second[0] - first[0];
    let dy = second[1] - first[1];
    let dz = second[2] - first[2];
    (dx * dx + dz * dz) < range * range && dy.abs() < height
}

fn detour_lerp(start: [f32; 3], end: [f32; 3], t: f32) -> [f32; 3] {
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

fn validate_area_index(area: usize) -> Result<i32, DetourQueryFilterError> {
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
    nav_mesh_queries: HashMap<(u32, u32), MMapNavMeshQuery>,
    nav_mesh: DetourNavMesh,
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
    loaded_mmaps: HashMap<u32, Option<MMapData>>,
    parent_map_data: HashMap<u32, u32>,
    loaded_tiles: u32,
    thread_safe_environment: bool,
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

    fn read_tile_blob_with_parent_fallback(
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

impl MmapTileHeader {
    #[must_use]
    pub const fn new(dt_version: u32) -> Self {
        Self {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 0,
            uses_liquids: true,
            padding: [0; 3],
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, MmapTileHeaderError> {
        if bytes.len() < MMAP_TILE_HEADER_SIZE_LIKE_CPP {
            return Err(MmapTileHeaderError::TooShort {
                actual: bytes.len(),
                expected: MMAP_TILE_HEADER_SIZE_LIKE_CPP,
            });
        }

        let header = Self {
            mmap_magic: read_u32(bytes, 0),
            dt_version: read_u32(bytes, 4),
            mmap_version: read_u32(bytes, 8),
            size: read_u32(bytes, 12),
            uses_liquids: bytes[16] != 0,
            padding: [bytes[17], bytes[18], bytes[19]],
        };

        if header.mmap_magic != MMAP_MAGIC_LIKE_CPP {
            return Err(MmapTileHeaderError::BadMagic {
                actual: header.mmap_magic,
                expected: MMAP_MAGIC_LIKE_CPP,
            });
        }

        if header.mmap_version != MMAP_VERSION_LIKE_CPP {
            return Err(MmapTileHeaderError::BadMmapVersion {
                actual: header.mmap_version,
                expected: MMAP_VERSION_LIKE_CPP,
            });
        }

        Ok(header)
    }

    pub fn validate_dt_version(&self, expected_dt_version: u32) -> Result<(), MmapTileHeaderError> {
        if self.dt_version != expected_dt_version {
            return Err(MmapTileHeaderError::BadDetourVersion {
                actual: self.dt_version,
                expected: expected_dt_version,
            });
        }

        Ok(())
    }

    #[must_use]
    pub fn to_bytes(self) -> [u8; MMAP_TILE_HEADER_SIZE_LIKE_CPP] {
        let mut bytes = [0; MMAP_TILE_HEADER_SIZE_LIKE_CPP];
        bytes[0..4].copy_from_slice(&self.mmap_magic.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.dt_version.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.mmap_version.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.size.to_le_bytes());
        bytes[16] = u8::from(self.uses_liquids);
        bytes[17..20].copy_from_slice(&self.padding);
        bytes
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum MmapTileHeaderError {
    #[error("mmap tile header is too short: got {actual} bytes, expected {expected}")]
    TooShort { actual: usize, expected: usize },
    #[error("bad mmap magic: got 0x{actual:08x}, expected 0x{expected:08x}")]
    BadMagic { actual: u32, expected: u32 },
    #[error("bad mmap version: got {actual}, expected {expected}")]
    BadMmapVersion { actual: u32, expected: u32 },
    #[error("bad Detour navmesh version: got {actual}, expected {expected}")]
    BadDetourVersion { actual: u32, expected: u32 },
}

#[must_use]
pub const fn pack_tile_id_like_cpp(x: i32, y: i32) -> u32 {
    ((x as u32) << 16) | (y as u32 & 0xffff)
}

#[must_use]
pub fn mmap_tile_coords_for_wow_position_like_cpp(x: f32, y: f32) -> (i32, i32) {
    (
        (CENTER_GRID_ID_LIKE_CPP as f32 - x / SIZE_OF_GRIDS_LIKE_CPP) as i32,
        (CENTER_GRID_ID_LIKE_CPP as f32 - y / SIZE_OF_GRIDS_LIKE_CPP) as i32,
    )
}

#[must_use]
pub fn map_file_name_like_cpp(map_id: u32) -> String {
    format!("mmaps/{map_id:04}.mmap")
}

#[must_use]
pub fn map_file_path_like_cpp(base_path: impl AsRef<Path>, map_id: u32) -> PathBuf {
    base_path.as_ref().join(map_file_name_like_cpp(map_id))
}

#[must_use]
pub fn tile_file_name_like_cpp(map_id: u32, x: i32, y: i32) -> String {
    format!("mmaps/{map_id:04}{x:02}{y:02}.mmtile")
}

#[must_use]
pub fn tile_file_path_like_cpp(
    base_path: impl AsRef<Path>,
    map_id: u32,
    x: i32,
    y: i32,
) -> PathBuf {
    base_path
        .as_ref()
        .join(tile_file_name_like_cpp(map_id, x, y))
}

pub fn read_mmap_tile_blob_file(
    path: impl AsRef<Path>,
    expected_dt_version: u32,
) -> Result<MmapTileBlob, MmapTileFileError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| MmapTileFileError::ReadTileFile {
        path: path.to_path_buf(),
        source,
    })?;

    MmapTileBlob::parse(&bytes, expected_dt_version).map_err(MmapTileFileError::BadTileBlob)
}

#[derive(Debug, Error)]
pub enum MmapTileFileError {
    #[error("failed to read mmap tile file {path:?}: {source}")]
    ReadTileFile { path: PathBuf, source: io::Error },
    #[error("bad mmap tile blob: {0}")]
    BadTileBlob(MmapTileBlobError),
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn read_f32(bytes: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// In-memory Detour navmesh fixtures for tests in this crate and downstream
/// crates.
///
/// Gated behind the `test-fixtures` feature (always on for this crate's own
/// tests) so nothing here reaches a production build.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_fixtures {
    use super::*;

    /// Side length in cells of [`obstacle_ring_tile_blob`].
    pub const OBSTACLE_TILE_CELLS: u16 = 3;
    /// Cell that is left out of [`obstacle_ring_tile_blob`], i.e. the obstacle.
    pub const OBSTACLE_TILE_HOLE: (u16, u16) = (1, 1);
    /// World size of one cell. It has to comfortably exceed
    /// [`SMOOTH_PATH_STEP_SIZE_LIKE_CPP`], otherwise one `FindSmoothPath` step
    /// would jump across the whole fixture and the resulting point list would
    /// say nothing about the route actually taken.
    pub const OBSTACLE_TILE_CELL_SIZE: f32 = 10.0;
    /// World extent of the fixture along one axis.
    pub const OBSTACLE_TILE_EXTENT: f32 = OBSTACLE_TILE_CELL_SIZE * OBSTACLE_TILE_CELLS as f32;

    /// Builds one navmesh tile shaped like a 3x3 grid of square quads with the
    /// centre quad missing, so a straight line through the centre is not
    /// walkable but a route around it is. This is the smallest navmesh that can
    /// distinguish "queried Detour" from "shortcut straight to the
    /// destination".
    ///
    /// Vertices use the same winding as `rustycore_dt_create_square_tile_data`
    /// (`(x,z) -> (x+1,z) -> (x+1,z+1) -> (x,z+1)`, y up), and edge `j` runs
    /// from vertex `j` to vertex `j+1`, so the neighbour half of each
    /// `rcPolyMesh` poly entry is `[-z, +x, +z, -x]`. `MESH_NULL_IDX` (0xffff)
    /// marks a border edge, matching `DetourNavMeshBuilder.cpp:525-546`.
    #[must_use]
    pub fn obstacle_ring_tile_blob(tile_x: i32, tile_y: i32) -> MmapTileBlob {
        obstacle_ring_tile_blob_at(tile_x, tile_y, 0.0, 0.0)
    }

    /// Same fixture, but with its geometry offset to `(origin_detour_x,
    /// origin_detour_z)` so it can be placed inside a navmesh tile other than
    /// `(0, 0)`.
    #[must_use]
    pub fn obstacle_ring_tile_blob_at(
        tile_x: i32,
        tile_y: i32,
        origin_detour_x: f32,
        origin_detour_z: f32,
    ) -> MmapTileBlob {
        obstacle_ring_tile_blob_at_height(tile_x, tile_y, origin_detour_x, 0.0, origin_detour_z)
    }

    /// Height-aware form of [`obstacle_ring_tile_blob_at`].
    ///
    /// Detour uses `(x, y, z)` with `y` vertical, while WoW uses `(x, y, z)`
    /// with `z` vertical. A connected fixture placed in a real world-map grid
    /// therefore has to preserve the live terrain height instead of silently
    /// building its polygons at zero.
    #[must_use]
    pub fn obstacle_ring_tile_blob_at_height(
        tile_x: i32,
        tile_y: i32,
        origin_detour_x: f32,
        origin_detour_y: f32,
        origin_detour_z: f32,
    ) -> MmapTileBlob {
        const NVP: usize = 4;
        const MESH_NULL_IDX: u16 = 0xffff;
        let cells = OBSTACLE_TILE_CELLS;
        let lattice = cells + 1;

        let mut verts: Vec<u16> =
            Vec::with_capacity(usize::from(lattice) * usize::from(lattice) * 3);
        for z in 0..lattice {
            for x in 0..lattice {
                verts.extend_from_slice(&[x, 0, z]);
            }
        }
        let vert_index = |x: u16, z: u16| z * lattice + x;

        // Poly index per walkable cell; the hole has none.
        let mut poly_index_of_cell = vec![MESH_NULL_IDX; usize::from(cells) * usize::from(cells)];
        let mut next_poly_index = 0u16;
        for z in 0..cells {
            for x in 0..cells {
                if (x, z) == OBSTACLE_TILE_HOLE {
                    continue;
                }
                poly_index_of_cell[usize::from(z) * usize::from(cells) + usize::from(x)] =
                    next_poly_index;
                next_poly_index += 1;
            }
        }
        let cell_poly = |x: i32, z: i32| -> u16 {
            if x < 0 || z < 0 || x >= i32::from(cells) || z >= i32::from(cells) {
                return MESH_NULL_IDX;
            }
            poly_index_of_cell[z as usize * usize::from(cells) + x as usize]
        };

        let mut polys: Vec<u16> = Vec::with_capacity(usize::from(next_poly_index) * NVP * 2);
        for z in 0..cells {
            for x in 0..cells {
                if (x, z) == OBSTACLE_TILE_HOLE {
                    continue;
                }
                polys.extend_from_slice(&[
                    vert_index(x, z),
                    vert_index(x + 1, z),
                    vert_index(x + 1, z + 1),
                    vert_index(x, z + 1),
                ]);
                let (xi, zi) = (i32::from(x), i32::from(z));
                polys.extend_from_slice(&[
                    cell_poly(xi, zi - 1),
                    cell_poly(xi + 1, zi),
                    cell_poly(xi, zi + 1),
                    cell_poly(xi - 1, zi),
                ]);
            }
        }

        let poly_count = usize::from(next_poly_index);
        // NAV_GROUND is the flag `create_path_query_filter_like_cpp` includes
        // for a walking creature; area 0 keeps the default cost.
        let poly_flags = vec![NavTerrainFlag::GROUND.bits(); poly_count];
        let poly_areas = vec![0u8; poly_count];
        let bmin = [origin_detour_x, origin_detour_y, origin_detour_z];
        let bmax = [
            origin_detour_x + OBSTACLE_TILE_EXTENT,
            origin_detour_y + OBSTACLE_TILE_CELL_SIZE,
            origin_detour_z + OBSTACLE_TILE_EXTENT,
        ];

        let mut data = std::ptr::null_mut();
        let mut data_size = 0;
        assert!(unsafe {
            rustycore_dt_create_poly_mesh_tile_data(
                tile_x,
                tile_y,
                verts.as_ptr(),
                (verts.len() / 3) as i32,
                polys.as_ptr(),
                poly_count as i32,
                NVP as i32,
                poly_flags.as_ptr(),
                poly_areas.as_ptr(),
                bmin.as_ptr(),
                bmax.as_ptr(),
                OBSTACLE_TILE_CELL_SIZE,
                OBSTACLE_TILE_CELL_SIZE,
                2.0,
                0.0,
                0.9,
                &mut data,
                &mut data_size,
            )
        });
        assert!(!data.is_null());
        assert!(data_size > 0);

        let bytes = unsafe { std::slice::from_raw_parts(data, data_size as usize) }.to_vec();
        unsafe { rustycore_dt_free(data.cast()) };

        MmapTileBlob {
            header: MmapTileHeader {
                mmap_magic: MMAP_MAGIC_LIKE_CPP,
                dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
                mmap_version: MMAP_VERSION_LIKE_CPP,
                size: data_size as u32,
                uses_liquids: true,
                padding: [0, 0, 0],
            },
            data: bytes,
        }
    }

    /// Navmesh params under which [`obstacle_ring_tile_blob`] occupies exactly
    /// tile `(0, 0)`.
    #[must_use]
    pub fn obstacle_ring_nav_mesh_params() -> DetourNavMeshParams {
        DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: OBSTACLE_TILE_EXTENT,
            tile_height: OBSTACLE_TILE_EXTENT,
            max_tiles: 4,
            max_polys: 256,
        }
    }

    /// Mesh whose only tile is [`obstacle_ring_tile_blob`].
    #[must_use]
    pub fn obstacle_ring_nav_mesh() -> DetourNavMesh {
        let params = obstacle_ring_nav_mesh_params();
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = obstacle_ring_tile_blob(0, 0);
        assert_ne!(mesh.add_tile(&tile).unwrap(), 0);
        mesh
    }

    /// Walking-creature filter, i.e. the C++ `CreateFilter` result for
    /// `CanWalk() == true` and `CanEnterWater() == false`.
    #[must_use]
    pub fn obstacle_ring_walk_filter() -> DetourQueryFilter {
        create_path_query_filter_like_cpp(PathQueryFilterContext::creature(
            true, false, false, false,
        ))
        .unwrap()
    }

    /// World bounds of the missing centre cell, as `x`/`z` in Detour space.
    #[must_use]
    pub fn obstacle_hole_bounds() -> (std::ops::RangeInclusive<f32>, std::ops::RangeInclusive<f32>)
    {
        let (hole_x, hole_z) = OBSTACLE_TILE_HOLE;
        let low_x = f32::from(hole_x) * OBSTACLE_TILE_CELL_SIZE;
        let low_z = f32::from(hole_z) * OBSTACLE_TILE_CELL_SIZE;
        (
            low_x..=(low_x + OBSTACLE_TILE_CELL_SIZE),
            low_z..=(low_z + OBSTACLE_TILE_CELL_SIZE),
        )
    }

    /// Navmesh params on the production 533.3333-unit tile grid, so fixture
    /// tiles are placed and looked up by exactly the rules `MMapManager` uses
    /// for real `.mmtile` data.
    #[must_use]
    pub fn obstacle_ring_world_nav_mesh_params() -> DetourNavMeshParams {
        obstacle_ring_world_nav_mesh_params_at(0.0, 0.0)
    }

    /// Production-grid params with an explicit Detour `(x, z)` origin.
    ///
    /// Real `.mmap` files choose an origin that keeps tile indices inside the
    /// navmesh's finite tile grid. Connected synthetic fixtures can live at
    /// negative WoW coordinates, so using a hardcoded zero origin would create
    /// a negative Detour tile index that the runtime correctly rejects.
    #[must_use]
    pub fn obstacle_ring_world_nav_mesh_params_at(
        origin_detour_x: f32,
        origin_detour_z: f32,
    ) -> DetourNavMeshParams {
        DetourNavMeshParams {
            origin: [origin_detour_x, 0.0, origin_detour_z],
            tile_width: SIZE_OF_GRIDS_LIKE_CPP,
            tile_height: SIZE_OF_GRIDS_LIKE_CPP,
            max_tiles: 4096,
            max_polys: 16_384,
        }
    }

    /// Writes the fixture into `base_path` as the on-disk mmaps layout
    /// `MMapManager` loads — `mmaps/<map>.mmap` on the production tile grid plus
    /// one `.mmtile` per requested WoW position — so a real runtime pathfinder
    /// can serve queries for `map_id` around each of them.
    ///
    /// Each tile is placed at the navmesh tile `dtNavMesh::calcTileLoc` derives
    /// for its position, with its geometry offset to that tile's corner, and
    /// named with the C++ 64x64 grid id for the same position.
    pub fn write_obstacle_ring_mmaps_like_cpp(
        base_path: impl AsRef<Path>,
        map_id: u32,
        wow_positions: &[(f32, f32)],
    ) {
        let positions = wow_positions
            .iter()
            .map(|&(wow_x, wow_y)| (wow_x, wow_y, 0.0))
            .collect::<Vec<_>>();
        write_obstacle_ring_mmaps_at_height_like_cpp(base_path, map_id, &positions);
    }

    /// Height-aware on-disk fixture writer used by connected C++/Rust capture
    /// fixtures. Each `(x, y, z)` tuple is in WoW coordinates.
    pub fn write_obstacle_ring_mmaps_at_height_like_cpp(
        base_path: impl AsRef<Path>,
        map_id: u32,
        wow_positions: &[(f32, f32, f32)],
    ) {
        let base_path = base_path.as_ref();
        std::fs::create_dir_all(base_path.join("mmaps")).unwrap();
        let origin_tile_x = wow_positions
            .iter()
            .map(|&(_, wow_y, _)| (wow_y / SIZE_OF_GRIDS_LIKE_CPP).floor() as i32)
            .min()
            .unwrap_or(0);
        let origin_tile_z = wow_positions
            .iter()
            .map(|&(wow_x, _, _)| (wow_x / SIZE_OF_GRIDS_LIKE_CPP).floor() as i32)
            .min()
            .unwrap_or(0);
        let origin_detour_x = origin_tile_x as f32 * SIZE_OF_GRIDS_LIKE_CPP;
        let origin_detour_z = origin_tile_z as f32 * SIZE_OF_GRIDS_LIKE_CPP;
        std::fs::write(
            map_file_path_like_cpp(base_path, map_id),
            obstacle_ring_world_nav_mesh_params_at(origin_detour_x, origin_detour_z).to_bytes(),
        )
        .unwrap();

        for &(wow_x, wow_y, wow_z) in wow_positions {
            // `wow_position_to_detour_like_cpp` puts WoW y on the Detour x axis
            // and WoW x on the Detour z axis, which is what `calcTileLoc`
            // divides by the tile size.
            let tile_x = (wow_y / SIZE_OF_GRIDS_LIKE_CPP).floor();
            let tile_z = (wow_x / SIZE_OF_GRIDS_LIKE_CPP).floor();
            let tile = obstacle_ring_tile_blob_at_height(
                tile_x as i32 - origin_tile_x,
                tile_z as i32 - origin_tile_z,
                tile_x * SIZE_OF_GRIDS_LIKE_CPP,
                wow_z,
                tile_z * SIZE_OF_GRIDS_LIKE_CPP,
            );

            let (grid_x, grid_y) = mmap_tile_coords_for_wow_position_like_cpp(wow_x, wow_y);
            let mut bytes = tile.header.to_bytes().to_vec();
            bytes.extend_from_slice(&tile.data);
            std::fs::write(
                tile_file_path_like_cpp(base_path, map_id, grid_x, grid_y),
                bytes,
            )
            .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disconnected_two_island_nav_mesh() -> DetourNavMesh {
        const NVP: usize = 4;
        const MESH_NULL_IDX: u16 = 0xffff;
        // Two ten-yard quads separated by a ten-yard void.
        let verts: [u16; 24] = [
            0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, // first island
            2, 0, 0, 3, 0, 0, 3, 0, 1, 2, 0, 1, // second island
        ];
        let polys: [u16; 16] = [
            0,
            1,
            2,
            3,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
            4,
            5,
            6,
            7,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
            MESH_NULL_IDX,
        ];
        let poly_flags = [NavTerrainFlag::GROUND.bits(); 2];
        let poly_areas = [0_u8; 2];
        let bmin = [0.0, 0.0, 0.0];
        let bmax = [30.0, 10.0, 10.0];
        let mut data = std::ptr::null_mut();
        let mut data_size = 0;
        assert!(unsafe {
            rustycore_dt_create_poly_mesh_tile_data(
                0,
                0,
                verts.as_ptr(),
                (verts.len() / 3) as i32,
                polys.as_ptr(),
                2,
                NVP as i32,
                poly_flags.as_ptr(),
                poly_areas.as_ptr(),
                bmin.as_ptr(),
                bmax.as_ptr(),
                10.0,
                10.0,
                2.0,
                0.0,
                0.9,
                &mut data,
                &mut data_size,
            )
        });
        assert!(!data.is_null());
        assert!(data_size > 0);
        let bytes = unsafe { std::slice::from_raw_parts(data, data_size as usize) }.to_vec();
        unsafe { rustycore_dt_free(data.cast()) };

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 30.0,
            tile_height: 10.0,
            max_tiles: 1,
            max_polys: 16,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = MmapTileBlob {
            header: MmapTileHeader {
                mmap_magic: MMAP_MAGIC_LIKE_CPP,
                dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
                mmap_version: MMAP_VERSION_LIKE_CPP,
                size: data_size as u32,
                uses_liquids: true,
                padding: [0; 3],
            },
            data: bytes,
        };
        assert_ne!(mesh.add_tile(&tile).unwrap(), 0);
        mesh
    }

    #[test]
    fn mmap_constants_and_nav_flags_match_cpp() {
        assert_eq!(MMAP_MAGIC_LIKE_CPP, 0x4d4d_4150);
        assert_eq!(MMAP_VERSION_LIKE_CPP, 15);
        assert_eq!(MMAP_TILE_HEADER_SIZE_LIKE_CPP, 20);

        assert!(DT_POLYREF64_LIKE_CPP);
        assert_eq!(DT_SALT_BITS_LIKE_CPP, 12);
        assert_eq!(DT_TILE_BITS_LIKE_CPP, 21);
        assert_eq!(DT_POLY_BITS_LIKE_CPP, 31);
        assert_eq!(DT_NAVMESH_MAGIC_LIKE_CPP, 0x444e_4156);
        assert_eq!(DT_NAVMESH_VERSION_LIKE_CPP, 7);
        assert_eq!(DT_NAVMESH_STATE_MAGIC_LIKE_CPP, 0x444e_4d53);
        assert_eq!(DT_NAVMESH_STATE_VERSION_LIKE_CPP, 1);
        assert_eq!(DT_EXT_LINK_LIKE_CPP, 0x8000);
        assert_eq!(DT_NULL_LINK_LIKE_CPP, 0xffff_ffff);
        assert_eq!(DT_OFFMESH_CON_BIDIR_LIKE_CPP, 1);
        assert_eq!(DT_MAX_AREAS_LIKE_CPP, 64);
        assert_eq!(DT_TILE_FREE_DATA_LIKE_CPP, 1);
        assert_eq!(DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP, 28);
        assert_eq!(DT_FAILURE_LIKE_CPP, 1_u32 << 31);
        assert_eq!(DT_SUCCESS_LIKE_CPP, 1_u32 << 30);
        assert_eq!(DT_IN_PROGRESS_LIKE_CPP, 1_u32 << 29);
        assert_eq!(DT_BUFFER_TOO_SMALL_LIKE_CPP, 1_u32 << 4);
        assert_eq!(DT_OUT_OF_MEMORY_LIKE_CPP, 1_u32 << 2);
        assert_eq!(DT_INVALID_PARAM_LIKE_CPP, 1_u32 << 3);
        assert_eq!(MAX_PATH_LENGTH_LIKE_CPP, 74);
        assert_eq!(MAX_POINT_PATH_LENGTH_LIKE_CPP, 74);
        assert_eq!(SMOOTH_PATH_STEP_SIZE_LIKE_CPP, 4.0);
        assert_eq!(SMOOTH_PATH_SLOP_LIKE_CPP, 0.3);
        assert!(detour_status_failed(DT_FAILURE_LIKE_CPP));
        assert!(!detour_status_failed(DT_SUCCESS_LIKE_CPP));

        assert_eq!(NAV_AREA_EMPTY_LIKE_CPP, 0);
        assert_eq!(NAV_AREA_GROUND_LIKE_CPP, 11);
        assert_eq!(NAV_AREA_GROUND_STEEP_LIKE_CPP, 10);
        assert_eq!(NAV_AREA_WATER_LIKE_CPP, 9);
        assert_eq!(NAV_AREA_MAGMA_SLIME_LIKE_CPP, 8);
        assert_eq!(NAV_AREA_ALL_MASK_LIKE_CPP, 0x3f);

        assert_eq!(NavTerrainFlag::EMPTY.bits(), 0x00);
        assert_eq!(NavTerrainFlag::GROUND.bits(), 0x01);
        assert_eq!(NavTerrainFlag::GROUND_STEEP.bits(), 0x02);
        assert_eq!(NavTerrainFlag::WATER.bits(), 0x04);
        assert_eq!(NavTerrainFlag::MAGMA_SLIME.bits(), 0x08);

        assert_eq!(DetourPathType::NORMAL.bits(), 0x01);
        assert_eq!(DetourPathType::SHORTCUT.bits(), 0x02);
        assert_eq!(DetourPathType::INCOMPLETE.bits(), 0x04);
        assert_eq!(DetourPathType::NOPATH.bits(), 0x08);
        assert_eq!(DetourPathType::NOT_USING_PATH.bits(), 0x10);
        assert_eq!(DetourPathType::SHORT.bits(), 0x20);
        assert_eq!(DetourPathType::FARFROMPOLY_START.bits(), 0x40);
        assert_eq!(DetourPathType::FARFROMPOLY_END.bits(), 0x80);
    }

    #[test]
    fn wow_detour_coordinate_flip_matches_pathgenerator_cpp() {
        let wow = [100.0, 200.0, 30.0];
        let detour = wow_position_to_detour_like_cpp(wow);
        assert_eq!(detour, [200.0, 30.0, 100.0]);
        assert_eq!(detour_position_to_wow_like_cpp(detour), wow);
    }

    #[test]
    fn detour_nav_mesh_params_round_trips_cpp_layout() {
        let params = DetourNavMeshParams {
            origin: [-17_066.666, -17_066.666, -2_000.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 4_096,
            max_polys: 32_768,
        };

        let bytes = params.to_bytes();
        assert_eq!(bytes.len(), DT_NAV_MESH_PARAMS_SIZE_LIKE_CPP);
        assert_eq!(DetourNavMeshParams::parse(&bytes), Ok(params));
        assert_eq!(
            DetourNavMeshParams::parse(&bytes[..27]),
            Err(DetourNavMeshParamsError::TooShort {
                actual: 27,
                expected: 28,
            })
        );
    }

    #[test]
    fn detour_nav_mesh_wrapper_initializes_vendored_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 16,
            max_polys: 128,
        };

        let mesh = DetourNavMesh::new(&params).unwrap();
        assert_eq!(mesh.max_tiles(), 16);
        assert!(!mesh.as_raw().is_null());
    }

    #[test]
    fn detour_nav_mesh_tile_wrapper_reports_cpp_add_and_remove_failures() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();

        let header = MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 128,
            uses_liquids: true,
            padding: [0, 0, 0],
        };
        let bad_tile = MmapTileBlob {
            header,
            data: vec![0; 128],
        };
        assert!(matches!(
            mesh.add_tile(&bad_tile),
            Err(DetourTileError::AddTileFailed { status })
                if detour_status_failed(status)
        ));
        assert_eq!(
            mesh.remove_tile(0),
            Err(DetourTileError::RemoveTileFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn detour_nav_mesh_adds_and_removes_generated_tile_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);

        let tile_ref = mesh.add_tile(&tile).unwrap();
        assert_ne!(tile_ref, 0);
        assert_eq!(mesh.calc_tile_loc([0.25, 0.0, 0.25]), (0, 0));
        assert!(mesh.has_tile_at(0, 0, 0));
        assert!(mesh.have_tile_for_wow_position_like_cpp([0.25, 0.25, 0.0]));
        assert!(!mesh.have_tile_for_wow_position_like_cpp([2.0, 2.0, 0.0]));
        mesh.remove_tile(tile_ref).unwrap();
        assert!(!mesh.has_tile_at(0, 0, 0));
    }

    #[test]
    fn detour_nav_mesh_query_initializes_like_mmap_manager_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mesh = DetourNavMesh::new(&params).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        assert!(!query.as_raw().is_null());
    }

    #[test]
    fn detour_query_filter_defaults_and_mutators_match_cpp() {
        let mut filter = DetourQueryFilter::new().unwrap();

        assert_eq!(filter.include_flags(), 0xffff);
        assert_eq!(filter.exclude_flags(), 0);
        assert_eq!(filter.area_cost(0).unwrap(), 1.0);
        assert_eq!(filter.area_cost(DT_MAX_AREAS_LIKE_CPP - 1).unwrap(), 1.0);

        filter.set_include_flags(
            (NavTerrainFlag::GROUND | NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME).bits(),
        );
        filter.set_exclude_flags(NavTerrainFlag::GROUND_STEEP.bits());
        filter
            .set_area_cost(NAV_AREA_MAGMA_SLIME_LIKE_CPP as usize, 100.0)
            .unwrap();

        assert_eq!(
            filter.include_flags(),
            (NavTerrainFlag::GROUND | NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME).bits()
        );
        assert_eq!(filter.exclude_flags(), NavTerrainFlag::GROUND_STEEP.bits());
        assert_eq!(
            filter.area_cost(NAV_AREA_MAGMA_SLIME_LIKE_CPP as usize),
            Ok(100.0)
        );
        assert_eq!(
            filter.area_cost(DT_MAX_AREAS_LIKE_CPP),
            Err(DetourQueryFilterError::AreaIndexOutOfRange {
                area: DT_MAX_AREAS_LIKE_CPP,
                max: DT_MAX_AREAS_LIKE_CPP,
            })
        );
    }

    #[test]
    fn path_query_filter_create_matches_cpp_owner_rules() {
        let ground_creature = create_path_query_filter_like_cpp(PathQueryFilterContext::creature(
            true, false, false, false,
        ))
        .unwrap();
        assert_eq!(
            ground_creature.include_flags(),
            NavTerrainFlag::GROUND.bits()
        );
        assert_eq!(
            ground_creature.exclude_flags(),
            NavTerrainFlag::EMPTY.bits()
        );

        let water_creature = create_path_query_filter_like_cpp(PathQueryFilterContext::creature(
            false, true, false, false,
        ))
        .unwrap();
        assert_eq!(
            water_creature.include_flags(),
            (NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME).bits()
        );

        let player = create_path_query_filter_like_cpp(PathQueryFilterContext::player()).unwrap();
        assert_eq!(
            player.include_flags(),
            (NavTerrainFlag::GROUND | NavTerrainFlag::WATER | NavTerrainFlag::MAGMA_SLIME).bits()
        );
    }

    #[test]
    fn path_query_filter_update_matches_cpp_force_water_and_combat_rules() {
        let mut context = PathQueryFilterContext::creature(true, false, true, false);
        context.force_enabled_flags = NavTerrainFlag::WATER;
        context.force_disabled_flags = NavTerrainFlag::MAGMA_SLIME;
        context.is_in_water = true;
        context.current_nav_terrain = NavTerrainFlag::MAGMA_SLIME;

        let filter = create_path_query_filter_like_cpp(context).unwrap();
        assert_eq!(
            filter.include_flags(),
            (NavTerrainFlag::GROUND
                | NavTerrainFlag::GROUND_STEEP
                | NavTerrainFlag::WATER
                | NavTerrainFlag::MAGMA_SLIME)
                .bits()
        );
        assert_eq!(filter.exclude_flags(), NavTerrainFlag::MAGMA_SLIME.bits());

        let mut evade_context = PathQueryFilterContext::creature(true, false, false, true);
        evade_context.is_under_water = true;
        evade_context.current_nav_terrain = NavTerrainFlag::WATER;
        let filter = create_path_query_filter_like_cpp(evade_context).unwrap();
        assert_eq!(
            filter.include_flags(),
            (NavTerrainFlag::GROUND | NavTerrainFlag::GROUND_STEEP | NavTerrainFlag::WATER).bits()
        );
    }

    #[test]
    fn detour_query_find_nearest_poly_matches_cpp_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        let tile_ref = mesh.add_tile(&tile).unwrap();
        assert_ne!(tile_ref, 0);

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        assert_ne!(nearest.poly_ref, 0);
        assert!((nearest.nearest_point[0] - 0.5).abs() < f32::EPSILON);
        assert!((nearest.nearest_point[1] - 0.0).abs() < f32::EPSILON);
        assert!((nearest.nearest_point[2] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn detour_query_find_path_returns_single_poly_for_same_start_end_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();
        let path = query
            .find_path(
                nearest.poly_ref,
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                4,
            )
            .unwrap();

        assert_eq!(path, vec![nearest.poly_ref]);
        assert_eq!(
            query.find_path(
                nearest.poly_ref,
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                0,
            ),
            Err(DetourNavMeshQueryError::FindPathFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn detour_query_find_straight_path_matches_cpp_single_poly_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();
        let path = query
            .find_path(
                nearest.poly_ref,
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                4,
            )
            .unwrap();

        let straight = query
            .find_straight_path([0.25, 0.0, 0.25], [0.75, 0.0, 0.75], &path, 4, 0)
            .unwrap();

        assert_eq!(straight.len(), 2);
        assert_eq!(straight[0].flags, DT_STRAIGHTPATH_START_LIKE_CPP);
        assert_eq!(straight[0].poly_ref, nearest.poly_ref);
        assert_eq!(straight[0].position, [0.25, 0.0, 0.25]);
        assert_eq!(straight[1].flags, DT_STRAIGHTPATH_END_LIKE_CPP);
        assert_eq!(straight[1].poly_ref, 0);
        assert_eq!(straight[1].position, [0.75, 0.0, 0.75]);

        assert_eq!(
            query.find_straight_path([0.25, 0.0, 0.25], [0.75, 0.0, 0.75], &path, 0, 0),
            Err(DetourNavMeshQueryError::FindStraightPathFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn detour_build_straight_poly_path_handles_same_poly_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let path = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            DetourOwnerCapabilitiesLikeCpp::default(),
            &[],
        )
        .unwrap();

        assert_eq!(path.poly_refs.len(), 1);
        assert_eq!(path.point_path.path_type, DetourPathType::NORMAL);
        // C++ reaches `BuildPointPath` for the `startPoly == endPoly` case
        // (`PathGenerator.cpp:287`), so the corridor builder leaves the points
        // to the caller instead of producing them twice.
        assert!(path.point_path.points.is_empty());
        assert_eq!(path.point_path.actual_end, [0.75, 0.0, 0.75]);
    }

    #[test]
    fn detour_build_straight_poly_path_reports_missing_poly_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mesh = DetourNavMesh::new(&params).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let path = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            DetourOwnerCapabilitiesLikeCpp::default(),
            &[],
        )
        .unwrap();

        assert!(path.poly_refs.is_empty());
        // C++ assigns `_type = PATHFIND_NOPATH` after `BuildShortcut()`
        // (`PathGenerator.cpp:207`), so `PATHFIND_SHORTCUT` does not survive.
        assert_eq!(path.point_path.path_type, DetourPathType::NOPATH);
        // C++ answers the hole-in-mesh case with `BuildShortcut()` and returns
        // before `BuildPointPath` (`PathGenerator.cpp:176-209`), so the two
        // shortcut points are already present here.
        assert_eq!(
            path.point_path.points,
            vec![[0.25, 0.0, 0.25], [0.75, 0.0, 0.75]]
        );
    }

    #[test]
    fn detour_build_raycast_poly_path_handles_empty_raycast_path_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let path =
            build_raycast_poly_path_like_cpp(&query, &filter, [0.25, 0.0, 0.25], [0.75, 0.0, 0.75])
                .unwrap();

        assert!(path.poly_refs.is_empty());
        assert_eq!(
            path.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::NOPATH
        );
        assert_eq!(
            path.point_path.points,
            vec![[0.25, 0.0, 0.25], [0.75, 0.0, 0.75]]
        );
    }

    #[test]
    fn detour_build_raycast_poly_path_marks_far_flags_on_missing_poly_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mesh = DetourNavMesh::new(&params).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let path =
            build_raycast_poly_path_like_cpp(&query, &filter, [0.25, 0.0, 0.25], [0.75, 0.0, 0.75])
                .unwrap();

        // With no polygons, native raycast fails because startPoly is invalid.
        // C++ converts that query error into BuildShortcut + NOPATH rather than
        // propagating it to CalculatePath.
        assert!(path.poly_refs.is_empty());
        assert!(path.start_far_from_poly);
        assert!(path.end_far_from_poly);
        assert_eq!(
            path.point_path.path_type,
            DetourPathType::SHORTCUT
                | DetourPathType::NOPATH
                | DetourPathType::FARFROMPOLY_START
                | DetourPathType::FARFROMPOLY_END
        );
    }

    #[test]
    fn calculate_build_point_failures_clear_the_corridor_like_cpp() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let start_detour = [5.0, 0.0, 15.0];
        let end_detour = [25.0, 0.0, 15.0];
        let start_wow = detour_position_to_wow_like_cpp(start_detour);
        let end_wow = detour_position_to_wow_like_cpp(end_detour);

        let fewer_than_two = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            start_wow,
            end_wow,
            DetourPathOptions {
                point_path_limit: 0,
                ..DetourPathOptions::default()
            },
        )
        .unwrap();
        assert!(fewer_than_two.poly_refs.is_empty());
        assert_eq!(
            fewer_than_two.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::NOPATH
        );

        // The oversized limit is deterministic fault injection into the Rust
        // query wrapper (`StraightPathBufferTooLarge`). Production limits are
        // bounded, but this reaches the same `dtStatusFailed` recovery C++
        // handles with BuildShortcut/Clear.
        let failed_query = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            start_wow,
            end_wow,
            DetourPathOptions {
                use_straight_path: true,
                point_path_limit: i32::MAX as usize + 1,
                ..DetourPathOptions::default()
            },
        )
        .unwrap();
        assert!(
            failed_query.poly_refs.is_empty(),
            "failed point query leaked {failed_query:?}"
        );
        assert_eq!(
            failed_query.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::NOPATH
        );
    }

    #[test]
    fn fixup_corridor_matches_cpp_common_polygon_splice() {
        assert_eq!(
            fixup_corridor_like_cpp(&[1, 2, 3, 4, 5], 8, &[7, 8, 3, 9]),
            vec![9, 3, 4, 5]
        );
        assert_eq!(
            fixup_corridor_like_cpp(&[1, 2, 3], 8, &[9, 8]),
            vec![1, 2, 3]
        );
        assert_eq!(
            fixup_corridor_like_cpp(&[1, 2, 3, 4, 5], 3, &[7, 8, 3, 9]),
            vec![9, 3, 4]
        );
    }

    #[test]
    fn get_steer_target_matches_cpp_slop_filter() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.25, 0.0, 0.25], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        let steer = get_steer_target_like_cpp(
            &query,
            [0.25, 2.0, 0.25],
            [0.75, 0.0, 0.75],
            SMOOTH_PATH_SLOP_LIKE_CPP,
            &[nearest.poly_ref],
        )
        .unwrap()
        .unwrap();

        assert_eq!(steer.position, [0.75, 2.0, 0.75]);
        assert_eq!(steer.flags, DT_STRAIGHTPATH_END_LIKE_CPP);
        assert_eq!(steer.poly_ref, 0);
    }

    #[test]
    fn find_smooth_path_matches_cpp_same_poly_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.25, 0.0, 0.25], [3.0, 5.0, 3.0], &filter)
            .unwrap();
        assert!(matches!(
            mesh.get_off_mesh_connection_poly_end_points(0, nearest.poly_ref),
            Err(DetourNavMeshError::OffMeshConnectionEndpointsFailed { .. })
        ));

        let smooth = find_smooth_path_like_cpp(
            &mesh,
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            &[nearest.poly_ref],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();

        assert_eq!(smooth, vec![[0.25, 0.0, 0.25], [0.75, 0.0, 0.75]]);
    }

    #[test]
    fn build_point_path_dispatches_straight_smooth_and_raycast_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.25, 0.0, 0.25], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        let smooth = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            [0.75, 0.0, 0.75],
            &[nearest.poly_ref],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            false,
            false,
        )
        .unwrap()
        .point_path;
        assert_eq!(smooth.points, vec![[0.25, 0.0, 0.25], [0.75, 0.0, 0.75]]);
        assert_eq!(smooth.path_type, DetourPathType::NORMAL);

        let straight = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            [0.75, 0.0, 0.75],
            &[nearest.poly_ref],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            true,
            false,
        )
        .unwrap()
        .point_path;
        assert_eq!(straight.points, smooth.points);
        assert_eq!(straight.path_type, DetourPathType::NORMAL);

        let raycast = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            [0.25, 0.0, 0.25],
            [0.75, 0.0, 0.75],
            [0.75, 0.0, 0.75],
            &[nearest.poly_ref],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            false,
            true,
        )
        .unwrap()
        .point_path;
        assert_eq!(raycast.points, vec![[0.25, 0.0, 0.25], [0.75, 0.0, 0.75]]);
        assert_eq!(raycast.path_type, DetourPathType::NOPATH);
    }

    #[test]
    fn build_point_path_reports_every_cpp_build_shortcut_clear() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let poly = query
            .find_nearest_poly([0.25, 0.0, 0.25], [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;
        let start = [0.25, 0.0, 0.25];
        let end = [0.75, 0.0, 0.75];

        let raycast = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            end,
            &[poly],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            false,
            true,
        )
        .unwrap();
        assert!(raycast.cleared_poly_path);
        assert_eq!(raycast.point_path.path_type, DetourPathType::NOPATH);

        let failed_query = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            end,
            &[0],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            true,
            false,
        )
        .unwrap();
        assert!(failed_query.cleared_poly_path);
        assert_eq!(
            failed_query.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::NOPATH
        );

        let fewer_than_two_points = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            end,
            &[],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            false,
            false,
        )
        .unwrap();
        assert!(fewer_than_two_points.cleared_poly_path);
        assert_eq!(
            fewer_than_two_points.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::NOPATH
        );

        let point_limit = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            end,
            &[poly],
            2,
            DetourPathType::NORMAL,
            false,
            false,
            false,
        )
        .unwrap();
        assert!(point_limit.cleared_poly_path);
        assert_eq!(
            point_limit.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::SHORT
        );

        let far_forced_destination = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            [10.0, 0.0, 10.0],
            &[poly],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::INCOMPLETE,
            true,
            false,
            false,
        )
        .unwrap();
        assert!(far_forced_destination.cleared_poly_path);
        assert_eq!(
            far_forced_destination.point_path.path_type,
            DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
        );
        assert_eq!(
            far_forced_destination.point_path.points,
            vec![start, [10.0, 0.0, 10.0]]
        );

        let near_forced_destination = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            [0.8, 0.0, 0.8],
            &[poly],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::INCOMPLETE,
            true,
            false,
            false,
        )
        .unwrap();
        assert!(!near_forced_destination.cleared_poly_path);
        assert_eq!(
            near_forced_destination.point_path.points.last(),
            Some(&[0.8, 0.0, 0.8])
        );

        let normal = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            end,
            end,
            &[poly],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::NORMAL,
            false,
            false,
            false,
        )
        .unwrap();
        assert!(!normal.cleared_poly_path);
    }

    #[test]
    fn calculate_detour_path_returns_wow_coordinates_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let path = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            [0.25, 0.25, 0.0],
            [0.75, 0.75, 0.0],
            DetourPathOptions::default(),
        )
        .unwrap();

        assert_eq!(path.point_path.path_type, DetourPathType::NORMAL);
        assert_eq!(
            path.point_path.points,
            vec![[0.25, 0.25, 0.0], [0.75, 0.75, 0.0]]
        );
        assert_eq!(path.point_path.actual_end, [0.75, 0.75, 0.0]);
    }

    #[test]
    fn path_poly_lookup_uses_the_cpp_3d_squared_threshold() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let poly = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;

        let (accepted, accepted_distance) =
            get_path_poly_by_position_like_cpp(&query, &[poly], [0.5, 1.7, 0.5]);
        assert_eq!(accepted, poly);
        assert!((accepted_distance - 1.7).abs() < f32::EPSILON);

        let (rejected, rejected_distance) =
            get_path_poly_by_position_like_cpp(&query, &[poly], [0.5, 1.8, 0.5]);
        assert_eq!(rejected, 0);
        assert!((rejected_distance - 1.8).abs() < f32::EPSILON);
    }

    #[test]
    fn reuse_previous_poly_path_cuts_subpath_and_rejects_raycast_like_cpp() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        mesh.add_tile(&generated_square_tile_blob(0, 0)).unwrap();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();

        let reused = reuse_previous_poly_path_like_cpp(
            &query,
            &filter,
            &[11, 22, 33, 44],
            22,
            44,
            [0.75, 0.0, 0.75],
            false,
        )
        .unwrap();
        assert_eq!(reused, PreviousPolyPathLikeCpp::PolyRefs(vec![22, 33, 44]));

        let recalculated = reuse_previous_poly_path_like_cpp(
            &query,
            &filter,
            &[11, 22, 33, 44],
            55,
            44,
            [0.75, 0.0, 0.75],
            false,
        )
        .unwrap();
        assert_eq!(recalculated, PreviousPolyPathLikeCpp::Recalculate);

        let raycast = reuse_previous_poly_path_like_cpp(
            &query,
            &filter,
            &[11, 22, 33, 44],
            22,
            55,
            [0.75, 0.0, 0.75],
            true,
        )
        .unwrap();
        assert_eq!(raycast, PreviousPolyPathLikeCpp::ShortcutNoPath);
    }

    #[test]
    fn empty_reuse_suffix_keeps_the_full_valid_prefix_and_clamps_the_point_path() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let start = [5.0, 0.0, 15.0];
        let old_end = [25.0, 0.0, 15.0];
        let start_poly = query
            .find_nearest_poly(start, [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;
        let old_end_poly = query
            .find_nearest_poly(old_end, [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;
        let previous = query
            .find_path(
                start_poly,
                old_end_poly,
                start,
                old_end,
                &filter,
                MAX_PATH_LENGTH_LIKE_CPP,
            )
            .unwrap();
        assert!(previous.len() >= 3);

        let two_poly_prefix = &previous[..2];
        // `endPoly == 0` deliberately injects the empty/failed `findPath`
        // result C++'s recovery branch documents. A healthy Detour query with
        // valid refs normally returns at least `suffixStartPoly`, so this is a
        // fault-injection test of the recovery invariant, not a claim that the
        // fixture naturally produces an empty suffix.
        let retained = reuse_previous_poly_path_like_cpp(
            &query,
            &filter,
            two_poly_prefix,
            start_poly,
            0,
            old_end,
            false,
        )
        .unwrap();
        let PreviousPolyPathLikeCpp::PolyRefs(retained) = retained else {
            panic!("an empty suffix must retain the usable two-poly prefix");
        };
        assert_eq!(retained, two_poly_prefix);

        let expected_clamp = query
            .closest_point_on_poly_boundary(*retained.last().unwrap(), old_end)
            .unwrap();
        let retained_point_path = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            old_end,
            old_end,
            &retained,
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::INCOMPLETE,
            false,
            false,
            false,
        )
        .unwrap()
        .point_path;
        assert_eq!(retained_point_path.actual_end, expected_clamp);
        assert_eq!(
            retained_point_path.points.last(),
            Some(&expected_clamp),
            "the retained corridor must stop at its valid boundary"
        );
        assert_ne!(
            retained_point_path.actual_end, old_end,
            "dropping the non-overlapping prefix tail would turn this into a remote same-poly jump"
        );

        let truncated_point_path = build_point_path_outcome_like_cpp(
            &mesh,
            &query,
            &filter,
            start,
            old_end,
            old_end,
            &retained[..1],
            MAX_POINT_PATH_LENGTH_LIKE_CPP,
            DetourPathType::INCOMPLETE,
            false,
            false,
            false,
        )
        .unwrap()
        .point_path;
        assert_eq!(
            truncated_point_path.points.last(),
            Some(&old_end),
            "the popped corridor incorrectly accepts the remote destination"
        );
        let segment_start =
            truncated_point_path.points[truncated_point_path.points.len().saturating_sub(2)];
        let (hole_x, hole_z) = obstacle_hole_bounds();
        assert!(
            segment_start[0] < *hole_x.start()
                && old_end[0] > *hole_x.end()
                && hole_z.contains(&segment_start[2])
                && hole_z.contains(&old_end[2]),
            "the final segment {segment_start:?} -> {old_end:?} must reproduce the obstacle crossing"
        );

        let degenerate = reuse_previous_poly_path_like_cpp(
            &query,
            &filter,
            &[start_poly],
            start_poly,
            0,
            old_end,
            false,
        )
        .unwrap();
        assert_eq!(degenerate, PreviousPolyPathLikeCpp::Recalculate);
    }

    #[test]
    fn partial_singleton_corridor_clamps_to_the_reachable_island_boundary() {
        let mesh = disconnected_two_island_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = create_path_query_filter_like_cpp(PathQueryFilterContext::creature(
            true, false, false, false,
        ))
        .unwrap();
        let start = [5.0, 0.0, 5.0];
        let requested_end = [25.0, 0.0, 5.0];
        let start_poly = query
            .find_nearest_poly(start, [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;
        let end_poly = query
            .find_nearest_poly(requested_end, [3.0, 5.0, 3.0], &filter)
            .unwrap()
            .poly_ref;
        assert_ne!(start_poly, end_poly);
        assert_eq!(
            query
                .find_path(
                    start_poly,
                    end_poly,
                    start,
                    requested_end,
                    &filter,
                    MAX_PATH_LENGTH_LIKE_CPP,
                )
                .unwrap(),
            vec![start_poly],
            "Detour retains the reachable start island as a valid partial corridor"
        );

        let path = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            detour_position_to_wow_like_cpp(start),
            detour_position_to_wow_like_cpp(requested_end),
            DetourPathOptions::default(),
        )
        .unwrap();

        assert_eq!(path.poly_refs, vec![start_poly]);
        assert!(
            path.point_path
                .path_type
                .contains(DetourPathType::INCOMPLETE)
        );
        assert!(
            path.point_path
                .points
                .iter()
                .all(|point| point[1] <= 10.001),
            "a partial singleton must never append a segment across the void: {path:?}"
        );
        assert_ne!(
            path.point_path.actual_end,
            detour_position_to_wow_like_cpp(requested_end)
        );
    }

    #[test]
    fn detour_query_closest_point_helpers_match_cpp_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        let (closest, over_poly) = query
            .closest_point_on_poly(nearest.poly_ref, [0.5, 2.0, 0.5])
            .unwrap();
        assert_eq!(closest, [0.5, 0.0, 0.5]);
        assert!(over_poly);

        let boundary = query
            .closest_point_on_poly_boundary(nearest.poly_ref, [2.0, 2.0, 0.5])
            .unwrap();
        assert_eq!(boundary, [1.0, 0.0, 0.5]);

        assert_eq!(
            query.closest_point_on_poly(0, [0.5, 0.0, 0.5]),
            Err(DetourNavMeshQueryError::ClosestPointOnPolyFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
        assert_eq!(
            query.closest_point_on_poly_boundary(0, [0.5, 0.0, 0.5]),
            Err(DetourNavMeshQueryError::ClosestPointOnPolyBoundaryFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            },)
        );
    }

    #[test]
    fn detour_query_get_poly_height_matches_cpp_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        assert_eq!(
            query.get_poly_height(nearest.poly_ref, [0.5, 7.0, 0.5]),
            Ok(0.0)
        );
        assert_eq!(
            query.get_poly_height(0, [0.5, 0.0, 0.5]),
            Err(DetourNavMeshQueryError::GetPolyHeightFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn detour_query_move_along_surface_matches_cpp_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        let moved = query
            .move_along_surface(
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                16,
            )
            .unwrap();

        assert_eq!(moved.result_position, [0.75, 0.0, 0.75]);
        assert_eq!(moved.visited, vec![nearest.poly_ref]);
        assert_eq!(
            query.move_along_surface(
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                0,
            ),
            Err(DetourNavMeshQueryError::MoveAlongSurfaceFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn detour_query_raycast_matches_cpp_shape() {
        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 16,
            max_polys: 128,
        };
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        mesh.add_tile(&tile).unwrap();

        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = DetourQueryFilter::new().unwrap();
        let nearest = query
            .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
            .unwrap();

        let raycast = query
            .raycast(
                nearest.poly_ref,
                [0.25, 0.0, 0.25],
                [0.75, 0.0, 0.75],
                &filter,
                16,
            )
            .unwrap();

        assert_eq!(raycast.hit_t, 0.0);
        assert!(raycast.path.is_empty());
        assert_eq!(
            query.raycast(0, [0.25, 0.0, 0.25], [0.75, 0.0, 0.75], &filter, 16,),
            Err(DetourNavMeshQueryError::RaycastFailed {
                status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
            })
        );
    }

    #[test]
    fn mmap_tile_header_round_trips_cpp_layout() {
        let header = MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: 7,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 123_456,
            uses_liquids: true,
            padding: [0, 0, 0],
        };

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), MMAP_TILE_HEADER_SIZE_LIKE_CPP);
        assert_eq!(MmapTileHeader::parse(&bytes), Ok(header));
        assert_eq!(
            MmapTileHeader::parse(&bytes)
                .unwrap()
                .validate_dt_version(7),
            Ok(())
        );
    }

    #[test]
    fn mmap_tile_header_rejects_cpp_load_failures() {
        assert_eq!(
            MmapTileHeader::parse(&[0; 19]),
            Err(MmapTileHeaderError::TooShort {
                actual: 19,
                expected: 20,
            })
        );

        let mut bad_magic = MmapTileHeader::new(7).to_bytes();
        bad_magic[0] = 0;
        assert!(matches!(
            MmapTileHeader::parse(&bad_magic),
            Err(MmapTileHeaderError::BadMagic { .. })
        ));

        let mut bad_version = MmapTileHeader::new(7).to_bytes();
        bad_version[8..12].copy_from_slice(&14_u32.to_le_bytes());
        assert!(matches!(
            MmapTileHeader::parse(&bad_version),
            Err(MmapTileHeaderError::BadMmapVersion { .. })
        ));

        let header = MmapTileHeader::new(7);
        assert_eq!(
            header.validate_dt_version(8),
            Err(MmapTileHeaderError::BadDetourVersion {
                actual: 7,
                expected: 8,
            })
        );
    }

    #[test]
    fn mmap_tile_blob_reads_header_and_data_like_cpp_before_add_tile() {
        let header = MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 4,
            uses_liquids: false,
            padding: [0, 0, 0],
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&[1, 2, 3, 4, 99]);

        let blob = MmapTileBlob::parse(&bytes, DT_NAVMESH_VERSION_LIKE_CPP).unwrap();
        assert_eq!(blob.header, header);
        assert_eq!(blob.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn mmap_tile_blob_rejects_cpp_load_failures_before_detour_ownership() {
        assert!(matches!(
            MmapTileBlob::parse(&[0; 19], DT_NAVMESH_VERSION_LIKE_CPP),
            Err(MmapTileBlobError::BadHeader(
                MmapTileHeaderError::TooShort { .. }
            ))
        ));

        let mut bad_dt_version = MmapTileHeader::new(DT_NAVMESH_VERSION_LIKE_CPP + 1).to_bytes();
        bad_dt_version[12..16].copy_from_slice(&0_u32.to_le_bytes());
        assert!(matches!(
            MmapTileBlob::parse(&bad_dt_version, DT_NAVMESH_VERSION_LIKE_CPP),
            Err(MmapTileBlobError::BadHeader(
                MmapTileHeaderError::BadDetourVersion { .. }
            ))
        ));

        let mut corrupt_size = MmapTileHeader::new(DT_NAVMESH_VERSION_LIKE_CPP)
            .to_bytes()
            .to_vec();
        corrupt_size[12..16].copy_from_slice(&5_u32.to_le_bytes());
        corrupt_size.extend_from_slice(&[1, 2, 3, 4]);
        assert_eq!(
            MmapTileBlob::parse(&corrupt_size, DT_NAVMESH_VERSION_LIKE_CPP),
            Err(MmapTileBlobError::CorruptedDataSize {
                declared: 5,
                available: 4,
            })
        );
    }

    #[test]
    fn mmap_tile_blob_file_reader_uses_cpp_file_shape() {
        let root = unique_test_dir("mmap-tile-blob-file-reader");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();
        let path = tile_file_path_like_cpp(&root, 571, 32, 48);

        let header = MmapTileHeader {
            mmap_magic: MMAP_MAGIC_LIKE_CPP,
            dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
            mmap_version: MMAP_VERSION_LIKE_CPP,
            size: 3,
            uses_liquids: true,
            padding: [0, 0, 0],
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend_from_slice(&[9, 8, 7]);
        std::fs::write(&path, bytes).unwrap();

        let blob = read_mmap_tile_blob_file(&path, DT_NAVMESH_VERSION_LIKE_CPP).unwrap();
        assert_eq!(blob.header, header);
        assert_eq!(blob.data, vec![9, 8, 7]);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_small_helpers_match_cpp() {
        assert_eq!(pack_tile_id_like_cpp(0x12, 0x34), 0x0012_0034);
        assert_eq!(
            mmap_tile_coords_for_wow_position_like_cpp(0.0, 0.0),
            (32, 32)
        );
        assert_eq!(
            mmap_tile_coords_for_wow_position_like_cpp(SIZE_OF_GRIDS_LIKE_CPP, 0.0),
            (31, 32)
        );
        assert_eq!(
            mmap_tile_coords_for_wow_position_like_cpp(-SIZE_OF_GRIDS_LIKE_CPP, 0.0),
            (33, 32)
        );
        assert_eq!(
            mmap_tile_coords_for_wow_position_like_cpp(0.0, SIZE_OF_GRIDS_LIKE_CPP),
            (32, 31)
        );
        assert_eq!(map_file_name_like_cpp(571), "mmaps/0571.mmap");
        assert_eq!(
            map_file_path_like_cpp("/srv/wow", 571),
            std::path::PathBuf::from("/srv/wow/mmaps/0571.mmap")
        );
        assert_eq!(
            tile_file_name_like_cpp(571, 32, 48),
            "mmaps/05713248.mmtile"
        );
        assert_eq!(
            tile_file_path_like_cpp("/srv/wow", 571, 32, 48),
            std::path::PathBuf::from("/srv/wow/mmaps/05713248.mmtile")
        );
    }

    #[test]
    fn mmap_manager_loads_map_params_and_caches_like_cpp() {
        let root = unique_test_dir("mmap-manager-loads-map-params");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [1.0, 2.0, 3.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 128,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let mut manager = MMapManager::new();
        assert!(manager.is_thread_safe_environment());
        assert_eq!(manager.get_loaded_maps_count(), 0);
        assert!(matches!(manager.load_map_data(&root, 1), Ok(true)));
        assert!(matches!(manager.load_map_data(&root, 1), Ok(true)));
        assert_eq!(manager.get_loaded_maps_count(), 1);
        assert_eq!(manager.get_loaded_tiles_count(), 0);
        assert_eq!(manager.get_nav_mesh_params(1), Some(params));
        let data = manager.get_mmap_data(1).unwrap();
        assert_eq!(data.loaded_tile_refs.len(), 0);
        assert_eq!(data.nav_mesh().max_tiles(), params.max_tiles as u32);
        assert!(manager.get_nav_mesh(1).is_some());
        assert!(manager.unload_map(1));
        assert!(!manager.unload_map(1));
        assert_eq!(manager.get_loaded_maps_count(), 1);
        assert_eq!(manager.get_nav_mesh_params(1), None);
        assert!(manager.get_nav_mesh(1).is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_loads_and_reuses_instance_queries_like_cpp() {
        let root = unique_test_dir("mmap-manager-instance-query");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 128,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let mut manager = MMapManager::new();
        assert!(matches!(
            manager.load_map_instance(&root, 1, 1, 42),
            Ok(true)
        ));
        let data = manager.get_mmap_data(1).unwrap();
        assert_eq!(data.nav_mesh_query_count(), 1);
        assert!(data.has_nav_mesh_query(1, 42));
        assert!(manager.get_nav_mesh_query(1, 1, 42).is_some());

        assert!(matches!(
            manager.load_map_instance(&root, 1, 1, 42),
            Ok(true)
        ));
        assert_eq!(manager.get_mmap_data(1).unwrap().nav_mesh_query_count(), 1);

        assert!(matches!(
            manager.load_map_instance(&root, 1, 1, 43),
            Ok(true)
        ));
        assert_eq!(manager.get_mmap_data(1).unwrap().nav_mesh_query_count(), 2);
        assert!(manager.unload_map_instance(1, 1, 42));
        assert!(!manager.unload_map_instance(1, 1, 42));
        assert!(!manager.unload_map_instance(999, 1, 43));
        assert_eq!(manager.get_mmap_data(1).unwrap().nav_mesh_query_count(), 1);

        assert!(manager.unload_map(1));
        assert!(manager.get_nav_mesh_query(1, 1, 43).is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_loads_and_unloads_tiles_like_cpp() {
        let root = unique_test_dir("mmap-manager-loads-tile");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 128,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();
        let tile = generated_square_tile_blob(0, 0);
        write_mmap_tile_blob(&tile_file_path_like_cpp(&root, 1, 0, 0), &tile);

        let mut manager = MMapManager::new();
        assert!(matches!(manager.load_map(&root, 1, 0, 0), Ok(true)));
        assert!(matches!(manager.load_map(&root, 1, 0, 0), Ok(false)));
        assert_eq!(manager.get_loaded_tiles_count(), 1);
        assert!(
            manager
                .get_mmap_data(1)
                .unwrap()
                .loaded_tile_refs
                .contains_key(&pack_tile_id_like_cpp(0, 0))
        );
        assert!(matches!(
            manager.load_map_instance(&root, 1, 1, 42),
            Ok(true)
        ));
        let filter = DetourQueryFilter::new().unwrap();
        let calculated = manager
            .get_mmap_data(1)
            .unwrap()
            .calculate_path_for_instance_like_cpp(
                1,
                42,
                &filter,
                [0.25, 0.25, 0.0],
                [0.75, 0.75, 0.0],
                DetourPathOptions::default(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            calculated.point_path.points,
            vec![[0.25, 0.25, 0.0], [0.75, 0.75, 0.0]]
        );
        assert!(
            manager
                .get_mmap_data(1)
                .unwrap()
                .calculate_path_for_instance_like_cpp(
                    1,
                    999,
                    &filter,
                    [0.25, 0.25, 0.0],
                    [0.75, 0.75, 0.0],
                    DetourPathOptions::default(),
                )
                .unwrap()
                .is_none()
        );
        assert!(
            manager
                .get_mmap_data(1)
                .unwrap()
                .calculate_path_for_instance_like_cpp(
                    1,
                    42,
                    &filter,
                    [0.25, 0.25, 0.0],
                    [2.0, 2.0, 0.0],
                    DetourPathOptions::default(),
                )
                .unwrap()
                .is_none()
        );

        assert!(matches!(manager.unload_map_tile(1, 0, 0), Ok(true)));
        assert!(matches!(manager.unload_map_tile(1, 0, 0), Ok(false)));
        assert_eq!(manager.get_loaded_tiles_count(), 0);
        assert!(
            manager
                .get_mmap_data(1)
                .unwrap()
                .loaded_tile_refs
                .is_empty()
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_loads_pathfinding_context_from_wow_position_like_cpp() {
        let root = unique_test_dir("mmap-manager-loads-path-context");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 4096,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();
        let tile = generated_square_tile_blob(32, 32);
        write_mmap_tile_blob(&tile_file_path_like_cpp(&root, 1, 32, 32), &tile);

        let mut manager = MMapManager::new();
        let loaded = manager
            .load_pathfinding_context_for_wow_position_like_cpp(&root, 1, 1, 42, 0.0, 0.0)
            .unwrap();

        assert_eq!(
            loaded,
            MMapPathfindingContextLoadLikeCpp {
                mesh_map_id: 1,
                instance_map_id: 1,
                instance_id: 42,
                tile_x: 32,
                tile_y: 32,
                map_data_available: true,
                instance_query_available: true,
                tile_available: true,
                tile_loaded: true,
            }
        );
        assert!(manager.get_nav_mesh_query(1, 1, 42).is_some());
        assert!(
            manager
                .get_mmap_data(1)
                .unwrap()
                .loaded_tile_refs
                .contains_key(&pack_tile_id_like_cpp(32, 32))
        );

        let reused = manager
            .load_pathfinding_context_for_wow_position_like_cpp(&root, 1, 1, 42, 0.0, 0.0)
            .unwrap();
        assert_eq!(
            reused,
            MMapPathfindingContextLoadLikeCpp {
                tile_available: true,
                tile_loaded: false,
                ..loaded
            }
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn connected_obstacle_fixture_loads_from_grid_file_and_routes_at_pinned_height() {
        let root = unique_test_dir("connected-obstacle-height");
        let centre = [-10_118.333, 2_681.667, 218.49];
        let start = [centre[0], centre[1] - 10.0, centre[2]];
        let end = [centre[0], centre[1] + 10.0, centre[2]];
        write_obstacle_ring_mmaps_at_height_like_cpp(
            &root,
            1,
            &[(centre[0], centre[1], centre[2])],
        );
        assert_eq!(
            mmap_tile_coords_for_wow_position_like_cpp(centre[0], centre[1]),
            (50, 26)
        );

        let mut manager = MMapManager::new();
        for point in [start, end] {
            let loaded = manager
                .load_pathfinding_context_for_wow_position_like_cpp(
                    &root, 1, 1, 0, point[0], point[1],
                )
                .unwrap();
            assert!(loaded.map_data_available);
            assert!(loaded.instance_query_available);
            assert!(loaded.tile_available);
        }

        let filter = obstacle_ring_walk_filter();
        let path = manager
            .get_mmap_data(1)
            .unwrap()
            .calculate_path_for_instance_like_cpp(
                1,
                0,
                &filter,
                start,
                end,
                DetourPathOptions::default(),
            )
            .unwrap()
            .expect("the pinned instance query is loaded");
        assert!(
            path.point_path.points.len() > 2,
            "the direct segment crosses the missing centre cell, so Detour must add a turn"
        );
        assert!((path.point_path.points[0][2] - centre[2]).abs() < 0.01);
        assert!((path.point_path.points.last().unwrap()[2] - centre[2]).abs() < 0.01);
        assert!(
            path.point_path.points[1..path.point_path.points.len() - 1]
                .iter()
                .all(|point| (point[2] - (centre[2] + 0.5)).abs() < 0.01),
            "C++ FindSmoothPath raises intermediate polygon heights by 0.5: {:?}",
            path.point_path.points
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_pathfinding_context_missing_tile_falls_back_like_cpp() {
        let root = unique_test_dir("mmap-manager-missing-path-context");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 4096,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let mut manager = MMapManager::new();
        let loaded = manager
            .load_pathfinding_context_for_wow_position_like_cpp(&root, 1, 1, 42, 0.0, 0.0)
            .unwrap();

        assert_eq!(
            loaded,
            MMapPathfindingContextLoadLikeCpp {
                mesh_map_id: 1,
                instance_map_id: 1,
                instance_id: 42,
                tile_x: 32,
                tile_y: 32,
                map_data_available: true,
                instance_query_available: true,
                tile_available: false,
                tile_loaded: false,
            }
        );
        assert_eq!(manager.get_loaded_tiles_count(), 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_reports_missing_or_bad_tiles_like_cpp() {
        let root = unique_test_dir("mmap-manager-bad-tile");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 128,
            max_polys: 16_384,
        };
        std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

        let mut manager = MMapManager::new();
        assert!(matches!(
            manager.load_map(&root, 1, 0, 0),
            Err(MMapManagerError::ReadTileFile { .. })
        ));

        let mut bad_header = MmapTileHeader::new(DT_NAVMESH_VERSION_LIKE_CPP).to_bytes();
        bad_header[0] = 0;
        std::fs::write(tile_file_path_like_cpp(&root, 1, 0, 0), bad_header).unwrap();
        assert!(matches!(
            manager.load_map(&root, 1, 0, 0),
            Err(MMapManagerError::TileFile { .. })
        ));
        assert_eq!(manager.get_loaded_tiles_count(), 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_tile_reader_uses_parent_fallback_like_cpp() {
        let root = unique_test_dir("mmap-manager-parent-tile");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let tile = generated_square_tile_blob(0, 0);
        write_mmap_tile_blob(&tile_file_path_like_cpp(&root, 571, 0, 0), &tile);

        let mut manager = MMapManager::new();
        manager.initialize_thread_unsafe([ThreadUnsafeMapData {
            map_id: 571,
            child_map_ids: vec![609],
        }]);

        let fallback = manager
            .read_tile_blob_with_parent_fallback(&root, 609, 0, 0)
            .unwrap();
        assert_eq!(fallback, tile);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_thread_unsafe_preloads_allowed_map_ids_like_cpp() {
        let root = unique_test_dir("mmap-manager-thread-unsafe");
        std::fs::create_dir_all(root.join("mmaps")).unwrap();

        let params = DetourNavMeshParams {
            origin: [10.0, 20.0, 30.0],
            tile_width: 533.3333,
            tile_height: 533.3333,
            max_tiles: 256,
            max_polys: 32_768,
        };
        std::fs::write(root.join("mmaps/0571.mmap"), params.to_bytes()).unwrap();

        let mut manager = MMapManager::new();
        manager.initialize_thread_unsafe([ThreadUnsafeMapData {
            map_id: 571,
            child_map_ids: vec![609],
        }]);

        assert!(!manager.is_thread_safe_environment());
        assert_eq!(manager.get_loaded_maps_count(), 1);
        assert_eq!(manager.get_nav_mesh_params(571), None);
        assert_eq!(manager.parent_map_id(609), Some(571));
        assert!(matches!(manager.load_map_data(&root, 571), Ok(true)));
        assert_eq!(manager.get_nav_mesh_params(571), Some(params));
        assert!(matches!(
            manager.load_map_data(&root, 1),
            Err(MMapManagerError::InvalidMapInThreadUnsafe { map_id: 1 })
        ));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mmap_manager_keeps_placeholder_after_missing_file_like_cpp() {
        let root = unique_test_dir("mmap-manager-missing-file");
        let mut manager = MMapManager::new();

        assert!(matches!(
            manager.load_map_data(&root, 999),
            Err(MMapManagerError::ReadMapFile { .. })
        ));
        assert_eq!(manager.get_loaded_maps_count(), 1);
        assert_eq!(manager.get_nav_mesh_params(999), None);
    }

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "rustycore-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    fn generated_square_tile_blob(tile_x: i32, tile_y: i32) -> MmapTileBlob {
        let mut data = std::ptr::null_mut();
        let mut data_size = 0;
        assert!(unsafe {
            rustycore_dt_create_square_tile_data(tile_x, tile_y, &mut data, &mut data_size)
        });
        assert!(!data.is_null());
        assert!(data_size > 0);

        let bytes = unsafe { std::slice::from_raw_parts(data, data_size as usize) }.to_vec();
        unsafe { rustycore_dt_free(data.cast()) };

        MmapTileBlob {
            header: MmapTileHeader {
                mmap_magic: MMAP_MAGIC_LIKE_CPP,
                dt_version: DT_NAVMESH_VERSION_LIKE_CPP,
                mmap_version: MMAP_VERSION_LIKE_CPP,
                size: data_size as u32,
                uses_liquids: true,
                padding: [0, 0, 0],
            },
            data: bytes,
        }
    }

    use crate::test_fixtures::*;

    fn write_mmap_tile_blob(path: &std::path::Path, tile: &MmapTileBlob) {
        let mut bytes = tile.header.to_bytes().to_vec();
        bytes.extend_from_slice(&tile.data);
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn detour_obstacle_fixture_leaves_the_centre_cell_unwalkable() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let half = OBSTACLE_TILE_CELL_SIZE / 2.0;

        // Every ring cell resolves to a polygon at its own centre.
        for centre in [
            [half, 0.0, half + OBSTACLE_TILE_CELL_SIZE],
            [
                half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
                0.0,
                half + OBSTACLE_TILE_CELL_SIZE,
            ],
            [half + OBSTACLE_TILE_CELL_SIZE, 0.0, half],
            [
                half + OBSTACLE_TILE_CELL_SIZE,
                0.0,
                half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
            ],
        ] {
            let (poly, distance) = get_poly_by_location_like_cpp(&query, &filter, centre).unwrap();
            assert_ne!(poly, 0, "ring cell {centre:?} must be walkable");
            assert!(distance < 1.0, "ring cell {centre:?} distance {distance}");
        }

        // The obstacle centre is covered by no polygon. C++
        // `GetPolyByLocation` has no distance cut-off on its
        // `findNearestPoly` branch, so it still answers with the closest ring
        // polygon — but at a distance of about half a cell, which is what
        // `BuildPolyPath`'s `distToStartPoly > 7.0f` test keys off.
        let hole = [
            half + OBSTACLE_TILE_CELL_SIZE,
            0.0,
            half + OBSTACLE_TILE_CELL_SIZE,
        ];
        let (_, hole_distance) = get_poly_by_location_like_cpp(&query, &filter, hole).unwrap();
        assert!(
            hole_distance >= half,
            "the obstacle centre must not sit inside a polygon, got distance {hole_distance}"
        );
    }

    #[test]
    fn detour_obstacle_fixture_preserves_connected_world_height() {
        let origin = [2_666.6667, 218.49, -10_133.333];
        let params = DetourNavMeshParams {
            origin,
            tile_width: OBSTACLE_TILE_EXTENT,
            tile_height: OBSTACLE_TILE_EXTENT,
            max_tiles: 4,
            max_polys: 256,
        };
        let tile = obstacle_ring_tile_blob_at_height(0, 0, origin[0], origin[1], origin[2]);
        let mut mesh = DetourNavMesh::new(&params).unwrap();
        assert_ne!(mesh.add_tile(&tile).unwrap(), 0);
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();

        let point = [
            origin[0] + OBSTACLE_TILE_CELL_SIZE / 2.0,
            origin[1],
            origin[2] + OBSTACLE_TILE_CELL_SIZE / 2.0,
        ];
        let nearest = query
            .find_nearest_poly(point, [3.0, 5.0, 3.0], &filter)
            .unwrap();
        assert_ne!(nearest.poly_ref, 0);
        assert!(
            (nearest.nearest_point[1] - origin[1]).abs() < 0.001,
            "the generated polygon must remain at the connected fixture's live terrain height"
        );
    }

    #[test]
    fn detour_path_around_obstacle_returns_intermediate_points_like_cpp() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
        let mid = half + OBSTACLE_TILE_CELL_SIZE;

        // `wow_position_to_detour_like_cpp` maps WoW (x, y, z) to Detour
        // (y, z, x), so a WoW position (mid, y, 0) sits at Detour (y, 0, mid).
        // Start and end are the centres of the -x and +x ring cells on the
        // middle row: the direct segment between them crosses the obstacle.
        let start_wow = [mid, half, 0.0];
        let end_wow = [mid, half + 2.0 * OBSTACLE_TILE_CELL_SIZE, 0.0];

        let path = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            start_wow,
            end_wow,
            DetourPathOptions::default(),
        )
        .unwrap();

        assert!(
            path.point_path.path_type.contains(DetourPathType::NORMAL),
            "expected a normal Detour path, got {:?}",
            path.point_path.path_type
        );
        assert!(
            !path
                .point_path
                .path_type
                .intersects(DetourPathType::NOPATH | DetourPathType::SHORTCUT),
            "a navmesh route exists, so this must not degrade to a shortcut: {:?}",
            path.point_path.path_type
        );
        assert!(
            path.poly_refs.len() > 1,
            "routing around the obstacle needs more than one polygon, got {:?}",
            path.poly_refs
        );
        assert!(
            path.point_path.points.len() > 2,
            "a detour around the obstacle must carry intermediate points, got {:?}",
            path.point_path.points
        );

        // No point may cross the obstacle, which is exactly what the straight
        // line between start and end would have done. Points come back in WoW
        // space, so WoW x is Detour z and WoW y is Detour x.
        let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
        for point in &path.point_path.points {
            let inside_hole =
                hole_detour_z.contains(&point[0]) && hole_detour_x.contains(&point[1]);
            assert!(
                !inside_hole,
                "point {point:?} crosses the obstacle; points: {:?}",
                path.point_path.points
            );
        }

        // The route has to leave the middle row to get around, i.e. at least
        // one point sits in the -z or +z ring row.
        assert!(
            path.point_path.points.iter().any(|point| {
                let detour_z = point[0];
                detour_z < *hole_detour_z.start() || detour_z > *hole_detour_z.end()
            }),
            "the route never leaves the blocked row: {:?}",
            path.point_path.points
        );
    }

    #[test]
    fn detour_no_poly_grants_flying_owner_the_cpp_not_using_path_shortcut() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();

        // Far outside the fixture, so both `GetPolyByLocation` lookups fail.
        let start = [10_000.0f32, 0.0, 10_000.0];
        let end = [10_100.0f32, 0.0, 10_100.0];

        // A ground creature keeps the plain `PATHFIND_NOPATH` C++ assigns at
        // `PathGenerator.cpp:207`.
        let ground = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp::default(),
            &[],
        )
        .unwrap();
        assert_eq!(ground.point_path.path_type, DetourPathType::NOPATH);

        // `CanFly()` turns the same hole into the launchable shortcut
        // (`PathGenerator.cpp:180,198-202`).
        let flying = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp {
                can_fly: true,
                ..DetourOwnerCapabilitiesLikeCpp::default()
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            flying.point_path.path_type,
            DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
        );
        assert_eq!(flying.point_path.points, vec![start, end]);
        assert!(flying.poly_refs.is_empty());

        // Falling alone is not the no-poly exception — C++ only consults
        // `IsFalling()` in the far-from-poly branch.
        let falling = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp {
                is_falling: true,
                ..DetourOwnerCapabilitiesLikeCpp::default()
            },
            &[],
        )
        .unwrap();
        assert_eq!(falling.point_path.path_type, DetourPathType::NOPATH);
    }

    #[test]
    fn detour_far_from_poly_shortcuts_for_flying_and_falling_owners_like_cpp() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let half = OBSTACLE_TILE_CELL_SIZE / 2.0;

        // Start on a ring polygon; end high above another one so
        // `GetPolyByLocation` still resolves a polygon (its search box grows to
        // 50 on Y) while `distToEndPoly > 7.0f`.
        let start = [half, 0.0, half];
        let end = [half + 2.0 * OBSTACLE_TILE_CELL_SIZE, 40.0, half];

        let ground = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp::default(),
            &[],
        )
        .unwrap();
        assert!(
            ground.end_far_from_poly,
            "fixture must actually trip the 7.0 yard far-from-poly test"
        );
        assert!(
            ground
                .point_path
                .path_type
                .contains(DetourPathType::INCOMPLETE),
            "a ground creature takes the clamp + INCOMPLETE arm, got {:?}",
            ground.point_path.path_type
        );
        assert!(
            ground.point_path.points.is_empty(),
            "the corridor still falls through to BuildPointPath"
        );

        // C++ `PathGenerator.cpp:232-239`: a flying owner shortcuts instead.
        let flying = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp {
                can_fly: true,
                ..DetourOwnerCapabilitiesLikeCpp::default()
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            flying.point_path.path_type,
            DetourPathType::NORMAL
                | DetourPathType::NOT_USING_PATH
                | DetourPathType::FARFROMPOLY_END
        );
        assert_eq!(flying.point_path.points, vec![start, end]);

        // A falling owner only shortcuts while moving *downwards*; here the end
        // is above the start, so the INCOMPLETE arm must stand.
        let falling_upwards = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            start,
            end,
            DetourOwnerCapabilitiesLikeCpp {
                is_falling: true,
                ..DetourOwnerCapabilitiesLikeCpp::default()
            },
            &[],
        )
        .unwrap();
        assert!(
            falling_upwards
                .point_path
                .path_type
                .contains(DetourPathType::INCOMPLETE)
        );

        // Falling towards a lower destination is the C++ charge exception.
        let falling_downwards = build_straight_poly_path_like_cpp(
            &query,
            &filter,
            [half, 40.0, half],
            [half + 2.0 * OBSTACLE_TILE_CELL_SIZE, 0.0, half],
            DetourOwnerCapabilitiesLikeCpp {
                is_falling: true,
                ..DetourOwnerCapabilitiesLikeCpp::default()
            },
            &[],
        )
        .unwrap();
        assert!(
            falling_downwards
                .point_path
                .path_type
                .contains(DetourPathType::NOT_USING_PATH),
            "got {:?}",
            falling_downwards.point_path.path_type
        );
    }

    #[test]
    fn detour_point_path_over_the_limit_reports_only_shortcut_short_like_cpp() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
        let mid = half + OBSTACLE_TILE_CELL_SIZE;

        let start_wow = [mid, half, 0.0];
        let end_wow = [mid, half + 2.0 * OBSTACLE_TILE_CELL_SIZE, 0.0];

        // The route around the obstacle needs more points than this, so C++
        // `BuildPointPath` takes its `pointCount >= _pointPathLimit` branch:
        // `BuildShortcut()` then `_type |= PATHFIND_SHORT`
        // (`PathGenerator.cpp:585-590`). Because `BuildShortcut()` assigns
        // `PATHFIND_SHORTCUT`, the corridor's `PATHFIND_NORMAL` must not
        // survive — a `NORMAL | SHORTCUT | SHORT` result would mean a second,
        // discarded point-path pass leaked its flags in.
        let path = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            start_wow,
            end_wow,
            DetourPathOptions {
                point_path_limit: 3,
                ..DetourPathOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            path.point_path.path_type,
            DetourPathType::SHORTCUT | DetourPathType::SHORT
        );
        assert_eq!(path.point_path.points.len(), 2);
        assert!(
            path.poly_refs.is_empty(),
            "C++ BuildShortcut calls Clear(), including the point-limit branch"
        );
    }

    #[test]
    fn detour_far_force_destination_shortcut_clears_the_corridor_like_cpp() {
        let mesh = obstacle_ring_nav_mesh();
        let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
        let filter = obstacle_ring_walk_filter();
        let half = OBSTACLE_TILE_CELL_SIZE / 2.0;

        // Detour `(x, y, z)` maps back to WoW `(z, x, y)`. The requested
        // destination is forty yards above its polygon, so BuildPolyPath clamps
        // it and marks the route incomplete. With forceDestination enabled the
        // clamped suffix is far enough from the request for C++ to call
        // BuildShortcut(), which must also Clear() the polygon corridor.
        let start_wow = [half, half, 0.0];
        let end_wow = [half, half + 2.0 * OBSTACLE_TILE_CELL_SIZE, 40.0];
        let path = calculate_detour_path_like_cpp(
            &mesh,
            &query,
            &filter,
            start_wow,
            end_wow,
            DetourPathOptions {
                force_destination: true,
                ..DetourPathOptions::default()
            },
        )
        .unwrap();

        assert_eq!(
            path.point_path.path_type,
            DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
        );
        assert_eq!(path.point_path.points, vec![start_wow, end_wow]);
        assert!(
            path.poly_refs.is_empty(),
            "the forceDestination BuildShortcut branch must not leak its old corridor"
        );
    }
}
