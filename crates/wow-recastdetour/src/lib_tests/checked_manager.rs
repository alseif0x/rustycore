use super::*;

const TEST_MAP_ID: u32 = 43_210;

fn checked_params(tile_width: f32, tile_height: f32) -> CheckedDetourNavMeshParams {
    CheckedDetourNavMeshParams::new(
        DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width,
            tile_height,
            max_tiles: 4,
            max_polys: 256,
        },
        CheckedNativeParamsLimits::default(),
    )
    .unwrap()
}

fn checked_square_from(bytes: &[u8]) -> CheckedDetour7GroundPayload<'_> {
    CheckedDetour7GroundPayload::parse(bytes).unwrap()
}

fn initialize_manager() -> MMapManager {
    let mut manager = MMapManager::new();
    manager.initialize_thread_unsafe([ThreadUnsafeMapData {
        map_id: TEST_MAP_ID,
        child_map_ids: Vec::new(),
    }]);
    manager
}

fn disconnected_ground_data() -> Vec<u8> {
    const MESH_NULL_IDX: u16 = 0xffff;
    let verts: [u16; 24] = [
        0, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 2, 0, 0, 3, 0, 0, 3, 0, 1, 2, 0, 1,
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
            8,
            polys.as_ptr(),
            2,
            4,
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
    bytes
}

#[test]
fn checked_params_reject_unsafe_values_and_only_accept_downward_limits() {
    let mut params = DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 1.0,
        tile_height: 1.0,
        max_tiles: 4,
        max_polys: 16,
    };
    assert!(CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()).is_ok());

    params.origin[0] = f32::INFINITY;
    assert!(matches!(
        CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()),
        Err(CheckedNativeParamsError::InvalidOrigin { axis: 0, .. })
    ));

    params.origin = [0.0; 3];
    params.tile_width = 0.0;
    assert!(matches!(
        CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()),
        Err(CheckedNativeParamsError::InvalidTileDimension { .. })
    ));

    params.tile_width = 1.0;
    params.max_tiles = CHECKED_NATIVE_MAX_TILE_SLOTS + 1;
    assert!(matches!(
        CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()),
        Err(CheckedNativeParamsError::InvalidTileSlots { .. })
    ));

    params.max_tiles = 4;
    params.max_polys = 0;
    assert!(matches!(
        CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()),
        Err(CheckedNativeParamsError::InvalidMaxPolys { .. })
    ));

    params.max_polys = i32::MIN;
    assert!(CheckedDetourNavMeshParams::new(params, CheckedNativeParamsLimits::default()).is_ok());
    assert!(
        CheckedNativeParamsLimits::new(
            CHECKED_NATIVE_MAX_TILE_SLOTS + 1,
            CHECKED_NATIVE_MAX_TILE_DIMENSION,
            CHECKED_GROUND_MAX_ABS_COORDINATE,
        )
        .is_err()
    );
}

#[test]
fn checked_manager_requires_initialized_known_map_and_rejects_parameter_conflicts() {
    let mut uninitialized = MMapManager::new();
    assert!(matches!(
        uninitialized.admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0)),
        Err(MMapAdmissionError::ThreadUnsafeMapSetRequired)
    ));

    let mut manager = initialize_manager();
    assert!(matches!(
        manager.admit_checked_nav_mesh_params(TEST_MAP_ID + 1, checked_params(1.0, 1.0)),
        Err(MMapAdmissionError::MapNotInThreadUnsafeSet { .. })
    ));
    assert!(matches!(
        manager.admit_checked_ground_tile(
            TEST_MAP_ID,
            1,
            2,
            &checked_square_from(&generated_square_tile_blob(0, 0).data)
        ),
        Err(MMapAdmissionError::NavMeshParamsNotAdmitted { .. })
    ));

    assert!(
        manager
            .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
            .unwrap()
    );
    assert!(
        !manager
            .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
            .unwrap()
    );
    assert!(matches!(
        manager.admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(2.0, 1.0)),
        Err(MMapAdmissionError::ConflictingNavMeshParams { .. })
    ));
}

#[test]
fn checked_manager_tracks_original_wow_key_and_preserves_state_on_native_failure() {
    let mut manager = initialize_manager();
    manager
        .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
        .unwrap();
    let fixture = generated_square_tile_blob(0, 0);
    let checked = checked_square_from(&fixture.data);

    assert!(
        manager
            .admit_checked_ground_tile(TEST_MAP_ID, 32, 31, &checked)
            .unwrap()
    );
    assert_eq!(manager.get_loaded_tiles_count(), 1);
    let original_key = pack_tile_id_like_cpp(32, 31);
    assert!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .loaded_tile_refs
            .contains_key(&original_key)
    );

    assert!(
        !manager
            .admit_checked_ground_tile(TEST_MAP_ID, 32, 31, &checked)
            .unwrap()
    );
    assert_eq!(manager.get_loaded_tiles_count(), 1);

    let native_failure = manager.admit_checked_ground_tile(TEST_MAP_ID, 33, 31, &checked);
    assert!(matches!(
        native_failure,
        Err(MMapAdmissionError::Tile(
            DetourTileError::AddTileFailed { .. }
        ))
    ));
    assert_eq!(manager.get_loaded_tiles_count(), 1);
    let failed_key = pack_tile_id_like_cpp(33, 31);
    assert!(
        !manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .loaded_tile_refs
            .contains_key(&failed_key)
    );

    assert!(manager.unload_map_tile(TEST_MAP_ID, 32, 31).unwrap());
    assert_eq!(manager.get_loaded_tiles_count(), 0);
    assert!(!manager.unload_map_tile(TEST_MAP_ID, 32, 31).unwrap());
    assert!(
        manager
            .admit_checked_ground_tile(TEST_MAP_ID, 33, 31, &checked)
            .unwrap()
    );
    assert_eq!(manager.get_loaded_tiles_count(), 1);
}

#[test]
fn checked_manager_query_budget_lifecycle_and_map_reload_use_existing_authority() {
    let mut manager = initialize_manager();
    manager
        .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
        .unwrap();
    let tile = generated_square_tile_blob(0, 0);
    let checked = checked_square_from(&tile.data);
    manager
        .admit_checked_ground_tile(TEST_MAP_ID, 32, 31, &checked)
        .unwrap();

    assert!(matches!(
        manager.admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 1, 3),
        Err(MMapAdmissionError::Query(
            DetourNavMeshQueryError::InvalidNodeLimit { max_nodes: 3 }
        ))
    ));
    assert!(
        manager
            .admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 1, 4)
            .unwrap()
    );
    assert!(
        manager
            .admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 2, 2048)
            .unwrap()
    );
    assert!(matches!(
        manager.admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 1, 8),
        Err(MMapAdmissionError::Query(
            DetourNavMeshQueryError::ConflictingNodeLimit {
                existing: 4,
                requested: 8,
                ..
            }
        ))
    ));
    assert_eq!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .nav_mesh_query_count(),
        2
    );
    assert!(matches!(
        manager.admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 1, 65_536),
        Err(MMapAdmissionError::Query(
            DetourNavMeshQueryError::InvalidNodeLimit { .. }
        ))
    ));

    assert!(manager.unload_map_instance(TEST_MAP_ID, TEST_MAP_ID, 1));
    assert!(
        manager
            .admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 1, 1024)
            .unwrap()
    );
    assert_eq!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .nav_mesh_query_count(),
        2
    );

    assert!(manager.unload_map(TEST_MAP_ID));
    assert_eq!(manager.get_loaded_tiles_count(), 0);
    assert!(manager.get_mmap_data(TEST_MAP_ID).is_none());
    assert!(
        manager
            .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
            .unwrap()
    );
    assert_eq!(manager.get_loaded_tiles_count(), 0);
}

#[test]
fn mmap_data_corridor_returns_complete_facts_and_explicit_missing_results() {
    let mut manager = initialize_manager();
    manager
        .admit_checked_nav_mesh_params(TEST_MAP_ID, checked_params(1.0, 1.0))
        .unwrap();
    let tile = generated_square_tile_blob(0, 0);
    let checked = checked_square_from(&tile.data);
    manager
        .admit_checked_ground_tile(TEST_MAP_ID, 32, 31, &checked)
        .unwrap();
    let filter = DetourQueryFilter::new().unwrap();

    assert!(matches!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .find_detour_corridor_for_instance(
                TEST_MAP_ID,
                9,
                &filter,
                [0.1, 0.0, 0.1],
                [0.9, 0.0, 0.9],
                [0.5, 1.0, 0.5],
                64,
            ),
        Err(DetourCorridorQueryError::MissingInstanceQuery { .. })
    ));
    manager
        .admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 9, 1024)
        .unwrap();

    let complete = manager
        .get_mmap_data(TEST_MAP_ID)
        .unwrap()
        .find_detour_corridor_for_instance(
            TEST_MAP_ID,
            9,
            &filter,
            [0.1, 0.0, 0.1],
            [0.9, 0.0, 0.9],
            [0.5, 1.0, 0.5],
            64,
        )
        .unwrap();
    assert!(complete.start.poly_ref.is_some());
    assert!(complete.end.poly_ref.is_some());
    let path = complete.path.unwrap();
    assert_ne!(path.status & DT_SUCCESS_LIKE_CPP, 0);
    assert_eq!(path.status & DT_PARTIAL_RESULT_LIKE_CPP, 0);
    assert_eq!(path.polys.len(), 1);

    let no_nearest = manager
        .get_mmap_data(TEST_MAP_ID)
        .unwrap()
        .find_detour_corridor_for_instance(
            TEST_MAP_ID,
            9,
            &filter,
            [100.0, 100.0, 100.0],
            [100.0, 100.0, 100.0],
            [0.0; 3],
            64,
        )
        .unwrap();
    assert!(no_nearest.start.poly_ref.is_none());
    assert!(no_nearest.end.poly_ref.is_none());
    assert!(no_nearest.path.is_none());

    assert!(matches!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .find_detour_corridor_for_instance(
                TEST_MAP_ID,
                9,
                &filter,
                [f32::NAN, 0.0, 0.0],
                [0.9, 0.0, 0.9],
                [0.5, 1.0, 0.5],
                64,
            ),
        Err(DetourCorridorQueryError::InvalidPosition { .. })
    ));
    assert!(matches!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .find_detour_corridor_for_instance(
                TEST_MAP_ID,
                9,
                &filter,
                [0.1, 0.0, 0.1],
                [0.9, 0.0, 0.9],
                [-0.1, 1.0, 0.5],
                64,
            ),
        Err(DetourCorridorQueryError::InvalidHalfExtent { .. })
    ));
}

#[test]
fn mmap_data_corridor_preserves_detour_partial_status_for_disconnected_islands() {
    let mut manager = initialize_manager();
    let params = CheckedDetourNavMeshParams::new(
        DetourNavMeshParams {
            origin: [0.0, 0.0, 0.0],
            tile_width: 30.0,
            tile_height: 10.0,
            max_tiles: 4,
            max_polys: 128,
        },
        CheckedNativeParamsLimits::default(),
    )
    .unwrap();
    manager
        .admit_checked_nav_mesh_params(TEST_MAP_ID, params)
        .unwrap();
    let bytes = disconnected_ground_data();
    let checked = CheckedDetour7GroundPayload::parse(&bytes).unwrap();
    manager
        .admit_checked_ground_tile(TEST_MAP_ID, 32, 31, &checked)
        .unwrap();
    manager
        .admit_instance_query(TEST_MAP_ID, TEST_MAP_ID, 10, 1024)
        .unwrap();
    let filter = DetourQueryFilter::new().unwrap();

    let partial = manager
        .get_mmap_data(TEST_MAP_ID)
        .unwrap()
        .find_detour_corridor_for_instance(
            TEST_MAP_ID,
            10,
            &filter,
            [5.0, 0.0, 5.0],
            [25.0, 0.0, 5.0],
            [2.0, 2.0, 2.0],
            64,
        )
        .unwrap();
    assert!(partial.start.poly_ref.is_some());
    assert!(partial.end.poly_ref.is_some());
    let path = partial.path.unwrap();
    assert_ne!(path.status & DT_PARTIAL_RESULT_LIKE_CPP, 0);
    assert!(!path.polys.is_empty());

    assert!(matches!(
        manager
            .get_mmap_data(TEST_MAP_ID)
            .unwrap()
            .find_detour_corridor_for_instance(
                TEST_MAP_ID,
                10,
                &filter,
                [5.0, 0.0, 5.0],
                [25.0, 0.0, 5.0],
                [2.0, 2.0, 2.0],
                CHECKED_CORRIDOR_MAX_PATH + 1,
            ),
        Err(DetourCorridorQueryError::InvalidPathCapacity { .. })
    ));
}
