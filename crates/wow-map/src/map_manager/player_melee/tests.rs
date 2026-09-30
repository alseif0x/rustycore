//! Direct contracts for the moved player-to-creature transition.
use super::*;
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use wow_constants::DeathState;
use wow_entities::CreatureAiState;

const SEED: u64 = 0xB6_504C_4159;

fn player(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn actor(health: u32) -> WorldCreature {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 1, 804_001,
    );
    let mut actor = WorldCreature::new(
        guid, 9999, Position::xyz(10.0, 10.0, 0.0),
        health, 25, 3, 5, 20.0, 100, 14, 0, 0,
    );
    actor.seed_runtime_rng_like_cpp(SEED);
    actor
}

fn ready_actor(health: u32, minimum: u32, maximum: u32) -> WorldCreature {
    let mut actor = actor(health);
    actor.advance_runtime_clock_like_cpp(2_000);
    let ai = actor.creature.ai_ownership_mut();
    ai.min_damage = minimum;
    ai.max_damage = maximum;
    ai.last_swing_ms = 0;
    ai.swing_timer_ms = 2_000;
    actor
}

fn swing(damage: u32) -> PlayerMeleeSwing {
    PlayerMeleeSwing::hit_like_cpp(damage)
}

fn assert_rng_untouched(actor: &mut WorldCreature) {
    let mut expected = StdRng::seed_from_u64(SEED);
    assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u32(), expected.next_u32());
}

#[test]
fn dead_admission_changes_neither_engagement_nor_rng_clock_or_motion() {
    let mut actor = actor(40);
    actor.creature.unit_mut().set_health(0);
    let revision = actor.creature.unit().health_state_revision_like_cpp();
    let lifecycle = actor.creature.loot_lifecycle_revision_like_cpp();
    assert!(actor.apply_player_melee(player(1), &[player(2)], None).is_none());
    assert_eq!(actor.state(), CreatureAiState::Idle);
    assert_eq!(actor.creature.unit().attacking(), None);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
    assert_eq!(actor.creature.unit().health_state_revision_like_cpp(), revision);
    assert_eq!(actor.creature.loot_lifecycle_revision_like_cpp(), lifecycle);
    assert_rng_untouched(&mut actor);
}

#[test]
fn unavailable_fallback_enters_combat_before_readiness_without_recording() {
    let mut actor = actor(40);
    actor.creature.ai_ownership_mut().swing_timer_ms = 2_000;
    assert!(actor.apply_player_melee(player(1), &[], None).is_none());
    assert_eq!(actor.state(), CreatureAiState::InCombat);
    assert_eq!(actor.creature.unit().attacking(), Some(player(1)));
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 0);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 0);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
    assert_eq!(actor.current_hp(), 40);
    assert_rng_untouched(&mut actor);
}

#[test]
fn invalid_fallback_bounds_retain_engagement_and_reject_without_draw_or_record() {
    let mut actor = ready_actor(40, 7, 3);
    assert!(actor.apply_player_melee(player(1), &[], None).is_none());
    assert_eq!(actor.state(), CreatureAiState::InCombat);
    assert!(!actor.runtime_rng_authority_complete_like_cpp());
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 0);
    assert_eq!(actor.current_hp(), 40);
    assert_rng_untouched(&mut actor);
}

#[test]
fn equal_bound_fallback_draws_once_and_records_existing_clock_and_attack_time() {
    let mut actor = ready_actor(40, 3, 3);
    actor.create_data.base_attack_time = 1_750;
    let hit = actor.apply_player_melee(player(1), &[], None).unwrap();
    assert_eq!(hit.swings, vec![(3, false, -1)]);
    assert_eq!(hit.swing_presentations, vec![(2, 1, 0, 3)]);
    assert_eq!(actor.current_hp(), 37);
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 2_000);
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 1_750);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 2_000);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
    let mut expected = StdRng::seed_from_u64(SEED);
    let _ = expected.next_u32();
    assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u32(), expected.next_u32());
}

#[test]
fn ranged_fallback_uses_the_existing_inclusive_rng_stream() {
    let mut actor = ready_actor(40, 3, 9);
    let mut expected = StdRng::seed_from_u64(SEED);
    let damage: u32 = expected.gen_range(3..=9);
    let hit = actor.apply_player_melee(player(1), &[], None).unwrap();
    assert_eq!(hit.swings, vec![(damage, false, -1)]);
    assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u32(), expected.next_u32());
}

#[test]
fn zero_bound_fallback_keeps_the_minimum_one_damage_and_one_draw() {
    let mut actor = ready_actor(40, 0, 0);
    let hit = actor.apply_player_melee(player(1), &[], None).unwrap();
    assert_eq!(hit.swings, vec![(1, false, -1)]);
    assert_eq!(hit.swing_presentations, vec![(2, 1, 0, 1)]);
    let mut expected = StdRng::seed_from_u64(SEED);
    let _ = expected.next_u32();
    assert_eq!(actor.runtime.runtime_rng_like_cpp.next_u32(), expected.next_u32());
}

#[test]
fn empty_canonical_batch_bypasses_readiness_bad_rng_bounds_and_swing_record() {
    let mut actor = actor(40);
    actor.creature.ai_ownership_mut().min_damage = 7;
    actor.creature.ai_ownership_mut().max_damage = 3;
    actor.creature.ai_ownership_mut().last_swing_ms = 11;
    let hit = actor.apply_player_melee(player(1), &[player(2)], Some(&[])).unwrap();
    assert!(hit.swings.is_empty());
    assert!(hit.swing_presentations.is_empty());
    assert_eq!((hit.entry, hit.level, hit.died, hit.move_stop), (9999, 25, false, None));
    assert_eq!(hit.values_update, actor.creature.unit().values_update());
    assert_eq!(actor.state(), CreatureAiState::InCombat);
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 11);
    assert!(actor.runtime_rng_authority_complete_like_cpp());
    assert!(actor.creature.tap_list().is_empty());
    assert_eq!(actor.current_hp(), 40);
    assert_rng_untouched(&mut actor);
}

#[test]
fn supplied_nonempty_batch_never_consumes_the_creature_timer_or_rng() {
    let mut actor = ready_actor(40, 7, 3);
    actor.creature.ai_ownership_mut().last_swing_ms = 19;
    let hit = actor.apply_player_melee(player(1), &[], Some(&[swing(4)])).unwrap();
    assert_eq!(hit.swings, vec![(4, false, -1)]);
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 19);
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 2_000);
    assert!(actor.runtime_rng_authority_complete_like_cpp());
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 2_000);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), 0);
    assert_rng_untouched(&mut actor);
}

#[test]
fn avoided_zero_damage_preserves_presentation_without_tap_health_or_threat() {
    let mut actor = actor(40);
    let revision = actor.creature.unit().health_state_revision_like_cpp();
    let avoided = PlayerMeleeSwing {
        damage: 0, original_damage: 11, blocked: 9, hit_info: 0x10, victim_state: 3,
    };
    let hit = actor.apply_player_melee(player(1), &[player(2)], Some(&[avoided])).unwrap();
    assert_eq!(hit.swings, vec![(0, false, -1)]);
    assert_eq!(hit.swing_presentations, vec![(0x10, 3, 9, 11)]);
    assert_eq!(actor.current_hp(), 40);
    assert_eq!(actor.creature.unit().health_state_revision_like_cpp(), revision);
    assert!(actor.creature.tap_list().is_empty());
    assert_eq!(actor.creature.unit().subsystems().combat.threat_value(player(1)), None);
    assert_rng_untouched(&mut actor);
}

#[test]
fn nonlethal_batch_adds_full_damage_threat_in_order_and_preserves_other_threat() {
    let mut actor = actor(40);
    {
        let combat = &mut actor.creature.unit_mut().subsystems_mut().combat;
        combat.add_threat(player(1), 3.0);
        combat.add_threat(player(2), 8.0);
    }
    let revision = actor.creature.unit().health_state_revision_like_cpp();
    let authority = actor.creature.unit().health_state_revision_authority_like_cpp();
    let lifecycle = actor.creature.loot_lifecycle_revision_like_cpp();
    let hit = actor.apply_player_melee(player(1), &[], Some(&[swing(4), swing(7)])).unwrap();
    assert_eq!(hit.swings, vec![(4, false, -1), (7, false, -1)]);
    assert_eq!(actor.current_hp(), 29);
    assert_eq!(actor.creature.unit().subsystems().combat.threat_value(player(1)), Some(14.0));
    assert_eq!(actor.creature.unit().subsystems().combat.threat_value(player(2)), Some(8.0));
    assert_eq!(actor.creature.unit().health_state_revision_like_cpp(), revision + 2);
    assert!(actor.creature.unit().shares_health_state_revision_authority_like_cpp(&authority));
    assert_eq!(actor.creature.loot_lifecycle_revision_like_cpp(), lifecycle);
    assert_eq!(hit.values_update, actor.creature.unit().values_update());
}

#[test]
fn tap_group_preserves_player_first_input_order_dedup_empty_and_soft_cap() {
    let mut actor = actor(40);
    let group = [player(2), ObjectGuid::EMPTY, player(2), player(3), player(4), player(5), player(6)];
    actor.apply_player_melee(player(1), &group, Some(&[swing(1)])).unwrap();
    assert_eq!(actor.creature.tap_list(), &[player(1), player(2), player(3), player(4), player(5)]);
    assert_eq!(actor.creature.tap_list().len(), wow_entities::CREATURE_TAPPERS_SOFT_CAP);
    actor.apply_player_melee(player(7), &[player(8)], Some(&[swing(1)])).unwrap();
    assert_eq!(actor.creature.tap_list(), &[player(1), player(2), player(3), player(4), player(5)]);
    assert!(actor.creature.has_loot_recipient());
}

#[test]
fn lethal_batch_clears_all_threat_attackers_and_stops_before_later_swings() {
    let mut actor = actor(2);
    let lifecycle = actor.creature.loot_lifecycle_revision_like_cpp();
    let revision = actor.creature.unit().health_state_revision_like_cpp();
    {
        let combat = &mut actor.creature.unit_mut().subsystems_mut().combat;
        combat.add_threat(player(2), 8.0);
        combat.add_attacker(player(2));
    }
    let hit = actor.apply_player_melee(player(1), &[], Some(&[swing(0), swing(3), swing(9)])).unwrap();
    assert_eq!(hit.swings, vec![(0, false, -1), (3, true, 1)]);
    assert_eq!(hit.swing_presentations, vec![(2, 1, 0, 0), (2, 1, 0, 3)]);
    assert!(hit.died);
    assert_eq!(actor.current_hp(), 0);
    assert_eq!(actor.state(), CreatureAiState::Dead);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Alive);
    assert_eq!(actor.creature.unit().health_state_revision_like_cpp(), revision + 1);
    assert_eq!(actor.creature.loot_lifecycle_revision_like_cpp(), lifecycle + 1);
    assert!(actor.creature.unit().subsystems().combat.sorted_threat_guids().is_empty());
    assert!(actor.creature.unit().subsystems().combat.attackers.is_empty());
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 0);
    assert_eq!(hit.values_update, actor.creature.unit().values_update());
    assert_rng_untouched(&mut actor);
}

#[test]
fn overdamage_preserves_the_original_unsigned_subtraction_then_i32_cast() {
    let mut actor = actor(1);
    let hit = actor.apply_player_melee(player(1), &[], Some(&[swing(u32::MAX)])).unwrap();
    assert_eq!(hit.swings, vec![(u32::MAX, true, -2)]);
    assert_eq!(hit.swing_presentations, vec![(2, 1, 0, u32::MAX)]);
}

#[test]
fn lethal_fallback_still_records_the_existing_clock_after_death() {
    let mut actor = ready_actor(2, 3, 3);
    actor.create_data.base_attack_time = 1_750;
    let hit = actor.apply_player_melee(player(1), &[], None).unwrap();
    assert_eq!(hit.swings, vec![(3, true, 1)]);
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 2_000);
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 1_750);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), 2_000);
}

#[test]
fn fallback_without_template_attack_time_preserves_the_timer_minimum_one() {
    let mut actor = ready_actor(40, 3, 3);
    actor.create_data.base_attack_time = 0;
    actor.creature.ai_ownership_mut().swing_timer_ms = 0;
    actor.apply_player_melee(player(1), &[], None).unwrap();
    assert_eq!(actor.creature.ai_ownership().swing_timer_ms, 1);
    assert_eq!(actor.creature.ai_ownership().last_swing_ms, 2_000);
}

#[test]
fn lethal_spline_stop_updates_position_before_values_without_advancing_clock() {
    let mut actor = ready_actor(2, 3, 3);
    actor.enter_combat(player(1));
    let (_, spline) = actor.begin_move_spline_like_cpp(Position::xyz(40.0, 10.0, 0.0)).unwrap();
    let old_id = spline.id();
    actor.advance_runtime_clock_like_cpp(100);
    let clock = actor.runtime_elapsed_ms_like_cpp();
    let motion_ticks = actor.runtime_motion_master_ticks_like_cpp();
    let hit = actor.apply_player_melee(player(1), &[], Some(&[swing(3)])).unwrap();
    let (position, spline_id) = hit.move_stop.unwrap();
    assert_eq!(position, actor.position());
    assert!(position.x > 10.0 && position.x < 40.0);
    assert!(spline_id > old_id);
    assert!(actor.runtime.active_move_spline.is_none());
    assert!(actor.creature.ai_ownership().move_target.is_none());
    assert_eq!(hit.values_update, actor.creature.unit().values_update());
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), clock);
    assert_eq!(actor.runtime_motion_master_ticks_like_cpp(), motion_ticks);
}
