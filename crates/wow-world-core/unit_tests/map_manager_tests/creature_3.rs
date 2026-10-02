//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_creature_detour_path_bridge_preserves_elevated_mmap_points_without_vmap() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
        50,
        2,
        5,
        10,
        20.0,
        100,
        14,
        0,
        wow_constants::UnitFlags::CAN_SWIM.bits(),
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
    let dst = Position::new(15.0, 12.0, 30.0, 0.0);
    let elevated_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 30.0], [12.0, 11.0, 30.0], [15.0, 12.0, 30.0]],
            actual_end: [15.0, 12.0, 30.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&elevated_path),
            false,
            Some(&terrain),
        )
        .expect("elevated detour path launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 30.0, 0.0),
            Position::new(12.0, 11.0, 30.0, 0.0),
            Position::new(15.0, 12.0, 30.0, 0.0),
        ]
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 30.0, 0.0))
    );
    assert!(creature.creature.can_swim_like_cpp());
    assert!(
        spline.flags().contains(MoveSplineFlag::CAN_SWIM),
        "MoveSplineInit must snapshot Unit::CanSwim like C++"
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
#[test]
fn world_creature_detour_path_bridge_does_not_join_unproven_flat_elevated_surfaces() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54353);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
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
    let dst = Position::new(15.0, 12.0, 30.0, 0.0);
    let projected_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 2.0], [12.0, 11.0, 2.0], [15.0, 12.0, 2.0]],
            actual_end: [15.0, 12.0, 2.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&projected_path),
            false,
            Some(&terrain),
        )
        .expect("projected elevated detour path launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "equal endpoint heights do not prove one continuous elevated surface"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
#[test]
fn world_creature_detour_path_bridge_does_not_invent_a_sloped_vmap_surface() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54354);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 30.0, 0.0),
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
    let dst = Position::new(15.0, 12.0, 34.0, 0.0);
    let projected_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, 2.0], [12.0, 11.0, 2.0], [15.0, 12.0, 2.0]],
            actual_end: [15.0, 12.0, 2.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&projected_path),
            false,
            Some(&terrain),
        )
        .expect("unproven sloped path still launches");

    assert_eq!(
        path.expect("path generator").path_points(),
        &[
            Position::new(10.0, 10.0, 2.0, 0.0),
            Position::new(12.0, 11.0, 2.0, 0.0),
            Position::new(15.0, 12.0, 2.0, 0.0),
        ],
        "without VMap height Rust must not guess a changing elevated surface"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, 2.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
#[test]
fn world_creature_detour_path_bridge_keeps_far_below_points_without_ground_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, -5.0, 0.0),
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
    let dst = Position::new(15.0, 12.0, -5.0, 0.0);
    let far_below_path = DetourPolyPath {
        poly_refs: vec![11, 22],
        point_path: wow_recastdetour::DetourPointPath {
            points: vec![[10.0, 10.0, -5.0], [12.0, 11.0, -5.0], [15.0, 12.0, -5.0]],
            actual_end: [15.0, 12.0, -5.0],
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    let (_from, spline, path) = creature
        .begin_random_move_spline_with_detour_path_and_terrain_like_cpp(
            dst,
            Some(&far_below_path),
            false,
            Some(&terrain),
        )
        .expect("far-below detour path launches without terrain lift");

    let path = path.expect("path generator");
    assert_eq!(
        path.path_points(),
        &[
            Position::new(10.0, 10.0, -5.0, 0.0),
            Position::new(12.0, 11.0, -5.0, 0.0),
            Position::new(15.0, 12.0, -5.0, 0.0),
        ],
        "points farther than DEFAULT_HEIGHT_SEARCH below raw ground keep C++ no-ground behavior"
    );
    assert_eq!(
        spline.final_destination(),
        Some(Position::new(15.0, 12.0, -5.0, 0.0))
    );

    let _ = std::fs::remove_dir_all(&data_dir);
}
#[test]
fn world_creature_random_detour_rejects_nopath_and_shortcut_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54327);
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
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);

    for path_type in [DetourPathType::NOPATH, DetourPathType::SHORTCUT] {
        let detour_path = DetourPolyPath {
            poly_refs: Vec::new(),
            point_path: wow_recastdetour::DetourPointPath {
                points: vec![[10.0, 10.0, 0.0], [20.0, 10.0, 0.0]],
                actual_end: [20.0, 10.0, 0.0],
                path_type,
            },
            start_far_from_poly: false,
            end_far_from_poly: false,
        };

        assert!(
            creature
                .begin_random_move_spline_with_detour_path_like_cpp(dst, Some(&detour_path), false)
                .is_none(),
            "C++ RandomMovementGenerator retries later instead of launching {:?} paths",
            path_type
        );
        assert!(creature.active_move_spline_like_cpp().is_none());
    }
}
#[test]
fn world_creature_random_missing_path_retries_instead_of_direct_fallback_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54340);
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
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 8.0;
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    creature.seed_runtime_rng_like_cpp(0x24_5A0);

    let mut resolver_called = false;
    let movement =
        creature.update_default_random_movement_with_path_resolver_like_cpp(10, true, |_query| {
            resolver_called = true;
            None
        });

    assert!(resolver_called);
    assert!(
        movement.is_none(),
        "C++ RandomMovementGenerator retries when PathGenerator cannot build a usable path"
    );
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert_eq!(
        creature
            .active_random_generator
            .as_ref()
            .expect("random generator")
            .timer_ms(),
        wow_movement::RANDOM_PATH_RETRY_MS_LIKE_CPP
    );
}
/// C++ `PathGenerator::UpdateFilter` adds `NAV_GROUND_STEEP` while the owner
/// `IsInCombat()` (`PathGenerator.cpp:694-696`). `Creature::enter_ai_combat`
/// does not set the client-visible `UNIT_FLAG_IN_COMBAT`, so the filter has
/// to read the runtime combat state the tick actually maintains.
#[test]
fn world_creature_chase_filter_includes_ground_steep_while_engaged_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54350);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 77);
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
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::Run as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);

    let idle = create_path_query_filter_like_cpp(creature.path_query_filter_context_like_cpp())
        .expect("filter");
    assert_eq!(
        idle.include_flags(),
        wow_recastdetour::NavTerrainFlag::GROUND.bits(),
        "an idle ground creature must not get NAV_GROUND_STEEP"
    );

    creature.enter_combat(victim);
    assert!(
        creature.creature.is_in_combat(),
        "enter_combat must leave a runtime combat signal the filter can read"
    );
    let engaged = create_path_query_filter_like_cpp(creature.path_query_filter_context_like_cpp())
        .expect("filter");
    assert_eq!(
        engaged.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits(),
        "C++ UpdateFilter grants steep ground to a creature in combat"
    );
}
/// C++ `Unit::isInAccessiblePlaceFor` branches on the victim's real
/// `IsInWater()`. Creature victims carry no liquid state here, and guessing
/// "not in water" would make an aquatic, non-walking chaser report
/// `CannotReachTarget` on a victim C++ would let it reach.
#[test]
fn world_creature_chase_unknown_water_does_not_block_an_aquatic_chaser_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54351);
    let victim_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54352);
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
    // A swim-only creature: no ground movement, no flight.
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(true);
    creature.creature.set_flight_movement_type_runtime_like_cpp(
        wow_constants::CreatureFlightMovementType::None as u8,
    );
    assert!(!creature.creature.can_walk_like_cpp());
    assert!(creature.creature.can_enter_water_like_cpp());

    let target = |in_water| ChaseTargetSnapshotLikeCpp {
        guid: victim_guid,
        position: Position::new(20.0, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water,
    };

    assert!(
        creature
            .chase_unit_snapshot_like_cpp(target(None))
            .target_accessible,
        "an unknown water state must not block a chaser that can enter water"
    );
    assert!(
        creature
            .chase_unit_snapshot_like_cpp(target(Some(true)))
            .target_accessible,
        "C++ takes the water branch and asks CanEnterWater()"
    );
    assert!(
        !creature
            .chase_unit_snapshot_like_cpp(target(Some(false)))
            .target_accessible,
        "C++ takes the else branch and asks CanWalk() || CanFly()"
    );
}
/// C++ installs a new `ChaseMovementGenerator` per `MoveChase`, and its
/// `AbstractFollower` is bound to that victim for the generator's life
/// (`ChaseMovementGenerator.cpp:68-76`). A victim switch must not inherit the
/// previous follower, timers or arrival inform counter.
#[test]
fn world_creature_chase_rebuilds_the_generator_when_the_victim_changes_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54353);
    let first = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 91);
    let second = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 92);
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

    let target = |victim, x| ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(x, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    creature.enter_combat(first);
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(first, 40.0),
        false,
        None,
        |_| None,
    );
    assert_eq!(
        creature
            .active_chase_generator_like_cpp()
            .and_then(wow_movement::ChaseMovementGenerator::target),
        Some(first)
    );

    creature.enter_combat(second);
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(second, 45.0),
        false,
        None,
        |_| None,
    );
    assert_eq!(
        creature
            .active_chase_generator_like_cpp()
            .and_then(wow_movement::ChaseMovementGenerator::target),
        Some(second),
        "the generator must follow the new victim, not the previous one"
    );
}
/// C++ replaces its owned `PathGenerator` before `CalculatePath` when
/// `moveToward != _movingTowards`, while keeping it for same-direction
/// recalculations (`ChaseMovementGenerator.cpp:170-175`). The retained
/// Detour corridor has to follow that exact lifecycle.
#[test]
fn world_creature_chase_direction_flip_resets_corridor_and_same_direction_reuses_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54361);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 98);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 60.0),
        true,
        None,
        |query| {
            assert!(query.previous_poly_refs.is_empty());
            Some(test_chase_corridor(vec![101, 102], 59.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[101, 102]);
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 65.0),
        true,
        None,
        |query| {
            assert_eq!(query.previous_poly_refs, vec![101, 102]);
            Some(test_chase_corridor(vec![201, 202], 64.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[201, 202]);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 10.5),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "a toward-to-away flip must discard the old path object"
            );
            Some(test_chase_corridor(vec![301, 302], 7.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert!(
        !creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[301, 302]);

    let launched = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 70.0),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "an away-to-toward flip must discard the old path object"
            );
            Some(test_chase_corridor(vec![401, 402], 69.0))
        },
    );
    assert!(matches!(launched, ChaseTickOutcomeLikeCpp::Launched(..)));
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards()
    );
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[401, 402]);
}
#[test]
fn world_creature_failed_direction_flip_does_not_publish_unlaunched_direction() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54362);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 99);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    assert!(matches!(
        creature.update_runtime_chase_movement_like_cpp(
            1,
            test_chase_target(victim, 60.0),
            true,
            None,
            |_| Some(test_chase_corridor(vec![501, 502], 59.0)),
        ),
        ChaseTickOutcomeLikeCpp::Launched(..)
    ));
    assert_eq!(creature.active_chase_path_poly_refs_like_cpp(), &[501, 502]);

    let failed = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 10.5),
        true,
        None,
        |query| {
            assert!(
                query.previous_poly_refs.is_empty(),
                "the old toward corridor must be gone before the away query"
            );
            None
        },
    );
    assert!(matches!(
        failed,
        ChaseTickOutcomeLikeCpp::Stopped(_) | ChaseTickOutcomeLikeCpp::Idle
    ));
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .moving_towards(),
        "a failed query must not publish a move-away direction that never launched"
    );
    assert!(creature.active_chase_path_poly_refs_like_cpp().is_empty());
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED),
        "failed path handling keeps the existing C++ inform lifecycle"
    );
}
#[test]
fn world_creature_nopath_after_arrival_does_not_reenable_consumed_inform_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54363);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 100);
    let mut creature = test_creature(guid);
    creature.enter_combat(victim);

    assert!(matches!(
        creature.update_runtime_chase_movement_like_cpp(
            1,
            test_chase_target(victim, 60.0),
            true,
            None,
            |_| Some(test_chase_corridor(vec![601, 602], 59.0)),
        ),
        ChaseTickOutcomeLikeCpp::Launched(..)
    ));

    let duration_ms = creature
        .active_move_spline_like_cpp()
        .expect("chase spline")
        .duration_ms();
    creature
        .backdate_runtime_clock_for_test(Duration::from_millis(u64::from(duration_ms as u32) + 50));
    assert!(creature.update_move_spline_like_cpp());
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        test_chase_target(victim, 60.0),
        true,
        None,
        |_| panic!("arrival must not calculate another path"),
    );
    assert!(
        !creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED)
    );
    assert!(
        creature.creature.take_ai_movement_inform().is_some(),
        "the successful spline arrival must consume and publish its inform"
    );

    let failed = creature.update_runtime_chase_movement_like_cpp(
        1,
        test_chase_target(victim, 100.0),
        true,
        None,
        |_| None,
    );
    assert!(matches!(
        failed,
        ChaseTickOutcomeLikeCpp::Stopped(_) | ChaseTickOutcomeLikeCpp::Idle
    ));
    let generator = creature
        .active_chase_generator_like_cpp()
        .expect("chase generator");
    assert!(generator.cannot_reach_target);
    assert!(
        !generator.has_flag(RuntimeMovementGeneratorFlags::INFORM_ENABLED),
        "NOPATH after arrival must preserve the consumed inform state"
    );

    let owner_position = creature.position();
    let in_range_target = ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(
            owner_position.x + 3.0,
            owner_position.y,
            owner_position.z,
            0.0,
        ),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };
    let snapshot = creature.chase_unit_snapshot_like_cpp(in_range_target);
    let bounds = creature
        .active_chase_generator_like_cpp()
        .expect("chase generator")
        .bounds_like_cpp(snapshot);
    assert!(
        !snapshot.owner_has_chase_move
            && wow_movement::generators::chase::position_okay_like_cpp(
                snapshot,
                Some(bounds.min_range),
                Some(bounds.max_range),
                None,
            ),
        "the final target must exercise the in-range, no-active-spline branch: \
         snapshot={snapshot:?}, bounds={bounds:?}"
    );
    assert_eq!(
        creature.update_runtime_chase_movement_like_cpp(
            100,
            in_range_target,
            true,
            None,
            |_| panic!("an in-range target must not calculate another path"),
        ),
        ChaseTickOutcomeLikeCpp::Idle
    );
    assert!(
        creature.creature.take_ai_movement_inform().is_none(),
        "a route that never launched must not publish a duplicate arrival inform"
    );
}
/// C++ chase calls `CalculatePath(x, y, z, owner->CanFly())`
/// (`ChaseMovementGenerator.cpp:196`), and `_forceDestination` is consumed
/// *inside* `BuildPointPath`, so the flag has to reach the query itself.
#[test]
fn world_creature_chase_passes_can_fly_as_force_destination_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54354);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 93);

    for can_fly in [false, true] {
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
        creature
            .creature
            .set_flight_movement_type_runtime_like_cpp(if can_fly {
                wow_constants::CreatureFlightMovementType::CanFly as u8
            } else {
                wow_constants::CreatureFlightMovementType::None as u8
            });
        assert_eq!(creature.creature.can_fly_like_cpp(), can_fly);
        creature.enter_combat(victim);

        let mut observed = None;
        let _ = creature.update_runtime_chase_movement_like_cpp(
            100,
            ChaseTargetSnapshotLikeCpp {
                guid: victim,
                position: Position::new(60.0, 10.0, 0.0, 0.0),
                combat_reach: 1.0,
                in_world: true,
                in_water: Some(false),
            },
            true,
            None,
            |query| {
                observed = Some(query.force_destination);
                None
            },
        );
        assert_eq!(
            observed,
            Some(can_fly),
            "the chase query must carry forceDest = CanFly()"
        );
    }
}
