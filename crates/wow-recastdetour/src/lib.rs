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

/// In-memory Detour navmesh fixtures for tests in this crate and downstream
/// crates.
///
/// Gated behind the `test-fixtures` feature (always on for this crate's own
/// tests) so nothing here reaches a production build.
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_fixtures;

mod navmesh;
#[cfg(test)]
#[path = "lib_tests/mod.rs"]
mod tests;
pub use navmesh::*;
