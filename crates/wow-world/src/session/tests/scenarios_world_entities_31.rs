//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn step_creature_movement_walking_waypoint_launches_monster_move_like_cpp() {
    let guid = test_creature_guid(200_004);
    let mut creature = make_test_world_creature(guid);
    let path = wow_movement::WaypointPath::new(
        42,
        vec![wow_movement::WaypointNode::new(1, 12.0, 10.0, 0.0)],
    );

    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        wow_movement::WaypointMovementAction::StopMoving
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingWaypoint,
        "C++ default waypoint motion is the active creature movement generator"
    );

    let config = MMapRuntimeConfigLikeCpp::default();
    let result = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        None,
        None,
        None,
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    );

    let bytes = result.expect("Waypoint StartMove must launch a MonsterMove packet");
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(
        opcode,
        wow_constants::ServerOpcodes::OnMonsterMove as u16,
        "Waypoint StartMove must be visible through the same MonsterMove fanout rail as C++ MoveSplineInit::Launch"
    );
    assert_eq!(creature.move_target().unwrap().x, 12.0);
    assert!(creature.active_move_spline_like_cpp().is_some());
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingWaypoint
    );
}
/// End-to-end proof for #24: the live creature tick queries a **real**
/// Detour navmesh through the runtime pathfinder and walks around an
/// obstacle instead of straight through it.
///
/// The fixture mesh is a walkable ring with an unwalkable centre cell, laid
/// out so that the straight segment from the creature to its waypoint node
/// crosses the hole. C++ `WaypointMovementGenerator<Creature>::StartMove`
/// reaches `MoveSplineInit::MoveTo(..., generatePath = true)`, which runs
/// `PathGenerator::CalculatePath` and hands the multi-point result to
/// `MovebyPath` (`MoveSplineInit.cpp:261-277`).
#[test]
fn step_creature_movement_waypoint_paths_around_real_navmesh_obstacle_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, obstacle_hole_bounds, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    // `wow_position_to_detour_like_cpp` maps WoW (x, y, z) to Detour
    // (y, z, x), so WoW x selects the Detour z row and WoW y the Detour x
    // column. Start and destination are the two ring cells on the middle
    // row, with the obstacle between them.
    let start = Position::new(half + OBSTACLE_TILE_CELL_SIZE, half, 0.0, 0.0);
    let destination = Position::new(
        half + OBSTACLE_TILE_CELL_SIZE,
        half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
        0.0,
        0.0,
    );

    let root = std::env::temp_dir().join(format!(
        "rustycore-step-navmesh-obstacle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(&root, MAP_ID, &[(start.x, start.y)]);

    let guid = test_creature_guid(200_024);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(MAP_ID, 0)
        .expect("bind the fixture map");
    creature.creature.set_ai_position(start);
    creature.creature.set_ai_home_position(start);

    let path = wow_movement::WaypointPath::new(
        77,
        vec![wow_movement::WaypointNode::new(
            10,
            destination.x,
            destination.y,
            destination.z,
        )],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        wow_movement::WaypointMovementAction::StopMoving
    );

    let worker = crate::map_manager::WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let config = MMapRuntimeConfigLikeCpp {
        data_dir: root.display().to_string(),
        enabled: true,
        ..Default::default()
    };

    let bytes = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        Some(&worker),
        None,
        None,
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    )
    .expect("the waypoint leg must launch a MonsterMove");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );

    let spline = creature
        .active_move_spline_like_cpp()
        .expect("the launched spline");
    let points = spline.create_object_path_points_like_cpp();

    // A straight line here would be two endpoints only (four with the
    // Catmull-Rom end duplication). Detour has to contribute real
    // intermediate waypoints, and none of them may cross the obstacle.
    assert!(
        points.len() > 4,
        "expected navmesh waypoints between the endpoints, got {points:?}"
    );
    let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
    for point in points.iter() {
        let inside_hole = hole_detour_z.contains(&point.x) && hole_detour_x.contains(&point.y);
        assert!(
            !inside_hole,
            "point {point:?} walks through the obstacle; points: {points:?}"
        );
    }
    // The route must actually leave the blocked row to get around.
    assert!(
        points
            .iter()
            .any(|point| point.x < *hole_detour_z.start() || point.x > *hole_detour_z.end()),
        "the route never leaves the blocked row: {points:?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
