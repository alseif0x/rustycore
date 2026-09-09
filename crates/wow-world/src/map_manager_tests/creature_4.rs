//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

/// `resolve_creature_detour_path_like_cpp` already answers a missing
/// navmesh with the C++ `BuildShortcut()` path, so a `None` from it means the
/// query was attempted and failed. C++ has no such case — its own failures
/// went through `BuildShortcut()` + `PATHFIND_NOPATH` — so it must stop and
/// retry rather than launch a straight line through blocked geometry.
#[test]
fn world_creature_chase_query_failure_stops_instead_of_straight_lining_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54355);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 94);
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
    creature.enter_combat(victim);
    let target = ChaseTargetSnapshotLikeCpp {
        guid: victim,
        position: Position::new(60.0, 10.0, 0.0, 0.0),
        combat_reach: 1.0,
        in_world: true,
        in_water: Some(false),
    };

    // Pathfinding attempted and failed: no spline, and cannot-reach is set.
    let outcome =
        creature.update_runtime_chase_movement_like_cpp(100, target, true, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert!(
        creature
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .cannot_reach_target,
        "a failed query must record cannot-reach like the C++ NOPATH branch"
    );

    // Pathfinding disabled for this map/owner: C++ CalculatePath answers with
    // BuildShortcut() + NORMAL|NOT_USING_PATH, which chase launches.
    let mut disabled = WorldCreature::new(
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
    disabled.enter_combat(victim);
    let outcome =
        disabled.update_runtime_chase_movement_like_cpp(100, target, false, None, |_| None);
    assert!(
        matches!(outcome, ChaseTickOutcomeLikeCpp::Launched(..)),
        "with no navmesh C++ still launches the shortcut, got {outcome:?}"
    );
    assert!(
        !disabled
            .active_chase_generator_like_cpp()
            .expect("chase generator")
            .cannot_reach_target,
        "a disabled navmesh is not a path failure"
    );
}
#[test]
fn world_creature_taunt_priority_expires_at_aura_duration_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54355);
    let taunter = ObjectGuid::create_player(1, 1);
    let newer_taunter = ObjectGuid::create_player(1, 3);
    let tank = ObjectGuid::create_player(1, 2);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
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
    {
        let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
        combat.add_threat(taunter, 10.0);
        combat.add_threat(newer_taunter, 20.0);
        combat.add_threat(tank, 100.0);
        assert_eq!(combat.reselect_victim(&HashSet::new()), Some(tank));
    }

    assert_eq!(
        creature.apply_taunt_aura_like_cpp(taunter, 100, 1, 2_000),
        Some(0)
    );
    assert_eq!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .reselect_victim(&HashSet::new()),
        Some(taunter),
        "an active taunt bypasses ordinary threat-switch thresholds"
    );
    assert_eq!(
        creature.apply_taunt_aura_like_cpp(newer_taunter, 100, 1, 1_000),
        Some(1)
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&newer_taunter)
    );
    {
        let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
        assert_eq!(combat.reselect_victim(&HashSet::new()), Some(newer_taunter));
        assert!(
            combat
                .set_threat_online_state(newer_taunter, wow_entities::ThreatOnlineState::Offline,)
        );
        assert_eq!(
            combat.reselect_victim(&HashSet::new()),
            Some(taunter),
            "C++ falls back to the older active taunt while the newest taunter is unavailable"
        );
        assert!(
            combat.set_threat_online_state(newer_taunter, wow_entities::ThreatOnlineState::Online,)
        );
    }

    creature.backdate_runtime_clock_for_test(Duration::from_millis(1_200));
    assert_eq!(creature.expire_taunt_auras_if_due_like_cpp(), vec![1]);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&taunter),
        "when the newest taunt expires C++ restores the still-active older taunt"
    );

    creature.backdate_runtime_clock_for_test(Duration::from_millis(2_200));
    assert_eq!(creature.expire_taunt_auras_if_due_like_cpp(), vec![0]);
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .sorted_threat_guids()
            .first(),
        Some(&tank)
    );

    let reset_slot = creature
        .apply_taunt_aura_like_cpp(taunter, 102, 1, 2_000)
        .expect("taunt aura slot");
    assert_eq!(creature.reset_combat(), vec![reset_slot]);
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .visible_auras
            .values()
            .all(|aura| aura.spell_id != 102),
        "C++ RemoveAurasOnEvade removes the visible taunt aura during combat reset"
    );
    assert!(
        creature.expire_taunt_auras_if_due_like_cpp().is_empty(),
        "an evade-removed taunt must not expire a second time"
    );
}
#[test]
fn world_creature_permanent_taunt_never_expires_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let taunter = ObjectGuid::create_player(1, 4);
    let tank = ObjectGuid::create_player(1, 5);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
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
    let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
    combat.add_threat(taunter, 10.0);
    combat.add_threat(tank, 100.0);
    assert_eq!(
        creature.apply_taunt_aura_like_cpp(taunter, 101, 1, -1),
        Some(0)
    );
    creature.backdate_runtime_clock_for_test(Duration::from_secs(86_400));
    assert!(creature.expire_taunt_auras_if_due_like_cpp().is_empty());
    assert_eq!(
        creature
            .creature
            .unit_mut()
            .subsystems_mut()
            .combat
            .reselect_victim(&HashSet::new()),
        Some(taunter)
    );
}
#[test]
fn world_creature_taunt_aura_persists_without_threat_reference_like_cpp() {
    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let caster = ObjectGuid::create_player(1, 4);
    let mut creature = WorldCreature::new(
        creature_guid,
        1,
        Position::default(),
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

    let slot = creature
        .apply_taunt_aura_like_cpp(caster, 355, 1, 2_000)
        .expect("C++ still applies MOD_TAUNT when EffectTaunt sees an empty threat list");

    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .combat
            .is_threat_list_empty(true)
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .visible_auras
            .contains_key(&slot)
    );
}
/// C++ adds `UNIT_STATE_EVADE` immediately before `MoveTargetedHome()`
/// (`CreatureAI.cpp:237`) and only `HomeMovementGenerator::DoFinalize` clears
/// it (`HomeMovementGenerator.cpp:143`). It is what makes the creature immune
/// to attacks and un-aggroable for the whole walk back, and
/// `SetTargetLocation` deliberately preserves it by clearing
/// `UNIT_STATE_ALL_ERASABLE & ~UNIT_STATE_EVADE` (`:60`).
#[test]
fn world_creature_home_return_holds_evade_state_until_finalize_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54356);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
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
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));
    creature.creature.unit_mut().set_health(17);
    assert!(!creature.creature.is_in_evade_mode_like_cpp());

    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert!(
        matches!(outcome, ChaseTickOutcomeLikeCpp::Launched(..)),
        "the home return must launch, got {outcome:?}"
    );
    assert!(
        creature.creature.is_in_evade_mode_like_cpp(),
        "the creature must be evading for the whole walk home, or a player \
         can damage and re-aggro a fully reset creature"
    );
    assert!(
        creature.creature.is_evading_attacks_like_cpp(),
        "C++ IsEvadingAttacks() gates damage while returning"
    );

    // Advancing while the spline is still running must not drop the state,
    // and C++ only fires the reached-home payload once `DoUpdate` has seen
    // the spline finalized (it is what sets `INFORM_ENABLED`).
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(creature.creature.is_in_evade_mode_like_cpp());
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "JustReachedHome must not fire while the creature is still walking"
    );

    // Let real time pass beyond the spline duration and advance it, exactly
    // as `Unit::Update` does before `MotionMaster::Update`.
    let duration_ms = creature
        .active_move_spline_like_cpp()
        .expect("home spline")
        .duration_ms();
    assert!(duration_ms > 0, "the home spline must have a duration");
    creature
        .backdate_runtime_clock_for_test(Duration::from_millis(u64::from(duration_ms as u32) + 50));
    assert!(creature.update_move_spline_like_cpp());

    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        !creature.creature.is_in_evade_mode_like_cpp(),
        "DoFinalize clears UNIT_STATE_EVADE on arrival"
    );
    assert_eq!(
        creature.state(),
        CreatureAiState::Idle,
        "the creature is home and idle again"
    );
    assert_eq!(
        creature.creature.unit().data().health,
        creature.creature.unit().data().max_health,
        "C++ SetSpawnHealth restores health on home finalization"
    );
    assert!(
        creature.take_home_health_restored_pending_like_cpp(),
        "the global tick must publish the health restored by home finalization"
    );
    assert!(
        !creature.take_home_health_restored_pending_like_cpp(),
        "the values update publication marker is consumed once"
    );
    assert!(
        creature.creature.take_ai_just_reached_home(),
        "C++ AI()->JustReachedHome() must fire on a natural home arrival"
    );
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "the callback is consumed once"
    );
}
/// C++ sets `UNIT_STATE_EVADE` before `MoveTargetedHome()` constructs the
/// path (`CreatureAI.cpp:237`), so `UpdateFilter` includes `NAV_GROUND_STEEP`
/// for the return route (`PathGenerator.cpp:694-696`). The bridge therefore
/// has to build the query *after* it enters evade, not let the caller sample
/// the filter one step early.
#[test]
fn world_creature_home_query_filter_is_sampled_after_entering_evade_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54357);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
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
    creature
        .creature
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));

    // The creature is neither in combat nor evading when the return starts,
    // which is exactly the state `reset_combat` leaves behind.
    assert!(!creature.creature.is_in_combat());
    assert!(!creature.creature.is_in_evade_mode_like_cpp());

    let mut observed = None;
    let _ = creature.update_runtime_home_movement_like_cpp(true, None, |query| {
        observed = Some(query.filter_context);
        None
    });

    let filter = create_path_query_filter_like_cpp(observed.expect("home query")).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits(),
        "the home route must be queried with the evade state already applied, \
         or it excludes steep polygons C++ allows"
    );
}
/// A chase that just switched victim must query with a fresh corridor. C++
/// installs a new `ChaseMovementGenerator` owning a new `PathGenerator`, so
/// there is nothing to reuse; feeding the previous victim's `_pathPolyRefs`
/// would let `BuildPolyPath`'s ~80% prefix branch steer the first spline back
/// toward the old target (`PathGenerator.cpp:339-413`).
#[test]
fn world_creature_chase_first_query_after_victim_switch_has_no_corridor_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54358);
    let first = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 95);
    let second = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 96);
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
    let corridor = |points: Vec<[f32; 3]>, refs: Vec<u64>| DetourPolyPath {
        poly_refs: refs,
        point_path: wow_recastdetour::DetourPointPath {
            actual_end: *points.last().expect("points"),
            points,
            path_type: DetourPathType::NORMAL,
        },
        start_far_from_poly: false,
        end_far_from_poly: false,
    };

    // First victim: the query produces a corridor the generator retains.
    creature.enter_combat(first);
    let mut first_query = None;
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(first, 60.0),
        true,
        None,
        |query| {
            first_query = Some(query.previous_poly_refs.clone());
            Some(corridor(
                vec![[10.0, 10.0, 0.0], [35.0, 10.0, 0.0], [59.0, 10.0, 0.0]],
                vec![101, 102, 103],
            ))
        },
    );
    assert_eq!(
        first_query.as_deref(),
        Some(&[][..]),
        "a brand new generator starts with no corridor"
    );
    assert_eq!(
        creature.active_chase_path_poly_refs_like_cpp(),
        &[101, 102, 103],
        "the corridor must be retained for the same victim"
    );

    // Switching victim must reset it before the next query is built.
    creature.enter_combat(second);
    let mut second_query = None;
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        target(second, 65.0),
        true,
        None,
        |query| {
            second_query = Some(query.previous_poly_refs.clone());
            None
        },
    );
    assert_eq!(
        second_query.as_deref(),
        Some(&[][..]),
        "the first query for a new victim must not carry the previous corridor"
    );
}
/// When the victim leaves the world, C++ chase `Update` returns false and
/// `MotionMaster` finalizes the generator (`ChaseMovementGenerator.cpp:101-103`,
/// `:251-260`). The runtime must drop the chase generator and clear
/// `UNIT_STATE_CHASE_MOVE`, not merely clear the corridor, or the creature is
/// re-selected as chasing and keeps driving toward the corpse every tick.
#[test]
fn world_creature_chase_finalizes_when_the_victim_leaves_the_world_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54359);
    let victim = ObjectGuid::create_world_object(HighGuid::Player, 0, 1, 0, 0, 0, 97);
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
    creature.enter_combat(victim);

    // A live victim far enough to trigger a launch installs the generator and
    // marks the creature chase-moving.
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        ChaseTargetSnapshotLikeCpp {
            guid: victim,
            position: Position::new(60.0, 10.0, 0.0, 0.0),
            combat_reach: 1.0,
            in_world: true,
            in_water: Some(false),
        },
        false,
        None,
        |_| None,
    );
    assert!(creature.active_chase_generator_like_cpp().is_some());
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits())
    );

    // The victim leaves the world: the snapshot reports `in_world: false`.
    let _ = creature.update_runtime_chase_movement_like_cpp(
        100,
        ChaseTargetSnapshotLikeCpp {
            guid: victim,
            position: Position::new(60.0, 10.0, 0.0, 0.0),
            combat_reach: 1.0,
            in_world: false,
            in_water: Some(false),
        },
        false,
        None,
        |_| None,
    );
    assert!(
        creature.active_chase_generator_like_cpp().is_none(),
        "the chase generator must be retired when the victim leaves the world"
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(wow_constants::UnitState::CHASE_MOVE.bits()),
        "UNIT_STATE_CHASE_MOVE must be cleared on finalize"
    );
    assert!(creature.active_chase_path_poly_refs_like_cpp().is_empty());
}
/// C++ `HomeMovementGenerator::SetTargetLocation` sets
/// `MOVEMENTGENERATOR_FLAG_INTERRUPTED` and returns without launching while
/// ROOT/STUNNED/DISTRACTED (`HomeMovementGenerator.cpp:53-58`); only the next
/// `DoUpdate` sets `INFORM_ENABLED` and finalizes (`:117-122`). Finalizing in
/// the initialize frame would skip `INFORM_ENABLED`, suppressing
/// `JustReachedHome` and clearing evade a frame early.
#[test]
fn world_creature_home_interrupted_survives_one_frame_before_finalize_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54360);
    let mut creature = WorldCreature::new(
        guid,
        1,
        Position::new(40.0, 10.0, 0.0, 0.0),
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
        .set_ai_home_position(Position::new(10.0, 10.0, 0.0, 0.0));
    // The creature is returning (what `reset_combat` leaves behind), but
    // rooted, so C++ `SetTargetLocation` interrupts instead of launching.
    creature.creature.set_ai_state(CreatureAiState::Returning);
    creature
        .creature
        .unit_mut()
        .add_unit_state(wow_constants::UnitState::ROOT.bits());

    // Initialize frame: interrupted, but the generator must stay installed
    // and evade must be held, and the reached-home callback must NOT fire.
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        creature.creature.is_in_evade_mode_like_cpp(),
        "evade must still be held while interrupted"
    );
    assert!(
        !creature.creature.take_ai_just_reached_home(),
        "JustReachedHome must not fire in the interrupt frame"
    );
    assert_eq!(
        creature.state(),
        CreatureAiState::Returning,
        "the interrupted home generator stays selected for one more tick"
    );

    // Next frame: the update path sees INTERRUPTED, sets INFORM_ENABLED and
    // finalizes, clearing evade and firing JustReachedHome.
    let outcome = creature.update_runtime_home_movement_like_cpp(false, None, |_| None);
    assert_eq!(outcome, ChaseTickOutcomeLikeCpp::Idle);
    assert!(
        !creature.creature.is_in_evade_mode_like_cpp(),
        "DoFinalize clears UNIT_STATE_EVADE on the update frame"
    );
    assert!(
        creature.creature.take_ai_just_reached_home(),
        "JustReachedHome fires on the finalize frame, even for an interrupted return"
    );
    assert_eq!(creature.state(), CreatureAiState::Idle);
}
#[test]
fn world_creature_random_launches_cpp_shortcut_when_navmesh_is_absent() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54341);
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
    creature.seed_runtime_rng_like_cpp(0x5434_1);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 8.0;
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));

    // C++ `CalculatePath` answers a missing navmesh/tile with
    // `BuildShortcut()` + `PATHFIND_NORMAL | PATHFIND_NOT_USING_PATH`
    // (`PathGenerator.cpp:79-86`). Neither bit is checked by
    // `RandomMovementGenerator::SetRandomLocation`
    // (`RandomMovementGenerator.cpp:146-153`), so the creature launches the
    // two-point path instead of retrying forever.
    let mut resolver_called = false;
    let movement =
        creature.update_default_random_movement_with_path_resolver_like_cpp(10, true, |query| {
            resolver_called = true;
            Some(detour_path_without_navmesh_like_cpp(
                query.start,
                query.destination,
            ))
        });

    assert!(resolver_called);
    let (from, spline) =
        movement.expect("C++ launches the no-navmesh shortcut instead of standing still");
    assert!(creature.active_move_spline_like_cpp().is_some());
    assert_eq!(
        creature.state(),
        wow_entities::CreatureAiState::WalkingRandom
    );

    // The launched spline is the C++ `BuildShortcut()` segment: straight
    // from the creature's position to the rolled wander destination, with
    // no navmesh waypoints in between.
    let destination = spline
        .final_destination()
        .expect("the launched shortcut has a destination");
    let shortcut = detour_path_without_navmesh_like_cpp(from, destination);
    let path = path_generator_from_detour_like_cpp(from, destination, &shortcut, false);
    assert_eq!(
        path.path_points(),
        &[from, destination],
        "the C++ no-navmesh case is a two-point shortcut"
    );
    assert_eq!(
        path.path_type(),
        PathType::NORMAL | PathType::NOT_USING_PATH
    );
}
#[test]
fn world_creature_path_query_filter_context_follows_cpp_create_filter() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54342);
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

    // C++ `CreatureMovementData` defaults to `Ground = Run` and
    // `Swim = true` (`Creature.cpp:58`), so `CanWalk()` and
    // `CanEnterWater()` both hold and `CreateFilter` includes
    // NAV_GROUND | NAV_WATER | NAV_MAGMA_SLIME.
    let context = creature.path_query_filter_context_like_cpp();
    assert_eq!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            can_walk: true,
            can_enter_water: true,
            in_combat: false,
            in_evade_mode: false,
        }
    );
    let filter = create_path_query_filter_like_cpp(context).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND
            | wow_recastdetour::NavTerrainFlag::WATER
            | wow_recastdetour::NavTerrainFlag::MAGMA_SLIME)
            .bits()
    );

    // A ground-only, non-swimming template must lose the water bits, which
    // the previously hardcoded context could never express.
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::None as u8,
    );
    creature.creature.set_swim_allowed_runtime_like_cpp(false);
    let context = creature.path_query_filter_context_like_cpp();
    assert_eq!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            can_walk: false,
            can_enter_water: false,
            in_combat: false,
            in_evade_mode: false,
        }
    );

    // C++ `UpdateFilter` adds NAV_GROUND_STEEP while the creature
    // `IsInCombat()` or `IsInEvadeMode()` (`PathGenerator.cpp:694-696`).
    creature.creature.set_ground_movement_type_runtime_like_cpp(
        wow_constants::CreatureGroundMovementType::Run as u8,
    );
    creature.creature.set_in_evade_mode_like_cpp(true);
    let context = creature.path_query_filter_context_like_cpp();
    assert!(matches!(
        context.owner,
        wow_recastdetour::PathQueryFilterOwner::Creature {
            in_evade_mode: true,
            ..
        }
    ));
    let filter = create_path_query_filter_like_cpp(context).expect("filter");
    assert_eq!(
        filter.include_flags(),
        (wow_recastdetour::NavTerrainFlag::GROUND | wow_recastdetour::NavTerrainFlag::GROUND_STEEP)
            .bits()
    );
}
#[test]
fn calculate_creature_detour_path_returns_none_until_runtime_mmap_exists_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54325);
    let creature = WorldCreature::new(
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
    let dst = Position::new(20.0, 10.0, 0.0, 0.0);
    let filter_context = PathQueryFilterContext::creature(true, false, false, false);

    assert_eq!(
        calculate_creature_detour_path_like_cpp(&creature, dst, None, 0, 0, filter_context, false),
        Ok(None)
    );

    let mmap_data = MMapData::new(wow_recastdetour::DetourNavMeshParams {
        origin: [0.0, 0.0, 0.0],
        tile_width: 533.3333,
        tile_height: 533.3333,
        max_tiles: 16,
        max_polys: 16,
    })
    .expect("navmesh allocation");
    assert_eq!(
        calculate_creature_detour_path_like_cpp(
            &creature,
            dst,
            Some(&mmap_data),
            0,
            0,
            filter_context,
            false,
        ),
        Ok(None)
    );
}
