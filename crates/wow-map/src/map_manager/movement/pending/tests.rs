//! Direct staged consumers and compatibility consumers of the same operation.

use super::*;
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};

fn creature(counter: i64) -> WorldCreature {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, counter,
    );
    WorldCreature::new(
        guid, 9999, Position::new(10.0, 10.0, 0.0, 0.0),
        25, 2, 3, 5, 20.0, 100, 14, 0, 0,
    )
}

fn target() -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: ObjectGuid::create_player(1, 701),
        position: Position::xyz(50.0, 10.0, 0.0),
        combat_reach: 1.5, in_world: true, in_water: None,
    }
}

fn detour(query: &CreaturePathQueryLikeCpp) -> DetourPolyPath {
    DetourPolyPath {
        poly_refs: vec![11, 22, 33],
        point_path: DetourPointPath {
            points: vec![
                [query.start.x, query.start.y, query.start.z],
                [(query.start.x + query.destination.x) / 2.0, query.start.y + 1.0, query.start.z],
                [query.destination.x, query.destination.y, query.destination.z],
            ],
            actual_end: [query.destination.x, query.destination.y, query.destination.z],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false, end_far_from_poly: false,
    }
}

fn path_request(progress: MovementProgress) -> (CreaturePathQueryLikeCpp, PathContinuation) {
    let MovementProgress::Pending(PendingMovement::Path(request)) = progress else {
        panic!("expected a path request before spline launch");
    };
    request.into_parts()
}

fn random_actor(counter: i64) -> WorldCreature {
    let mut actor = creature(counter);
    actor.creature.set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    actor.creature.ai_ownership_mut().wander_radius = 3.0;
    actor.seed_runtime_rng_like_cpp(0x5757);
    actor
}

fn waypoint_actor(counter: i64) -> WorldCreature {
    let mut actor = creature(counter);
    actor.initialize_default_waypoint_movement_like_cpp(Some(WaypointPath::new(77, vec![
        wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0),
        wow_movement::WaypointNode::new(20, 40.0, 10.0, 0.0),
    ])));
    actor
}

#[test]
fn home_staged_and_synchronous_launches_and_finalize_state_agree() {
    let mut staged = creature(410_001);
    let mut synchronous = creature(410_001);
    for actor in [&mut staged, &mut synchronous] {
        actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
        actor.creature.set_ai_state(CreatureAiState::Returning);
    }
    let progress = prepare_home(&mut staged, true, false);
    assert!(staged.creature.is_in_evade_mode_like_cpp());
    assert!(staged.active_move_spline_like_cpp().is_none());
    let (query, continuation) = path_request(progress);
    assert!(query.previous_poly_refs.is_empty());
    let response = Some(detour(&query));
    let MovementProgress::Complete(MovementCompletion::Home(result)) = continuation.resume(&mut staged, response) else {
        panic!("terrain-free home must complete after one path response");
    };
    let mut queries = 0;
    let expected = synchronous.update_runtime_home_movement_like_cpp(true, None, |actual| {
        queries += 1;
        assert_eq!(actual.destination, query.destination);
        assert_eq!(actual.filter_context, query.filter_context);
        Some(detour(&actual))
    });
    assert_eq!(queries, 1);
    assert_eq!(result, expected);
    assert_eq!(staged.creature.unit().unit_state(), synchronous.creature.unit().unit_state());
    assert_eq!(staged.runtime.active_home_generator.as_ref(), synchronous.runtime.active_home_generator.as_ref());
}

#[test]
fn random_staged_and_synchronous_consume_the_same_five_draws_once() {
    let mut staged = random_actor(410_002);
    let mut synchronous = random_actor(410_002);
    staged.runtime.active_random_path_poly_refs = vec![91, 92];
    synchronous.runtime.active_random_path_poly_refs = vec![91, 92];
    let progress = prepare_random(&mut staged, 200, true, false, true);
    let (query, continuation) = path_request(progress);
    assert!(query.previous_poly_refs.is_empty(), "initialization precedes the query");
    assert!(staged.active_move_spline_like_cpp().is_none());
    let response = Some(detour(&query));
    let MovementProgress::Complete(MovementCompletion::Random(result)) = continuation.resume(&mut staged, response) else {
        panic!("terrain-free random must complete after one path response");
    };
    let expected = synchronous.update_default_random_movement_with_path_resolver_like_cpp(200, true, |actual| {
        assert_eq!(actual.destination, query.destination);
        Some(detour(&actual))
    });
    assert!(result.is_some());
    assert_eq!(result, expected);
    assert_eq!(staged.runtime.active_random_generator.as_ref(), synchronous.runtime.active_random_generator.as_ref());
    assert_eq!(staged.active_random_path_poly_refs_like_cpp(), &[11, 22, 33]);
    let mut expected_rng = StdRng::seed_from_u64(0x5757);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: i32 = expected_rng.gen_range(4..=10);
    for _ in 0..8 {
        let next = expected_rng.next_u64();
        assert_eq!(staged.runtime.runtime_rng_like_cpp.next_u64(), next);
        assert_eq!(synchronous.runtime.runtime_rng_like_cpp.next_u64(), next);
    }
    assert_eq!(staged.runtime_elapsed_ms_like_cpp(), 0);
    assert_eq!(staged.runtime_motion_master_ticks_like_cpp(), 0);
}

#[test]
fn waypoint_staged_and_synchronous_return_the_same_action_and_spline() {
    let mut staged = waypoint_actor(410_003);
    let mut synchronous = waypoint_actor(410_003);
    let diff = wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32;
    let progress = prepare_waypoint(&mut staged, diff, None, true, false, true);
    let (query, continuation) = path_request(progress);
    assert!(staged.active_move_spline_like_cpp().is_none());
    let response = Some(detour(&query));
    let MovementProgress::Complete(MovementCompletion::Waypoint(action, movement)) = continuation.resume(&mut staged, response) else {
        panic!("terrain-free waypoint must complete after one path response");
    };
    let expected = synchronous.update_default_waypoint_movement_with_path_resolver_like_cpp(diff, true, |actual| {
        assert_eq!(actual.destination, query.destination);
        Some(detour(&actual))
    });
    assert_eq!((action, movement), expected);
    assert_eq!(staged.active_waypoint_generator_like_cpp(), synchronous.active_waypoint_generator_like_cpp());
}

#[test]
fn chase_staged_and_synchronous_reset_the_victim_corridor_before_the_query() {
    let mut staged = creature(410_004);
    let mut synchronous = creature(410_004);
    for actor in [&mut staged, &mut synchronous] {
        actor.enter_combat(target().guid);
        actor.runtime.active_chase_path_poly_refs = vec![91, 92];
    }
    let progress = prepare_chase(&mut staged, 200, target(), true, false);
    let (query, continuation) = path_request(progress);
    assert!(query.previous_poly_refs.is_empty());
    let response = Some(detour(&query));
    let MovementProgress::Complete(MovementCompletion::Chase(result)) = continuation.resume(&mut staged, response) else {
        panic!("terrain-free chase must complete after one path response");
    };
    let expected = synchronous.update_runtime_chase_movement_like_cpp(200, target(), true, None, |actual| {
        assert_eq!(actual.destination, query.destination);
        assert_eq!(actual.owner, query.owner);
        Some(detour(&actual))
    });
    assert_eq!(result, expected);
    assert_eq!(staged.active_chase_generator_like_cpp(), synchronous.active_chase_generator_like_cpp());
    assert_eq!(staged.active_chase_path_poly_refs_like_cpp(), &[11, 22, 33]);
}

#[test]
fn absent_path_response_keeps_each_familys_distinct_failure_contract() {
    let mut home = creature(410_005);
    home.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    let progress = prepare_home(&mut home, true, false);
    let (_, continuation) = path_request(progress);
    assert!(matches!(continuation.resume(&mut home, None),
        MovementProgress::Complete(MovementCompletion::Home(ChaseTickOutcomeLikeCpp::Launched(..)))));

    let mut waypoint = waypoint_actor(410_006);
    let progress = prepare_waypoint(&mut waypoint, wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32, None, true, false, true);
    let (_, continuation) = path_request(progress);
    assert!(matches!(continuation.resume(&mut waypoint, None),
        MovementProgress::Complete(MovementCompletion::Waypoint(WaypointMovementAction::Launch(_), Some(_)))));

    let mut random = random_actor(410_007);
    let progress = prepare_random(&mut random, 200, true, false, true);
    let (_, continuation) = path_request(progress);
    assert!(matches!(continuation.resume(&mut random, None), MovementProgress::Complete(MovementCompletion::Random(None))));
    assert!(random.active_move_spline_like_cpp().is_none());

    let mut chase = creature(410_008);
    chase.enter_combat(target().guid);
    let progress = prepare_chase(&mut chase, 200, target(), true, false);
    let (_, continuation) = path_request(progress);
    assert!(matches!(continuation.resume(&mut chase, None), MovementProgress::Complete(MovementCompletion::Chase(ChaseTickOutcomeLikeCpp::Idle))));
    assert!(chase.active_chase_generator_like_cpp().unwrap().cannot_reach_target);
    let mut disabled = creature(410_009);
    disabled.enter_combat(target().guid);
    assert!(matches!(prepare_chase(&mut disabled, 200, target(), false, false),
        MovementProgress::Complete(MovementCompletion::Chase(ChaseTickOutcomeLikeCpp::Launched(..)))));
}

#[test]
fn chase_normalized_owned_points_keep_corridor_and_launch_confirmation() {
    let mut actor = creature(410_019);
    actor.enter_combat(target().guid);
    let from = actor.position();
    let progress = prepare_chase(&mut actor, 200, target(), true, true);
    let (query, continuation) = path_request(progress);
    assert_eq!(query.start, from);
    assert_eq!(query.destination, target().position);
    let response = Some(detour(&query));
    let mut progress = continuation.resume(&mut actor, response);
    let mut normalized_points = Vec::new();
    let result = loop {
        progress = match progress {
            MovementProgress::Pending(PendingMovement::StaticHeight(request)) => {
                normalized_points.push(request.request.point);
                assert!(actor.active_chase_path_poly_refs_like_cpp().is_empty(),
                    "chase publishes the corridor only after a successful launch");
                assert!(actor.active_move_spline_like_cpp().is_none());
                request.resume(&mut actor, 2.0)
            }
            MovementProgress::Complete(MovementCompletion::Chase(result)) => break result,
            _ => panic!("one path response, then valid static heights only"),
        };
    };
    assert_eq!(normalized_points, vec![
        target().position, from, Position::xyz(30.0, 11.0, 0.0), target().position,
    ], "actual_end precedes points; raw repeated endpoint remains independently sampled");
    let ChaseTickOutcomeLikeCpp::Launched(actual_from, spline) = result else {
        panic!("normal chase path launches after its final height response");
    };
    assert_eq!(actual_from, from);
    assert!(spline.create_object_path_points_like_cpp().contains(&Position::xyz(30.0, 11.0, 2.0)));
    assert_eq!(actor.active_chase_path_poly_refs_like_cpp(), &[11, 22, 33]);
    assert!(!actor.active_chase_generator_like_cpp().unwrap().cannot_reach_target);
    assert!(actor.creature.unit().has_unit_state(UnitState::CHASE_MOVE.bits()));
}

#[test]
fn static_height_response_samples_capabilities_after_height_and_skips_grid() {
    let mut actor = creature(410_010);
    actor.creature.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    actor.creature.unit_mut().set_hover_height_like_cpp(1.0);
    let point = Position::new(11.0, 12.0, 2.0, 0.75);
    let terrain::GroundProgress::Static(request) = terrain::prepare_ground(&actor, point, true) else { panic!("static height first"); };
    assert_eq!(request.probe_z, point.z + Z_OFFSET_FIND_HEIGHT);
    // Instrument the read boundary within this exclusively owned test actor.
    // This is not a claim that concurrent production writes are admissible.
    actor.creature.unit_mut().set_hover_height_like_cpp(3.0);
    let terrain::GroundProgress::Complete(normalized) = request.resume(&actor, 10.0) else { panic!("valid static ground skips grid"); };
    assert_eq!(normalized, Position::new(11.0, 12.0, 13.0, 0.75));
}

#[test]
fn grid_fallback_is_lazy_and_capabilities_are_sampled_after_its_response() {
    let mut actor = creature(410_011);
    actor.creature.unit_mut().world_mut().set_map(7, 19).unwrap();
    actor.creature.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    actor.creature.unit_mut().set_hover_height_like_cpp(1.0);
    let point = Position::xyz(11.0, 12.0, 2.0);
    let terrain::GroundProgress::Static(request) = terrain::prepare_ground(&actor, point, true) else { panic!("static first"); };
    assert_eq!(request.map_id, 7);
    let terrain::GroundProgress::Grid(request) = request.resume(&actor, INVALID_HEIGHT) else { panic!("invalid static requires grid"); };
    assert_eq!(request.map_id, 7);
    actor.creature.unit_mut().set_hover_height_like_cpp(4.0);
    assert_eq!(request.resume(&actor, 3.0), Position::xyz(11.0, 12.0, 7.0));
}

#[test]
fn missing_terrain_returns_the_raw_point_without_height_requests() {
    let mut actor = creature(410_012);
    actor.creature.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    actor.creature.unit_mut().set_hover_height_like_cpp(9.0);
    let point = Position::new(11.0, 12.0, -17.0, 0.5);
    let terrain::GroundProgress::Complete(actual) = terrain::prepare_ground(&actor, point, false) else { panic!("no terrain requests without terrain"); };
    assert_eq!(actual, point);
    assert_eq!(actor.normalize_path_position_z_like_cpp(point, None), point);
}

#[test]
fn destination_then_actual_end_then_repeated_points_launch_only_after_last_resume() {
    let mut actor = creature(410_013);
    actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    let home = actor.home_position();
    let mut progress = prepare_home(&mut actor, true, true);
    let mut points = Vec::new();
    let mut queries = 0;
    let result = loop {
        assert!(actor.active_move_spline_like_cpp().is_none(), "no publication before all height responses");
        progress = match progress {
            MovementProgress::Complete(result) => break result,
            MovementProgress::Pending(PendingMovement::StaticHeight(request)) => {
                points.push(request.request.point);
                let ground = request.request.point.z;
                request.resume(&mut actor, ground)
            }
            MovementProgress::Pending(PendingMovement::GridHeight(_)) => panic!("all static samples valid"),
            MovementProgress::Pending(PendingMovement::Path(request)) => {
                queries += 1;
                let (query, continuation) = request.into_parts();
                assert_eq!(query.destination, home);
                let detour = DetourPolyPath {
                    poly_refs: vec![11, 22],
                    point_path: DetourPointPath {
                        actual_end: [home.x, home.y, home.z],
                        points: vec![[30.0, 10.0, 0.0], [20.0, 10.0, 0.0], [20.0, 10.0, 0.0], [home.x, home.y, home.z]],
                        path_type: DetourPathType::NORMAL,
                    },
                    start_far_from_poly: false, end_far_from_poly: false,
                };
                continuation.resume(&mut actor, Some(detour))
            }
        };
        if matches!(&progress, MovementProgress::Complete(_)) {
            let MovementProgress::Complete(result) = progress else { unreachable!() };
            break result;
        }
    };
    assert_eq!(queries, 1);
    assert_eq!(points, vec![home, home, Position::xyz(30.0, 10.0, 0.0), Position::xyz(20.0, 10.0, 0.0), Position::xyz(20.0, 10.0, 0.0), home]);
    assert!(matches!(result, MovementCompletion::Home(ChaseTickOutcomeLikeCpp::Launched(..))));
    assert_eq!(actor.spline_id(), 1, "every continuation is consumed; there is one launch");
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0, "resume does not replay a diff");
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
}

#[test]
fn random_classifies_updates_corridor_and_generator_and_roaming_before_terrain() {
    let mut actor = random_actor(410_014);
    let progress = prepare_random(&mut actor, 200, true, true, true);
    let (query, continuation) = path_request(progress);
    let response = Some(detour(&query));
    let mut progress = continuation.resume(&mut actor, response);
    assert!(matches!(&progress, MovementProgress::Pending(PendingMovement::StaticHeight(_))));
    assert_eq!(actor.active_random_path_poly_refs_like_cpp(), &[11, 22, 33]);
    assert!(actor.creature.unit().has_unit_state(UnitState::ROAMING_MOVE.bits()));
    assert!(actor.active_move_spline_like_cpp().is_none());
    loop {
        progress = match progress {
            MovementProgress::Pending(PendingMovement::StaticHeight(request)) => {
                let ground = request.request.point.z;
                request.resume(&mut actor, ground)
            }
            MovementProgress::Complete(MovementCompletion::Random(Some(_))) => break,
            _ => panic!("no second query, grid sample or random generator update"),
        };
    }
    let mut expected_rng = StdRng::seed_from_u64(0x5757);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: i32 = expected_rng.gen_range(4..=10);
    for _ in 0..8 {
        assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64());
    }
}

#[test]
fn waiting_random_does_not_query_or_reroll() {
    let mut staged = random_actor(410_015);
    let mut synchronous = random_actor(410_015);
    assert!(staged.update_default_random_movement_with_path_resolver_like_cpp(200, false, |_| panic!("disabled")).is_some());
    assert!(synchronous.update_default_random_movement_with_path_resolver_like_cpp(200, false, |_| panic!("disabled")).is_some());
    assert!(matches!(prepare_random(&mut staged, 0, true, true, false), MovementProgress::Complete(MovementCompletion::Random(None))));
    assert!(synchronous.update_default_random_movement_after_spline_like_cpp(0, true, None, |_| panic!("waiting random has no query")).is_none());
    assert_eq!(staged.runtime.active_random_generator.as_ref(), synchronous.runtime.active_random_generator.as_ref());
    for _ in 0..8 {
        assert_eq!(staged.runtime.runtime_rng_like_cpp.next_u64(), synchronous.runtime.runtime_rng_like_cpp.next_u64());
    }
}

#[test]
fn waypoint_zero_diff_arrival_handoff_returns_the_chained_launch_action() {
    let mut staged = waypoint_actor(410_016);
    let mut synchronous = waypoint_actor(410_016);
    for actor in [&mut staged, &mut synchronous] {
        actor.update_default_waypoint_movement_like_cpp(wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32);
        actor.runtime.active_move_spline.as_mut().unwrap().finalize();
        assert!(actor.update_move_spline_like_cpp());
    }
    let progress = prepare_waypoint(&mut staged, 0, None, true, false, false);
    let (query, continuation) = path_request(progress);
    assert_eq!(query.destination, Position::xyz(40.0, 10.0, 0.0));
    let response = Some(detour(&query));
    let MovementProgress::Complete(MovementCompletion::Waypoint(action, movement)) = continuation.resume(&mut staged, response) else { panic!("chained launch completes"); };
    let expected = synchronous.update_default_waypoint_movement_after_spline_like_cpp(0, true, None, |query| Some(detour(&query)));
    assert_eq!((action, movement), expected);
    assert!(matches!(action, WaypointMovementAction::Launch(launch) if launch.node_id == 20));
    assert_eq!(staged.active_waypoint_generator_like_cpp(), synchronous.active_waypoint_generator_like_cpp());
}

#[test]
fn waypoint_wait_roll_and_random_handoff_keep_the_two_draw_sequence() {
    let mut actor = creature(410_017);
    actor.seed_runtime_rng_like_cpp(0x91_5EED);
    let mut path = WaypointPath::new(91, vec![
        wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0),
        wow_movement::WaypointNode::new(20, 12.0, 10.0, 0.0),
        wow_movement::WaypointNode::new(30, 13.0, 10.0, 0.0),
    ]);
    path.follow_path_backwards_from_end_to_start = true;
    actor.runtime.active_waypoint_generator = Some(WaypointMovementGenerator::from_path(
        path, true, Some(10_000), None, wow_movement::MovementWalkRunSpeedSelectionMode::Default,
        Some((1_000, 2_000)), Some(5.0), true, true,
    ));
    for expected_node in [10, 20, 30] {
        assert!(matches!(prepare_waypoint(&mut actor, 0, None, false, false, true),
            MovementProgress::Complete(MovementCompletion::Waypoint(WaypointMovementAction::Launch(launch), Some(_))) if launch.node_id == expected_node));
        actor.runtime.active_move_spline.as_mut().unwrap().finalize();
        assert!(actor.update_move_spline_like_cpp());
    }
    let reference = actor.position();
    let mut expected_rng = StdRng::seed_from_u64(0x91_5EED);
    let angle: f32 = expected_rng.gen_range(0.0..(2.0 * std::f32::consts::PI));
    let distance: f32 = expected_rng.gen_range(0.0..=5.0);
    let progress = prepare_waypoint(&mut actor, 0, Some(1_500), false, true, false);
    assert!(matches!(progress, MovementProgress::Complete(MovementCompletion::Waypoint(WaypointMovementAction::Arrived(arrived), Some(_)))
        if arrived.move_random_at_path_end == Some(WaypointRandomAtPathEnd { wander_distance: 5.0, duration_ms: 1_500 })));
    assert_eq!(actor.move_target(), Some(Position::new(
        reference.x + angle.cos() * distance, reference.y + angle.sin() * distance,
        reference.z, angle + std::f32::consts::PI,
    )));
    assert_eq!(actor.active_waypoint_generator_like_cpp().and_then(WaypointMovementGenerator::duration_ms), Some(8_500));
    for _ in 0..8 {
        assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64());
    }
}

#[test]
fn drop_of_pending_home_neither_launches_nor_claims_completion() {
    let mut actor = creature(410_018);
    actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    let progress = prepare_home(&mut actor, true, false);
    assert!(matches!(&progress, MovementProgress::Pending(PendingMovement::Path(_))));
    drop(progress);
    assert!(actor.creature.is_in_evade_mode_like_cpp());
    assert!(actor.runtime.active_home_generator.is_some());
    assert!(actor.active_move_spline_like_cpp().is_none());
    assert_eq!(actor.spline_id(), 0);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
}
