//! Waypoint generator regressions.
//!
//! Moved out of waypoint.rs under #685; every test is unchanged.

use super::*;

#[test]
fn waypoint_constructor_and_initialize_match_cpp_shape() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        true,
        Some(10_000),
        Some(7.0),
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    assert_eq!(waypoint.kind(), MovementGeneratorType::Waypoint);
    assert_eq!(waypoint.state().mode, MovementGeneratorMode::Default);
    assert_eq!(waypoint.state().priority, MovementGeneratorPriority::Normal);
    assert_eq!(
        waypoint.state().flags,
        MovementGeneratorFlags::INITIALIZATION_PENDING
    );
    assert_eq!(
        waypoint.state().base_unit_state,
        UNIT_STATE_WAYPOINT_ROAMING_LIKE_CPP
    );

    assert_eq!(
        waypoint.initialize_like_cpp(true, 0, None),
        WaypointMovementAction::StopMoving
    );
    assert!(!waypoint.has_flag(MovementGeneratorFlags::INITIALIZED));
    assert_eq!(
        waypoint.next_move_time_ms(),
        WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP
    );
    assert_eq!(waypoint.stop_moving_calls, 1);
}

#[test]
fn waypoint_pause_resume_and_reset_use_cpp_guard_timer() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        true,
        None,
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    waypoint.pause_like_cpp(500);
    assert!(waypoint.has_flag(MovementGeneratorFlags::TIMED_PAUSED));
    assert_eq!(waypoint.next_move_time_ms(), 500);
    waypoint.pause_like_cpp(0);
    assert!(waypoint.has_flag(MovementGeneratorFlags::PAUSED));
    assert_eq!(
        waypoint.next_move_time_ms(),
        WAYPOINT_RESUME_GUARD_MS_LIKE_CPP
    );
    waypoint.resume_like_cpp(0);
    assert_eq!(
        waypoint.next_move_time_ms(),
        WAYPOINT_RESUME_GUARD_MS_LIKE_CPP
    );

    waypoint.next_move_time_ms = 0;
    waypoint.reset_like_cpp(true);
    assert_eq!(
        waypoint.next_move_time_ms(),
        WAYPOINT_RESUME_GUARD_MS_LIKE_CPP
    );
}

#[test]
fn waypoint_initial_start_adds_initialized_and_launches_current_node() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        true,
        None,
        Some(4.0),
        MovementWalkRunSpeedSelectionMode::ForceWalk,
        None,
        None,
        false,
        false,
    );
    waypoint.initialize_like_cpp(true, 0, None);
    let action = waypoint.update_like_cpp(true, 1_000, snapshot(), None);
    assert_eq!(
        action,
        WaypointMovementAction::Launch(WaypointLaunchPlan {
            node_id: 10,
            path_id: 77,
            destination: Position::new(1.0, 0.0, 0.0, 0.0),
            generate_path: false,
            disable_transport_transform: false,
            facing: None,
            walk: Some(true),
            animation: None,
            velocity: Some(4.0),
            add_unit_state: UNIT_STATE_WAYPOINT_ROAMING_MOVE_LIKE_CPP,
            signal_formation: true,
        })
    );
    assert!(waypoint.has_flag(MovementGeneratorFlags::INITIALIZED));
    assert_eq!(
        waypoint.waypoint_started,
        vec![WaypointStarted {
            node_id: 10,
            path_id: 77
        }]
    );
}

#[test]
fn waypoint_arrival_sets_delay_informs_and_updates_current_info() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        true,
        None,
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    waypoint.initialize_like_cpp(true, 0, None);
    waypoint.start_move_like_cpp(snapshot(), false);
    waypoint.next_move_time_ms = 0;
    waypoint.add_flag(MovementGeneratorFlags::INFORM_ENABLED);
    waypoint.compute_next_node_like_cpp();
    waypoint.remove_flag(MovementGeneratorFlags::INFORM_ENABLED);

    let action = waypoint.update_like_cpp(true, 1, snapshot(), None);
    assert_eq!(
        action,
        WaypointMovementAction::Arrived(WaypointArrivalAction {
            clear_roaming_move: true,
            timer_ms: Some(500),
            move_random_at_path_end: None,
            duration_after_wait_ms: None,
            inform: WaypointInform {
                movement_type: MovementGeneratorType::Waypoint,
                node_id: 20,
                path_id: 77,
            },
            current_info: WaypointCurrentInfo {
                node_id: 20,
                path_id: 77,
            },
        })
    );
    assert!(waypoint.has_flag(MovementGeneratorFlags::INFORM_ENABLED));
}

#[test]
fn waypoint_compute_next_node_matches_cpp_forward_and_backward_modes() {
    let mut forward = WaypointMovementGenerator::from_path(
        path(),
        true,
        None,
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    assert!(forward.compute_next_node_like_cpp());
    assert_eq!(forward.current_node(), 1);
    assert!(forward.compute_next_node_like_cpp());
    assert_eq!(forward.current_node(), 2);
    assert!(forward.compute_next_node_like_cpp());
    assert_eq!(forward.current_node(), 0);

    let mut back_path = path();
    back_path.follow_path_backwards_from_end_to_start = true;
    let mut back = WaypointMovementGenerator::from_path(
        back_path,
        true,
        None,
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        true,
        true,
    );
    assert!(back.compute_next_node_like_cpp());
    assert_eq!(back.current_node(), 1);
    assert!(!back.is_returning_to_start());
    assert!(back.compute_next_node_like_cpp());
    assert_eq!(back.current_node(), 2);
    assert!(!back.is_returning_to_start());
    assert!(back.compute_next_node_like_cpp());
    assert_eq!(back.current_node(), 1);
    assert!(back.is_returning_to_start());
    assert!(back.compute_next_node_like_cpp());
    assert_eq!(back.current_node(), 0);
}

#[test]
fn waypoint_path_end_finalizes_and_records_path_ended() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        false,
        None,
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    waypoint.initialize_like_cpp(true, 0, None);
    waypoint.add_flag(MovementGeneratorFlags::INITIALIZED | MovementGeneratorFlags::INFORM_ENABLED);
    waypoint.current_node = 2;

    assert_eq!(
        waypoint.start_move_like_cpp(snapshot(), false),
        WaypointMovementAction::PathEnded(WaypointPathEnded {
            node_id: 30,
            path_id: 77,
        })
    );
    assert!(waypoint.has_flag(MovementGeneratorFlags::FINALIZED));
    assert_eq!(
        waypoint.current_info_updates,
        vec![WaypointCurrentInfo {
            node_id: 0,
            path_id: 0
        }]
    );
}

#[test]
fn waypoint_path_end_wait_can_push_random_and_count_duration_like_cpp() {
    let mut back_path = path();
    back_path.follow_path_backwards_from_end_to_start = true;
    let mut waypoint = WaypointMovementGenerator::from_path(
        back_path,
        true,
        Some(10_000),
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        Some((1_000, 2_000)),
        Some(5.0),
        true,
        true,
    );
    waypoint.current_node = 2;
    let action = waypoint.on_arrived_like_cpp(Some(1_500));
    assert_eq!(
        action.move_random_at_path_end,
        Some(WaypointRandomAtPathEnd {
            wander_distance: 5.0,
            duration_ms: 1_500,
        })
    );
    assert_eq!(action.duration_after_wait_ms, Some(8_500));
}

#[test]
fn waypoint_deactivate_finalize_and_duration_match_cpp() {
    let mut waypoint = WaypointMovementGenerator::from_path(
        path(),
        true,
        Some(100),
        None,
        MovementWalkRunSpeedSelectionMode::Default,
        None,
        None,
        false,
        true,
    );
    assert_eq!(
        waypoint.update_like_cpp(true, 100, snapshot(), None),
        WaypointMovementAction::DurationFinished
    );
    assert!(waypoint.has_flag(MovementGeneratorFlags::INFORM_ENABLED));

    assert_eq!(
        waypoint.deactivate_like_cpp(),
        WaypointFinalizeAction {
            clear_roaming_move: true,
            stop_moving: false,
            set_walk_false: false,
        }
    );
    assert_eq!(
        waypoint.finalize_like_cpp(true),
        WaypointFinalizeAction {
            clear_roaming_move: true,
            stop_moving: true,
            set_walk_false: true,
        }
    );
}
