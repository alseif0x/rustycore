// SPDX-License-Identifier: GPL-3.0-or-later
// Moved 2026-10-20 from lib.rs; existing fixture symbols and ABI are unchanged.
//! Native builders and deallocator used only by synthetic navmesh fixtures.

unsafe extern "C" {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) fn rustycore_dt_free(ptr: *mut std::ffi::c_void);

    #[cfg(test)]
    pub(super) fn rustycore_dt_create_square_tile_data(
        tile_x: i32,
        tile_y: i32,
        out_data: *mut *mut u8,
        out_data_size: *mut i32,
    ) -> bool;

    // Test-only fixture builder; see `test_fixtures::obstacle_ring_tile_blob`.
    #[cfg(any(test, feature = "test-fixtures"))]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn rustycore_dt_create_poly_mesh_tile_data(
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
