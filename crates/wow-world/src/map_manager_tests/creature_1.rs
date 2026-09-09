//! Creature scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn only_loaded_grid_creature_bridge_completes_spell_aura_authorities_like_cpp() {
    let generic = test_creature(ObjectGuid::new(0, 90_001));
    assert!(
        !generic
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        !generic
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );

    let mut previously_authorized = generic.creature.clone();
    previously_authorized
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_hit_aura_authority_inert_like_cpp(true);
    previously_authorized
        .unit_mut()
        .subsystems_mut()
        .auras
        .set_spell_cast_log_aura_authority_inert_like_cpp(true);
    let generic_bridge =
        WorldCreature::from_canonical(previously_authorized, generic.create_data.clone());
    assert!(
        !generic_bridge
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        !generic_bridge
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );

    let canonical = test_creature(ObjectGuid::new(0, 90_002)).creature;
    let loaded_grid = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);
    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()
    );
    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );
}
#[test]
fn world_creature_motion_master_chase_interrupts_random_and_resumes_default_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_009);
    let target = ObjectGuid::create_player(1, 7009);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);

    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Random)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 1);

    creature.enter_combat(target);
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Chase),
        "C++ MotionMaster::Add places normal-priority active chase above the default random generator"
    );
    let represented_chase = creature
        .creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(represented_chase.kind, MovementGeneratorKind::Chase);
    assert_eq!(represented_chase.target_guid, Some(target));
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Chase)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 2);

    creature.reset_combat();
    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Random),
        "removing active chase must expose and reset the deactivated default generator"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Random)
    );
    assert_eq!(creature.runtime_motion_master_ticks_like_cpp(), 3);
}
#[test]
fn world_creature_motion_master_preserves_high_priority_point_above_chase_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_011);
    let target = ObjectGuid::create_player(1, 7011);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);
    creature
        .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("launch point spline");
    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .move_charge(42);

    creature.enter_combat(target);

    assert_eq!(
        creature.runtime_motion_master_current_kind_like_cpp(),
        Some(RuntimeMovementGeneratorType::Point),
        "C++ keeps highest-priority charge/point above normal-priority chase"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Point)
    );
    assert!(
        creature.active_move_spline_like_cpp().is_some(),
        "selecting chase must not stop the higher-priority point spline"
    );

    creature.finish_move();
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(50),
        Some(RuntimeMovementGeneratorType::Chase),
        "finishing the higher-priority point spline pops its represented generator and exposes chase"
    );
}
#[test]
fn world_creature_motion_master_expires_finite_distract_and_exposes_chase_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_012);
    let target = ObjectGuid::create_player(1, 7012);
    let mut creature = test_creature(guid);
    creature
        .begin_distract_movement_like_cpp(10, 1.25)
        .expect("launch finite distract");
    creature.enter_combat(target);

    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(10),
        Some(RuntimeMovementGeneratorType::Distract),
        "C++ Distract remains selected while its timer has not expired"
    );
    assert_eq!(
        creature.tick_runtime_motion_master_like_cpp(1),
        Some(RuntimeMovementGeneratorType::Chase),
        "the represented finite generator must pop and remove its runtime proxy"
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase
    );
}
#[test]
fn world_creature_death_and_loot_keep_game_time_and_monotonic_deadlines_separate_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70_010);
    let mut creature = test_creature(guid);
    creature.creature.set_respawn_compatibility_mode(false);
    creature.creature.set_corpse_delay(60, false);

    let death_game_time_secs = 1_700_000_000;
    assert!(
        creature
            .creature
            .apply_ai_damage_before_death_state_at_game_time_like_cpp(50, 0, death_game_time_secs,)
    );
    creature.complete_death_state_after_kill_hooks_at_game_time_like_cpp(death_game_time_secs);

    let completion_ms = creature
        .creature
        .ai_ownership()
        .death_time_ms
        .expect("death completion must record the monotonic mirror");
    assert_eq!(completion_ms, 0);
    let completion_now = Instant::now();
    assert_eq!(
        creature.creature.corpse_remove_time(),
        death_game_time_secs + 60
    );
    assert_eq!(creature.creature.respawn_time(), death_game_time_secs + 30);
    assert_eq!(
        creature.respawn_at_from_death_at_game_time_like_cpp(completion_now, death_game_time_secs,),
        completion_now + Duration::from_secs(30)
    );

    let loot_now = completion_now + Duration::from_secs(3);
    let loot_game_time_secs = death_game_time_secs + 3;
    assert!(creature.all_loot_removed_from_corpse_at_game_time_like_cpp(
        loot_now,
        loot_game_time_secs,
        0.5,
        false,
    ));
    assert_eq!(
        creature.creature.corpse_remove_time(),
        loot_game_time_secs + 30
    );
    assert_eq!(creature.creature.respawn_time(), loot_game_time_secs + 60);
    assert_eq!(
        creature
            .corpse_despawn_deadline_ms_like_cpp()
            .map(|deadline| deadline.saturating_sub(creature.runtime_elapsed_ms_like_cpp())),
        Some(30_000)
    );
    assert_eq!(
        creature.respawn_at_from_death_at_game_time_like_cpp(loot_now, loot_game_time_secs,),
        loot_now + Duration::from_secs(60)
    );
}
#[test]
fn world_creature_runtime_rng_replaces_timer_seeded_damage_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70001);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(0xA141_BEEF);

    let rolls: Vec<u32> = (0..16)
        .map(|_| creature.roll_damage().expect("authoritative damage roll"))
        .collect();

    assert!(rolls.iter().all(|roll| (5..=10).contains(roll)));
    assert!(
        rolls.iter().any(|roll| *roll != rolls[0]),
        "damage rolls should come from owned RNG, not now_ms/spline_id: {rolls:?}"
    );
}
#[test]
fn creature_spell_hit_roll_consumes_owned_runtime_rng_sequence_like_cpp() {
    let seed = 0x5E11_117_u64;
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70002);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(seed);
    let mut expected_rng = StdRng::seed_from_u64(seed);

    let actual: Vec<u32> = (0..16)
        .map(|_| {
            creature
                .random_creature_spell_hit_roll_like_cpp()
                .expect("authoritative creature-spell hit roll")
        })
        .collect();
    let expected: Vec<u32> = (0..16).map(|_| expected_rng.gen_range(0..=9_999)).collect();

    assert_eq!(actual, expected);
}
#[test]
fn world_creature_spell_rng_tombstone_preserves_legacy_melee_and_movement() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70007);
    let mut creature = test_creature(guid);
    creature.seed_runtime_rng_like_cpp(0x7007);
    assert!(creature.runtime_rng_authority_complete_like_cpp());
    assert!(creature.random_creature_spell_hit_roll_like_cpp().is_some());

    creature.invalidate_runtime_rng_authority_like_cpp();
    creature.reset_creature_spell_schedule_like_cpp();
    creature.seed_runtime_rng_like_cpp(0x7008);

    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert_eq!(creature.random_creature_spell_hit_roll_like_cpp(), None);
    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 10_000),
        None
    );
    assert_eq!(
        creature.random_creature_spell_delay_like_cpp(5_000, 5_000),
        None,
        "even equal C++ bounds consume the permanently lost RNG stream"
    );
    assert!(creature.roll_damage().is_some());
    assert!(creature.pick_wander_destination().is_some());
    assert!(
        creature
            .pick_random_destination_from_current_position_like_cpp(12.0)
            .is_some()
    );
    assert!(creature.reset_wander_timer());
    creature.creature.ai_ownership_mut().wander_steps_remaining = 0;
    assert!(creature.record_random_movement_launch_like_cpp());
    assert!(creature.schedule_after_random_movement_like_cpp());
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    assert!(creature.initialize_default_random_movement_like_cpp());

    let cloned = creature.clone();
    assert!(!cloned.runtime_rng_authority_complete_like_cpp());
}
#[test]
fn world_creature_random_movement_walk_rule_matches_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70003);
    let mut creature = test_creature(guid);

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::Walk as u8,
    );
    assert!(creature.random_movement_walk_like_cpp());

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
    );
    assert!(!creature.random_movement_walk_like_cpp());

    creature.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::CanRun as u8,
    );
    creature
        .creature
        .set_movement_flags_runtime_like_cpp(MovementFlag::NONE);
    assert!(!creature.random_movement_walk_like_cpp());
    creature
        .creature
        .set_movement_flags_runtime_like_cpp(MovementFlag::WALKING);
    assert!(creature.random_movement_walk_like_cpp());
}
#[test]
fn world_creature_random_spline_uses_walk_or_run_speed_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70004);
    let mut walker = test_creature(guid);
    walker.create_data.speed_walk_rate = 1.0;
    walker.create_data.speed_run_rate = 1.0;
    walker.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::Walk as u8,
    );
    let (_, walk_spline) = walker
        .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("walk random spline");

    let mut runner = test_creature(guid);
    runner.create_data.speed_walk_rate = 1.0;
    runner.create_data.speed_run_rate = 1.0;
    runner.creature.set_random_movement_type_runtime_like_cpp(
        wow_constants::CreatureRandomMovementType::AlwaysRun as u8,
    );
    let (_, run_spline) = runner
        .begin_random_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
        .expect("run random spline");

    assert!((walk_spline.duration_ms() - 4_000).abs() <= 1);
    assert!((run_spline.duration_ms() - 1_429).abs() <= 1);
    assert!(
        run_spline.duration_ms() < walk_spline.duration_ms(),
        "C++ RandomMovementGenerator SetWalk(false) uses run speed"
    );
}
#[test]
fn world_creature_default_random_initializes_generator_without_spline_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    creature.seed_runtime_rng_like_cpp(0x7005);

    assert!(creature.initialize_default_random_movement_like_cpp());

    assert_eq!(creature.move_target(), None);
    assert!(creature.active_move_spline_like_cpp().is_none());
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    assert_eq!(creature.state(), wow_entities::CreatureAiState::Idle);
    assert!(
        (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
        "C++ RandomMovementGenerator::DoInitialize seeds 2..10 steps but SetRandomLocation consumes the first step later"
    );
    assert_eq!(
        creature.creature.ai_ownership().wander_delay_ms,
        0,
        "C++ RandomMovementGenerator::DoInitialize resets its timer to 0 so the next update can choose a path"
    );
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Random
    );
}
#[test]
fn world_creature_random_wander_steps_pause_only_after_step_batch_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70006);
    let mut creature = test_creature(guid);
    creature
        .creature
        .set_default_movement_type_runtime_like_cpp(wow_entities::MovementGeneratorType::Random);
    creature.creature.ai_ownership_mut().wander_steps_remaining = 2;

    assert!(creature.record_random_movement_launch_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 1);
    assert!(creature.schedule_after_random_movement_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_delay_ms, 0);

    assert!(creature.record_random_movement_launch_like_cpp());
    assert_eq!(creature.creature.ai_ownership().wander_steps_remaining, 0);
    assert!(creature.schedule_after_random_movement_like_cpp());
    assert!(
        (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
        "C++ RandomMovementGenerator pauses 4..10 seconds only after its wander step batch"
    );
    assert!(
        (2..=10).contains(&creature.creature.ai_ownership().wander_steps_remaining),
        "C++ RandomMovementGenerator reseeds 2..10 wander steps after a pause"
    );
}
#[test]
fn world_creature_interaction_pause_stops_and_updates_home_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70005);
    let mut creature = test_creature(guid);
    let current = Position::new(14.0, 15.0, 16.0, 1.5);
    creature.creature.unit_mut().world_mut().relocate(current);
    creature
        .creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .start_spline(42, 1_000);

    assert!(creature.pause_interaction_movement_like_cpp());

    let motion = &creature.creature.unit().subsystems().motion;
    assert!(motion.paused);
    assert!(motion.stopped);
    assert!(!motion.spline.enabled);
    assert_eq!(creature.home_position(), current);

    creature
        .creature
        .set_interaction_pause_timer_ms_runtime_like_cpp(0);
    assert!(!creature.pause_interaction_movement_like_cpp());
}
#[test]
fn world_creature_wander_rng_matches_cpp_random_movement_bounds() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70002);
    let mut creature = test_creature(guid);
    creature.creature.ai_ownership_mut().wander_radius = 12.0;
    creature.seed_runtime_rng_like_cpp(0x5757);

    for _ in 0..24 {
        let dst = creature
            .pick_wander_destination()
            .expect("authoritative wander destination");
        let dist = creature.home_position().distance(&dst);
        assert!(
            dist <= creature.creature.ai_ownership().wander_radius + f32::EPSILON,
            "wander destination {dst:?} was {dist} yd from home"
        );
    }

    for _ in 0..24 {
        assert!(creature.reset_wander_timer());
        assert!(
            (4_000..=10_000).contains(&creature.creature.ai_ownership().wander_delay_ms),
            "C++ RandomMovementGenerator pauses with urand(4, 10) seconds"
        );
    }
}
#[test]
fn test_creature_add_remove() {
    let mut grid = Grid::new(0, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );

    assert!(grid.add_creature(creature.clone()));
    assert_eq!(grid.creature_count(), 1);
    assert!(grid.get_creature(guid).is_some());

    assert!(grid.remove_creature(guid));
    assert_eq!(grid.creature_count(), 0);
    assert!(grid.get_creature(guid).is_none());
}
#[test]
fn test_duplicate_creature_rejected() {
    let mut grid = Grid::new(0, 0);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );

    assert!(grid.add_creature(creature.clone()));
    assert!(!grid.add_creature(creature)); // Duplicate should fail
}
#[test]
fn test_add_creature_to_map() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let creature = WorldCreature::new(
        guid,
        1,
        Position::new(10.0, 10.0, 0.0, 0.0),
        50,
        1,
        5,
        10,
        20.0,
        0,
        35,
        0,
        0,
    );

    assert!(manager.add_creature(0, 0, 0, 0, creature));
    assert!(manager.get_creature(0, 0, 0, 0, guid).is_some());
}
#[test]
fn map_manager_uses_canonical_creature_guid_position_and_runtime() {
    let mut manager = MapManager::new();
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
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

    assert!(manager.add_creature(0, 0, 0, 0, creature));
    let stored = manager
        .find_creature(0, 0, guid)
        .expect("canonical creature stored");
    assert_eq!(stored.guid(), guid);
    assert_eq!(stored.position(), Position::new(10.0, 10.0, 0.0, 0.0));
    assert_eq!(stored.current_hp(), 50);

    manager
        .find_creature_mut(0, 0, guid)
        .expect("canonical creature mutable")
        .take_damage(25);
    let stored = manager
        .find_creature(0, 0, guid)
        .expect("canonical creature stored");
    assert_eq!(stored.current_hp(), 25);
    assert_eq!(stored.creature.unit().data().health, 25);
}
#[test]
fn world_creature_move_spline_bridge_advances_and_finalizes_like_cpp_unit_tick() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54321);
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
        .unit_mut()
        .world_mut()
        .set_map(0, 0)
        .expect("bind test creature to map");
    creature.backdate_runtime_clock_for_test(Duration::from_secs(10));
    let dst = Position::new(15.0, 10.0, 0.0, 0.0);

    let (from, spline) = creature
        .begin_move_spline_like_cpp(dst)
        .expect("valid two-point spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.spline_id(), 2);
    assert!(
        creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ MoveSplineInit::Launch writes MOVEMENTFLAG_FORWARD to Unit::m_movementInfo"
    );
    assert!(
        MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD),
        "the create bridge must mirror Unit::m_movementInfo after Launch"
    );
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(motion_spline.enabled);
    assert!(!motion_spline.finalized);
    assert_eq!(motion_spline.spline_id, spline.id());
    assert_eq!(motion_spline.duration_ms, spline.duration_ms() as u32);
    assert_eq!(motion_spline.final_destination, Some((15, 10, 0)));

    let duration_ms = spline.duration_ms() as u32;
    let now_ms = creature.runtime_elapsed_ms_like_cpp();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms / 2));
    assert!(!creature.update_move_spline_like_cpp());
    let mid = creature.position();
    assert!(mid.x > 10.0 && mid.x < 15.0, "mid position was {mid:?}");
    assert_eq!(
        creature
            .creature
            .unit()
            .subsystems()
            .motion
            .spline
            .progress_ms,
        duration_ms / 2
    );

    let now_ms = creature.runtime_elapsed_ms_like_cpp();
    creature.creature.ai_ownership_mut().move_start_ms =
        now_ms.saturating_sub(u64::from(duration_ms));
    assert!(creature.update_move_spline_like_cpp());
    assert!(creature.active_move_spline.is_none());
    assert_eq!(creature.position(), dst);
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(!motion_spline.enabled);
    assert!(motion_spline.finalized);
    assert_eq!(motion_spline.progress_ms, motion_spline.duration_ms);
    assert!(
        !creature
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::FORWARD),
        "C++ Unit::DisableSpline removes MOVEMENTFLAG_FORWARD on arrival"
    );
    assert!(
        !MovementFlag::from_bits_retain(creature.create_data.movement_flags)
            .contains(MovementFlag::FORWARD),
        "the create bridge must mirror Unit::m_movementInfo after DisableSpline"
    );
    assert!(
        !creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
}
#[test]
fn world_creature_move_spline_by_path_uses_cpp_moveby_path_bridge() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54322);
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
    let path = [
        Position::new(10.0, 10.0, 0.0, 0.0),
        Position::new(12.0, 11.0, 0.0, 0.0),
        Position::new(15.0, 12.0, 0.0, 0.0),
    ];

    let (from, spline) = creature
        .begin_move_spline_by_path_like_cpp(path)
        .expect("valid multi-point path spline");

    assert_eq!(from, Position::new(10.0, 10.0, 0.0, 0.0));
    assert!(creature.active_move_spline.is_some());
    assert_eq!(creature.spline_id(), 2);
    assert_eq!(creature.move_target(), Some(path[2]));
    assert_eq!(spline.final_destination(), Some(path[2]));
    assert_eq!(spline.monster_move_path_data().points, vec![path[2]]);
    assert_eq!(spline.monster_move_path_data().packed_deltas.len(), 1);
    assert!(
        creature
            .creature
            .unit()
            .has_unit_state(UnitState::ROAMING_MOVE.bits())
    );
    let motion_spline = &creature.creature.unit().subsystems().motion.spline;
    assert!(motion_spline.enabled);
    assert_eq!(motion_spline.spline_id, spline.id());
    assert_eq!(motion_spline.final_destination, Some((15, 12, 0)));
}
#[test]
fn world_creature_waypoint_default_initialize_stores_generator_and_stops_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54329);
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

    let action = creature.initialize_default_waypoint_movement_like_cpp(Some(path));

    assert_eq!(action, WaypointMovementAction::StopMoving);
    assert!(creature.creature.unit().subsystems().motion.stopped);
    let generator = creature
        .active_waypoint_generator_like_cpp()
        .expect("waypoint generator stored");
    assert_eq!(
        generator.next_move_time_ms(),
        wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP
    );
    assert_eq!(generator.stop_moving_calls, 1);
}
#[test]
fn world_creature_waypoint_default_initialize_missing_path_does_not_stop_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 54330);
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

    let action = creature.initialize_default_waypoint_movement_like_cpp(None);

    assert_eq!(action, WaypointMovementAction::MissingPath);
    assert!(!creature.creature.unit().subsystems().motion.stopped);
    assert!(creature.active_waypoint_generator_like_cpp().is_some());
}
#[test]
fn world_creature_waypoint_default_initialize_resolves_owner_path_like_cpp() {
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
    creature.creature.load_path_like_cpp(90_001);
    let path = WaypointPath::new(
        90_001,
        vec![wow_movement::WaypointNode::new(10, 11.0, 10.0, 0.0)],
    );

    let action =
        creature.initialize_default_waypoint_movement_with_path_resolver_like_cpp(|path_id| {
            (path_id == path.id).then_some(path.clone())
        });

    assert_eq!(action, WaypointMovementAction::StopMoving);
    assert!(
        creature.creature.unit().subsystems().motion.stopped,
        "C++ DoInitialize calls owner->StopMoving() after sWaypointMgr resolves the path"
    );
    assert_eq!(
        creature
            .active_waypoint_generator_like_cpp()
            .map(WaypointMovementGenerator::next_move_time_ms),
        Some(wow_movement::WAYPOINT_INITIAL_DELAY_MS_LIKE_CPP)
    );
}
