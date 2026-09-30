//! Real WorldCreature transitions; no packet/catalog/session facsimile.
use super::*;
use wow_constants::{DeathState, UnitDynFlags, UnitFlags};
use wow_core::{Position, guid::HighGuid};
use wow_entities::{Creature, CreatureAiState};

fn actor() -> WorldCreature {
    let mut creature = Creature::new(false);
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 1, 0, 42, 901);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().set_map(1, 0).unwrap();
    creature.unit_mut().world_mut().relocate(Position::xyz(10.0, 20.0, 30.0));
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.unit_mut().subsystems_mut().combat.initialize_threat_list_capability(true);
    let data = WorldCreature::create_data_from_canonical_like_cpp(&creature);
    WorldCreature::from_canonical(creature, data)
}

fn hit(actor: &mut WorldCreature, amount: u32, suppress: bool, no_initial: bool)
    -> CreatureDamageOutcome
{
    let mut damage = actor.begin_represented_damage().unwrap().apply(
        amount, ObjectGuid::create_player(1, 1), None, suppress, no_initial, 2.0, 0.5,
    );
    if damage.died() { let _ = damage.stop_after_kill(); }
    damage.finish()
}

#[test]
fn dead_admission_does_not_tap_damage_or_touch_clocks() {
    let mut actor = actor();
    actor.creature.unit_mut().set_health(0);
    let before = (
        actor.creature.unit().health_state_revision_like_cpp(),
        actor.creature.loot_lifecycle_revision_like_cpp(),
        actor.runtime_elapsed_ms_like_cpp(),
    );
    assert!(actor.begin_represented_damage().is_none());
    assert!(actor.creature.tap_list().is_empty());
    assert_eq!(actor.current_hp(), 0);
    assert_eq!((
        actor.creature.unit().health_state_revision_like_cpp(),
        actor.creature.loot_lifecycle_revision_like_cpp(),
        actor.runtime_elapsed_ms_like_cpp(),
    ), before);
}

#[test]
fn nonlethal_damage_taps_in_group_order_and_uses_original_threat_product() {
    let mut actor = actor();
    let caster = ObjectGuid::create_player(1, 1);
    let second = ObjectGuid::create_player(1, 2);
    let third = ObjectGuid::create_player(1, 3);
    let group = [second, caster, ObjectGuid::EMPTY, third, second];
    let damage = actor.begin_represented_damage().unwrap().apply(
        10, caster, Some((caster, &group)), false, false, 2.0, 0.25,
    );
    assert!(!damage.died());
    let result = damage.finish();
    assert_eq!(result.pre_hit_health, 100);
    assert_eq!(result.threat_value, Some(5.0));
    assert!(result.newly_engaged);
    assert_eq!(actor.current_hp(), 90);
    assert_eq!(actor.creature.tap_list(), &[caster, second, third]);
    assert_eq!(actor.creature.ai_ownership().combat_target, Some(caster));
}

#[test]
fn zero_and_suppressed_hits_preserve_damage_and_skip_engagement() {
    for (amount, suppress) in [(0, false), (10, true)] {
        let mut actor = actor();
        let result = hit(&mut actor, amount, suppress, false);
        assert_eq!(result.pre_hit_health, 100);
        assert_eq!(actor.current_hp(), 100 - amount);
        assert_eq!(result.threat_value, None);
        assert!(!result.newly_engaged);
        assert_eq!(actor.creature.ai_ownership().combat_target, None);
    }
}

#[test]
fn no_initial_threat_gate_is_sampled_after_damage_and_respects_existing_combat() {
    let mut actor = actor();
    let first = hit(&mut actor, 10, false, true);
    assert_eq!(actor.current_hp(), 90);
    assert_eq!(first.threat_value, None);
    assert!(!first.newly_engaged);
    actor.enter_combat(ObjectGuid::create_player(1, 2));
    let second = hit(&mut actor, 10, false, true);
    assert_eq!(actor.current_hp(), 80);
    assert_eq!(second.threat_value, Some(10.0));
    assert!(!second.newly_engaged);
    assert_eq!(actor.creature.ai_ownership().combat_target, Some(ObjectGuid::create_player(1, 2)));
}

#[test]
fn dropping_applied_phase_keeps_damage_tap_and_deferred_death_effects() {
    let mut actor = actor();
    let caster = ObjectGuid::create_player(1, 1);
    actor.advance_runtime_clock_like_cpp(37);
    let clock = actor.runtime_elapsed_ms_like_cpp();
    let applied = actor.begin_represented_damage().unwrap().apply(
        100, caster, Some((caster, &[])), false, false, 1.0, 1.0,
    );
    assert!(applied.died());
    drop(applied);
    assert_eq!(actor.current_hp(), 0);
    assert_eq!(actor.creature.tap_list(), &[caster]);
    assert_eq!(actor.creature.ai_state(), CreatureAiState::Dead);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Alive);
    assert_eq!(actor.creature.ai_ownership().death_time_ms, Some(clock));
    assert_eq!(actor.creature.ai_ownership().corpse_despawn_at_ms, None);
    assert_eq!(actor.runtime_elapsed_ms_like_cpp(), clock);
}

#[test]
fn lethal_phase_stops_before_values_and_keeps_pre_hit_health_without_threat() {
    let mut actor = actor();
    let entry = actor.entry();
    let mut applied = actor.begin_represented_damage().unwrap().apply(
        150, ObjectGuid::create_player(1, 1), None, false, false, 2.0, 0.5,
    );
    assert!(applied.died());
    assert_eq!(applied.entry(), entry);
    assert!(applied.stop_after_kill().is_none());
    let result = applied.finish();
    assert_eq!(result.pre_hit_health, 100);
    assert_eq!(result.threat_value, None);
    assert!(!result.newly_engaged);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Alive);
    assert_eq!(actor.current_hp(), 0);
}

#[test]
fn finalization_resets_dynamic_flags_then_applies_only_resolved_corpse_flags() {
    for (lootable, can_skin) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut actor = actor();
        actor.creature.unit_mut().world_mut().object_mut().set_dynamic_flag(UnitDynFlags::Lootable as u32);
        let _ = hit(&mut actor, 100, false, false);
        let clock = actor.runtime_elapsed_ms_like_cpp();
        let _ = actor.finalize_represented_kill(lootable, can_skin);
        assert_eq!(actor.creature.unit().death_state(), DeathState::Corpse);
        let object = actor.creature.unit().world().object();
        assert_eq!(object.has_dynamic_flag(UnitDynFlags::Lootable as u32), lootable);
        assert_eq!(object.has_dynamic_flag(UnitDynFlags::CanSkin as u32), can_skin);
        assert_eq!(actor.creature.unit().unit_flags_like_cpp().contains(UnitFlags::SKINNABLE), can_skin);
        assert!(actor.creature.runtime_state().save_respawn_requested);
        assert_eq!(actor.runtime_elapsed_ms_like_cpp(), clock);
        let deadline = actor.creature.ai_ownership().corpse_despawn_at_ms;
        assert!(deadline.is_some());
        let _ = actor.finalize_represented_kill(lootable, can_skin);
        assert_eq!(actor.creature.ai_ownership().corpse_despawn_at_ms, deadline);
        assert_eq!(actor.runtime_elapsed_ms_like_cpp(), clock);
    }
}

#[test]
fn live_finalization_retains_existing_completion_noop_and_flag_application() {
    let mut actor = actor();
    let _ = actor.finalize_represented_kill(true, false);
    assert_eq!(actor.current_hp(), 100);
    assert_eq!(actor.creature.unit().death_state(), DeathState::Alive);
    assert!(actor.creature.unit().world().object().has_dynamic_flag(UnitDynFlags::Lootable as u32));
    assert!(!actor.creature.runtime_state().save_respawn_requested);
}
