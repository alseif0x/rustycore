//! Domain callback, ordering and motor continuity cases; no worker or packets.

use super::fixtures::*;
use super::{ChaseTargetSnapshotLikeCpp, CreatureMovementSource, CreatureMovementStep};
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use std::cell::RefCell;
use wow_constants::UnitState;
use wow_core::{ObjectGuid, Position};
use wow_entities::CreatureAiState;
use wow_movement::MovementGeneratorType;

#[test]
fn idle_skips_both_callbacks_and_preserves_the_rng_stream() {
    let mut creature = make_test_world_creature(test_creature_guid(300_001));
    creature.seed_runtime_rng_like_cpp(0x5191);
    let mut expected_rng = StdRng::seed_from_u64(0x5191);
    let elapsed = creature.runtime_elapsed_ms_like_cpp();
    let ticks = creature.runtime_motion_master_ticks_like_cpp();
    let result = creature.step_movement(
        37,
        None,
        None,
        |_, _| panic!("idle has no path policy query"),
        |_, _, _, _| panic!("idle has no path query"),
    );
    assert!(result.is_none());
    assert_eq!(creature.runtime_elapsed_ms_like_cpp(), elapsed + 37);
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), ticks + 1);
    assert_eq!(
        creature.runtime.runtime_rng_like_cpp.next_u64(),
        expected_rng.next_u64()
    );
}

#[test]
fn dead_advances_clock_and_motion_master_without_advancing_spline_or_callbacks() {
    let mut creature = make_test_world_creature(test_creature_guid(300_002));
    creature
        .begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0))
        .unwrap();
    creature.creature.mark_ai_dead(0);
    let progress = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .spline
        .progress_ms;
    let elapsed = creature.runtime_elapsed_ms_like_cpp();
    let ticks = creature.runtime_motion_master_ticks_like_cpp();
    assert!(
        creature
            .step_movement(
                71,
                None,
                None,
                |_, _| panic!("dead movement must not call policy"),
                |_, _, _, _| panic!("dead movement must not call path"),
            )
            .is_none()
    );
    assert_eq!(creature.runtime_elapsed_ms_like_cpp(), elapsed + 71);
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), ticks + 1);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .progress_ms,
        progress
    );
    assert!(!creature.is_alive());
}

#[test]
fn home_queries_after_evade_and_preserves_map_phase_and_launch_origin() {
    let mut creature = make_test_world_creature(test_creature_guid(300_003));
    creature
        .creature
        .unit_mut()
        .world_mut()
        .set_map(7, 19)
        .unwrap();
    creature
        .creature
        .set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    creature.creature.set_ai_state(CreatureAiState::Returning);
    let from = creature.position();
    let home = creature.home_position();
    let phase = creature.phase_shift().clone();
    let owner = creature.detour_owner_capabilities_like_cpp();
    let events = RefCell::new(Vec::new());
    let result = creature.step_movement(
        23,
        None,
        None,
        |map, ignore| {
            assert_eq!(map, 7);
            assert!(!ignore);
            events.borrow_mut().push("policy");
            true
        },
        |query, map, instance, query_phase| {
            events.borrow_mut().push("path");
            assert_eq!((map, instance), (7, 19));
            assert_eq!(query_phase, &phase);
            assert_eq!(query.start, from);
            assert_eq!(query.destination, home);
            assert_eq!(query.owner, owner);
            assert!(!query.force_destination);
            assert!(query.previous_poly_refs.is_empty());
            assert!(matches!(
                query.filter_context.owner,
                wow_recastdetour::PathQueryFilterOwner::Creature {
                    in_evade_mode: true,
                    ..
                }
            ));
            None
        },
    );
    assert_eq!(*events.borrow(), vec!["policy", "path"]);
    match result.expect("home falls back to the direct segment") {
        CreatureMovementStep::Launch {
            source,
            from: launched_from,
            spline,
        } => {
            assert_eq!(source, CreatureMovementSource::Home);
            assert_eq!(launched_from, from);
            assert_eq!(spline.id(), creature.spline_id());
        }
        other => panic!("expected home launch, got {other:?}"),
    }
}

#[test]
fn ignore_pathfinding_reaches_policy_but_does_not_resolve_a_path() {
    let mut creature = make_test_world_creature(test_creature_guid(300_004));
    creature
        .creature
        .set_ai_position(Position::xyz(30.0, 10.0, 0.0));
    creature.creature.set_ai_state(CreatureAiState::Returning);
    creature
        .creature
        .unit_mut()
        .add_unit_state(UnitState::IGNORE_PATHFINDING.bits());
    let mut policies = 0;
    let result = creature.step_movement(
        25,
        None,
        None,
        |_, ignore| {
            policies += 1;
            assert!(ignore);
            !ignore
        },
        |_, _, _, _| panic!("disabled pathfinding must stay lazy"),
    );
    assert_eq!(policies, 1);
    assert!(matches!(
        result,
        Some(CreatureMovementStep::Launch {
            source: CreatureMovementSource::Home,
            ..
        })
    ));
}

#[test]
fn waypoint_wait_calls_policy_but_path_waits_until_the_launch_frame() {
    let mut creature = make_test_world_creature(test_creature_guid(300_005));
    creature.initialize_default_waypoint_movement_like_cpp(Some(wow_movement::WaypointPath::new(
        7,
        vec![wow_movement::WaypointNode::new(1, 20.0, 10.0, 0.0)],
    )));
    let events = RefCell::new(Vec::new());
    let mut drive = |creature: &mut crate::map_manager::WorldCreature, diff| {
        creature.step_movement(
            diff,
            None,
            None,
            |_, _| {
                events.borrow_mut().push("policy");
                true
            },
            |query, _, _, _| {
                events.borrow_mut().push("path");
                assert_eq!(query.destination.x, 20.0);
                assert!(query.previous_poly_refs.is_empty());
                None
            },
        )
    };
    assert!(drive(&mut creature, 1).is_none());
    assert_eq!(*events.borrow(), vec!["policy"]);
    let result = drive(
        &mut creature,
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP as u32,
    );
    assert_eq!(*events.borrow(), vec!["policy", "policy", "path"]);
    assert!(matches!(
        result,
        Some(CreatureMovementStep::Launch {
            source: CreatureMovementSource::Waypoint,
            ..
        })
    ));
}

#[test]
fn random_initialization_clears_old_corridor_before_the_lazy_query() {
    let mut creature = make_test_world_creature(test_creature_guid(300_006));
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 5.0;
    creature.seed_runtime_rng_like_cpp(0x5757);
    creature.runtime.active_random_path_poly_refs = vec![11, 12];
    let events = RefCell::new(Vec::new());
    let filter = creature.path_query_filter_context_like_cpp();
    let owner = creature.detour_owner_capabilities_like_cpp();
    let result = creature.step_movement(
        200,
        None,
        None,
        |_, _| {
            events.borrow_mut().push("policy");
            true
        },
        |query, _, _, _| {
            events.borrow_mut().push("path");
            assert!(query.previous_poly_refs.is_empty());
            assert_eq!(query.filter_context, filter);
            assert_eq!(query.owner, owner);
            None
        },
    );
    assert_eq!(*events.borrow(), vec!["policy", "path"]);
    assert!(
        result.is_none(),
        "a failed random path must keep its existing failure behavior"
    );
    assert!(creature.active_random_path_poly_refs_like_cpp().is_empty());
}

#[test]
fn first_random_leg_preserves_the_five_draw_sequence_and_next_rng_value() {
    let mut creature = make_test_world_creature(test_creature_guid(300_007));
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 3.0;
    creature.seed_runtime_rng_like_cpp(0x5757);
    let mut expected_rng = StdRng::seed_from_u64(0x5757);
    // Existing initialization draw, then destination distance/angle, steps and pause.
    let _: u8 = expected_rng.gen_range(2..=10);
    let distance: f32 = expected_rng.gen_range(0.0..=1.0);
    let angle: f32 = expected_rng.gen_range(0.0..=1.0);
    let _: u8 = expected_rng.gen_range(2..=10);
    let _: i32 = expected_rng.gen_range(4..=10);
    let expected_destination = wow_movement::compute_random_destination_like_cpp(
        creature.position(),
        3.0,
        distance,
        angle,
    )
    .destination;
    let result = creature.step_movement(
        200,
        None,
        None,
        |_, _| false,
        |_, _, _, _| panic!("policy disabled paths"),
    );
    assert!(matches!(
        result,
        Some(CreatureMovementStep::Launch {
            source: CreatureMovementSource::Random,
            ..
        })
    ));
    assert_eq!(creature.move_target(), Some(expected_destination));
    for _ in 0..8 {
        assert_eq!(
            creature.runtime.runtime_rng_like_cpp.next_u64(),
            expected_rng.next_u64()
        );
    }
}

#[test]
fn missing_chase_target_stops_spline_and_clears_chase_state_before_callbacks() {
    let mut creature = make_test_world_creature(test_creature_guid(300_008));
    let target = ObjectGuid::create_player(1, 123);
    creature.enter_combat(target);
    creature
        .begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0))
        .unwrap();
    creature.runtime.active_chase_generator = Some(wow_movement::ChaseMovementGenerator::new(
        target, None, None,
    ));
    creature.runtime.active_chase_path_poly_refs = vec![31, 32];
    creature
        .creature
        .unit_mut()
        .add_unit_state(UnitState::CHASE_MOVE.bits());
    let result = creature.step_movement(
        25,
        None,
        None,
        |_, _| panic!("missing target must not call policy"),
        |_, _, _, _| panic!("missing target must not call path"),
    );
    let Some(CreatureMovementStep::Stop(stop)) = result else {
        panic!("active chase must stop")
    };
    assert_eq!(stop.position, creature.position());
    assert_eq!(stop.spline_id, creature.spline_id());
    assert!(creature.active_chase_generator_like_cpp().is_none());
    assert!(creature.active_chase_path_poly_refs_like_cpp().is_empty());
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::CHASE_MOVE.bits())
    );
    assert!(
        creature
            .active_move_spline_like_cpp()
            .is_none_or(wow_movement::MoveSpline::finalized)
    );
}

#[test]
fn chase_new_victim_resets_corridor_before_query_and_disabled_policy_still_launches() {
    let target = ChaseTargetSnapshotLikeCpp {
        guid: ObjectGuid::create_player(1, 124),
        position: Position::xyz(50.0, 10.0, 0.0),
        combat_reach: 1.5,
        in_world: true,
        in_water: None,
    };
    let mut creature = make_test_world_creature(test_creature_guid(300_009));
    creature.enter_combat(target.guid);
    creature.runtime.active_chase_path_poly_refs = vec![41, 42];
    let events = RefCell::new(Vec::new());
    creature.step_movement(
        200,
        Some(target),
        None,
        |_, _| {
            events.borrow_mut().push("policy");
            true
        },
        |query, _, _, _| {
            events.borrow_mut().push("path");
            assert_eq!(query.destination, target.position);
            assert!(query.previous_poly_refs.is_empty());
            None
        },
    );
    assert_eq!(*events.borrow(), vec!["policy", "path"]);

    let mut direct = make_test_world_creature(test_creature_guid(300_010));
    direct.enter_combat(target.guid);
    assert!(matches!(
        direct.step_movement(
            200,
            Some(target),
            None,
            |_, _| false,
            |_, _, _, _| panic!("disabled chase path"),
        ),
        Some(CreatureMovementStep::Launch {
            source: CreatureMovementSource::Chase,
            ..
        })
    ));
}

#[test]
fn highest_point_and_distract_keep_selection_without_random_or_chase_queries() {
    for point in [true, false] {
        let mut creature =
            make_test_world_creature(test_creature_guid(if point { 300_011 } else { 300_012 }));
        creature
            .creature
            .set_default_movement_type_runtime_like_cpp(
                wow_entities::MovementGeneratorType::Random,
            );
        creature.creature.ai_ownership_mut().wander_radius = 3.0;
        if point {
            // Charge installs Point at Highest; merely launching a spline does not.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_charge(42);
            creature
                .begin_move_spline_like_cpp(Position::xyz(50.0, 10.0, 0.0))
                .unwrap();
        } else {
            creature
                .begin_distract_movement_like_cpp(8_000, 1.25)
                .unwrap();
        }
        creature.enter_combat(ObjectGuid::create_player(1, 125));
        creature.seed_runtime_rng_like_cpp(0x7171);
        let mut expected_rng = StdRng::seed_from_u64(0x7171);
        assert!(
            creature
                .step_movement(
                    25,
                    None,
                    None,
                    |_, _| panic!("selected Point/Distract has no path policy"),
                    |_, _, _, _| panic!("selected Point/Distract has no path query"),
                )
                .is_none()
        );
        assert_eq!(
            creature.runtime_motion_master_current_kind_like_cpp(),
            Some(if point {
                MovementGeneratorType::Point
            } else {
                MovementGeneratorType::Distract
            })
        );
        assert_eq!(
            creature.runtime.runtime_rng_like_cpp.next_u64(),
            expected_rng.next_u64()
        );
        if point {
            assert!(creature.active_move_spline_like_cpp().is_some());
        } else {
            // The facing spline may finish before the Distract timer; its
            // generator must remain selected while that timer is still live.
            assert!(
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .motion
                    .active_generators
                    .iter()
                    .any(
                        |generator| generator.kind == wow_entities::MovementGeneratorKind::Distract
                    )
            );
        }
    }
}
