//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_creature_waypoint_update_launches_initial_node_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54331);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let path = WaypointPath::new(
        77,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );

    let action = creature.update_default_waypoint_movement_like_cpp(
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    );

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 10);
            assert_eq!(launch.path_id, 77);
            assert_eq!(launch.destination, Position::new(11.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected initial waypoint launch, got {other:?}"),
    }
    assert!(creature.active_move_spline.is_some());
    assert_eq!(
        creature.move_target(),
        Some(Position::new(11.0, 10.0, 0.0, 0.0))
    );
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .expect("waypoint generator")
            .waypoint_started
            .len(),
        1
    );
}
#[test]
fn world_creature_waypoint_generate_path_uses_detour_point_path_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54348);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let destination = Position::new(30.0, 10.0, 0.0, 0.0);
    let path = WaypointPath::new(
        77,
        vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    let mut resolver_calls = 0;

    let (action, launched) = creature.update_default_waypoint_movement_with_path_resolver_like_cpp(
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
        true,
        |query| {
            resolver_calls += 1;
            assert_eq!(query.start, Position::new(10.0, 10.0, 0.0, 0.0));
            assert_eq!(query.destination, destination);
            assert_eq!(query.point_path_limit, MAX_POINT_PATH_LENGTH_LIKE_CPP);
            Some(DetourPolyPath {
                poly_refs: vec![11, 22, 33],
                point_path: wow_recastdetour::DetourPointPath {
                    points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                    actual_end: [30.0, 10.0, 0.0],
                    path_type: DetourPathType::NORMAL,
                },
                start_far_from_poly: false,
                end_far_from_poly: false,
            })
        },
    );

    assert!(matches!(action, WaypointMovementAction::Launch(_)));
    assert_eq!(resolver_calls, 1);
    let (_from, spline) = launched.expect("waypoint detour path launches");
    assert_eq!(spline.final_destination(), Some(destination));
    assert!(
        spline
            .create_object_path_points_like_cpp()
            .contains(&Position::new(20.0, 15.0, 2.0, 0.0)),
        "C++ MoveSplineInit::MoveTo(generatePath=true) switches to MovebyPath(PathGenerator::GetPath())"
    );
    assert!(
        !spline.monster_move_path_data().packed_deltas.is_empty(),
        "a generated multi-point waypoint path must not serialize as a single direct segment"
    );
}
#[test]
fn world_creature_waypoint_generate_path_nopath_falls_back_direct_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54349);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let destination = Position::new(30.0, 10.0, 0.0, 0.0);
    let path = WaypointPath::new(
        77,
        vec![wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    let mut resolver_calls = 0;

    let (_action, launched) = creature
        .update_default_waypoint_movement_with_path_resolver_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
            true,
            |_query| {
                resolver_calls += 1;
                Some(DetourPolyPath {
                    poly_refs: Vec::new(),
                    point_path: wow_recastdetour::DetourPointPath {
                        points: vec![[10.0, 10.0, 0.0], [20.0, 15.0, 2.0], [30.0, 10.0, 0.0]],
                        actual_end: [30.0, 10.0, 0.0],
                        path_type: DetourPathType::NOPATH,
                    },
                    start_far_from_poly: false,
                    end_far_from_poly: false,
                })
            },
        );

    assert_eq!(resolver_calls, 1);
    let (_from, spline) = launched.expect("waypoint direct fallback launches");
    assert_eq!(spline.final_destination(), Some(destination));
    assert!(
        spline.monster_move_path_data().packed_deltas.is_empty(),
        "C++ MoveSplineInit::MoveTo(generatePath=true) falls back to a direct path when PathGenerator reports NOPATH"
    );
}
#[test]
fn world_creature_waypoint_launch_applies_land_takeoff_anim_tier_like_cpp() {
    for (move_type, expected_anim_tier) in [
        (wow_movement::WaypointMoveType::Land, 0),
        (wow_movement::WaypointMoveType::TakeOff, 2),
    ] {
        let guid = ObjectGuid::create_world_object(
            HighGuid::Creature,
            0,
            1,
            0,
            0,
            1,
            54335 + i64::from(expected_anim_tier),
        );
        let mut creature = WorldCreature::new(
            guid,
            1,
            Position::new(10.0, 10.0, 0.0, 0.0),
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
        let mut path = WaypointPath::new(
            90 + expected_anim_tier as u32,
            vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
        );
        path.move_type = move_type;
        assert_eq!(
            creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
            WaypointMovementAction::StopMoving
        );

        assert!(matches!(
            creature.update_default_waypoint_movement_like_cpp(
                wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
            ),
            WaypointMovementAction::Launch(_)
        ));

        assert_eq!(
            creature
                .active_move_spline
                .as_ref()
                .and_then(MoveSpline::anim_tier)
                .map(|anim| anim.anim_tier),
            Some(expected_anim_tier)
        );
    }
}
#[test]
fn world_creature_waypoint_arrival_records_inform_and_launches_next_node_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54332);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let path = WaypointPath::new(
        77,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0).with_delay(500),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("initial waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let arrived = creature.update_default_waypoint_movement_like_cpp(0);

    match arrived {
        WaypointMovementAction::Arrived(arrived) => {
            assert_eq!(arrived.inform.node_id, 10);
            assert_eq!(arrived.inform.path_id, 77);
            assert_eq!(arrived.timer_ms, Some(500));
        }
        other => panic!("expected waypoint arrival, got {other:?}"),
    }
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );

    let next = creature.update_default_waypoint_movement_like_cpp(500);

    match next {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 77);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected next waypoint launch, got {other:?}"),
    }
    assert_eq!(
        creature.move_target(),
        Some(Position::new(12.0, 10.0, 0.0, 0.0))
    );
}
#[test]
fn world_creature_waypoint_arrival_without_delay_launches_next_node_same_tick_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54333);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let path = WaypointPath::new(
        88,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("single waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let action = creature.update_default_waypoint_movement_like_cpp(0);

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 88);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected same-tick next waypoint launch, got {other:?}"),
    }
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );
    assert_eq!(
        creature.move_target(),
        Some(Position::new(12.0, 10.0, 0.0, 0.0))
    );
}
#[test]
fn world_creature_waypoint_tick_advances_spline_before_motionmaster_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54338);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let path = WaypointPath::new(
        92,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        ],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("initial waypoint spline")
        .finalize();
    assert!(
        !creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .finalized,
        "the represented MotionSubsystem is stale until Unit::UpdateSplineMovement runs"
    );

    let action = creature.update_default_waypoint_movement_like_cpp(0);

    match action {
        WaypointMovementAction::Launch(launch) => {
            assert_eq!(launch.node_id, 20);
            assert_eq!(launch.path_id, 92);
            assert_eq!(launch.destination, Position::new(12.0, 10.0, 0.0, 0.0));
        }
        other => panic!("expected next waypoint launch after spline advance, got {other:?}"),
    }
    assert_eq!(
        creature.creature.ai_ownership().last_movement_inform,
        Some(wow_entities::CreatureMovementInform {
            movement_type: MovementGeneratorKind::Waypoint.trinity_id(),
            movement_id: 10,
        })
    );
}
#[test]
fn world_creature_waypoint_single_node_path_ends_same_tick_after_arrival_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54334);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    let path = WaypointPath::new(
        89,
        vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
    );
    assert_eq!(
        creature.initialize_default_waypoint_movement_like_cpp(Some(path)),
        WaypointMovementAction::StopMoving
    );
    assert!(matches!(
        creature.update_default_waypoint_movement_like_cpp(
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        ),
        WaypointMovementAction::Launch(_)
    ));
    creature
        .active_move_spline
        .as_mut()
        .expect("single waypoint spline")
        .finalize();
    assert!(creature.update_move_spline_like_cpp());

    let ended = creature.update_default_waypoint_movement_like_cpp(0);

    match ended {
        WaypointMovementAction::PathEnded(ended) => {
            assert_eq!(ended.node_id, 10);
            assert_eq!(ended.path_id, 89);
        }
        other => panic!("expected waypoint path end, got {other:?}"),
    }
    assert_eq!(
        creature.home_position(),
        Position::new(11.0, 10.0, 0.0, 0.0)
    );
}
#[test]
fn world_creature_waypoint_path_end_random_handoff_launches_active_random_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54337);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    creature.seed_runtime_rng_like_cpp(0x91_5EED);
    let mut path = WaypointPath::new(
        91,
        vec![
            wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
            wow_movement::WaypointNode::new(30, 13.0, 10.0, 0.0),
        ],
    );
    path.follow_path_backwards_from_end_to_start = true;
    creature.active_waypoint_generator = Some(WaypointMovementGenerator::from_path(
        path,
        true,
        Some(10_000),
        None,
        wow_movement::MovementWalkRunSpeedSelectionMode::Default,
        Some((1_000, 2_000)),
        Some(5.0),
        true,
        true,
    ));

    for expected_node in [10, 20, 30] {
        match creature.update_default_waypoint_movement_like_cpp(0) {
            WaypointMovementAction::Launch(launch) => assert_eq!(launch.node_id, expected_node),
            other => panic!("expected waypoint launch for node {expected_node}, got {other:?}"),
        }
        creature
            .active_move_spline
            .as_mut()
            .expect("active waypoint spline")
            .finalize();
        assert!(creature.update_move_spline_like_cpp());
    }

    let action = creature.update_default_waypoint_movement_with_wait_roll_like_cpp(0, Some(1_500));

    match action {
        WaypointMovementAction::Arrived(arrived) => {
            assert_eq!(arrived.inform.node_id, 30);
            assert_eq!(
                arrived.move_random_at_path_end,
                Some(WaypointRandomAtPathEnd {
                    wander_distance: 5.0,
                    duration_ms: 1_500,
                })
            );
            assert_eq!(arrived.duration_after_wait_ms, Some(8_500));
        }
        other => panic!("expected endpoint random handoff arrival, got {other:?}"),
    }
    let random_target = creature
        .move_target()
        .expect("C++ MoveRandom handoff should launch an active random spline");
    assert!(creature.active_move_spline.is_some());
    assert!(
        random_target.distance_2d(&Position::new(13.0, 10.0, 0.0, 0.0)) <= 5.001,
        "C++ RandomMovementGenerator chooses a destination within _wanderDistance of its reference"
    );
    assert_eq!(
        creature.active_waypoint_random_at_path_end_like_cpp(),
        Some(WaypointRandomAtPathEnd {
            wander_distance: 5.0,
            duration_ms: 1_500,
        })
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .and_then(WaypointMovementGenerator::duration_ms),
        Some(8_500)
    );

    assert_eq!(
        creature.update_default_waypoint_movement_like_cpp(100),
        WaypointMovementAction::Continue
    );
    assert!(creature.active_move_spline.is_some());
    assert_eq!(
        creature.active_waypoint_random_at_path_end_like_cpp(),
        Some(WaypointRandomAtPathEnd {
            wander_distance: 5.0,
            duration_ms: 1_400,
        })
    );
}
#[test]
fn world_creature_detour_path_bridge_uses_moveby_path_or_direct_fallback_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54324);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
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
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let normal_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 0.0], [12.0, 11.0, 0.0], [15.0, 12.0, 0.0]],
            actual_end: [15.0, 12.0, 0.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };
    let dst = Position::new(15.0, 12.0, 0.0, 0.0);

    let (from, spline, path) = creature
        .begin_move_spline_with_detour_path_like_cpp(dst, Some(&normal_path), false)
        .expect("detour path launches");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(spline.final_destination(), Some(dst));
    assert_eq!(spline.monster_move_path_data().points, vec![dst]);
    let path = path.expect("path generator");
    assert_eq!(path.path_type(), PathType::NORMAL);
    assert_eq!(path.poly_length(), 2);
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 0.0, 0.0),
            Position::new(12.0, 11.0, 0.0, 0.0),
            dst
        ]
    );

    let nopath = DetourPolyPath {
        poly_refs: Vec::new(),
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[15.0, 12.0, 0.0], [20.0, 10.0, 0.0]],
            actual_end: [20.0, 10.0, 0.0],
            path_type: DetourPathType::NOPATH,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };
    let fallback_dst = Position::new(20.0, 10.0, 0.0, 0.0);

    let (_from, fallback_spline, fallback_path) = creature
        .begin_move_spline_with_detour_path_like_cpp(fallback_dst, Some(&nopath), false)
        .expect("direct fallback launches");

    assert_eq!(fallback_spline.final_destination(), Some(fallback_dst));
    assert!(
        fallback_path
            .expect("fallback path metadata")
            .path_type()
            .contains(PathType::NOPATH)
    );
}
#[test]
fn world_creature_detour_path_bridge_normalizes_points_to_terrain_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54350);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 1.75, 0.0),
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
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 2.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    assert!(
        (terrain.static_height_like_cpp(1, 10.0, 10.0, 51.75) - 2.0).abs() < 1e-3,
        "synthetic terrain tile must cover the test path"
    );
    let dst = Position::new(15.0, 12.0, 1.75, 0.0);
    let normal_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 1.75], [12.0, 11.0, 1.75], [15.0, 12.0, 1.75]],
            actual_end: [15.0, 12.0, 1.75],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&normal_path),
            false,
            Some(&terrain),
        )
        .expect("terrain-normalized detour path launches");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "C++ PathGenerator::NormalizePath calls UpdateAllowedPositionZ for every path point"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
#[test]
fn world_creature_detour_path_bridge_raises_low_mmap_points_to_grid_ground_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54351);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 43.0, 0.0),
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
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(1, 0)
        .expect("bind test creature to terrain map");
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let data_dir = temp_dir_with_constant_tile(1, 31, 31, 50.0);
    let terrain = LiveTerrainHeights::new(&data_dir);
    assert!(
        terrain.static_height_like_cpp(1, 10.0, 10.0, 43.0 + Z_OFFSET_FIND_HEIGHT)
            <= INVALID_HEIGHT,
        "the C++ probe gate rejects this low Rust MMap point"
    );
    assert!(
        (terrain.grid_height_like_cpp(1, 10.0, 10.0) - 50.0).abs() < 1e-3,
        "synthetic terrain still has a usable raw ground height"
    );
    let dst = Position::new(15.0, 12.0, 43.0, 0.0);
    let low_mmap_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 43.0], [12.0, 11.0, 43.0], [15.0, 12.0, 43.0]],
            actual_end: [15.0, 12.0, 43.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&low_mmap_path),
            false,
            Some(&terrain),
        )
        .expect("terrain-normalized low detour path launches");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, 50.0, 0.0),
            Position::new(12.0, 11.0, 50.0, 0.0),
            Position::new(15.0, 12.0, 50.0, 0.0),
        ],
        "NormalizePath must not serialize underground Rust MMap points to the client"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 50.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
