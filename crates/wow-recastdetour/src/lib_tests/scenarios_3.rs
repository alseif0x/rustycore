//! Navmesh FFI regressions, part 3 of 3.
//!
//! Moved out of the lib_tests.rs root under #658; every test is unchanged.

use super::*;

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
