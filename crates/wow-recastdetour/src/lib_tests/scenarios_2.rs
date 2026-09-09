//! Navmesh FFI regressions, part 2 of 3.
//!
//! Moved out of the lib_tests.rs root under #658; every test is unchanged.

use super::*;

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
