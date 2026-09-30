//! Step continuations preserve the one-frame prefix and dispatch captures.

use super::fixtures::*;
use super::{ChaseTargetSnapshotLikeCpp, CreatureMovementSource, CreatureMovementStep, StepPending, StepProgress};
use crate::map_manager::{CreaturePathQueryLikeCpp, WorldCreature};
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use wow_constants::UnitState;
use wow_core::{ObjectGuid, Position};
use wow_entities::CreatureAiState;
use wow_movement::WaypointPath;
use wow_recastdetour::{DetourPathType, DetourPointPath, DetourPolyPath};

fn target() -> ChaseTargetSnapshotLikeCpp {
    ChaseTargetSnapshotLikeCpp {
        guid: ObjectGuid::create_player(1, 801),
        position: Position::xyz(50.0, 10.0, 0.0),
        combat_reach: 1.5, in_world: true, in_water: None,
    }
}

fn actor(source: CreatureMovementSource, counter: i64) -> WorldCreature {
    let mut actor = make_test_world_creature(test_creature_guid(counter));
    actor.seed_runtime_rng_like_cpp(0x5757);
    actor.creature.unit_mut().world_mut().set_map(7, 19).unwrap();
    actor.creature.unit_mut().world_mut().phase_shift_mut().insert(42);
    actor.creature.unit_mut().world_mut().phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
    match source {
        CreatureMovementSource::Home => {
            actor.creature.set_ai_position(Position::xyz(30.0, 10.0, 0.0));
            actor.creature.set_ai_state(CreatureAiState::Returning);
        }
        CreatureMovementSource::Random => {
            actor.creature.set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
            actor.creature.ai_ownership_mut().wander_radius = 3.0;
            actor.runtime.active_random_path_poly_refs = vec![91, 92];
        }
        CreatureMovementSource::Waypoint => {
            actor.initialize_default_waypoint_movement_like_cpp(Some(WaypointPath::new(77, vec![
                wow_movement::WaypointNode::new(10, 30.0, 10.0, 0.0),
                wow_movement::WaypointNode::new(20, 40.0, 10.0, 0.0),
            ])));
        }
        CreatureMovementSource::Chase => {
            actor.enter_combat(target().guid);
            actor.runtime.active_chase_path_poly_refs = vec![91, 92];
        }
    }
    actor
}

fn detour(query: &CreaturePathQueryLikeCpp) -> DetourPolyPath {
    DetourPolyPath {
        poly_refs: vec![11, 22, 33],
        point_path: DetourPointPath {
            actual_end: [query.destination.x, query.destination.y, query.destination.z],
            points: vec![
                [query.start.x, query.start.y, query.start.z],
                [(query.start.x + query.destination.x) / 2.0, query.start.y + 1.0, query.start.z],
                [query.destination.x, query.destination.y, query.destination.z],
            ],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false, end_far_from_poly: false,
    }
}

fn assert_same_result(actual: Option<CreatureMovementStep>, expected: Option<CreatureMovementStep>) {
    match (actual, expected) {
        (None, None) => {}
        (Some(CreatureMovementStep::Stop(actual)), Some(CreatureMovementStep::Stop(expected))) => {
            assert_eq!(actual, expected);
        }
        (Some(CreatureMovementStep::Launch { source, from, spline }),
         Some(CreatureMovementStep::Launch { source: expected_source, from: expected_from, spline: expected_spline })) => {
            assert_eq!(source, expected_source);
            assert_eq!(from, expected_from);
            assert_eq!(spline, expected_spline);
        }
        (actual, expected) => panic!("different movement outcomes: {actual:?}, {expected:?}"),
    }
}

#[test]
fn all_four_staged_families_match_sync_state_corridor_and_one_clock_prefix() {
    for (index, source) in [
        CreatureMovementSource::Home, CreatureMovementSource::Random,
        CreatureMovementSource::Waypoint, CreatureMovementSource::Chase,
    ].into_iter().enumerate() {
        // Construct two real actors; snapshot Clone resets their motors and
        // cannot be used to manufacture an ownership-preserving comparison.
        let mut staged = actor(source, 420_001 + index as i64);
        let mut synchronous = actor(source, 420_001 + index as i64);
        let phase = staged.phase_shift().clone();
        let diff = if source == CreatureMovementSource::Waypoint {
            wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32
        } else { 200 };
        let elapsed = staged.runtime_elapsed_ms_like_cpp();
        let ticks = staged.runtime_motion_master_ticks_like_cpp();
        let mut policy_calls = 0;
        let progress = staged.prepare_movement_step(diff, Some(target()), false, |map, ignore| {
            policy_calls += 1;
            assert_eq!(map, 7);
            assert!(!ignore);
            true
        });
        assert_eq!(policy_calls, 1);
        assert_eq!(staged.runtime_elapsed_ms_like_cpp(), elapsed + u64::from(diff));
        assert_eq!(staged.runtime_motion_master_ticks_like_cpp(), ticks + 1);
        let StepProgress::Pending(StepPending::Path(request)) = progress else { panic!("selected family needs one query"); };
        let (query, continuation) = request.into_parts();
        assert_eq!((continuation.map_id(), continuation.instance_id()), (7, 19));
        assert_eq!(continuation.phase_shift(), &phase);
        assert!(query.previous_poly_refs.is_empty(), "generator initialization precedes the actual query");
        let response = Some(detour(&query));
        let StepProgress::Complete(result) = continuation.resume(&mut staged, response) else { panic!("no heights without terrain"); };
        assert_eq!(staged.runtime_elapsed_ms_like_cpp(), elapsed + u64::from(diff));
        assert_eq!(staged.runtime_motion_master_ticks_like_cpp(), ticks + 1);
        assert_eq!(policy_calls, 1);
        let mut synchronous_queries = 0;
        let expected = synchronous.step_movement(diff, Some(target()), None,
            |map, ignore| { assert_eq!(map, 7); assert!(!ignore); true },
            |actual, map, instance, actual_phase| {
                synchronous_queries += 1;
                assert_eq!((map, instance), (7, 19));
                assert_eq!(actual_phase, &phase);
                assert_eq!(actual.start, query.start);
                assert_eq!(actual.destination, query.destination);
                assert_eq!(actual.filter_context, query.filter_context);
                assert_eq!(actual.owner, query.owner);
                assert_eq!(actual.previous_poly_refs, query.previous_poly_refs);
                Some(detour(&actual))
            },
        );
        assert_eq!(synchronous_queries, 1);
        assert_same_result(result, expected);
        assert_eq!(staged.runtime.active_home_generator.as_ref(), synchronous.runtime.active_home_generator.as_ref());
        assert_eq!(staged.runtime.active_random_generator.as_ref(), synchronous.runtime.active_random_generator.as_ref());
        assert_eq!(staged.runtime.active_waypoint_generator.as_ref(), synchronous.runtime.active_waypoint_generator.as_ref());
        assert_eq!(staged.runtime.active_chase_generator.as_ref(), synchronous.runtime.active_chase_generator.as_ref());
        assert_eq!(staged.active_random_path_poly_refs_like_cpp(), synchronous.active_random_path_poly_refs_like_cpp());
        assert_eq!(staged.active_chase_path_poly_refs_like_cpp(), synchronous.active_chase_path_poly_refs_like_cpp());
        assert_eq!(staged.creature.unit().unit_state(), synchronous.creature.unit().unit_state());
        assert_eq!(staged.creature.ai_ownership(), synchronous.creature.ai_ownership());
        for _ in 0..8 {
            assert_eq!(staged.runtime.runtime_rng_like_cpp.next_u64(), synchronous.runtime.runtime_rng_like_cpp.next_u64());
        }
    }
}

#[test]
fn terrain_before_path_retains_original_map_instance_phase_and_never_replays_prefix() {
    let mut actor = actor(CreatureMovementSource::Home, 420_010);
    let phase = actor.phase_shift().clone();
    let home = actor.home_position();
    let mut policy_calls = 0;
    let mut progress = actor.prepare_movement_step(37, None, true, |map, ignore| {
        policy_calls += 1;
        assert_eq!(map, 7);
        assert!(!ignore);
        true
    });
    assert!(matches!(&progress, StepProgress::Pending(StepPending::StaticHeight(_))));
    let mut stages = Vec::new();
    let mut queries = 0;
    let result = loop {
        assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
        assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
        assert_eq!(policy_calls, 1);
        progress = match progress {
            StepProgress::Complete(result) => break result,
            StepProgress::Pending(StepPending::StaticHeight(request)) => {
                let (query, continuation) = request.into_parts();
                assert_eq!(query.map_id, 7);
                assert_eq!(query.probe_z, query.point.z + wow_entities::Z_OFFSET_FIND_HEIGHT);
                stages.push(("static", query.point));
                assert!(actor.active_move_spline_like_cpp().is_none());
                continuation.resume(&mut actor, wow_entities::INVALID_HEIGHT)
            }
            StepProgress::Pending(StepPending::GridHeight(request)) => {
                let (query, continuation) = request.into_parts();
                assert_eq!(query.map_id, 7);
                stages.push(("grid", query.point));
                continuation.resume(&mut actor, query.point.z + 0.5)
            }
            StepProgress::Pending(StepPending::Path(request)) => {
                queries += 1;
                let (query, continuation) = request.into_parts();
                assert_eq!((continuation.map_id(), continuation.instance_id()), (7, 19));
                assert_eq!(continuation.phase_shift(), &phase);
                assert_eq!(query.destination, Position::new(home.x, home.y, home.z + 0.5, home.orientation));
                assert!(matches!(query.filter_context.owner,
                    wow_recastdetour::PathQueryFilterOwner::Creature { in_evade_mode: true, .. }));
                stages.push(("path", query.destination));
                let response = Some(detour(&query));
                continuation.resume(&mut actor, response)
            }
        };
    };
    assert_eq!(queries, 1);
    assert_eq!(stages[0], ("static", home));
    assert_eq!(stages[1], ("grid", home));
    assert_eq!(stages[2].0, "path");
    assert_eq!(stages[3].0, "static", "actual_end starts post-query normalization");
    assert_eq!(stages[3].1, stages[2].1);
    assert_eq!(stages.len(), 11, "one destination pair, one path query, actual_end and three point pairs");
    assert!(matches!(result, Some(CreatureMovementStep::Launch { source: CreatureMovementSource::Home, .. })));
    assert_eq!(actor.spline_id(), 1);
    let mut expected_rng = StdRng::seed_from_u64(0x5757);
    for _ in 0..8 { assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64()); }
}

#[test]
fn staged_dead_step_advances_clock_and_motion_once_without_spline_policy_or_query() {
    let mut actor = make_test_world_creature(test_creature_guid(420_011));
    actor.begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0)).unwrap();
    actor.creature.mark_ai_dead(0);
    let spline_progress = actor.creature.unit().subsystems().motion.spline.progress_ms;
    assert!(matches!(actor.prepare_movement_step(71, None, true,
        |_, _| panic!("dead step cannot query policy")), StepProgress::Complete(None)));
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 71);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
    assert_eq!(actor.creature.unit().subsystems().motion.spline.progress_ms, spline_progress);
}

#[test]
fn staged_idle_step_keeps_rng_and_skips_policy_and_terrain() {
    let mut actor = make_test_world_creature(test_creature_guid(420_012));
    actor.seed_runtime_rng_like_cpp(0x5191);
    assert!(matches!(actor.prepare_movement_step(37, None, true,
        |_, _| panic!("idle has no policy")), StepProgress::Complete(None)));
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
    let mut expected_rng = StdRng::seed_from_u64(0x5191);
    assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64());
}

#[test]
fn waiting_waypoint_calls_policy_once_and_drains_only_one_diff() {
    let mut staged = actor(CreatureMovementSource::Waypoint, 420_013);
    let mut synchronous = actor(CreatureMovementSource::Waypoint, 420_013);
    let mut policy_calls = 0;
    let progress = staged.prepare_movement_step(23, None, true, |_, _| { policy_calls += 1; true });
    assert!(matches!(progress, StepProgress::Complete(None)));
    assert_eq!(policy_calls, 1);
    assert!(synchronous.step_movement(23, None, None, |_, _| true,
        |_, _, _, _| panic!("waiting waypoint has no query")).is_none());
    assert_eq!(staged.runtime.active_waypoint_generator.as_ref(), synchronous.runtime.active_waypoint_generator.as_ref());
    assert_eq!(staged.runtime_elapsed_ms_like_cpp(), 23);
    assert_eq!(staged.runtime_motion_master_ticks_like_cpp(), 1);
}

#[test]
fn waiting_random_preserves_five_draw_stream_and_does_not_create_pending_work() {
    let mut actor = actor(CreatureMovementSource::Random, 420_014);
    assert!(actor.step_movement(200, None, None, |_, _| false,
        |_, _, _, _| panic!("disabled initial query")).is_some());
    let mut policy_calls = 0;
    let progress = actor.prepare_movement_step(1, None, true, |_, _| { policy_calls += 1; true });
    assert!(matches!(progress, StepProgress::Complete(None)));
    assert_eq!(policy_calls, 1);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 201);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 2);
    let mut expected_rng = StdRng::seed_from_u64(0x5757);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: i32 = expected_rng.gen_range(4..=10);
    for _ in 0..8 { assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u64(), expected_rng.next_u64()); }
}

#[test]
fn missing_chase_target_completes_stop_before_policy() {
    let mut staged = actor(CreatureMovementSource::Chase, 420_015);
    let mut synchronous = actor(CreatureMovementSource::Chase, 420_015);
    for actor in [&mut staged, &mut synchronous] {
        actor.begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0)).unwrap();
        actor.creature.unit_mut().add_unit_state(UnitState::CHASE_MOVE.bits());
    }
    let StepProgress::Complete(result) = staged.prepare_movement_step(25, None, true,
        |_, _| panic!("missing target stops before policy")) else { panic!("missing target must complete"); };
    assert!(matches!(&result, Some(CreatureMovementStep::Stop(_))));
    let expected = synchronous.step_movement(25, None, None,
        |_, _| panic!("missing target stops before policy"),
        |_, _, _, _| panic!("missing target has no query"));
    assert_same_result(result, expected);
    assert!(staged.active_chase_generator_like_cpp().is_none());
    assert!(!staged.creature.unit().has_unit_state(UnitState::CHASE_MOVE.bits()));
    assert_eq!(staged.runtime_elapsed_ms_like_cpp(), 25);
    assert_eq!(staged.runtime_motion_master_ticks_like_cpp(), 1);
}

#[test]
fn failed_chase_path_response_translates_stop_without_second_clock() {
    let mut actor = actor(CreatureMovementSource::Chase, 420_016);
    actor.begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0)).unwrap();
    let progress = actor.prepare_movement_step(25, Some(target()), false, |_, _| true);
    let StepProgress::Pending(StepPending::Path(request)) = progress else { panic!("chase requests its path"); };
    let (_, continuation) = request.into_parts();
    let StepProgress::Complete(result) = continuation.resume(&mut actor, None) else { panic!("failure completes without heights"); };
    assert!(matches!(result, Some(CreatureMovementStep::Stop(_))));
    assert!(actor.active_chase_generator_like_cpp().unwrap().cannot_reach_target);
    assert!(actor.active_chase_path_poly_refs_like_cpp().is_empty());
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 25);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
}

#[test]
fn ignore_pathfinding_reaches_policy_and_completes_direct_home_without_query() {
    let mut actor = actor(CreatureMovementSource::Home, 420_017);
    actor.creature.unit_mut().add_unit_state(UnitState::IGNORE_PATHFINDING.bits());
    let mut policy_calls = 0;
    let progress = actor.prepare_movement_step(37, None, false, |map, ignore| {
        policy_calls += 1;
        assert_eq!(map, 7);
        assert!(ignore);
        false
    });
    assert!(matches!(progress, StepProgress::Complete(Some(CreatureMovementStep::Launch {
        source: CreatureMovementSource::Home, ..
    }))));
    assert_eq!(policy_calls, 1);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
}

#[test]
fn dropping_step_pending_preserves_advanced_prefix_without_fictitious_completion() {
    let mut actor = actor(CreatureMovementSource::Home, 420_018);
    let progress = actor.prepare_movement_step(37, None, true, |_, _| true);
    assert!(matches!(&progress, StepProgress::Pending(StepPending::StaticHeight(_))));
    drop(progress);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 37);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 1);
    assert!(actor.creature.is_in_evade_mode_like_cpp());
    assert!(actor.runtime.active_home_generator.is_some());
    assert!(actor.active_move_spline_like_cpp().is_none());
    assert_eq!(actor.spline_id(), 0);
}
