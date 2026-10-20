//! Raw Detour path-query status regressions.

use super::*;

const ROUTE_START: [f32; 3] = [5.0, 0.0, 5.0];
const ROUTE_END: [f32; 3] = [25.0, 0.0, 25.0];
const ROUTE_HALF_EXTENTS: [f32; 3] = [3.0, 5.0, 3.0];

fn obstacle_route_refs(
    query: &DetourNavMeshQuery<'_>,
    filter: &DetourQueryFilter,
) -> (DetourPolyRef, DetourPolyRef) {
    let start = query
        .find_nearest_poly(ROUTE_START, ROUTE_HALF_EXTENTS, filter)
        .unwrap()
        .poly_ref;
    let end = query
        .find_nearest_poly(ROUTE_END, ROUTE_HALF_EXTENTS, filter)
        .unwrap()
        .poly_ref;
    assert_ne!(start, 0);
    assert_ne!(end, 0);
    assert_ne!(start, end);
    (start, end)
}

#[test]
fn successful_path_exposes_native_status_and_matches_legacy_corridor() {
    assert_eq!(DT_OUT_OF_NODES_LIKE_CPP, 1 << 5);
    assert_eq!(DT_PARTIAL_RESULT_LIKE_CPP, 1 << 6);

    let mesh = obstacle_ring_nav_mesh();
    let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
    let filter = obstacle_ring_walk_filter();
    let (start, _) = obstacle_route_refs(&query, &filter);

    let result = query
        .find_path_with_status(
            start,
            start,
            ROUTE_START,
            ROUTE_START,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();

    assert_eq!(result.status, DT_SUCCESS_LIKE_CPP);
    assert_eq!(result.polys, vec![start]);
    assert_eq!(
        query.find_path(
            start,
            start,
            ROUTE_START,
            ROUTE_START,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        ),
        Ok(result.polys.clone())
    );
}

#[test]
fn connected_nontrivial_path_exposes_success_and_matches_legacy_corridor() {
    let mesh = obstacle_ring_nav_mesh();
    let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
    let filter = obstacle_ring_walk_filter();
    let (start, end) = obstacle_route_refs(&query, &filter);

    let result = query
        .find_path_with_status(
            start,
            end,
            ROUTE_START,
            ROUTE_END,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();

    assert_eq!(result.status, DT_SUCCESS_LIKE_CPP);
    assert!(result.polys.len() > 1);
    assert_eq!(result.polys.last(), Some(&end));
    assert_eq!(
        query.find_path(
            start,
            end,
            ROUTE_START,
            ROUTE_END,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        ),
        Ok(result.polys.clone())
    );
}

#[test]
fn disconnected_path_preserves_partial_result_status_and_legacy_corridor() {
    let mesh = disconnected_two_island_nav_mesh();
    let query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
    let filter = DetourQueryFilter::new().unwrap();
    let start_pos = [5.0, 0.0, 5.0];
    let end_pos = [25.0, 0.0, 5.0];
    let start = query
        .find_nearest_poly(start_pos, ROUTE_HALF_EXTENTS, &filter)
        .unwrap()
        .poly_ref;
    let end = query
        .find_nearest_poly(end_pos, ROUTE_HALF_EXTENTS, &filter)
        .unwrap()
        .poly_ref;
    assert_ne!(start, 0);
    assert_ne!(end, 0);
    assert_ne!(start, end);

    let result = query
        .find_path_with_status(
            start,
            end,
            start_pos,
            end_pos,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();

    assert_eq!(
        result.status,
        DT_SUCCESS_LIKE_CPP | DT_PARTIAL_RESULT_LIKE_CPP
    );
    assert_eq!(result.polys, vec![start]);
    assert_eq!(
        query.find_path(
            start,
            end,
            start_pos,
            end_pos,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        ),
        Ok(result.polys.clone())
    );
}

#[test]
fn node_exhaustion_and_short_output_buffer_keep_detour_detail_bits() {
    let mesh = obstacle_ring_nav_mesh();
    let filter = obstacle_ring_walk_filter();

    let full_query = DetourNavMeshQuery::new(&mesh, 1024).unwrap();
    let (start, end) = obstacle_route_refs(&full_query, &filter);
    let complete = full_query
        .find_path_with_status(
            start,
            end,
            ROUTE_START,
            ROUTE_END,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();
    assert_eq!(complete.status, DT_SUCCESS_LIKE_CPP);
    assert!(complete.polys.len() > 4);
    assert_eq!(complete.polys.first(), Some(&start));
    assert_eq!(complete.polys.last(), Some(&end));

    let constrained_query = DetourNavMeshQuery::new(&mesh, 4).unwrap();
    let exhausted = constrained_query
        .find_path_with_status(
            start,
            end,
            ROUTE_START,
            ROUTE_END,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        )
        .unwrap();
    assert_eq!(
        exhausted.status,
        DT_SUCCESS_LIKE_CPP | DT_PARTIAL_RESULT_LIKE_CPP | DT_OUT_OF_NODES_LIKE_CPP
    );
    assert_eq!(exhausted.polys.first(), Some(&start));
    assert_ne!(exhausted.polys.last(), Some(&end));
    assert!(exhausted.polys.len() <= 4);
    assert_eq!(
        constrained_query.find_path(
            start,
            end,
            ROUTE_START,
            ROUTE_END,
            &filter,
            MAX_PATH_LENGTH_LIKE_CPP,
        ),
        Ok(exhausted.polys.clone())
    );

    let truncated = full_query
        .find_path_with_status(start, end, ROUTE_START, ROUTE_END, &filter, 1)
        .unwrap();
    assert_eq!(
        truncated.status,
        DT_SUCCESS_LIKE_CPP | DT_BUFFER_TOO_SMALL_LIKE_CPP
    );
    assert_eq!(truncated.polys.len(), 1);
    assert_eq!(
        full_query.find_path(start, end, ROUTE_START, ROUTE_END, &filter, 1),
        Ok(truncated.polys.clone())
    );
}

#[test]
fn invalid_refs_zero_capacity_and_oversized_capacity_preserve_existing_errors() {
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
    let valid_ref = query
        .find_nearest_poly([0.5, 0.0, 0.5], [3.0, 5.0, 3.0], &filter)
        .unwrap()
        .poly_ref;
    assert_ne!(valid_ref, 0);

    let invalid_ref_error = DetourNavMeshQueryError::FindPathFailed {
        status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
    };
    assert_eq!(
        query.find_path_with_status(0, valid_ref, [0.5, 0.0, 0.5], [0.5, 0.0, 0.5], &filter, 4),
        Err(invalid_ref_error)
    );
    assert_eq!(
        query.find_path(0, valid_ref, [0.5, 0.0, 0.5], [0.5, 0.0, 0.5], &filter, 4),
        Err(invalid_ref_error)
    );

    assert_eq!(
        query.find_path_with_status(
            valid_ref,
            valid_ref,
            [0.5, 0.0, 0.5],
            [0.5, 0.0, 0.5],
            &filter,
            0,
        ),
        Err(DetourNavMeshQueryError::FindPathFailed {
            status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
        })
    );
    assert_eq!(
        query.find_path(
            valid_ref,
            valid_ref,
            [0.5, 0.0, 0.5],
            [0.5, 0.0, 0.5],
            &filter,
            0,
        ),
        Err(DetourNavMeshQueryError::FindPathFailed {
            status: DT_FAILURE_LIKE_CPP | DT_INVALID_PARAM_LIKE_CPP,
        })
    );

    if let Some(too_large) = (i32::MAX as usize).checked_add(1) {
        let too_large_error = DetourNavMeshQueryError::PathBufferTooLarge {
            max_path: too_large,
        };
        assert_eq!(
            query.find_path_with_status(0, 0, [0.0; 3], [0.0; 3], &filter, too_large),
            Err(too_large_error)
        );
        assert_eq!(
            query.find_path(0, 0, [0.0; 3], [0.0; 3], &filter, too_large),
            Err(too_large_error)
        );
    }
}

#[test]
fn both_query_constructors_reject_unsupported_node_limits_before_native_init() {
    let params = DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 1.0,
        tile_height: 1.0,
        max_tiles: 16,
        max_polys: 128,
    };
    let mesh = DetourNavMesh::new(&params).unwrap();

    for max_nodes in [-1, 0, 1, 2, 3, 65_536, i32::MAX] {
        assert!(matches!(
            DetourNavMeshQuery::new(&mesh, max_nodes),
            Err(DetourNavMeshQueryError::InvalidNodeLimit {
                max_nodes: actual
            }) if actual == max_nodes
        ));
        assert!(matches!(
            MMapNavMeshQuery::new(&mesh, max_nodes),
            Err(DetourNavMeshQueryError::InvalidNodeLimit {
                max_nodes: actual
            }) if actual == max_nodes
        ));
    }
}

#[test]
fn both_query_constructors_accept_supported_node_limit_boundaries() {
    let params = DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 1.0,
        tile_height: 1.0,
        max_tiles: 16,
        max_polys: 128,
    };
    let mesh = DetourNavMesh::new(&params).unwrap();

    for max_nodes in [4, 65_535] {
        assert!(DetourNavMeshQuery::new(&mesh, max_nodes).is_ok());
        assert!(MMapNavMeshQuery::new(&mesh, max_nodes).is_ok());
    }
}
