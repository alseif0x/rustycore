//! Behaviour tests for [`super`].
//!
//! Extracted from `lib.rs`, which was 5,664 lines of which
//! 2,470 — 44% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

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
    write_obstacle_ring_mmaps_at_height_like_cpp(&root, 1, &[(centre[0], centre[1], centre[2])]);
    assert_eq!(
        mmap_tile_coords_for_wow_position_like_cpp(centre[0], centre[1]),
        (50, 26)
    );

    let mut manager = MMapManager::new();
    for point in [start, end] {
        let loaded = manager
            .load_pathfinding_context_for_wow_position_like_cpp(&root, 1, 1, 0, point[0], point[1])
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
        let inside_hole = hole_detour_z.contains(&point[0]) && hole_detour_x.contains(&point[1]);
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
        DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH | DetourPathType::FARFROMPOLY_END
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
