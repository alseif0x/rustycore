//! Movement scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_mmap_pathfinder_resolves_mesh_map_from_phase_shift_like_cpp() {
    let data_dir = unique_temp_data_dir("mmap-phase-shift-mesh-map");
    let grid_idx = terrain_grid_bitset_index_like_cpp(31, 31).expect("valid grid index");
    fs::write(
        data_dir.join("maps").join("0571.tilelist"),
        tilelist_like_cpp([]),
    )
    .expect("write parent tilelist");
    fs::write(
        data_dir.join("maps").join("0609.tilelist"),
        tilelist_like_cpp([grid_idx]),
    )
    .expect("write child tilelist");
    let mut pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
        &data_dir,
        [(571, vec![609]), (609, Vec::new())],
    );
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    let request = WorldMMapPathRequestLikeCpp {
        start: Position::new(0.0, 0.0, 0.0, 0.0),
        destination: Position::new(20.0, 0.0, 0.0, 0.0),
        mesh_map_id: 571,
        instance_map_id: 571,
        instance_id: 42,
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        owner: DetourOwnerCapabilitiesLikeCpp::default(),
        previous_poly_refs: Vec::new(),
        force_destination: false,
        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
        phase_shift,
    };

    assert_eq!(
        pathfinder.resolve_mesh_map_id_for_path_request_like_cpp(&request),
        609
    );
    fs::remove_dir_all(data_dir).expect("remove test dir");
}
#[test]
fn detour_path_without_navmesh_matches_cpp_calculate_path_early_return() {
    let start = Position::new(10.0, 10.0, 3.0, 0.0);
    let destination = Position::new(25.0, 18.0, 4.0, 1.0);

    let path = detour_path_without_navmesh_like_cpp(start, destination);

    // C++ `BuildShortcut()` is exactly "start -> actual end", and
    // `CalculatePath` types it `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
    // (`PathGenerator.cpp:83-85`).
    assert_eq!(
        path.point_path.path_type,
        DetourPathType::NORMAL | DetourPathType::NOT_USING_PATH
    );
    assert_eq!(
        path.point_path.points,
        vec![[10.0, 10.0, 3.0], [25.0, 18.0, 4.0]]
    );
    assert_eq!(path.point_path.actual_end, [25.0, 18.0, 4.0]);
    assert!(path.poly_refs.is_empty());
    assert!(!path.start_far_from_poly);
    assert!(!path.end_far_from_poly);

    // The type must be usable, otherwise the random generator would refuse
    // to launch it the way it refuses NOPATH/SHORTCUT.
    let path_type = path_type_from_detour_like_cpp(path.point_path.path_type);
    assert_eq!(
        random_path_result_from_path_type_like_cpp(path_type),
        RandomPathResult::Success
    );
    assert!(!path_type.intersects(PathType::NOPATH | PathType::SHORTCUT));
}
/// C++ `PathGenerator::CalculatePath` needs `HaveTile(start)` **and**
/// `HaveTile(dest)` (`PathGenerator.cpp:79-86`); it gets both because
/// `TerrainInfo::LoadMMap` pushes each grid's `.mmtile` in as the grid
/// loads. RustyCore loads tiles on demand from the path request, so the
/// pathfinder has to demand-load the destination's tile too — otherwise a
/// destination one tile over reports "no navmesh" and the caller degrades to
/// a straight line even though the mesh is on disk.
#[test]
fn world_mmap_pathfinder_demand_loads_the_destination_tile_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    // WoW y drives the Detour x axis, so stepping y across
    // `SIZE_OF_GRIDS_LIKE_CPP` moves both the navmesh tile and the grid file
    // the tile is named after.
    let start = Position::new(half, half, 0.0, 0.0);
    let destination = Position::new(half, SIZE_OF_GRIDS_LIKE_CPP + half, 0.0, 0.0);
    assert_ne!(
        wow_recastdetour::mmap_tile_coords_for_wow_position_like_cpp(start.x, start.y),
        wow_recastdetour::mmap_tile_coords_for_wow_position_like_cpp(destination.x, destination.y),
        "the fixture must straddle a grid-file seam to be meaningful"
    );

    let root = unique_test_dir("world-mmap-pathfinder-destination-tile");
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(
        &root,
        MAP_ID,
        &[(start.x, start.y), (destination.x, destination.y)],
    );

    let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&root);
    let result = pathfinder.calculate_path_from_positions_like_cpp(
        start,
        destination,
        MAP_ID,
        MAP_ID,
        0,
        PathQueryFilterContext::creature(true, false, false, false),
        DetourOwnerCapabilitiesLikeCpp::default(),
        &[],
        false,
        MAX_POINT_PATH_LENGTH_LIKE_CPP,
    );

    // The query must actually run: reporting `Ok(None)` here would mean the
    // destination tile was never loaded and the caller silently
    // straight-lines.
    let path = result
        .expect("query must not error")
        .expect("both endpoint tiles are on disk, so the query must run");

    // The two fixture tiles are disconnected islands, so `findPath` returns
    // a partial corridor that never reaches `endPoly`. That is the C++
    // `PATHFIND_INCOMPLETE` tail in `BuildPolyPath`
    // (`PathGenerator.cpp:519-522`) — a real navmesh answer, not a shortcut.
    assert!(
        path.point_path
            .path_type
            .contains(DetourPathType::INCOMPLETE),
        "expected a partial navmesh corridor, got {:?}",
        path.point_path.path_type
    );
    assert!(
        !path
            .point_path
            .path_type
            .intersects(DetourPathType::NOT_USING_PATH),
        "the mesh was queried, so this must not be the no-navmesh shortcut: {:?}",
        path.point_path.path_type
    );
    assert_eq!(
        pathfinder.mmap_manager().get_loaded_tiles_count(),
        2,
        "the start tile and the destination tile must both be resident"
    );

    let _ = std::fs::remove_dir_all(&root);
}
#[test]
fn world_mmap_pathfinder_falls_back_when_runtime_tile_missing_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-missing-tile");
    std::fs::create_dir_all(root.join("mmaps")).unwrap();
    let params = wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 4096,
        max_polys: 16_384,
    };
    std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54326);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(0.0, 0.0, 0.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        0,
    );
    let mut pathfinder = WorldMMapPathfinderLikeCpp::new(&root);
    let filter_context = PathQueryFilterContext::creature(true, false, false, false);

    assert_eq!(
        pathfinder.calculate_creature_path_like_cpp(
            &creature,
            Position::new(20.0, 0.0, 0.0, 0.0),
            1,
            1,
            42,
            filter_context,
            false,
        ),
        Ok(None)
    );
    assert!(
        pathfinder
            .mmap_manager()
            .get_nav_mesh_query(1, 1, 42)
            .is_some()
    );
    assert_eq!(pathfinder.mmap_manager().get_loaded_tiles_count(), 0);

    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn world_mmap_pathfinder_worker_keeps_detour_off_session_thread_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-worker-missing-tile");
    std::fs::create_dir_all(root.join("mmaps")).unwrap();
    let params = wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 4096,
        max_polys: 16_384,
    };
    std::fs::write(root.join("mmaps/0001.mmap"), params.to_bytes()).unwrap();

    let worker = WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let result = worker.calculate_path_like_cpp(WorldMMapPathRequestLikeCpp {
        start: Position::new(0.0, 0.0, 0.0, 0.0),
        destination: Position::new(20.0, 0.0, 0.0, 0.0),
        mesh_map_id: 1,
        instance_map_id: 1,
        instance_id: 42,
        filter_context: PathQueryFilterContext::creature(true, false, false, false),
        owner: DetourOwnerCapabilitiesLikeCpp::default(),
        previous_poly_refs: Vec::new(),
        force_destination: false,
        point_path_limit: MAX_POINT_PATH_LENGTH_LIKE_CPP,
        phase_shift: PhaseShift::default(),
    });

    assert_eq!(result, Ok(None));

    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn world_mmap_pathfinder_initializes_thread_unsafe_parent_map_data_like_cpp() {
    let root = unique_test_dir("world-mmap-pathfinder-parent-map-data");
    let pathfinder = WorldMMapPathfinderLikeCpp::new_with_parent_map_data_like_cpp(
        &root,
        [(571, vec![609]), (609, Vec::new())],
    );

    assert!(!pathfinder.mmap_manager().is_thread_safe_environment());
    assert_eq!(pathfinder.mmap_manager().get_loaded_maps_count(), 2);
    assert_eq!(pathfinder.mmap_manager().parent_map_id(609), Some(571));
}
#[test]
fn pending_respawn_rebuild_preserves_random_movement_type_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.random_movement_type = wow_constants::CreatureRandomMovementType::AlwaysRun as u8;

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.random_movement_type_like_cpp(),
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
        "C++ respawn keeps using Creature::GetMovementTemplate(); Rust respawn must preserve the captured Random movement metadata"
    );
}
#[test]
fn pending_respawn_rebuild_preserves_default_movement_and_path_like_cpp() {
    let mut pending = make_pending_respawn(Instant::now());
    pending.default_movement_type = MovementGeneratorType::Waypoint;
    pending.waypoint_path_id = 9_002;

    let creature = world_creature_from_pending_respawn_like_cpp(&pending, 0);

    assert_eq!(
        creature.creature.default_movement_type(),
        MovementGeneratorType::Waypoint,
        "C++ respawn reload path uses Creature::LoadFromDB/LoadCreaturesAddon and keeps the selected default motion"
    );
    assert_eq!(
        creature.creature.waypoint_path_id_like_cpp(),
        9_002,
        "C++ Creature::LoadCreaturesAddon preserves nonzero PathId for waypoint movement after respawn"
    );
}
