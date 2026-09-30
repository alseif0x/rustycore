//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn step_creature_movement_random_should_wander_returns_monster_move_and_state_walking_random() {
    let guid = test_creature_guid(200_001);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    // Trigger random wander: delay=0, wander_radius=3.0, move_start_ms=0.
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    creature.seed_runtime_rng_like_cpp(0x5757);
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false, // disable pathfinding → simple straight-line spline
        ..Default::default()
    };

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    // Must return Some with a serialised MonsterMove packet.
    assert!(
        result.is_some(),
        "Random + should_wander must produce a MonsterMove packet"
    );
    let bytes = result.unwrap();
    // Opcode bytes [0..2] must equal OnMonsterMove.
    let opcode = u16::from_le_bytes([bytes[0], bytes[1]]);
    assert_eq!(
        opcode,
        wow_constants::ServerOpcodes::OnMonsterMove as u16,
        "packet opcode must be OnMonsterMove"
    );
    // State must have transitioned to WalkingRandom.
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingRandom,
        "state must be WalkingRandom after launching a wander spline"
    );
    assert_eq!(
        creature.runtime_motion_master_ticks_like_cpp(),
        1,
        "the map movement frame must call MotionMaster::Update once"
    );
}
#[test]
fn step_creature_movement_chase_priority_interrupts_active_random_spline_like_cpp() {
    let guid = test_creature_guid(200_013);
    let target = ObjectGuid::create_player(1, 20_013);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }
    creature.seed_runtime_rng_like_cpp(0x2013);
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    let first =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);
    assert!(first.is_some(), "precondition: random launches a spline");
    assert!(creature.active_move_spline_like_cpp().is_some());

    creature.enter_combat(target);
    let interrupted =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200)
            .expect("active chase must stop the lower-priority random spline");

    assert_eq!(
        u16::from_le_bytes([interrupted[0], interrupted[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(wow_movement::MovementGeneratorType::Chase)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 2);
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::InCombat,
        "the inline random generator must not run below active chase"
    );
}
/// Chase must ask Detour for a route to its victim and walk *around* an
/// obstacle, mirroring C++ `ChaseMovementGenerator::Update`
/// (`ChaseMovementGenerator.cpp:154-236`), which builds a `PathGenerator`
/// for the victim and launches `init.MovebyPath(_path->GetPath())`.
#[test]
fn step_creature_movement_chase_paths_around_real_navmesh_obstacle_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, obstacle_hole_bounds, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    // Same geometry as the waypoint end-to-end test: the creature and its
    // victim sit on opposite ring cells of the middle row, so the direct
    // segment crosses the unwalkable centre cell.
    let start = Position::new(half + OBSTACLE_TILE_CELL_SIZE, half, 0.0, 0.0);
    let victim_position = Position::new(
        half + OBSTACLE_TILE_CELL_SIZE,
        half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
        0.0,
        0.0,
    );

    let root = std::env::temp_dir().join(format!(
        "rustycore-step-chase-obstacle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(&root, MAP_ID, &[(start.x, start.y)]);

    let guid = test_creature_guid(200_025);
    let victim_guid = test_creature_guid(200_026);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(MAP_ID, 0)
        .expect("bind the fixture map");
    creature.creature.set_ai_position(start);
    creature.creature.set_ai_home_position(start);
    creature.enter_combat(victim_guid);

    let worker = crate::map_manager::WorldMMapPathfinderWorkerLikeCpp::spawn(&root);
    let config = MMapRuntimeConfigLikeCpp {
        data_dir: root.display().to_string(),
        enabled: true,
        ..Default::default()
    };
    let target = crate::map_manager::ChaseTargetSnapshotLikeCpp {
        guid: victim_guid,
        position: victim_position,
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    let bytes = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        Some(&worker),
        None,
        Some(target),
        200,
    )
    .expect("chase must launch a MonsterMove toward the victim");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(wow_movement::MovementGeneratorType::Chase)
    );

    let spline = creature
        .active_move_spline_like_cpp()
        .expect("chase launched a spline");
    let points = spline.create_object_path_points_like_cpp();
    assert!(
        points.len() > 4,
        "chase must carry navmesh waypoints, not a straight line: {points:?}"
    );

    let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
    for point in points.iter() {
        let inside_hole = hole_detour_z.contains(&point.x) && hole_detour_x.contains(&point.y);
        assert!(
            !inside_hole,
            "chase point {point:?} walks through the obstacle: {points:?}"
        );
    }
    assert!(
        points
            .iter()
            .any(|point| point.x < *hole_detour_z.start() || point.x > *hole_detour_z.end()),
        "the chase route never leaves the blocked row: {points:?}"
    );

    // C++ bails out of the whole tick without a spline when the path is
    // NOPATH; here it succeeded, so the creature must be marked as chasing.
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits()),
        "a launched chase sets UNIT_STATE_CHASE_MOVE"
    );

    let _ = std::fs::remove_dir_all(&root);
}
/// Evade return must walk a navmesh route home instead of snapping there.
/// C++ `HomeMovementGenerator<Creature>::SetTargetLocation` launches
/// `init.MoveTo(GetHomePosition())` with `generatePath = true`
/// (`HomeMovementGenerator.cpp:60-82`).
#[test]
fn step_creature_movement_home_paths_around_real_navmesh_obstacle_like_cpp() {
    use wow_recastdetour::test_fixtures::{
        OBSTACLE_TILE_CELL_SIZE, obstacle_hole_bounds, write_obstacle_ring_mmaps_like_cpp,
    };

    const MAP_ID: u32 = 1;
    let half = OBSTACLE_TILE_CELL_SIZE / 2.0;
    let home = Position::new(half + OBSTACLE_TILE_CELL_SIZE, half, 0.0, 0.0);
    let away = Position::new(
        half + OBSTACLE_TILE_CELL_SIZE,
        half + 2.0 * OBSTACLE_TILE_CELL_SIZE,
        0.0,
        0.0,
    );

    let root = std::env::temp_dir().join(format!(
        "rustycore-step-home-obstacle-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_obstacle_ring_mmaps_like_cpp(&root, MAP_ID, &[(home.x, home.y)]);

    let guid = test_creature_guid(200_027);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(MAP_ID, 0)
        .expect("bind the fixture map");
    // Home is across the obstacle from where the creature stands.
    creature.creature.set_ai_home_position(home);
    creature.creature.set_ai_position(away);
    creature
        .creature
        .set_ai_state(wow_entities::CreatureAiState::Returning);

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
        200,
    )
    .expect("evade return must launch a MonsterMove toward home");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::OnMonsterMove as u16
    );

    let spline = creature
        .active_move_spline_like_cpp()
        .expect("home launched a spline");
    let points = spline.create_object_path_points_like_cpp();
    assert!(
        points.len() > 4,
        "the return trip must be a navmesh route, not a teleport/straight line: {points:?}"
    );
    let (hole_detour_x, hole_detour_z) = obstacle_hole_bounds();
    for point in points.iter() {
        let inside_hole = hole_detour_z.contains(&point.x) && hole_detour_x.contains(&point.y);
        assert!(
            !inside_hole,
            "home point {point:?} crosses the obstacle: {points:?}"
        );
    }
    // C++ `SetTargetLocation` adds `UNIT_STATE_ROAMING_MOVE` before launching.
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::ROAMING_MOVE.bits())
    );
    // The creature must still be returning: C++ only finalizes once the
    // spline reports finalized.
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::Returning,
        "the home generator stays alive until its spline finalizes"
    );

    let _ = std::fs::remove_dir_all(&root);
}
