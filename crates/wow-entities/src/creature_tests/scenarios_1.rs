//! Creature regressions, part 1 of 4.
//!
//! Moved out of the creature_tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn creature_search_formation_like_cpp_requests_only_with_spawn_and_info() {
    let mut creature = Creature::new(false);
    creature.set_spawn_id(1234);
    creature.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

    let outcome = creature.search_formation_like_cpp();

    assert_eq!(outcome.spawn_id, 1234);
    assert!(!outcome.is_summon);
    assert!(outcome.formation_info_found);
    assert_eq!(outcome.leader_spawn_id, Some(77));
    assert!(outcome.add_to_group_requested);
}

#[test]
fn creature_search_formation_like_cpp_skips_summon_and_zero_spawn() {
    let mut summon = Creature::new(false);
    summon.set_spawn_id(1234);
    summon.set_summon_like_cpp(true);
    summon.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

    let summon_outcome = summon.search_formation_like_cpp();
    assert!(summon_outcome.is_summon);
    assert!(summon_outcome.formation_info_found);
    assert_eq!(summon_outcome.leader_spawn_id, None);
    assert!(!summon_outcome.add_to_group_requested);

    let mut zero_spawn = Creature::new(false);
    zero_spawn.set_formation_info_like_cpp(Some(formation_info_like_cpp(77)));

    let zero_spawn_outcome = zero_spawn.search_formation_like_cpp();
    assert_eq!(zero_spawn_outcome.spawn_id, 0);
    assert!(!zero_spawn_outcome.is_summon);
    assert!(zero_spawn_outcome.formation_info_found);
    assert_eq!(zero_spawn_outcome.leader_spawn_id, None);
    assert!(!zero_spawn_outcome.add_to_group_requested);
}

#[test]
fn creature_search_formation_like_cpp_skips_missing_formation_info() {
    let mut creature = Creature::new(false);
    creature.set_spawn_id(1234);

    let outcome = creature.search_formation_like_cpp();

    assert_eq!(outcome.spawn_id, 1234);
    assert!(!outcome.is_summon);
    assert!(!outcome.formation_info_found);
    assert_eq!(outcome.leader_spawn_id, None);
    assert!(!outcome.add_to_group_requested);
}

#[test]
fn creature_constructor_matches_cpp_base_state() {
    let creature = Creature::new(false);

    assert_eq!(creature.unit().world().object().type_id(), TypeId::Unit);
    assert_eq!(
        creature.unit().world().object().type_mask(),
        TypeMask::OBJECT | TypeMask::UNIT
    );
    assert!(!creature.unit().world().is_world_object());
    assert_eq!(creature.player_damage_req(), 0);
    assert_eq!(creature.corpse_remove_time(), 0);
    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.respawn_delay(), DEFAULT_RESPAWN_DELAY_SECS);
    assert_eq!(creature.corpse_delay(), DEFAULT_CORPSE_DELAY_SECS);
    assert!(!creature.ignore_corpse_decay_ratio());
    assert_eq!(creature.wander_distance(), 0.0);
    assert_eq!(
        creature.boundary_check_time(),
        DEFAULT_BOUNDARY_CHECK_TIME_MS
    );
    assert_eq!(creature.combat_pulse_time(), 0);
    assert_eq!(creature.combat_pulse_delay(), 0);
    assert_eq!(creature.react_state(), ReactState::Aggressive);
    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Idle
    );
    assert_eq!(creature.waypoint_path_id_like_cpp(), 0);
    assert_eq!(creature.spawn_id(), 0);
    assert_eq!(creature.equipment_id(), 0);
    assert_eq!(creature.original_equipment_id(), 0);
    assert!(!creature.already_call_assistance());
    assert!(!creature.already_searched_assistance());
    assert!(!creature.cannot_reach_target());
    assert_eq!(creature.cannot_reach_timer(), 0);
    assert_eq!(creature.melee_damage_school_mask(), 0x1);
    assert_eq!(creature.original_entry(), 0);
    assert!(creature.trigger_just_appeared());
    assert!(!creature.respawn_compatibility_mode());
    assert_eq!(creature.last_damaged_time(), 0);
    assert!(creature.regenerate_health());
    assert!(!creature.is_missing_can_swim_flag_out_of_combat());
    assert_eq!(creature.gossip_menu_id(), 0);
    assert_eq!(creature.sparring_health_pct(), 0.0);
    assert_eq!(creature.regen_timer(), CREATURE_REGEN_INTERVAL_MS);
    assert_eq!(creature.spells(), [0; MAX_CREATURE_SPELLS]);
    assert!(!creature.disable_reputation_gain());
    assert_eq!(creature.sight_distance(), DEFAULT_MONSTER_SIGHT_DISTANCE);
    assert_eq!(creature.combat_distance(), 0.0);
    assert_eq!(creature.loot_mode(), LOOT_MODE_DEFAULT);
    assert!(!creature.is_temp_world_object());
    assert_eq!(creature.cleanup_before_delete_count(), 0);
    assert!(!creature.grid_unload_delete_requested());
    assert!(!creature.grid_unload_respawn_relocation_requested());
    assert_eq!(creature.ai_ownership().loot_id, 0);
    assert_eq!(creature.ai_ownership().gold_min, 0);
    assert_eq!(creature.ai_ownership().gold_max, 0);
    assert_eq!(creature.ai_ownership().boss_id, None);
    assert_eq!(creature.ai_ownership().dungeon_encounter_id, 0);
    assert_eq!(creature.ai_ownership().terrain_swap_map, -1);
    assert_eq!(creature.ai_ownership().last_movement_inform, None);
}

#[test]
fn creature_sparring_damage_clamps_at_configured_health_pct_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(52);
    creature.set_sparring_health_pct_like_cpp(50.0);

    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(true, false, 5),
        2,
        "C++ prevents creature-vs-creature sparring damage from crossing the threshold"
    );
}

#[test]
fn creature_sparring_damage_is_zero_and_fake_at_or_below_threshold_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(50);
    creature.set_sparring_health_pct_like_cpp(50.0);

    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(true, false, 5),
        0
    );
    assert!(creature.should_fake_damage_from_like_cpp(true, false));
}

#[test]
fn creature_sparring_ignores_non_creature_or_player_controlled_attackers_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(50);
    creature.set_sparring_health_pct_like_cpp(50.0);

    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(false, false, 5),
        5
    );
    assert!(!creature.should_fake_damage_from_like_cpp(false, false));
    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(true, true, 5),
        5
    );
    assert!(!creature.should_fake_damage_from_like_cpp(true, true));
}

#[test]
fn creature_sparring_ignores_player_owned_victims_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(50);
    creature.set_sparring_health_pct_like_cpp(50.0);
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(Some(ObjectGuid::create_player(1, 42)));

    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(true, false, 5),
        5
    );
    assert!(!creature.should_fake_damage_from_like_cpp(true, false));
}

#[test]
fn creature_sparring_damage_preserves_fractional_health_pct_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(1_000);
    creature.unit_mut().set_health(506);
    creature.set_sparring_health_pct_like_cpp(50.5);

    assert_eq!(
        creature.calculate_damage_for_sparring_like_cpp(true, false, 10),
        1,
        "C++ stores sparring pct as float; truncating to u8 would incorrectly allow 6 damage"
    );
    creature.unit_mut().set_health(505);
    assert!(creature.should_fake_damage_from_like_cpp(true, false));
}

#[test]
fn creature_ai_ownership_derives_identity_health_and_position() {
    let mut creature = Creature::new(false);
    let guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, 12345);
    let position = Position::new(1.0, 2.0, 3.0, 4.0);

    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(987);
    creature.unit_mut().world_mut().relocate(position);
    creature.unit_mut().set_level(22);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(35);
    creature.set_ai_home_position(position);

    assert_eq!(creature.ai_guid(), guid);
    assert_eq!(creature.ai_entry(), 987);
    assert_eq!(creature.ai_level(), 22);
    assert_eq!(creature.ai_current_health(), 35);
    assert_eq!(creature.ai_max_health(), 40);
    assert_eq!(creature.ai_position(), position);
    assert_eq!(creature.ai_home_position(), position);
}

#[test]
fn creature_ai_ownership_enter_and_reset_combat() {
    let mut creature = Creature::new(false);
    let home = Position::new(10.0, 20.0, 30.0, 1.0);
    let attacker = ObjectGuid::create_player(1, 7);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(35);
    creature.set_ai_home_position(home);

    creature.enter_ai_combat(attacker);
    assert!(!creature.take_ai_damage(1, 10));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
    assert_eq!(creature.ai_ownership().combat_target, Some(attacker));
    assert_eq!(creature.unit().attacking(), Some(attacker));
    assert!(creature.last_damaged_time() > 10);

    creature.reset_ai_combat(55);
    assert_eq!(creature.ai_state(), CreatureAiState::Returning);
    assert_eq!(creature.ai_ownership().combat_target, None);
    assert_eq!(creature.unit().attacking(), None);
    assert_eq!(
        creature.ai_current_health(),
        34,
        "C++ restores spawn health only when HomeMovementGenerator finalizes"
    );
    assert_eq!(creature.ai_ownership().move_target, Some(home));
    assert_eq!(creature.ai_ownership().move_start_ms, 55);
    assert_eq!(creature.last_damaged_time(), 0);
}

#[test]
fn creature_ai_ownership_damage_and_death_syncs_unit_state() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);
    creature.ai_ownership_mut().respawn_time_secs = 30;
    let game_time_secs = 1_700_000_000;

    assert_eq!(creature.current_health(), 40);
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);
    assert!(!creature.take_ai_damage_at_game_time_like_cpp(15, 10, game_time_secs,));
    assert_eq!(creature.current_health(), 25);

    assert!(creature.take_ai_damage_at_game_time_like_cpp(100, 20, game_time_secs,));
    assert_eq!(creature.current_health(), 0);
    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(creature.ai_state(), CreatureAiState::Dead);
    assert_eq!(creature.ai_ownership().death_time_ms, Some(20));
    assert_eq!(
        creature.corpse_remove_time(),
        game_time_secs + i64::from(DEFAULT_CORPSE_DELAY_SECS)
    );
    assert_eq!(creature.respawn_time(), game_time_secs + 30);
    assert!(creature.runtime_state().save_respawn_requested);
    assert!(!creature.should_ai_respawn(29_999));
    assert!(creature.should_ai_respawn(30_020));
}

#[test]
fn creature_ai_damage_records_aggro_reset_expiry_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);

    assert!(!creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(15, 10, 1_000));

    assert_eq!(
        creature.last_damaged_time(),
        1_000 + MAX_AGGRO_RESET_TIME_SECS_LIKE_CPP
    );
}

#[test]
fn creature_ai_lethal_damage_does_not_record_aggro_reset_expiry_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);

    assert!(creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(100, 10, 1_000));

    assert_eq!(creature.last_damaged_time(), 0);
    assert_eq!(
        creature.ai_ownership().corpse_despawn_at_ms,
        None,
        "C++ arms corpse removal only after kill hooks reach JUST_DIED"
    );
}

#[test]
fn creature_ai_damage_does_not_record_player_owned_aggro_reset_like_cpp() {
    let owner = ObjectGuid::create_player(1, 42);
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_owner_guid(Some(owner));

    assert!(!creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(15, 10, 1_000));

    assert_eq!(creature.last_damaged_time(), 0);
}

#[test]
fn creature_world_boss_uses_type_flags_and_excludes_summons_like_cpp() {
    let mut creature = Creature::new(false);
    creature.set_type_flags_runtime_like_cpp(CreatureTypeFlags::BOSS_MOB.bits());
    assert!(creature.is_world_boss_like_cpp());

    creature.set_summon_like_cpp(true);

    assert!(!creature.is_world_boss_like_cpp());
}

#[test]
fn creature_ai_lethal_damage_can_defer_death_state_until_kill_hooks_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);
    creature.ai_ownership_mut().respawn_time_secs = 30;
    let local_elapsed_ms = 20;
    let game_time_secs = 1_700_000_000;
    let completion_local_elapsed_ms = 3_020;
    let completion_game_time_secs = game_time_secs + 3;

    assert!(
        creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            100,
            local_elapsed_ms,
            game_time_secs,
        )
    );
    assert_eq!(creature.current_health(), 0);
    assert_eq!(creature.ai_state(), CreatureAiState::Dead);
    assert_eq!(
        creature.ai_ownership().death_time_ms,
        Some(local_elapsed_ms)
    );
    assert_eq!(creature.unit().death_state(), DeathState::Alive);
    assert_eq!(creature.corpse_remove_time(), 0);
    assert_eq!(creature.ai_ownership().corpse_despawn_at_ms, None);
    assert!(!creature.runtime_state().save_respawn_requested);

    creature.complete_ai_death_state_after_kill_hooks_like_cpp(
        completion_local_elapsed_ms,
        completion_game_time_secs,
    );
    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(
        creature.ai_ownership().death_time_ms,
        Some(completion_local_elapsed_ms)
    );
    assert_eq!(
        creature.ai_ownership().corpse_despawn_at_ms,
        Some(
            completion_local_elapsed_ms
                + u64::from(DEFAULT_CORPSE_DELAY_SECS).saturating_mul(1_000)
        )
    );
    assert_eq!(
        creature.corpse_remove_time(),
        completion_game_time_secs + i64::from(DEFAULT_CORPSE_DELAY_SECS)
    );
    assert_eq!(creature.respawn_time(), completion_game_time_secs + 30);
    assert!(creature.runtime_state().save_respawn_requested);
}

#[test]
fn creature_corpse_loot_flags_apply_after_death_state_like_cpp() {
    let mut creature = Creature::new(false);
    creature.unit_mut().set_max_health(40);
    creature.unit_mut().set_health(40);
    let game_time_secs = 1_700_000_000;
    creature.apply_ai_damage_before_death_state_at_game_time_like_cpp(100, 20, game_time_secs);
    creature.complete_ai_death_state_after_kill_hooks_like_cpp(20, game_time_secs);

    creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, true);

    assert!(
        creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::Lootable as u32)
    );
    assert!(
        creature
            .unit()
            .world()
            .object()
            .has_dynamic_flag(UnitDynFlags::CanSkin as u32)
    );
    assert!(
        creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE)
    );
}

#[test]
fn creature_ai_ownership_respawn_aggro_and_corpse_timer() {
    let mut creature = Creature::new(false);
    let home = Position::new(10.0, 20.0, 30.0, 1.0);
    let attacker = ObjectGuid::create_player(1, 7);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(80);
    creature.set_ai_home_position(home);
    creature.set_ai_position(Position::new(11.0, 20.0, 30.0, 1.0));
    creature.ai_ownership_mut().aggro_radius = 5.0;

    assert!(!creature.try_ai_aggro(attacker, &Position::new(30.0, 20.0, 30.0, 0.0)));
    assert!(creature.try_ai_aggro(attacker, &Position::new(12.0, 20.0, 30.0, 0.0)));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);

    let game_time_secs = 1_700_000_000;
    creature.mark_ai_dead_at_game_time_like_cpp(100, game_time_secs);
    assert_eq!(
        creature.corpse_remove_time(),
        game_time_secs + i64::from(DEFAULT_CORPSE_DELAY_SECS)
    );
    assert_eq!(
        creature.respawn_time(),
        game_time_secs + i64::from(DEFAULT_RESPAWN_DELAY_SECS)
    );
    creature.set_ai_corpse_despawn_at(Some(130));
    creature.set_last_damaged_time_like_cpp(1_010);
    assert_eq!(creature.ai_ownership().corpse_despawn_at_ms, Some(130));
    creature.respawn_ai(200);
    assert!(creature.is_alive());
    assert_eq!(creature.current_health(), 80);
    assert_eq!(creature.position(), home);
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);
    assert_eq!(creature.ai_ownership().combat_target, None);
    assert_eq!(creature.ai_ownership().corpse_despawn_at_ms, None);
    assert_eq!(creature.last_damaged_time(), 0);
}

#[test]
fn creature_try_ai_aggro_requires_aggressive_react_state_like_cpp() {
    let mut creature = Creature::new(false);
    let player = ObjectGuid::create_player(1, 7);
    let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
    let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(80);
    creature.set_ai_position(creature_pos);
    creature.ai_ownership_mut().aggro_radius = 5.0;

    // C++ `CreatureAI::MoveInLineOfSight` gates normal proximity aggro on
    // `HasReactState(REACT_AGGRESSIVE)` before `CanStartAttack`.
    creature.set_react_state(ReactState::Passive);
    assert!(!creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature.set_react_state(ReactState::Defensive);
    assert!(!creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature.set_react_state(ReactState::Aggressive);
    assert!(creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
}

#[test]
fn creature_try_ai_aggro_rejects_non_positive_radius_like_cpp() {
    let mut creature = Creature::new(false);
    let player = ObjectGuid::create_player(1, 7);
    let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(80);
    creature.set_ai_position(creature_pos);

    // Rust uses aggro_radius=0 for non-aggro neutral spawns (for example
    // faction 35). The legacy session path already rejected that before
    // calling into creature AI; the global map-owned path must keep the
    // same C++ CanStartAttack-style no-aggro gate.
    creature.ai_ownership_mut().aggro_radius = 0.0;
    assert!(!creature.try_ai_aggro(player, &creature_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature.ai_ownership_mut().aggro_radius = -1.0;
    assert!(!creature.try_ai_aggro(player, &creature_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature.ai_ownership_mut().aggro_radius = 1.0;
    assert!(creature.try_ai_aggro(player, &creature_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
}

#[test]
fn creature_try_ai_aggro_rejects_immune_to_pc_like_cpp() {
    let mut creature = Creature::new(false);
    let player = ObjectGuid::create_player(1, 7);
    let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
    let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(80);
    creature.set_ai_position(creature_pos);
    creature.ai_ownership_mut().aggro_radius = 5.0;

    // C++ `Creature::CanStartAttack` rejects `IsImmuneToPC()` when the
    // target has `UNIT_FLAG_PLAYER_CONTROLLED`; this helper only scans
    // player candidates, so the target side is implied here.
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_PC);
    assert!(!creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::IMMUNE_TO_NPC);
    assert!(creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
}

#[test]
fn creature_try_ai_aggro_rejects_excessive_z_distance_like_cpp() {
    let player = ObjectGuid::create_player(1, 7);
    let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);

    let mut rejected = Creature::new(false);
    rejected.unit_mut().set_max_health(80);
    rejected.unit_mut().set_health(80);
    rejected.unit_mut().set_combat_reach(1.0);
    rejected.set_ai_position(creature_pos);
    rejected.ai_ownership_mut().aggro_radius = 10.0;
    assert!(!rejected.try_ai_aggro_with_target_combat_reach_like_cpp(
        player,
        &Position::new(12.0, 20.0, 34.6, 0.0),
        0.5,
    ));
    assert_eq!(rejected.ai_state(), CreatureAiState::Idle);

    let mut accepted = Creature::new(false);
    accepted.unit_mut().set_max_health(80);
    accepted.unit_mut().set_health(80);
    accepted.unit_mut().set_combat_reach(1.0);
    accepted.set_ai_position(creature_pos);
    accepted.ai_ownership_mut().aggro_radius = 10.0;
    assert!(accepted.try_ai_aggro_with_target_combat_reach_like_cpp(
        player,
        &Position::new(12.0, 20.0, 34.5, 0.0),
        0.5,
    ));
    assert_eq!(accepted.ai_state(), CreatureAiState::InCombat);

    let mut combat_distance = Creature::new(false);
    combat_distance.unit_mut().set_max_health(80);
    combat_distance.unit_mut().set_health(80);
    combat_distance.unit_mut().set_combat_reach(1.0);
    combat_distance.set_combat_distance_like_cpp(1.0);
    combat_distance.set_ai_position(creature_pos);
    combat_distance.ai_ownership_mut().aggro_radius = 10.0;
    assert!(
        combat_distance.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 35.5, 0.0),
            0.5,
        )
    );
    assert_eq!(combat_distance.ai_state(), CreatureAiState::InCombat);

    let mut flying = Creature::new(false);
    flying.unit_mut().set_max_health(80);
    flying.unit_mut().set_health(80);
    flying.unit_mut().set_combat_reach(1.0);
    flying.set_ai_position(creature_pos);
    flying.ai_ownership_mut().aggro_radius = 100.0;
    flying.set_flight_movement_type_runtime_like_cpp(
        CreatureFlightMovementType::DisableGravity as u8,
    );
    assert!(
        flying.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 60.0, 0.0),
            0.5,
        ),
        "C++ Creature::CanFly bypasses the z-distance gate for Flight != None"
    );
    assert_eq!(flying.ai_state(), CreatureAiState::InCombat);

    let mut dynamic_disable_gravity = Creature::new(false);
    dynamic_disable_gravity.unit_mut().set_max_health(80);
    dynamic_disable_gravity.unit_mut().set_health(80);
    dynamic_disable_gravity.unit_mut().set_combat_reach(1.0);
    dynamic_disable_gravity.set_ai_position(creature_pos);
    dynamic_disable_gravity.ai_ownership_mut().aggro_radius = 100.0;
    dynamic_disable_gravity.set_movement_flags_runtime_like_cpp(MovementFlag::DISABLE_GRAVITY);
    assert!(dynamic_disable_gravity.is_flying_like_cpp());
    assert!(
        dynamic_disable_gravity.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 60.0, 0.0),
            0.5,
        ),
        "C++ Unit::IsFlying makes Creature::CanFly true for DISABLE_GRAVITY"
    );

    let mut dynamic_flying = Creature::new(false);
    dynamic_flying.unit_mut().set_max_health(80);
    dynamic_flying.unit_mut().set_health(80);
    dynamic_flying.unit_mut().set_combat_reach(1.0);
    dynamic_flying.set_ai_position(creature_pos);
    dynamic_flying.ai_ownership_mut().aggro_radius = 100.0;
    dynamic_flying.set_movement_flags_runtime_like_cpp(MovementFlag::FLYING);
    assert!(dynamic_flying.is_flying_like_cpp());
    assert!(
        dynamic_flying.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 60.0, 0.0),
            0.5,
        ),
        "C++ Unit::IsFlying makes Creature::CanFly true for FLYING"
    );

    let mut dynamic_can_fly_only = Creature::new(false);
    dynamic_can_fly_only.unit_mut().set_max_health(80);
    dynamic_can_fly_only.unit_mut().set_health(80);
    dynamic_can_fly_only.unit_mut().set_combat_reach(1.0);
    dynamic_can_fly_only.set_ai_position(creature_pos);
    dynamic_can_fly_only.ai_ownership_mut().aggro_radius = 100.0;
    dynamic_can_fly_only.set_movement_flags_runtime_like_cpp(MovementFlag::CAN_FLY);
    assert!(
        !dynamic_can_fly_only.is_flying_like_cpp(),
        "C++ Unit::IsFlying ignores MOVEMENTFLAG_CAN_FLY by itself"
    );
    assert!(
        !dynamic_can_fly_only.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 60.0, 0.0),
            0.5,
        )
    );

    let mut invalid_flight = Creature::new(false);
    invalid_flight.unit_mut().set_max_health(80);
    invalid_flight.unit_mut().set_health(80);
    invalid_flight.unit_mut().set_combat_reach(1.0);
    invalid_flight.set_ai_position(creature_pos);
    invalid_flight.ai_ownership_mut().aggro_radius = 100.0;
    invalid_flight
        .set_flight_movement_type_runtime_like_cpp(CREATURE_FLIGHT_MOVEMENT_TYPE_MAX_LIKE_CPP);
    assert_eq!(
        invalid_flight.flight_movement_type_like_cpp(),
        CreatureFlightMovementType::None as u8
    );
    assert!(
        !invalid_flight.try_ai_aggro_with_target_combat_reach_like_cpp(
            player,
            &Position::new(12.0, 20.0, 60.0, 0.0),
            0.5,
        )
    );
}

#[test]
fn creature_accessibility_capabilities_follow_movement_template_like_cpp() {
    let mut creature = Creature::new(false);

    assert!(creature.can_walk_like_cpp());
    assert!(creature.can_enter_water_like_cpp());
    assert!(!creature.can_fly_like_cpp());

    creature.set_ground_movement_type_runtime_like_cpp(CreatureGroundMovementType::None as u8);
    creature.set_swim_allowed_runtime_like_cpp(true);
    assert!(!creature.can_walk_like_cpp());
    assert!(creature.can_enter_water_like_cpp());

    creature.set_swim_allowed_runtime_like_cpp(false);
    assert!(!creature.can_enter_water_like_cpp());

    creature.set_flight_movement_type_runtime_like_cpp(
        CreatureFlightMovementType::DisableGravity as u8,
    );
    assert!(creature.can_fly_like_cpp());
}

#[test]
fn creature_can_enter_water_honors_unit_can_swim_like_cpp() {
    let mut creature = Creature::new(false);
    creature.set_swim_allowed_runtime_like_cpp(false);
    assert!(!creature.can_enter_water_like_cpp());

    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::CAN_SWIM);
    assert!(creature.can_enter_water_like_cpp());

    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::CAN_SWIM | UnitFlags::CANT_SWIM);
    assert!(!creature.can_enter_water_like_cpp());
}

#[test]
fn creature_engage_temporarily_adds_template_swim_capability_like_cpp() {
    let mut creature = Creature::new(false);
    creature.set_swim_allowed_runtime_like_cpp(true);
    assert!(!creature.can_swim_like_cpp());

    creature.enter_ai_combat(ObjectGuid::create_player(1, 7));
    assert!(creature.can_swim_like_cpp());
    assert!(creature.is_missing_can_swim_flag_out_of_combat());

    creature.restore_can_swim_flag_after_home_like_cpp();
    assert!(!creature.can_swim_like_cpp());

    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::CAN_SWIM);
    creature.refresh_can_swim_flag_like_cpp(true);
    assert!(!creature.is_missing_can_swim_flag_out_of_combat());
    creature.restore_can_swim_flag_after_home_like_cpp();
    assert!(creature.can_swim_like_cpp());
}

#[test]
fn creature_try_ai_aggro_rejects_civilian_like_cpp() {
    let mut creature = Creature::new(false);
    let player = ObjectGuid::create_player(1, 7);
    let creature_pos = Position::new(10.0, 20.0, 30.0, 0.0);
    let player_pos = Position::new(11.0, 20.0, 30.0, 0.0);
    creature.unit_mut().set_max_health(80);
    creature.unit_mut().set_health(80);
    creature.set_ai_position(creature_pos);
    creature.ai_ownership_mut().aggro_radius = 5.0;

    // C++ `Creature::CanStartAttack` returns false immediately for
    // `IsCivilian()`, which is backed by `CREATURE_FLAG_EXTRA_CIVILIAN`.
    creature.set_flags_extra_runtime_like_cpp(CreatureFlagsExtra::CIVILIAN.bits());
    assert!(creature.is_civilian_like_cpp());
    assert!(!creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::Idle);

    creature.set_flags_extra_runtime_like_cpp(0);
    assert!(!creature.is_civilian_like_cpp());
    assert!(creature.try_ai_aggro(player, &player_pos));
    assert_eq!(creature.ai_state(), CreatureAiState::InCombat);
}

#[test]
fn creature_ai_ownership_wander_and_packet_metadata_are_canonical() {
    let mut creature = Creature::new(false);
    assert!(creature.can_ai_wander());
    creature.ai_ownership_mut().npc_flags = 0x80;
    assert!(!creature.can_ai_wander());
    creature.ai_ownership_mut().npc_flags = 0;
    creature.set_template_rooted_like_cpp(true);
    assert!(creature.is_template_rooted_like_cpp());
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::ROOT)
    );
    assert!(!creature.can_ai_wander());
    creature.set_template_rooted_like_cpp(false);
    assert!(
        !creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::ROOT)
    );
    assert!(creature.can_ai_wander());

    creature.set_display_id(1234, true, None);
    creature.set_faction(35);
    creature.ai_ownership_mut().unit_flags = 0x20;
    creature.ai_ownership_mut().min_damage = 5;
    creature.ai_ownership_mut().max_damage = 9;

    assert_eq!(creature.ai_ownership().display_id, 1234);
    assert_eq!(creature.ai_ownership().faction, 35);
    assert_eq!(creature.ai_ownership().unit_flags, 0x20);
    assert_eq!(creature.ai_ownership().min_damage, 5);
    assert_eq!(creature.ai_ownership().max_damage, 9);
}

#[test]
fn creature_ai_movement_inform_records_cpp_type_and_id_payload() {
    let mut creature = Creature::new(false);

    creature.record_ai_movement_inform(15, 8);
    assert_eq!(
        creature.ai_ownership().last_movement_inform,
        Some(CreatureMovementInform {
            movement_type: 15,
            movement_id: 8,
        })
    );
    assert_eq!(
        creature.take_ai_movement_inform(),
        Some(CreatureMovementInform {
            movement_type: 15,
            movement_id: 8,
        })
    );
    assert_eq!(creature.ai_ownership().last_movement_inform, None);
}

#[test]
fn creature_power_index_matches_cpp_stat_system() {
    let mut creature = Creature::new(false);

    assert_eq!(creature.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(creature.get_power_index(PowerType::ComboPoints), Some(2));
    assert_eq!(creature.get_power_index(PowerType::Energy), None);

    creature.set_power_type(PowerType::Energy);
    assert_eq!(creature.power_type(), PowerType::Energy);
    assert_eq!(creature.get_power_index(PowerType::Energy), Some(0));
    assert_eq!(creature.get_power_index(PowerType::Mana), None);
    assert_eq!(creature.get_power_index(PowerType::ComboPoints), Some(2));
}

#[test]
fn creature_respawn_and_corpse_setters_match_cpp_fields() {
    let mut creature = Creature::new(false);

    creature.set_respawn_delay(45);
    creature.set_respawn_time(1234);
    creature.set_corpse_delay(10, true);
    creature.set_respawn_compatibility_mode(true);
    creature.set_spawn_id(99);

    assert_eq!(creature.respawn_delay(), 45);
    assert_eq!(creature.respawn_time(), 1234);
    assert_eq!(creature.corpse_delay(), 10);
    assert!(creature.ignore_corpse_decay_ratio());
    assert!(creature.respawn_compatibility_mode());
    assert_eq!(creature.spawn_id(), 99);
}

#[test]
fn creature_display_with_model_updates_unit_dimensions_like_cpp() {
    let mut creature = Creature::new(false);

    creature.unit_mut().world_mut().object_mut().set_scale(2.0);
    creature.set_display_id(
        1234,
        true,
        Some(CreatureModelDimensions {
            bounding_radius: 0.3,
            combat_reach: 1.5,
        }),
    );

    let scale = 2.0 * crate::DEFAULT_PLAYER_DISPLAY_SCALE;
    assert_eq!(creature.unit().data().display_id, 1234);
    assert_eq!(creature.unit().data().native_display_id, 1234);
    assert_eq!(creature.unit().data().bounding_radius, 0.3 * scale);
    assert_eq!(creature.unit().data().combat_reach, 1.5 * scale);
}
