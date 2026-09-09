//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

/// C++ `Unit::Update(p_time)` passes the same `p_time` first to
/// `UpdateSplineMovement` and then to `MotionMaster::Update`. Wall time between
/// calls must not advance either side independently.
#[test]
fn step_creature_movement_advances_spline_and_generator_from_one_diff_like_cpp() {
    let guid = test_creature_guid(200_025);
    let mut creature = make_test_world_creature(guid);
    creature.backdate_runtime_clock_for_test(Duration::from_secs(1));
    let (_, spline) = creature
        .begin_move_spline_like_cpp(Position::new(20.0, 0.0, 0.0, 0.0))
        .expect("a non-zero move must launch a spline");
    assert!(spline.duration_ms() > 17);

    let elapsed_before = creature.runtime_elapsed_ms_like_cpp();
    let motion_ticks_before = creature.runtime_motion_master_ticks_like_cpp();
    std::thread::sleep(Duration::from_millis(25));
    assert_eq!(
        creature.runtime_elapsed_ms_like_cpp(),
        elapsed_before,
        "scheduler wall time is not a creature clock"
    );

    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };
    let _ = step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 17);

    assert_eq!(creature.runtime_elapsed_ms_like_cpp(), elapsed_before + 17);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .progress_ms,
        17
    );
    assert_eq!(
        creature.runtime_motion_master_ticks_like_cpp(),
        motion_ticks_before + 1
    );
}
/// Live-cadence reproduction: drive many ticks with the same propagated diff
/// advancing both the creature clock and generator, crossing several
/// `wanderSteps` pause boundaries.
/// Mirrors the production legacy loop after the real-diff fix. The creature
/// must keep producing wander legs for the whole window — never permanently
/// stall for scheduler-lag-scaled pauses after a few steps
/// (pathfinding OFF → pure wander logic).
#[test]
fn step_creature_movement_random_keeps_wandering_over_many_ticks_like_cpp() {
    use std::time::Duration;

    let guid = test_creature_guid(200_024);
    let mut creature = make_test_world_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 5.0;
    }
    creature.seed_runtime_rng_like_cpp(0x1234);
    creature.backdate_runtime_clock_for_test(Duration::from_secs(3600));
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    // ~30s of simulated time at 50ms ticks: 600 ticks. Pauses are 4-10s, so
    // this window must contain many legs across multiple pause boundaries.
    let diff_ms: u32 = 50;
    let mut launches = 0usize;
    let mut launch_ticks: Vec<usize> = Vec::new();
    for tick in 0..600usize {
        if step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, diff_ms)
            .is_some()
        {
            launches += 1;
            launch_ticks.push(tick);
        }
    }

    assert!(
        launches >= 5,
        "wander must keep launching legs across the whole window; got {launches} launches at ticks {launch_ticks:?}"
    );
    // No sustained stall: at least one launch must occur in the final third
    // of the window.
    let last = *launch_ticks.last().unwrap_or(&0);
    assert!(
        last >= 400,
        "wander stalled: last launch at tick {last} of 600 (launches at {launch_ticks:?})"
    );
}
/// Positive re-arm: after the first wander spline finalizes from propagated
/// tick time and that same `diff_ms` has drained the generator timer,
/// `RandomMovementGenerator::DoUpdate` must roll a new destination
/// and launch the next leg (a fresh MonsterMove).
///
/// C++ ref: `RandomMovementGenerator<Creature>::DoUpdate`
/// (`_timer.Passed() && owner->movespline->Finalized()` → `SetRandomLocation`).
/// The creature clock is set to a deterministic logical elapsed value, so the
/// spline can be finalized without sleeping.
#[test]
fn step_creature_movement_random_rearms_next_leg_after_spline_finalizes_like_cpp() {
    use std::time::Duration;

    let guid = test_creature_guid(200_021);
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
    creature.seed_runtime_rng_like_cpp(0x5757);
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false, // straight-line spline (no pathfinding)
        ..Default::default()
    };

    // Leg 1 launches a wander spline; the generator timer is set to its
    // duration.
    let leg1 = step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);
    assert!(leg1.is_some(), "leg 1 must launch a wander spline");
    let duration = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .spline
        .duration_ms;
    assert!(duration > 0, "leg 1 spline must have a positive duration");
    let duration_u = duration as u32;

    // Advance the creature clock past the spline duration so the next
    // `update_move_spline_like_cpp` finalizes it (mirrors real time passing).
    creature.backdate_runtime_clock_for_test(
        Duration::from_secs(10) + Duration::from_millis(u64::from(duration_u) + 1),
    );

    // Leg 2: feed the *real* elapsed diff (duration + 1). The generator
    // timer reaches 0 and, with the spline finalized, the next leg is armed.
    let leg2 = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        None,
        None,
        None,
        duration_u + 1,
    );
    assert!(
        leg2.is_some(),
        "leg 2 must launch once the spline finalized and the real elapsed diff \
         drained the generator timer (C++ DoUpdate re-arm)"
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingRandom,
        "creature must stay in WalkingRandom across legs"
    );
}
/// Negative / regression guard for the legacy-runtime constant-diff lag:
/// if the per-tick `diff_ms` is far smaller than the spline duration (e.g.
/// a fixed `tick_interval_ms` while real iterations lag), the generator
/// timer does not reach 0 in the same frame even though the spline (real
/// clock) is already finalized. Repeated undersized diffs eventually catch
/// up, but stretch every wait in proportion to scheduler lag. The
/// production fix passes the *real* elapsed diff (like C++
/// `World::Update(diff)`).
#[test]
fn step_creature_movement_random_waits_when_diff_undershoots_spline_duration_like_cpp() {
    use std::time::Duration;

    let guid = test_creature_guid(200_022);
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
    creature.seed_runtime_rng_like_cpp(0x5757);
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let config = MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    };

    let leg1 = step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);
    assert!(leg1.is_some());
    let duration = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .spline
        .duration_ms;
    assert!(
        duration > 10,
        "spline must be longer than the undershooting per-tick diff"
    );
    let duration_u = duration as u32;

    // Finalize the spline by the real clock...
    creature.backdate_runtime_clock_for_test(
        Duration::from_secs(10) + Duration::from_millis(u64::from(duration_u) + 1),
    );
    // ...but feed only a tiny constant diff (the buggy loop behavior). The
    // generator timer barely moves and stays > 0, so no re-arm.
    let leg2 = step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 10);
    assert!(
        leg2.is_none(),
        "a per-tick diff far smaller than the spline duration must NOT re-arm \
         (this is the constant-diff lag the production fix removes)"
    );
}
#[test]
fn step_creature_movement_walking_random_finished_returns_none_and_state_idle() {
    let guid = test_creature_guid(200_002);
    let mut creature = make_test_world_creature(guid);
    // Put creature into WalkingRandom with no active spline and no move_target
    // so that movement_finished() == true.
    creature
        .creature
        .set_ai_state(wow_entities::CreatureAiState::WalkingRandom);
    // Ensure no move_target (movement_finished returns true when None).
    creature.creature.ai_ownership_mut().move_target = None;

    let config = MMapRuntimeConfigLikeCpp::default();

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    assert!(
        result.is_none(),
        "WalkingRandom + movement finished must return None"
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::Idle,
        "state must transition back to Idle after movement finished"
    );
}
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
/// Patrol progression: a creature with a multi-node DB waypoint path must
/// advance from node to node, launching a fresh MonsterMove toward each.
/// This exercises the same real-clock-finalize + diff-drained-timer re-arm
/// the wander generator uses (C++ `WaypointMovementGenerator<Creature>::
/// DoUpdate` → `OnArrived` → `StartMove`), and is the patrol half of #23:
/// the loader/store/resolver wiring already exists, so once fed the real
/// elapsed diff the creature patrols live.
#[test]
fn step_creature_movement_waypoint_progresses_through_nodes_with_real_diff_like_cpp() {
    use std::time::Duration;

    let guid = test_creature_guid(200_023);
    let mut creature = make_test_world_creature(guid);
    let mut clock_elapsed = Duration::from_secs(10);
    creature.backdate_runtime_clock_for_test(clock_elapsed);
    let path = wow_movement::WaypointPath::new(
        7,
        vec![
            wow_movement::WaypointNode::new(1, 12.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(2, 20.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        wow_movement::WaypointMovementAction::StopMoving
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingWaypoint
    );
    let config = MMapRuntimeConfigLikeCpp::default();

    // First leg launches after the initial delay; it heads to node 1.
    let leg1 = step_creature_movement_like_cpp(
        &mut creature,
        guid,
        &config,
        None,
        None,
        None,
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    );
    assert!(leg1.is_some(), "node 1 leg must launch a MonsterMove");
    let mut targets = vec![creature.move_target().map(|p| p.x).unwrap_or(f32::NAN)];

    // Drive several ticks, finalizing each spline via the clock seam and
    // feeding the *real* elapsed diff. The patrol must reach node 2.
    for _ in 0..6 {
        let duration = creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .duration_ms
            .max(1) as u32;
        clock_elapsed += Duration::from_millis(u64::from(duration) + 1);
        creature.backdate_runtime_clock_for_test(clock_elapsed);
        let _ = step_creature_movement_like_cpp(
            &mut creature,
            guid,
            &config,
            None,
            None,
            None,
            duration + 1,
        );
        if let Some(t) = creature.move_target() {
            targets.push(t.x);
        }
    }

    assert!(
        targets.iter().any(|x| (*x - 12.0).abs() < f32::EPSILON),
        "patrol must launch toward node 1 (x=12); saw {targets:?}"
    );
    assert!(
        targets.iter().any(|x| (*x - 20.0).abs() < f32::EPSILON),
        "patrol must progress to node 2 (x=20); saw {targets:?}"
    );
}
#[test]
fn step_creature_movement_dead_ready_respawn_waits_for_lifecycle_owner_like_cpp() {
    let guid = test_creature_guid(200_003);
    let mut creature = make_test_world_creature(guid);
    // Kill the creature: death_time_ms=0, respawn_time_secs=0 so
    // should_respawn() is true immediately (now_ms >= 0 + 0*1000 = 0).
    creature.creature.mark_ai_dead(0);
    creature.creature.ai_ownership_mut().respawn_time_secs = 0;

    assert!(
        !creature.is_alive(),
        "creature must be dead before the call"
    );
    assert!(
        creature.should_respawn(),
        "creature must be ready to respawn"
    );

    let config = MMapRuntimeConfigLikeCpp::default();

    let result =
        step_creature_movement_like_cpp(&mut creature, guid, &config, None, None, None, 200);

    assert!(result.is_none(), "dead creature movement must return None");
    assert!(
        !creature.is_alive(),
        "movement must not bypass corpse removal, DB timer cleanup, and visibility recreation"
    );
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::Dead,
        "global lifecycle remains the sole respawn owner"
    );
}
