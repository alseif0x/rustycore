//! Behaviour tests for [`super`].
//!
//! Extracted verbatim from `creature.rs`, which was 6,940 lines of which
//! 3,090 — 45% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant.

#![cfg(test)]

use super::*;
use crate::MovementGeneratorKind;
use crate::{
    AURA_STATE_DEFENSIVE, AURA_STATE_DEFENSIVE_2, AppliedAuraRef, AuraRef, CurrentSpellRef,
    CurrentSpellSlot, DIMINISHING_STUN, DiminishingLevel, OwnedAuraRef,
};
use wow_constants::SpellState;
use wow_core::guid::HighGuid;

fn formation_info_like_cpp(leader_spawn_id: u64) -> CreatureFormationInfoLikeCpp {
    CreatureFormationInfoLikeCpp {
        leader_spawn_id,
        follow_dist: 7.0,
        follow_angle_radians: 1.25,
        group_ai: 3,
        leader_waypoint_ids: [11, 12],
    }
}

fn owned_loot_fixture_like_cpp(
    coins: u32,
    unlooted_count: u8,
    allowed_looters: Vec<ObjectGuid>,
) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters,
        items: Vec::new(),
        looted_by_player: false,
    }
}

fn poll_immediately_ready<F: std::future::Future>(future: F) -> F::Output {
    struct NoopWake;

    impl std::task::Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    let waker = std::task::Waker::from(std::sync::Arc::new(NoopWake));
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("expected the uncontended claim to be ready"),
    }
}

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

#[test]
fn creature_react_state_and_faction_use_unit_fields() {
    let mut creature = Creature::new(false);

    creature.set_react_state(ReactState::Passive);
    creature.set_faction(35);

    assert!(creature.has_react_state(ReactState::Passive));
    assert_eq!(creature.unit().data().faction_template, 35);
}

#[test]
fn creature_grid_unload_helpers_apply_represented_state() {
    let victim = wow_core::ObjectGuid::new(1, 2);
    let dynamic_object = wow_core::ObjectGuid::new(1, 3);
    let area_trigger = wow_core::ObjectGuid::new(1, 4);
    let mut creature = Creature::new(false);
    creature.unit_mut().set_attacking(Some(victim));
    creature.unit_mut().world_mut().set_current_cell(7, 8);
    creature.register_dynamic_object(dynamic_object);
    creature.register_area_trigger(area_trigger);

    creature.set_destroyed_object(true);
    creature.remove_all_dyn_objects();
    creature.remove_all_area_triggers();
    creature.combat_stop();
    creature.request_respawn_relocation_from_grid_unload();
    creature.cleanup_before_delete();
    creature.request_delete_from_grid_unload();

    assert!(creature.unit().world().object().is_destroyed_object());
    assert!(creature.dynamic_objects().is_empty());
    assert_eq!(
        creature.removed_dynamic_objects_from_grid_unload(),
        &[dynamic_object]
    );
    assert!(creature.area_triggers().is_empty());
    assert_eq!(
        creature.removed_area_triggers_from_grid_unload(),
        &[area_trigger]
    );
    assert_eq!(creature.unit().attacking(), None);
    assert!(creature.grid_unload_respawn_relocation_requested());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
    assert!(creature.grid_unload_delete_requested());
    assert_eq!(creature.unit().world().current_cell(), None);
    assert!(!creature.unit().world().object().is_in_grid());
}

fn creature_lifecycle_template() -> CreatureTemplateLifecycleRecord {
    let mut spells = [0; MAX_CREATURE_SPELLS];
    spells[0] = 133;
    spells[3] = 116;
    CreatureTemplateLifecycleRecord {
        entry: 1001,
        original_entry: 9001,
        difficulty_id: 2,
        name: "lifecycle wolf".to_string(),
        ai_name: "SmartAI".to_string(),
        script_name: "npc_lifecycle_wolf".to_string(),
        required_expansion: 2,
        unit_class: 1,
        trainer_class: 4,
        faction: 14,
        npc_flags: 0x1_0000_0040,
        display_id: 2001,
        model_dimensions: Some(CreatureModelDimensions {
            bounding_radius: 0.4,
            combat_reach: 1.2,
        }),
        scale: 1.5,
        speed_walk: 0.8,
        speed_run: 1.25,
        spells,
        classification: 3,
        damage_school: wow_constants::spell::SpellSchools::Nature as u8,
        unit_flags: UnitFlags::IMMUNE_TO_NPC.bits(),
        unit_flags2: UnitFlags2::FEIGN_DEATH.bits(),
        unit_flags3: UnitFlags3::AI_OBSTACLE.bits(),
        flags_extra: CreatureFlagsExtra::CIVILIAN.bits()
            | CreatureFlagsExtra::USE_OFFHAND_ATTACK.bits(),
        static_flags: [0; 8],
        creature_type: 9,
        type_flags: 0x20,
        loot_id: 7_001,
        skin_loot_id: 7_002,
        gold_min: 17,
        gold_max: 29,
        movement_type: MovementGeneratorType::Idle,
        ground_movement_type: CreatureGroundMovementType::Run as u8,
        swim_allowed: true,
        flight_movement_type: CreatureFlightMovementType::DisableGravity as u8,
        rooted: false,
        chase_movement_type: CreatureChaseMovementType::Run as u8,
        random_movement_type: CreatureRandomMovementType::Walk as u8,
        interaction_pause_timer_ms: DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        min_level: 70,
        max_level: 72,
        equipment_id: 4,
        original_equipment_id: -4,
    }
}

fn creature_lifecycle_spawn() -> CreatureSpawnLifecycleRecord {
    CreatureSpawnLifecycleRecord {
        spawn_id: 44_000,
        map_id: 571,
        instance_id: 3,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        home_position: Position::new(5.0, 6.0, 7.0, 1.0),
        phase_id: Some(169),
        phase_group: Some(12),
        terrain_swap_map: Some(609),
        spawn_group_id: Some(77),
        spawn_group_name: Some("lifecycle group".to_string()),
        pool_id: Some(88),
        equipment_id: Some(9),
        original_equipment_id: Some(-9),
        wander_distance: 12.5,
        respawn_delay: 45,
        respawn_time: 123_456,
        movement_type: MovementGeneratorType::Idle,
        string_id: Some("creature-string".to_string()),
        is_active: false,
        inactive_by_spawn_group: true,
        duplicate_spawn_found: true,
        add_to_map: true,
        respawn_compatibility_mode: true,
    }
}

fn vehicle_seat_def(
    seat_index: i8,
    can_enter_or_exit: bool,
) -> (i8, VehicleSeatInfo, VehicleSeatAddon) {
    (
        seat_index,
        VehicleSeatInfo {
            id: 10_000 + u32::from(seat_index.unsigned_abs()),
            attachment_offset: Position::ZERO,
            can_enter_or_exit,
            usable_by_override: false,
            can_control: false,
            can_switch_from_seat: false,
            ejectable: false,
            disables_gravity: false,
            passenger_not_selectable: false,
            keep_pet: false,
        },
        VehicleSeatAddon::default(),
    )
}

fn creature_lifecycle_create_record() -> CreatureCreateLifecycleRecord {
    CreatureCreateLifecycleRecord {
        guid: ObjectGuid::new(8, 1001),
        entry: 1001,
        map_id: 571,
        instance_id: 3,
        position: Position::new(1.0, 2.0, 3.0, 4.0),
        dynamic: false,
        vehicle_id: Some(101),
        vehicle_kit_create_input: Some(VehicleKitCreateInputLikeCpp {
            vehicle_id: 101,
            creature_entry: 1001,
            loading: true,
            seat_defs: vec![vehicle_seat_def(0, true), vehicle_seat_def(2, false)],
        }),
        add_to_world_vehicle_reset_context: None,
        template: creature_lifecycle_template(),
        spawn: None,
        selected_level: 71,
        stats: CreatureLifecycleStats::new(5_000, 4_500, 1_000, 750),
        selected_display_id: 3001,
        selected_model_dimensions: Some(CreatureModelDimensions {
            bounding_radius: 0.5,
            combat_reach: 2.0,
        }),
        selected_equipment_id: 6,
        selected_original_equipment_id: -6,
        selected_virtual_items: [(10_001, 3, 4), (10_002, 5, 6), (0, 0, 0)],
        corpse_delay: 90,
        ignore_corpse_decay_ratio: true,
        addon: None,
    }
}

#[test]
fn creature_lifecycle_create_sets_ignore_pathfinding_from_flags_extra_like_cpp() {
    // C++ `Creature::Create` (`Creature.cpp:1154-1155`).
    let baseline = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    assert!(
        !baseline
            .unit()
            .has_unit_state(UnitState::IGNORE_PATHFINDING.bits()),
        "a template without the flag must keep using the navmesh"
    );

    let mut record = creature_lifecycle_create_record();
    record.template.flags_extra |= CreatureFlagsExtra::IGNORE_PATHFINDING.bits();
    let ignoring = Creature::create_from_lifecycle(record);
    assert!(
        ignoring
            .unit()
            .has_unit_state(UnitState::IGNORE_PATHFINDING.bits()),
        "CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING must add UNIT_STATE_IGNORE_PATHFINDING"
    );
    assert_eq!(
        CreatureFlagsExtra::IGNORE_PATHFINDING.bits(),
        0x2000_0000,
        "flag value must match CreatureData.h:363"
    );
}

#[test]
fn creature_lifecycle_create_applies_template_stats_and_clean_baseline() {
    let creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());

    assert_eq!(
        creature.unit().world().object().guid(),
        ObjectGuid::new(8, 1001)
    );
    assert_eq!(creature.unit().world().object().entry(), 1001);
    assert_eq!(creature.unit().world().map_id(), 571);
    assert_eq!(creature.unit().world().instance_id(), 3);
    assert_eq!(
        creature.unit().world().position(),
        Position::new(1.0, 2.0, 3.0, 4.0)
    );
    assert_eq!(creature.unit().data().race, 0);
    assert_eq!(creature.unit().data().class_id, 1);
    assert_eq!(creature.unit().data().faction_template, 14);
    assert_eq!(creature.unit().npc_flags_like_cpp(), [0x40, 0x1]);
    assert_eq!(
        creature.unit().unit_flags_like_cpp(),
        UnitFlags::IMMUNE_TO_NPC
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE
    );
    assert_eq!(creature.unit().data().display_id, 3001);
    assert_eq!(creature.unit().data().native_display_id, 3001);
    assert_eq!(creature.unit().world().object().scale(), 1.5);
    assert_eq!(
        creature.unit().data().bounding_radius,
        0.5 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
    );
    assert_eq!(
        creature.unit().data().combat_reach,
        2.0 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
    );
    let speed_rate = creature.unit().speed_rate();
    assert_eq!(speed_rate[UnitMoveType::Walk as usize], 0.8);
    assert_eq!(speed_rate[UnitMoveType::Run as usize], 1.25);
    assert_eq!(speed_rate[UnitMoveType::Swim as usize], 1.0);
    assert_eq!(speed_rate[UnitMoveType::Flight as usize], 1.0);
    assert_eq!(creature.unit().data().mod_casting_speed, 1.0);
    assert_eq!(creature.unit().data().mod_spell_haste, 1.0);
    assert_eq!(creature.unit().data().mod_haste, 1.0);
    assert_eq!(creature.unit().data().mod_ranged_haste, 1.0);
    assert_eq!(creature.unit().data().mod_haste_regen, 1.0);
    assert_eq!(creature.unit().data().mod_time_rate, 1.0);
    assert!(creature.unit().can_dual_wield_like_cpp());
    assert_eq!(creature.unit().data().virtual_items[0].item_id, 10_001);
    assert_eq!(
        creature.unit().data().virtual_items[0].item_appearance_mod_id,
        3
    );
    assert_eq!(creature.unit().data().virtual_items[0].item_visual, 4);
    assert_eq!(creature.unit().data().virtual_items[1].item_id, 10_002);
    assert_eq!(
        creature.unit().data().virtual_items[1].item_appearance_mod_id,
        5
    );
    assert_eq!(creature.unit().data().virtual_items[1].item_visual, 6);
    assert_eq!(
        creature.unit().data().virtual_items[2],
        VisibleItemValues::default()
    );
    assert_eq!(creature.spells()[0], 133);
    assert_eq!(creature.spells()[3], 116);
    assert_eq!(
        creature.melee_damage_school_mask(),
        1 << (wow_constants::spell::SpellSchools::Nature as u8),
        "C++ Creature::UpdateEntry applies SetMeleeDamageSchool(cInfo->dmgschool)"
    );
    assert_eq!(creature.equipment_id(), 6);
    assert_eq!(creature.original_equipment_id(), -6);
    let kit = creature.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id(), 101);
    assert!(kit.active());
    assert!(!kit.installed());
    assert_eq!(kit.seat_count(), 2);
    assert_eq!(kit.usable_seat_num(), 1);
    let create_outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .unwrap();
    assert_eq!(create_outcome.kit_id, Some(101));
    assert!(create_outcome.created);
    assert_eq!(create_outcome.seat_count, 2);
    assert_eq!(create_outcome.usable_seat_num, 1);
    assert!(create_outcome.unit_update_flag_vehicle_represented);
    assert!(create_outcome.unit_type_mask_vehicle_represented);
    assert!(creature.has_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE));
    assert!(creature.is_vehicle_unit_type_like_cpp());
    assert!(!creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(!create_outcome.send_set_vehicle_rec_id_represented);
    assert!(create_outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!create_outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(create_outcome.update_display_power_represented);
    assert!(create_outcome.init_movement_info_for_base_represented);
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    assert_eq!(creature.unit().data().level, 71);
    assert_eq!(creature.unit().data().max_health, 5_000);
    assert_eq!(creature.unit().data().health, 4_500);
    assert_eq!(creature.unit().get_max_power(PowerType::Mana), 1_000);
    assert_eq!(creature.unit().get_power(PowerType::Mana), 750);
    assert_eq!(creature.lifecycle_metadata().spawn_health, Some(4_500));
    assert_eq!(creature.lifecycle_metadata().spawn_mana, Some(750));
    assert_eq!(
        creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
        [BASE_MINDAMAGE, BASE_MAXDAMAGE]
    );
    assert_eq!(creature.corpse_delay(), 90);
    assert!(creature.ignore_corpse_decay_ratio());
    assert!(creature.respawn_compatibility_mode());
    assert_eq!(creature.lifecycle_metadata().template_entry, 1001);
    assert_eq!(creature.lifecycle_metadata().original_entry, 9001);
    assert_eq!(creature.lifecycle_metadata().difficulty_id, 2);
    assert_eq!(creature.lifecycle_metadata().ai_name, "SmartAI");
    assert_eq!(
        creature.lifecycle_metadata().script_name,
        "npc_lifecycle_wolf"
    );
    assert_eq!(creature.lifecycle_metadata().required_expansion, 2);
    assert_eq!(creature.lifecycle_metadata().classification, 3);
    assert_eq!(
        creature.lifecycle_metadata().damage_school,
        wow_constants::spell::SpellSchools::Nature as u8
    );
    assert_eq!(
        creature.lifecycle_metadata().flight_movement_type,
        CreatureFlightMovementType::DisableGravity as u8
    );
    assert_eq!(creature.unit().changed_object_type_mask(), 0);
}

#[test]
fn creature_lifecycle_retains_combat_log_stats_and_selects_attack_power_like_cpp() {
    let combat_log_stats = CreatureCombatLogStatsLikeCpp {
        attack_power: 111,
        ranged_attack_power: 222,
        spell_power: 333,
        armor: 444,
    };

    let mut melee_record = creature_lifecycle_create_record();
    melee_record.stats.combat_log = combat_log_stats;
    let melee = Creature::create_from_lifecycle(melee_record);

    assert_eq!(melee.combat_log_stats_like_cpp(), combat_log_stats);
    assert_eq!(melee.combat_log_attack_power_like_cpp(), 111);

    let mut hunter_record = creature_lifecycle_create_record();
    hunter_record.template.unit_class = Class::Hunter as u8;
    hunter_record.stats.combat_log = combat_log_stats;
    let hunter = Creature::create_from_lifecycle(hunter_record);

    assert_eq!(hunter.combat_log_stats_like_cpp(), combat_log_stats);
    assert_eq!(hunter.combat_log_attack_power_like_cpp(), 222);
}

#[test]
fn creature_lifecycle_seeds_selected_non_mana_power_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.stats = CreatureLifecycleStats {
        max_health: 5_000,
        health: 4_500,
        power_type: PowerType::Focus,
        base_mana: 600,
        max_power: 100,
        power: 25,
        min_damage: BASE_MINDAMAGE,
        max_damage: BASE_MAXDAMAGE,
        combat_log: CreatureCombatLogStatsLikeCpp::default(),
    };

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.power_type(), PowerType::Focus);
    assert_eq!(creature.unit().get_create_mana_like_cpp(), 600);
    assert_eq!(creature.unit().get_max_power(PowerType::Focus), 100);
    assert_eq!(creature.unit().get_power(PowerType::Focus), 25);
    assert_eq!(
        creature.unit().get_max_power(PowerType::Mana),
        0,
        "changing display power removes POWER_MANA from the creature power index"
    );
}

#[test]
fn creature_lifecycle_create_without_spawn_applies_dynamic_respawn_compatibility() {
    let mut record = creature_lifecycle_create_record();
    record.dynamic = false;
    record.spawn = None;
    let static_creature = Creature::create_from_lifecycle(record);
    assert!(static_creature.respawn_compatibility_mode());

    let mut record = creature_lifecycle_create_record();
    record.dynamic = true;
    record.spawn = None;
    let dynamic_creature = Creature::create_from_lifecycle(record);
    assert!(!dynamic_creature.respawn_compatibility_mode());
}

#[test]
fn creature_runtime_just_respawned_uses_represented_spawn_health_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_power(PowerType::Mana, 1);
    creature.unit_mut().set_death_state(DeathState::Corpse);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature.unit().data().health,
        4_500,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls SetSpawnHealth instead of always SetFullHealth for non-pets"
    );
    assert_eq!(
        creature.unit().get_power(PowerType::Mana),
        750,
        "C++ SetSpawnHealth restores the represented spawn mana source"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0x40, 0x1],
        "C++ JUST_RESPAWNED reloads ChooseCreatureFlags output from the creature template baseline"
    );
    assert_eq!(
        creature.unit().unit_flags_like_cpp(),
        UnitFlags::IMMUNE_TO_NPC
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE
    );
}

#[test]
fn creature_runtime_just_respawned_pet_uses_full_health_and_skips_non_pet_resets_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::new((HighGuid::Pet as i64) << 58, 44));
    creature.unit_mut().set_max_health(9_000);
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_power(PowerType::Mana, 1);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .replace_all_dynamic_flags(0x44);
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
    creature
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::FEIGN_DEATH);
    creature
        .unit_mut()
        .set_unit_flags3_like_cpp(UnitFlags3::AI_OBSTACLE);
    creature.set_melee_damage_school_like_cpp(wow_constants::spell::SpellSchools::Fire as u8);
    creature.unit_mut().set_death_state(DeathState::Corpse);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature.unit().data().health,
        9_000,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls SetFullHealth for pets"
    );
    assert_eq!(
        creature.unit().get_power(PowerType::Mana),
        1,
        "C++ pet branch does not run the non-pet spawn mana restore"
    );
    assert_eq!(
        creature.unit().world().object().dynamic_flags(),
        0x44,
        "C++ non-pet block owns ReplaceAllDynamicFlags(UNIT_DYNFLAG_NONE)"
    );
    assert!(
        creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT),
        "C++ non-pet block owns unit flag reload/removal"
    );
    assert_eq!(
        creature.melee_damage_school_like_cpp(),
        wow_constants::spell::SpellSchools::Fire as u8,
        "C++ non-pet block owns SetMeleeDamageSchool"
    );
    assert_eq!(creature.unit().death_state(), DeathState::Alive);
}

#[test]
fn creature_runtime_just_respawned_initializes_motion_when_not_blocked_by_formation_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .set_current_generator(MovementGeneratorKind::Chase);
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase
    );

    let plan = creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert!(plan.contains(CreatureRuntimeAction::InitializeMotion));
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Idle,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls Motion_Initialize, which falls through to MotionMaster::Initialize when formation does not block it"
    );
}

#[test]
fn creature_runtime_just_respawned_preserves_non_leader_formation_motion_until_group_runtime_like_cpp()
 {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let mut spawn = creature_lifecycle_spawn();
    spawn.spawn_id = 44_001;
    let mut creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });
    creature.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: 44_000,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .set_current_generator(MovementGeneratorKind::Chase);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase,
        "C++ non-leader formed creatures wait for CreatureGroup state; Rust has no live CreatureGroup::IsFormed runtime here yet"
    );
}

#[test]
fn aim_initialize_like_cpp_represents_normal_creature_without_formation_or_vehicle() {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let creature = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create,
        spawn: creature_lifecycle_spawn(),
    });

    let outcome = creature.aim_initialize_like_cpp();

    assert_eq!(outcome.guid, creature.guid());
    assert_eq!(outcome.spawn_id, 44_000);
    assert!(outcome.aim_create_represented);
    assert!(outcome.motion_initialize_represented);
    assert!(!outcome.formation_present);
    assert!(!outcome.formation_leader);
    assert!(!outcome.formation_move_idle_represented);
    assert!(!outcome.motion_initialize_requires_formed_state);
    assert!(outcome.motion_master_initialize_represented);
    assert!(outcome.ai_selected_represented);
    assert!(outcome.ai_initialize_represented);
    assert!(!outcome.vehicle_reset_expected);
    assert!(outcome.succeeded);
}

#[test]
fn aim_initialize_like_cpp_reports_formation_leader_and_non_leader_without_move_idle() {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let spawn = creature_lifecycle_spawn();
    let mut leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create: create.clone(),
        spawn: spawn.clone(),
    });
    leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: spawn.spawn_id,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));

    let leader_outcome = leader.aim_initialize_like_cpp();
    assert!(leader_outcome.formation_present);
    assert!(leader_outcome.formation_leader);
    assert!(!leader_outcome.formation_move_idle_represented);
    assert!(!leader_outcome.motion_initialize_requires_formed_state);
    assert!(leader_outcome.motion_master_initialize_represented);

    let mut non_leader_spawn = spawn;
    non_leader_spawn.spawn_id = 44_001;
    let mut non_leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create,
        spawn: non_leader_spawn,
    });
    non_leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: 44_000,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));

    let non_leader_outcome = non_leader.aim_initialize_like_cpp();
    assert!(non_leader_outcome.formation_present);
    assert!(!non_leader_outcome.formation_leader);
    assert!(!non_leader_outcome.formation_move_idle_represented);
    assert!(non_leader_outcome.motion_initialize_requires_formed_state);
    assert!(!non_leader_outcome.motion_master_initialize_represented);
}

#[test]
fn creature_lifecycle_vehicle_entry_missing_preserves_identity_without_local_kit_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.vehicle_id = Some(909);
    record.vehicle_kit_create_input = None;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(909));
    assert!(creature.unit().subsystems().vehicle.kit.is_none());
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .unwrap();
    assert_eq!(outcome.kit_id, Some(909));
    assert!(!outcome.created);
    assert_eq!(outcome.seat_count, 0);
    assert_eq!(outcome.usable_seat_num, 0);
    assert!(!outcome.unit_update_flag_vehicle_represented);
    assert!(!outcome.unit_type_mask_vehicle_represented);
    assert_eq!(creature.unit_type_mask_like_cpp(), 0);
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(!outcome.send_set_vehicle_rec_id_represented);
    assert!(!outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!outcome.update_display_power_represented);
    assert!(!outcome.init_movement_info_for_base_represented);
}

#[test]
fn creature_unit_type_mask_helpers_match_cpp_bitmask_semantics() {
    let mut creature = Creature::new(false);

    assert_eq!(creature.unit_type_mask_like_cpp(), 0);
    assert!(!creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(!creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(creature.can_have_threat_list_like_cpp());
    assert!(
        creature
            .unit()
            .subsystems()
            .combat
            .owner_can_have_threat_list
    );

    creature.add_unit_type_mask_like_cpp(
        UNIT_MASK_TOTEM | UNIT_MASK_GUARDIAN | UNIT_MASK_CONTROLABLE_GUARDIAN,
    );
    assert!(creature.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM));
    assert!(creature.is_totem_unit_type_like_cpp());
    assert!(creature.is_guardian_unit_type_like_cpp());
    assert!(creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(!creature.can_have_threat_list_like_cpp());
    assert!(
        !creature
            .unit()
            .subsystems()
            .combat
            .owner_can_have_threat_list
    );

    creature.remove_unit_type_mask_like_cpp(UNIT_MASK_GUARDIAN);
    assert!(creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.can_have_threat_list_like_cpp());
}

#[test]
fn creature_lifecycle_create_applies_resolved_base_weapon_damage() {
    let mut record = creature_lifecycle_create_record();
    record.stats.min_damage = 3.5;
    record.stats.max_damage = 7.25;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(
        creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
        [3.5, 7.25]
    );
}

#[test]
fn creature_lifecycle_load_from_db_applies_spawn_bridge_state() {
    let create = creature_lifecycle_create_record();
    let spawn = creature_lifecycle_spawn();
    let creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });

    assert_eq!(creature.spawn_id(), 44_000);
    assert_eq!(creature.wander_distance(), 12.5);
    assert_eq!(creature.respawn_delay(), 45);
    assert_eq!(creature.respawn_time(), 123_456);
    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Idle
    );
    assert_eq!(creature.unit().world().map_id(), 571);
    assert_eq!(creature.unit().world().instance_id(), 3);
    assert_eq!(
        creature.unit().world().position(),
        Position::new(1.0, 2.0, 3.0, 4.0)
    );
    assert!(creature.respawn_compatibility_mode());
    assert_eq!(creature.equipment_id(), 9);
    assert_eq!(creature.original_equipment_id(), -9);
    let metadata = creature.lifecycle_metadata();
    assert_eq!(metadata.home_position, Position::new(5.0, 6.0, 7.0, 1.0));
    assert_eq!(metadata.phase_id, Some(169));
    assert_eq!(metadata.terrain_swap_map, Some(609));
    assert_eq!(metadata.spawn_group_id, Some(77));
    assert_eq!(
        metadata.spawn_group_name.as_deref(),
        Some("lifecycle group")
    );
    assert_eq!(metadata.string_id.as_deref(), Some("creature-string"));
    assert!(metadata.add_to_map_requested);
    assert!(metadata.map_insertion_requested);
    assert!(metadata.duplicate_spawn_found);
    assert!(!metadata.is_spawn_active);
    assert!(metadata.inactive_by_spawn_group);
    assert_eq!(creature.unit().changed_object_type_mask(), 0);
}

#[test]
fn creature_lifecycle_waypoint_default_survives_add_to_world_motion_initialize_like_cpp() {
    let create = creature_lifecycle_create_record();
    let mut spawn = creature_lifecycle_spawn();
    spawn.movement_type = MovementGeneratorType::Waypoint;
    let mut creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });

    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Waypoint
    );

    let outcome = creature.unit_mut().add_to_world_like_cpp();

    assert!(
        outcome
            .motion_master_add_to_world
            .had_initialization_pending
    );
    let current = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(
        current.kind,
        MovementGeneratorKind::Waypoint,
        "C++ FactorySelector::SelectMovementGenerator uses Creature::GetDefaultMovementType during MotionMaster::InitializeDefault"
    );
    assert!(current.has_flag(crate::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
}

#[test]
fn creature_runtime_default_movement_setter_syncs_motion_generator_like_cpp() {
    let mut creature = Creature::new(false);
    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Idle
    );

    creature.set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Waypoint);

    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Waypoint
    );
}

#[test]
fn creature_lifecycle_loads_represented_addon_local_fields_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 9_001,
        mount_display_id: 12_345,
        stand_state: UnitStandStateType::Kneel,
        vis_flags: 0x12,
        anim_tier: 2,
        sheath_state: SheathState::Ranged,
        pvp_flags: UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP,
        emote: 77,
        ai_anim_kit_id: 11,
        movement_anim_kit_id: 22,
        melee_anim_kit_id: 33,
        visibility_distance_type: VisibilityDistanceTypeLikeCpp::Gigantic,
        auras: vec![70_001, 70_002],
        aura_applications: Vec::new(),
    });

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(
        creature.unit().data().mount_display_id,
        12_345,
        "C++ Creature::LoadCreaturesAddon calls Mount(addon->mount) when mount != 0"
    );
    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_001,
        "C++ Creature::LoadCreaturesAddon copies nonzero addon PathId into _waypointPathId"
    );
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Kneel,
        "C++ Creature::LoadCreaturesAddon calls SetStandState(addon->standState)"
    );
    assert_eq!(
        creature.unit().vis_flags_like_cpp(),
        0x12,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllVisFlags(addon->visFlags)"
    );
    assert_eq!(
        creature.unit().anim_tier_like_cpp(),
        2,
        "C++ Creature::LoadCreaturesAddon calls SetAnimTier(addon->animTier, false)"
    );
    assert_eq!(
        creature.unit().sheath_like_cpp(),
        SheathState::Ranged,
        "C++ Creature::LoadCreaturesAddon calls SetSheath(addon->sheathState)"
    );
    assert_eq!(
        creature.unit().pet_flags_like_cpp(),
        0,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllPetFlags(UNIT_PET_FLAG_NONE)"
    );
    assert_eq!(
        creature.unit().shapeshift_form_like_cpp(),
        ShapeShiftForm::None,
        "C++ Creature::LoadCreaturesAddon calls SetShapeshiftForm(FORM_NONE)"
    );
    assert_eq!(
        creature.unit().pvp_flags_like_cpp(),
        UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllPvpFlags(addon->pvpFlags)"
    );
    assert_eq!(
        creature.unit().emote_state_like_cpp(),
        77,
        "C++ Creature::LoadCreaturesAddon calls SetEmoteState(addon->emote) when emote != 0"
    );
    assert_eq!(
        creature.unit().ai_anim_kit_id_like_cpp(),
        11,
        "C++ Creature::LoadCreaturesAddon calls SetAIAnimKitId(addon->aiAnimKit)"
    );
    assert_eq!(
        creature.unit().movement_anim_kit_id_like_cpp(),
        22,
        "C++ Creature::LoadCreaturesAddon calls SetMovementAnimKitId(addon->movementAnimKit)"
    );
    assert_eq!(
        creature.unit().melee_anim_kit_id_like_cpp(),
        33,
        "C++ Creature::LoadCreaturesAddon calls SetMeleeAnimKitId(addon->meleeAnimKit)"
    );
    assert_eq!(
        creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp(),
        Some(VisibilityDistanceTypeLikeCpp::Gigantic.distance_like_cpp()),
        "C++ Creature::LoadCreaturesAddon calls SetVisibilityDistanceOverride for non-Normal addon visibility"
    );
    assert!(
        creature
            .unit()
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::GIGANTIC_AOI),
        "C++ SetVisibilityDistanceOverride sets the matching UNIT_FLAG2_*_AOI flag"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_001),
        "C++ Creature::LoadCreaturesAddon applies listed permanent addon auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_002),
        "C++ Creature::LoadCreaturesAddon applies each listed addon aura"
    );
}

#[test]
fn creature_lifecycle_addon_applies_hover_movement_flag_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.template.ground_movement_type = CreatureGroundMovementType::Hover as u8;
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp::default());

    let creature = Creature::create_from_lifecycle(record);

    assert!(creature.can_hover_like_cpp());
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::HOVER),
        "C++ Creature::LoadCreaturesAddon calls AddUnitMovementFlag(MOVEMENTFLAG_HOVER) when CanHover()"
    );
}

#[test]
fn creature_load_path_sets_waypoint_path_id_like_cpp() {
    let mut creature = Creature::new(false);

    creature.load_path_like_cpp(9_123);

    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_123,
        "C++ Creature::LoadPath stores the waypoint path id used by WaypointMovementGenerator::DoInitialize"
    );
}

#[test]
fn creature_runtime_respawn_reloads_represented_addon_local_fields_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 9_002,
        mount_display_id: 22_222,
        stand_state: UnitStandStateType::Sit,
        vis_flags: 0x04,
        anim_tier: 3,
        sheath_state: SheathState::Melee,
        pvp_flags: UnitPvpFlags::SANCTUARY,
        emote: 0,
        ai_anim_kit_id: 44,
        movement_anim_kit_id: 55,
        melee_anim_kit_id: 66,
        visibility_distance_type: VisibilityDistanceTypeLikeCpp::Large,
        auras: vec![70_003],
        aura_applications: Vec::new(),
    });
    let mut creature = Creature::create_from_lifecycle(record);
    creature.unit_mut().set_mount_display_id(1);
    creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sleep);
    creature.unit_mut().replace_all_vis_flags_like_cpp(0x02);
    creature.unit_mut().set_anim_tier_like_cpp(1);
    creature.unit_mut().set_sheath_like_cpp(SheathState::Ranged);
    creature.unit_mut().replace_all_pet_flags_like_cpp(0x03);
    creature
        .unit_mut()
        .set_shapeshift_form_like_cpp(ShapeShiftForm::CatForm);
    creature
        .unit_mut()
        .replace_all_pvp_flags_like_cpp(UnitPvpFlags::FFA_PVP);
    creature.unit_mut().set_emote_state_like_cpp(99);
    creature.unit_mut().set_ai_anim_kit_id_like_cpp(1);
    creature.unit_mut().set_movement_anim_kit_id_like_cpp(2);
    creature.unit_mut().set_melee_anim_kit_id_like_cpp(3);

    creature.set_death_state_runtime(DeathState::JustDied, 1_000);
    creature.set_death_state_runtime(DeathState::JustRespawned, 2_000);

    assert_eq!(
        creature.unit().data().mount_display_id,
        22_222,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls LoadCreaturesAddon after Unit::setDeathState(ALIVE)"
    );
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Sit
    );
    assert_eq!(creature.unit().vis_flags_like_cpp(), 0x04);
    assert_eq!(creature.unit().anim_tier_like_cpp(), 3);
    assert_eq!(creature.unit().sheath_like_cpp(), SheathState::Melee);
    assert_eq!(creature.unit().pet_flags_like_cpp(), 0);
    assert_eq!(
        creature.unit().shapeshift_form_like_cpp(),
        ShapeShiftForm::None
    );
    assert_eq!(
        creature.unit().pvp_flags_like_cpp(),
        UnitPvpFlags::SANCTUARY
    );
    assert_eq!(creature.unit().ai_anim_kit_id_like_cpp(), 44);
    assert_eq!(creature.unit().movement_anim_kit_id_like_cpp(), 55);
    assert_eq!(creature.unit().melee_anim_kit_id_like_cpp(), 66);
    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_002,
        "C++ respawn reload path calls LoadCreaturesAddon and preserves nonzero PathId"
    );
    assert_eq!(
        creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp(),
        Some(VisibilityDistanceTypeLikeCpp::Large.distance_like_cpp())
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_003)
    );
    assert_eq!(
        creature.unit().emote_state_like_cpp(),
        0,
        "C++ addon emote 0 skips SetEmoteState; the preceding death path already cleared the emote"
    );
}

#[test]
fn creature_lifecycle_health_is_clamped_to_max_health() {
    let mut record = creature_lifecycle_create_record();
    record.stats.max_health = 100;
    record.stats.health = 150;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.unit().data().max_health, 100);
    assert_eq!(creature.unit().data().health, 100);
}

#[test]
fn creature_lifecycle_plan_preserves_trinity_critical_order() {
    let plan = CreatureLifecyclePlan::trinity_create_load_from_db();

    assert!(plan.occurs_before(
        CreatureLifecycleStep::LookupTemplateAndDifficulty,
        CreatureLifecycleStep::InitEntryAndCreateFromProto
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::RelocateAndValidatePosition,
        CreatureLifecycleStep::InitEntryAndCreateFromProto
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::SelectLevel,
        CreatureLifecycleStep::UpdateLevelDependantStats
    ));
    assert!(plan.occurs_before(
        CreatureLifecycleStep::UpdateLevelDependantStats,
        CreatureLifecycleStep::AddToMap
    ));
    assert_eq!(plan.steps().last(), Some(&CreatureLifecycleStep::AddToMap));
}

#[test]
fn creature_lifecycle_create_with_spawn_cleans_object_and_unit_masks() {
    let mut record = creature_lifecycle_create_record();
    record.spawn = Some(creature_lifecycle_spawn());

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.unit().values_update().changed_object_type_mask, 0);
    assert_eq!(
        creature
            .unit()
            .world()
            .object()
            .values_update()
            .changed_object_type_mask,
        0
    );
    assert_eq!(creature.spawn_id(), 44_000);
}

#[test]
fn creature_runtime_just_died_sets_corpse_respawn_and_clears_combat_bridge_state() {
    let now = 10_000;
    let victim = ObjectGuid::new(1, 2);
    let player = ObjectGuid::new(1, 3);
    let melee_spell = CurrentSpellRef::new(400, Some(player), None);
    let generic_spell = CurrentSpellRef::new(401, Some(player), None).with_cast_time_ms(1_000);
    let channeled_spell = CurrentSpellRef::new(402, Some(player), None)
        .with_cast_time_ms(1_000)
        .with_state(SpellState::Delayed);
    let applied_death_removed = AppliedAuraRef::new(501, player, 0, 0x1);
    let applied_passive = AppliedAuraRef::new(502, player, 1, 0x1);
    let applied_death_persistent = AppliedAuraRef::new(503, player, 2, 0x1);
    let owned_death_removed = OwnedAuraRef::new(601, player, None);
    let owned_passive = OwnedAuraRef::new(602, player, None);
    let owned_death_persistent = OwnedAuraRef::new(603, player, None);
    let mut creature = Creature::new(false);
    creature.set_respawn_compatibility_mode(true);
    creature.set_respawn_delay(45);
    creature.set_corpse_delay(15, false);
    creature.unit_mut().set_max_health(200);
    creature.unit_mut().set_health(125);
    creature.set_power_type(PowerType::Energy);
    creature.unit_mut().set_max_power(PowerType::Energy, 100);
    creature.unit_mut().set_power(PowerType::Energy, 45);
    creature.unit_mut().set_emote_state_like_cpp(88);
    creature
        .unit_mut()
        .set_stand_state_like_cpp(UnitStandStateType::Sit);
    creature.unit_mut().add_unit_state(UnitState::MOVING.bits());
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .start_spline(77, 1_000);
    creature
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .enter_vehicle(ObjectGuid::new(1, 700), Some(1));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_summon_slot(1, ObjectGuid::new(1, 701));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .set_charmed(ObjectGuid::new(1, 702));
    creature
        .unit_mut()
        .subsystems_mut()
        .control
        .add_controlled(ObjectGuid::new(1, 703));
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::PET_IN_COMBAT);
    creature.unit_mut().set_npc_flags_like_cpp(0x40);
    creature.unit_mut().set_npc_flags2_like_cpp(0x2);
    creature.unit_mut().set_mount_display_id(1234);
    creature.set_movement_flags_runtime_like_cpp(
        MovementFlag::HOVER
            | MovementFlag::DISABLE_GRAVITY
            | MovementFlag::CAN_FLY
            | MovementFlag::FLYING,
    );
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .modify_aura_state(AURA_STATE_DEFENSIVE, true);
    creature
        .unit_mut()
        .subsystems_mut()
        .auras
        .modify_aura_state(AURA_STATE_DEFENSIVE_2, true);
    {
        let auras = &mut creature.unit_mut().subsystems_mut().auras;
        for aura in [
            applied_death_removed,
            applied_passive,
            applied_death_persistent,
        ] {
            auras.add_applied(aura);
        }
        for aura in [owned_death_removed, owned_passive, owned_death_persistent] {
            auras.add_owned(aura);
        }
        auras.set_aura_death_policy_like_cpp(applied_passive.aura_ref(), true, false);
        auras.set_aura_death_policy_like_cpp(applied_death_persistent.aura_ref(), false, true);
        auras.set_aura_death_policy_like_cpp(owned_passive.aura_ref(), true, false);
        auras.set_aura_death_policy_like_cpp(owned_death_persistent.aura_ref(), false, true);
    }
    creature.unit_mut().subsystems_mut().auras.incr_diminishing(
        DIMINISHING_STUN,
        DiminishingLevel::Immune,
        1_000,
    );
    creature
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Melee, melee_spell);
    creature
        .unit_mut()
        .set_current_cast_spell(CurrentSpellSlot::Generic, generic_spell);
    creature
        .unit_mut()
        .subsystems_mut()
        .spells
        .set_current_spell(CurrentSpellSlot::Channeled, channeled_spell);
    creature.unit_mut().set_target(victim);
    creature.set_represented_spell_focus_like_cpp(9001, player, 1.25, true);
    creature.unit_mut().set_attacking(Some(victim));
    creature.unit_mut().world_mut().set_active(true);
    creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(player, 7.5);
    creature
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_attacker(player);
    creature.set_tapped_by_player(player, &[]);

    let plan = creature.set_death_state_runtime(DeathState::JustDied, now);

    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(creature.corpse_remove_time(), now + 15);
    assert_eq!(creature.respawn_time(), now + 45 + 15);
    assert_eq!(creature.unit().data().target, ObjectGuid::EMPTY);
    assert_eq!(
        creature.spell_focus_state_like_cpp().spell_id,
        None,
        "C++ Creature::setDeathState(JUST_DIED) releases spell focus before clearing target"
    );
    assert_eq!(
        creature.spell_focus_state_like_cpp().delay_ms,
        0,
        "C++ DoNotReacquireSpellFocusTarget cancels the delayed target snapback"
    );
    assert!(
        !creature.unit().has_unit_state(UnitState::FOCUSING.bits()),
        "C++ ReleaseSpellFocus clears UNIT_STATE_FOCUSING for AI_DOESNT_FACE_TARGET spells"
    );
    assert_eq!(creature.unit().attacking(), None);
    assert_eq!(creature.unit().data().health, 0);
    assert_eq!(creature.unit().get_power(PowerType::Energy), 0);
    assert_eq!(creature.unit().emote_state_like_cpp(), 0);
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Stand
    );
    assert!(
        !UnitState::from_bits_truncate(creature.unit().unit_state()).intersects(UnitState::MOVING),
        "C++ Unit::StopMoving clears UNIT_STATE_MOVING during StopOnDeath"
    );
    assert!(
        creature.unit().subsystems().motion.stopped,
        "C++ MotionMaster::StopOnDeath calls Unit::StopMoving"
    );
    assert!(
        creature.unit().subsystems().motion.spline.finalized,
        "C++ Unit::setDeathState(JUST_DIED) disables/interrupts the movement spline when StopOnDeath succeeds"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0, 0],
        "C++ Creature::setDeathState(JUST_DIED) calls ReplaceAllNpcFlags(0) and ReplaceAllNpcFlags2(0)"
    );
    assert_eq!(
        creature.unit().data().mount_display_id,
        0,
        "C++ Creature::setDeathState(JUST_DIED) calls SetMountDisplayId(0)"
    );
    assert_eq!(
        creature.movement_flags_like_cpp(),
        MovementFlag::CAN_FLY | MovementFlag::FLYING,
        "C++ death calls SetHover(false,false) and SetDisableGravity(false,false), but does not unset CAN_FLY/FLYING here"
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Melee),
        Some(melee_spell)
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Generic),
        None
    );
    assert_eq!(
        creature.unit().current_spell(CurrentSpellSlot::Channeled),
        None
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_death_removed),
        "C++ RemoveAllAurasOnDeath removes non-passive, non-death-persistent applied auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_passive),
        "C++ RemoveAllAurasOnDeath preserves passive applied auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_applied(applied_death_persistent),
        "C++ RemoveAllAurasOnDeath preserves death-persistent applied auras"
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_owned(owned_death_removed),
        "C++ RemoveAllAurasOnDeath removes non-passive, non-death-persistent owned auras"
    );
    assert!(creature.unit().subsystems().auras.has_owned(owned_passive));
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_owned(owned_death_persistent)
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .removed_auras
            .contains(&AuraRef::new(501, player))
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .removed_auras
            .contains(&AuraRef::new(601, player))
    );
    assert_eq!(
        creature.unit().subsystems().vehicle.vehicle_guid,
        None,
        "C++ Unit::setDeathState(non-alive) calls ExitVehicle before RemoveAllControlled"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .control
            .summon_slots
            .iter()
            .all(|guid| guid.is_empty()),
        "C++ Unit::setDeathState(non-alive) calls UnsummonAllTotems before RemoveAllControlled"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .control
            .controlled_guids
            .is_empty(),
        "C++ Unit::setDeathState(non-alive) calls RemoveAllControlled"
    );
    assert_eq!(creature.unit().subsystems().control.charmed_guid, None);
    assert!(
        !creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::PET_IN_COMBAT),
        "C++ RemoveAllControlled clears UNIT_FLAG_PET_IN_COMBAT for non-pets"
    );
    assert!(
        !creature.unit().world().is_active(),
        "C++ Creature::setDeathState(JUST_DIED) calls setActive(false)"
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_aura_state(AURA_STATE_DEFENSIVE)
    );
    assert!(
        !creature
            .unit()
            .subsystems()
            .auras
            .has_aura_state(AURA_STATE_DEFENSIVE_2)
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .auras
            .get_diminishing(DIMINISHING_STUN, 1_000),
        DiminishingLevel::Level1
    );
    assert_eq!(
        creature.unit().subsystems().combat.threat_value(player),
        None,
        "C++ Unit::setDeathState(JUST_DIED) calls CombatStop before death-state side effects"
    );
    assert!(creature.unit().subsystems().combat.attackers.is_empty());
    assert!(creature.runtime_state().save_respawn_requested);
    assert!(plan.contains(CreatureRuntimeAction::SaveRespawnTime));
    assert!(plan.contains(CreatureRuntimeAction::ReleaseSpellFocus));
    assert!(plan.contains(CreatureRuntimeAction::CancelSpellFocusReacquire));
    assert!(plan.contains(CreatureRuntimeAction::ClearTarget));
    assert!(
        !plan.contains(CreatureRuntimeAction::MoveFall),
        "the legacy death-state entry point has no map-height context, so it must not fake C++ MoveFall"
    );

    let mut non_compat = Creature::new(false);
    non_compat.set_respawn_compatibility_mode(false);
    non_compat.set_respawn_delay(45);
    non_compat.set_corpse_delay(15, false);
    non_compat.set_death_state_runtime(DeathState::JustDied, now);
    assert_eq!(non_compat.respawn_time(), now + 45);
    assert_eq!(non_compat.corpse_remove_time(), now + 15);
    assert_eq!(non_compat.unit().death_state(), DeathState::Corpse);
}

#[test]
fn creature_runtime_just_died_can_start_represented_move_fall_with_map_context_like_cpp() {
    let mut creature = Creature::new(false);
    creature.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);

    let plan = creature.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: false,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );

    assert!(
        plan.contains(CreatureRuntimeAction::MoveFall),
        "C++ Creature::setDeathState(JUST_DIED) calls MoveFall after clearing hover/gravity when the pre-clear state was flying/hovering"
    );
    assert_eq!(creature.unit().death_state(), DeathState::Corpse);
    assert_eq!(creature.movement_flags_like_cpp(), MovementFlag::empty());
    let current = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(current.kind, MovementGeneratorKind::Effect);
    assert_eq!(current.movement_id, 77);
    assert_eq!(current.duration_ms, Some(850));
}

#[test]
fn creature_runtime_just_died_move_fall_honors_cpp_underwater_and_root_guards() {
    let mut underwater = Creature::new(false);
    underwater.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    let underwater_plan = underwater.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: true,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );
    assert!(!underwater_plan.contains(CreatureRuntimeAction::MoveFall));

    let mut rooted = Creature::new(false);
    rooted.set_movement_flags_runtime_like_cpp(MovementFlag::HOVER);
    rooted.unit_mut().add_unit_state(UnitState::ROOT.bits());
    let rooted_plan = rooted.set_death_state_runtime_with_fall_like_cpp(
        DeathState::JustDied,
        1_000,
        Some(CreatureDeathFallContextLikeCpp {
            is_underwater: false,
            has_valid_ground_height: true,
            vertical_delta: 12.5,
            movement_id: 77,
            duration_ms: 850,
        }),
    );
    assert!(!rooted_plan.contains(CreatureRuntimeAction::MoveFall));
}

#[test]
fn creature_owned_loot_is_looted_matches_cpp_gold_and_unlooted_count() {
    assert!(CreatureOwnedLoot::default().is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(1, 0).is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(0, 1).is_looted_like_cpp());
    assert!(!CreatureOwnedLoot::new(1, 1).is_looted_like_cpp());
}

#[test]
fn creature_owns_full_loot_authority_and_clear_retires_it() {
    let mut creature = Creature::new(false);
    let full_loot = CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins: 17,
        unlooted_count: 2,
        loot_type: 1,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: Vec::new(),
        looted_by_player: false,
    };
    creature.initialize_shared_loot_authority_like_cpp(full_loot);

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 2))
    );
    assert!(!creature.loot_authority_like_cpp().is_retired_like_cpp());
    creature.clear_loot_like_cpp();
    assert!(creature.loot_authority_like_cpp().is_retired_like_cpp());
    assert!(
        creature
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .is_none()
    );
}

#[test]
fn creature_rebind_retires_displaced_authority_and_its_lease() {
    let player = ObjectGuid::create_player(1, 77);
    let displaced = OwnedLootAuthority::new();
    displaced.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(17, 0, vec![player])),
        HashMap::new(),
    );
    let mut creature = Creature::new(false);
    assert!(creature.rebind_loot_authority_like_cpp(displaced.clone()));
    let lease = poll_immediately_ready(displaced.reserve_money_like_cpp(player))
        .expect("the allowed player reserves the displaced authority");

    let replacement = OwnedLootAuthority::new();
    replacement.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(23, 0, vec![player])),
        HashMap::new(),
    );
    assert!(creature.rebind_loot_authority_like_cpp(replacement.clone()));

    assert!(displaced.is_retired_like_cpp());
    assert!(
        creature
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&replacement)
    );
    assert_eq!(
        lease.commit_like_cpp(),
        Err(wow_loot::LootClaimCommitError::StaleGeneration),
        "a lease against the displaced Arc must not commit after rebind"
    );
    assert_eq!(
        replacement.shared_snapshot_like_cpp().unwrap().loot.coins,
        23
    );
}

#[test]
fn creature_fully_looted_reads_active_authority_without_summary_refresh() {
    let authority = OwnedLootAuthority::new();
    authority.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(17, 0, Vec::new())),
        HashMap::new(),
    );
    let mut creature = Creature::new(false);
    creature.rebind_loot_authority_like_cpp(authority.clone());
    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 0))
    );
    assert!(!creature.is_fully_looted_like_cpp());

    authority.replace_like_cpp(
        Some(owned_loot_fixture_like_cpp(0, 0, Vec::new())),
        HashMap::new(),
    );

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(17, 0)),
        "the compatibility summary remains deliberately stale"
    );
    assert!(
        creature.is_fully_looted_like_cpp(),
        "lifecycle decisions must read the active object-owned authority"
    );
}

#[test]
fn creature_is_fully_looted_checks_shared_and_personal_loot_like_cpp() {
    let looted_player = ObjectGuid::create_player(1, 7);
    let unlooted_player = ObjectGuid::create_player(1, 8);
    let mut creature = Creature::new(false);

    assert!(creature.is_fully_looted_like_cpp());
    assert_eq!(creature.shared_loot_like_cpp(), None);

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(5, 0));
    assert!(!creature.is_fully_looted_like_cpp());

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());

    creature.set_personal_loot_like_cpp(looted_player, CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());
    assert_eq!(
        creature.personal_loot_like_cpp(looted_player),
        Some(&CreatureOwnedLoot::default())
    );

    creature.set_personal_loot_like_cpp(unlooted_player, CreatureOwnedLoot::new(0, 1));
    assert!(!creature.is_fully_looted_like_cpp());

    creature.set_personal_loot_like_cpp(unlooted_player, CreatureOwnedLoot::default());
    assert!(creature.is_fully_looted_like_cpp());
}

#[test]
fn creature_loot_for_player_matches_cpp_shared_vs_personal_precedence() {
    let first = ObjectGuid::create_player(1, 7);
    let second = ObjectGuid::create_player(1, 8);
    let mut creature = Creature::new(false);

    assert_eq!(creature.loot_for_player_like_cpp(first), None);

    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(5, 0));
    assert_eq!(
        creature.loot_for_player_like_cpp(first),
        Some(&CreatureOwnedLoot::new(5, 0))
    );
    assert_eq!(
        creature.loot_for_player_like_cpp(second),
        Some(&CreatureOwnedLoot::new(5, 0))
    );

    creature.set_personal_loot_like_cpp(first, CreatureOwnedLoot::new(0, 1));
    assert_eq!(
        creature.loot_for_player_like_cpp(first),
        Some(&CreatureOwnedLoot::new(0, 1))
    );
    assert_eq!(creature.loot_for_player_like_cpp(second), None);

    creature.set_personal_loot_like_cpp(second, CreatureOwnedLoot::new(9, 0));
    assert_eq!(
        creature.loot_for_player_like_cpp(second),
        Some(&CreatureOwnedLoot::new(9, 0))
    );
}

#[test]
fn creature_clear_loot_resets_shared_and_personal_loot_like_cpp() {
    let player = ObjectGuid::create_player(1, 9);
    let mut creature = Creature::new(false);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(1, 1));
    creature.set_personal_loot_like_cpp(player, CreatureOwnedLoot::new(0, 1));

    creature.clear_loot_like_cpp();

    assert_eq!(creature.shared_loot_like_cpp(), None);
    assert_eq!(creature.personal_loot_count_like_cpp(), 0);
    assert!(creature.is_fully_looted_like_cpp());
}

#[test]
fn creature_clear_personal_loot_preserves_shared_loot_like_cpp() {
    let player = ObjectGuid::create_player(1, 10);
    let mut creature = Creature::new(false);
    creature.set_shared_loot_like_cpp(CreatureOwnedLoot::new(3, 0));
    creature.set_personal_loot_like_cpp(player, CreatureOwnedLoot::new(0, 1));

    creature.clear_personal_loot_like_cpp();

    assert_eq!(
        creature.shared_loot_like_cpp(),
        Some(&CreatureOwnedLoot::new(3, 0))
    );
    assert_eq!(creature.personal_loot_count_like_cpp(), 0);
    assert_eq!(
        creature.loot_for_player_like_cpp(player),
        Some(&CreatureOwnedLoot::new(3, 0))
    );
}

#[test]
fn creature_spell_focus_release_and_cancel_match_cpp_state_transitions() {
    let original_target = ObjectGuid::new(1, 10);
    let cast_target = ObjectGuid::new(1, 11);
    let mut creature = Creature::new(false);
    creature.unit_mut().set_target(original_target);
    creature.set_represented_spell_focus_like_cpp(700, cast_target, 2.5, true);

    assert_eq!(
        creature.spell_focus_state_like_cpp().target,
        original_target
    );
    assert_eq!(creature.unit().data().target, ObjectGuid::EMPTY);
    assert!(creature.unit().has_unit_state(UnitState::FOCUSING.bits()));

    creature.release_spell_focus_like_cpp(None, false, false, false);

    assert_eq!(creature.spell_focus_state_like_cpp().spell_id, None);
    assert_eq!(creature.spell_focus_state_like_cpp().delay_ms, 1);
    assert!(!creature.unit().has_unit_state(UnitState::FOCUSING.bits()));
    assert!(creature.has_spell_focus_like_cpp(None));

    creature.do_not_reacquire_spell_focus_target_like_cpp();

    assert_eq!(creature.spell_focus_state_like_cpp().spell_id, None);
    assert_eq!(creature.spell_focus_state_like_cpp().delay_ms, 0);
    assert!(!creature.has_spell_focus_like_cpp(None));
}

#[test]
fn creature_runtime_just_respawned_resets_represented_runtime_state() {
    let player = ObjectGuid::new(1, 3);
    let mut creature = Creature::new(false);
    creature.set_ai_identity_runtime(
        100,
        35,
        0x40,
        (UnitFlags::IMMUNE_TO_NPC | UnitFlags::IN_COMBAT).bits(),
    );
    creature.set_npc_flags2_runtime_like_cpp(0x2);
    creature.set_unit_flags2_runtime_like_cpp(UnitFlags2::FEIGN_DEATH.bits());
    creature.set_unit_flags3_runtime_like_cpp(UnitFlags3::AI_OBSTACLE.bits());
    creature.unit_mut().set_max_health(250);
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_death_state(DeathState::Corpse);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_dynamic_flag(UnitDynFlags::Lootable as u32);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_dynamic_flag(UnitDynFlags::CanSkin as u32);
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
    creature
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::empty());
    creature
        .unit_mut()
        .set_unit_flags3_like_cpp(UnitFlags3::empty());
    creature.unit_mut().set_npc_flags_like_cpp(0);
    creature.unit_mut().set_npc_flags2_like_cpp(0);
    creature.unit_mut().add_unit_state(
        UnitState::DIED.bits()
            | UnitState::CHARGING.bits()
            | UnitState::ROAMING_MOVE.bits()
            | UnitState::IGNORE_PATHFINDING.bits(),
    );
    creature.player_damage_req = 42;
    creature.cannot_reach_target = true;
    creature.cannot_reach_timer = 900;
    creature.set_respawn_time(123);
    creature.corpse_remove_time = 99;
    creature.set_pickpocket_loot_restore(777);
    creature.loot_mode = 0x4;
    creature.set_tapped_by_player(player, &[]);
    creature.set_melee_damage_school_like_cpp(wow_constants::spell::SpellSchools::Fire as u8);

    let plan = creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(creature.unit().death_state(), DeathState::Alive);
    assert_eq!(creature.unit().data().health, 250);
    assert_eq!(
        creature.unit().world().object().dynamic_flags(),
        0,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls ReplaceAllDynamicFlags(UNIT_DYNFLAG_NONE)"
    );
    assert!(
        !creature
            .unit()
            .unit_flags_like_cpp()
            .intersects(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT),
        "C++ Unit::setDeathState(JUST_RESPAWNED) removes SKINNABLE and Creature respawn removes IN_COMBAT"
    );
    assert!(
        creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IMMUNE_TO_NPC),
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags via ChooseCreatureFlags"
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH,
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags2"
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE,
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags3"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0x40, 0x2],
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads npcFlags/npcFlags2"
    );
    assert_eq!(
        creature.melee_damage_school_mask(),
        1 << (wow_constants::spell::SpellSchools::Normal as u8),
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads melee damage school from cInfo->dmgschool"
    );
    assert_eq!(
        UnitState::from_bits_truncate(creature.unit().unit_state()),
        UnitState::IGNORE_PATHFINDING,
        "C++ Creature::setDeathState(JUST_RESPAWNED) clears UNIT_STATE_ALL_ERASABLE but preserves IGNORE_PATHFINDING"
    );
    assert!(creature.tap_list().is_empty());
    assert_eq!(creature.player_damage_req(), 0);
    assert!(!creature.cannot_reach_target());
    assert_eq!(creature.cannot_reach_timer(), 0);
    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.corpse_remove_time(), 0);
    assert_eq!(creature.pickpocket_loot_restore(), 0);
    assert_eq!(creature.loot_mode(), LOOT_MODE_DEFAULT);
    assert!(creature.trigger_just_appeared());
    assert!(plan.contains(CreatureRuntimeAction::ClearTapList));
    assert!(plan.contains(CreatureRuntimeAction::ResetAi));
}

#[test]
fn creature_runtime_forced_despawn_immediate_matches_compat_and_noncompat_bridges() {
    let now = 20_000;
    let mut compat = Creature::new(false);
    compat.set_respawn_compatibility_mode(true);
    compat.set_respawn_delay(300);
    compat.set_corpse_delay(60, false);

    let plan = compat.forced_despawn_runtime(0, 42, now);

    assert_eq!(compat.unit().death_state(), DeathState::Dead);
    assert_eq!(compat.respawn_delay(), 300);
    assert_eq!(compat.corpse_delay(), 60);
    assert_eq!(compat.respawn_time(), now + 42);
    assert_eq!(compat.corpse_remove_time(), now);
    assert!(plan.contains(CreatureRuntimeAction::DestroyVisibility));
    assert!(plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));

    let mut delayed = Creature::new(false);
    let delayed_plan = delayed.forced_despawn_runtime(500, 0, now);
    assert!(delayed.runtime_state().forced_despawn_pending);
    assert!(delayed_plan.contains(CreatureRuntimeAction::RequestDelayedForcedDespawn));

    let mut non_compat = Creature::new(false);
    non_compat.set_respawn_compatibility_mode(false);
    non_compat.set_respawn_delay(55);
    let non_compat_plan = non_compat.forced_despawn_runtime(0, 0, now);
    assert_eq!(non_compat.respawn_time(), now + 55);
    assert!(non_compat.runtime_state().save_respawn_requested);
    assert!(non_compat.runtime_state().object_remove_requested);
    assert!(non_compat_plan.contains(CreatureRuntimeAction::SaveRespawnTime));
    assert!(non_compat_plan.contains(CreatureRuntimeAction::RequestObjectRemove));
}

#[test]
fn creature_corpse_removal_retires_arc_held_loot_authority() {
    let mut creature = Creature::new(false);
    creature.replace_loot_authority_like_cpp(None, HashMap::new());
    let authority = creature.loot_authority_like_cpp().clone();
    creature.unit_mut().set_death_state(DeathState::Corpse);
    assert!(!authority.is_retired_like_cpp());

    let plan = creature.remove_corpse_runtime(20_000, true, false);

    assert!(plan.contains(CreatureRuntimeAction::RemoveLoot));
    assert!(
        authority.is_retired_like_cpp(),
        "corpse removal must invalidate claims that outlive the map object"
    );
}

#[test]
fn creature_runtime_all_loot_removed_updates_corpse_and_respawn_like_trinity() {
    let now = 1_000;
    let mut creature = Creature::new(false);
    creature.set_corpse_delay(60, false);
    creature.set_respawn_delay(300);
    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(now + 100);

    let plan = creature.all_loot_removed_from_corpse(now, 0.5, false);

    assert_eq!(creature.corpse_remove_time(), now + 30);
    assert_eq!(creature.respawn_time(), now + 330);
    assert!(plan.contains(CreatureRuntimeAction::UpdateLoot));

    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(now + 1_000);
    creature.all_loot_removed_from_corpse(now, 0.5, true);
    assert_eq!(creature.corpse_remove_time(), now);
    assert_eq!(creature.respawn_time(), now + 1_000);

    creature.set_corpse_delay(60, true);
    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(0);
    creature.all_loot_removed_from_corpse(now, 0.01, false);
    assert_eq!(creature.corpse_remove_time(), now + 60);
}

#[test]
fn creature_runtime_tap_list_group_soft_cap_and_evade_clear_rules() {
    let player = ObjectGuid::new(1, 1);
    let group = [
        ObjectGuid::new(1, 2),
        ObjectGuid::new(1, 3),
        ObjectGuid::new(1, 4),
        ObjectGuid::new(1, 5),
        ObjectGuid::new(1, 6),
    ];
    let mut creature = Creature::new(false);

    creature.set_tapped_by_player(player, &group);

    assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
    assert!(creature.is_tapped_by(player));
    assert!(creature.is_tapped_by(group[0]));
    assert!(!creature.is_tapped_by(group[4]));
    assert!(creature.has_loot_recipient());

    creature.set_dont_clear_tap_list_on_evade(true);
    assert!(creature.dont_clear_tap_list_on_evade());
    creature.clear_tap_list_for_evade();
    assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
    creature.clear_tap_list();
    assert!(creature.tap_list().is_empty());

    let mut spawned_creature = Creature::new(false);
    spawned_creature.set_spawn_id(99);
    spawned_creature.set_dont_clear_tap_list_on_evade(true);
    assert!(!spawned_creature.dont_clear_tap_list_on_evade());
}

#[test]
fn creature_evading_attacks_matches_cpp_evade_or_cannot_reach() {
    let mut creature = Creature::new(false);

    assert!(!creature.is_in_evade_mode_like_cpp());
    assert!(!creature.is_evading_attacks_like_cpp());

    creature.set_in_evade_mode_like_cpp(true);
    assert!(creature.is_in_evade_mode_like_cpp());
    assert!(creature.is_evading_attacks_like_cpp());

    creature.set_in_evade_mode_like_cpp(false);
    assert!(!creature.is_evading_attacks_like_cpp());

    creature.set_cannot_reach_target_like_cpp(true);
    assert!(creature.cannot_reach_target());
    assert!(creature.is_evading_attacks_like_cpp());

    creature.cannot_reach_timer = 500;
    creature.set_cannot_reach_target_like_cpp(false);
    assert!(!creature.cannot_reach_target());
    assert_eq!(creature.cannot_reach_timer(), 0);
    assert!(!creature.is_evading_attacks_like_cpp());
}

#[test]
fn creature_lifecycle_init_entry_derives_static_flags_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.vehicle_id = None;
    record.vehicle_kit_create_input = None;
    record.template.rooted = false;
    record.template.flags_extra |= CreatureFlagsExtra::NO_XP.bits();
    record.template.static_flags[0] =
        CreatureStaticFlags::SESSILE.bits() | CreatureStaticFlags::NO_MELEE_FLEE.bits();
    record.template.type_flags |= CreatureTypeFlags::TREAT_AS_RAID_UNIT.bits();

    let creature = Creature::create_from_lifecycle(record);
    let primary =
        CreatureStaticFlags::from_bits_truncate(creature.lifecycle_metadata().static_flags[0]);
    let flags4 =
        CreatureStaticFlags4::from_bits_truncate(creature.lifecycle_metadata().static_flags[3]);

    assert!(primary.contains(CreatureStaticFlags::SESSILE));
    assert!(primary.contains(CreatureStaticFlags::NO_MELEE_FLEE));
    assert!(primary.contains(CreatureStaticFlags::NO_XP));
    assert!(!creature.can_give_experience_like_cpp());
    assert!(flags4.contains(CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS));
    assert!(creature.is_template_rooted_like_cpp());
    assert!(creature.unit().has_unit_state(UnitState::ROOT.bits()));
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::ROOT)
    );
    assert!(!creature.can_melee_like_cpp());
}

#[test]
fn creature_can_melee_reflects_primary_static_no_melee_flag_like_cpp() {
    let mut creature = Creature::new(false);
    assert!(creature.can_melee_like_cpp());

    let mut static_flags = [0; 8];
    static_flags[0] = CreatureStaticFlags::NO_MELEE_FLEE.bits();
    creature.set_static_flags_runtime_like_cpp(static_flags);

    assert!(!creature.can_melee_like_cpp());
}

#[test]
fn creature_runtime_update_plan_covers_dead_corpse_and_alive_branches() {
    let now = 50_000;
    let mut dead = Creature::new(false);
    dead.set_respawn_compatibility_mode(true);
    dead.set_respawn_time(now);
    dead.unit_mut().set_death_state(DeathState::Dead);
    let dead_plan = dead.runtime_update_plan(1, now, CreatureRuntimeUpdateContext::default());
    assert!(dead_plan.contains(CreatureRuntimeAction::ResetAi));
    assert_eq!(dead.unit().death_state(), DeathState::Alive);

    let mut corpse = Creature::new(false);
    corpse.set_respawn_compatibility_mode(true);
    corpse.unit_mut().set_death_state(DeathState::Corpse);
    corpse.corpse_remove_time = now;
    let corpse_plan = corpse.runtime_update_plan(
        1,
        now,
        CreatureRuntimeUpdateContext {
            has_loot: true,
            ..CreatureRuntimeUpdateContext::default()
        },
    );
    assert!(corpse_plan.contains(CreatureRuntimeAction::UpdateLoot));
    assert!(corpse_plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));
    assert_eq!(corpse.unit().death_state(), DeathState::Dead);

    let mut alive = Creature::new(false);
    alive.boundary_check_time = 10;
    alive.combat_pulse_delay = 2;
    alive.combat_pulse_time = 1;
    alive.regen_timer = 1;
    alive.cannot_reach_timer = CREATURE_NOPATH_EVADE_TIME_MS - 5;
    let alive_plan = alive.runtime_update_plan(
        10,
        now,
        CreatureRuntimeUpdateContext {
            is_engaged: true,
            is_dungeon: true,
            has_map_players: true,
            cannot_reach_target: true,
            ..CreatureRuntimeUpdateContext::default()
        },
    );
    assert!(alive_plan.contains(CreatureRuntimeAction::NotifyJustAppeared));
    assert!(alive_plan.contains(CreatureRuntimeAction::BoundaryCheck));
    assert!(alive_plan.contains(CreatureRuntimeAction::CombatPulse));
    assert!(alive_plan.contains(CreatureRuntimeAction::RegeneratePower));
    assert!(alive_plan.contains(CreatureRuntimeAction::Evade(
        CreatureRuntimeEvadeReason::NoPath
    )));
}
