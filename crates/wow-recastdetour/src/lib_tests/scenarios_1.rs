//! Navmesh FFI regressions, part 1 of 3.
//!
//! Moved out of the lib_tests.rs root under #658; every test is unchanged.

use super::*;

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
