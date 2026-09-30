//! Original clock/generator/state cases moved from the application boundary.

use super::fixtures::*;
use std::time::Duration;
use wow_core::Position;

#[test]
fn step_creature_movement_idle_zero_wander_radius_stays_still_like_cpp() {
    let guid = test_creature_guid(200_011);
    let mut creature = make_test_world_creature(guid);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 0.0;
    }

    let result =
        step(&mut creature, 200, false);

    assert!(
        result.is_none(),
        "C++ Creature::Create forces RANDOM_MOTION_TYPE to IDLE_MOTION_TYPE when m_wanderDistance is zero"
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(creature.creature.ai_ownership().move_target.is_none());
}

#[test]
fn step_creature_movement_idle_positive_wander_radius_stays_still_like_cpp() {
    let guid = test_creature_guid(200_012);
    let mut creature = make_test_world_creature(guid);
    {
        let ai = creature.creature.ai_ownership_mut();
        ai.wander_delay_ms = 0;
        ai.move_start_ms = 0;
        ai.wander_radius = 3.0;
    }

    let result =
        step(&mut creature, 200, false);

    assert!(
        result.is_none(),
        "C++ IdleMovementGenerator must not use RandomMovementGenerator wander distance"
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(creature.creature.ai_ownership().move_target.is_none());
}

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

    let _ = step(&mut creature, 17, false);

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

    // ~30s of simulated time at 50ms ticks: 600 ticks. Pauses are 4-10s, so
    // this window must contain many legs across multiple pause boundaries.
    let diff_ms: u32 = 50;
    let mut launches = 0usize;
    let mut launch_ticks: Vec<usize> = Vec::new();
    for tick in 0..600usize {
        if step(&mut creature, diff_ms, false)
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

    // Leg 1 launches a wander spline; the generator timer is set to its
    // duration.
    let leg1 = step(&mut creature, 200, false);
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
    let leg2 = step(&mut creature, duration_u + 1, false);
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

    let leg1 = step(&mut creature, 200, false);
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
    let leg2 = step(&mut creature, 10, false);
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


    let result =
        step(&mut creature, 200, true);

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

    // First leg launches after the initial delay; it heads to node 1.
    let leg1 = step(&mut creature, wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32, true);
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
        let _ = step(&mut creature, duration + 1, true);
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


    let result =
        step(&mut creature, 200, true);

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
