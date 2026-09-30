// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical absorb-pool commit regressions, authored without execution.

use super::{
    AppliedAuraRef, AuraApplicationLikeCpp, AuraCastProvenanceLikeCpp, AuraSubsystem,
    RepresentedAuraEffectAmountLikeCpp,
};
use std::time::Instant;
use wow_core::ObjectGuid;

fn runtime_application(
    slot: u8,
    amounts: Vec<RepresentedAuraEffectAmountLikeCpp>,
) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id: 20,
        difficulty_id: 0,
        caster_guid: ObjectGuid::create_player(1, 7),
        slot,
        duration_total: 30_000,
        duration_remaining: 20_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 3,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 99,
        represented_effect_amounts: amounts,
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

fn amount(effect_index: u8, amount: i32) -> RepresentedAuraEffectAmountLikeCpp {
    RepresentedAuraEffectAmountLikeCpp {
        effect_index,
        amount,
    }
}

#[test]
fn runtime_absorb_commit_updates_only_first_match_and_clamps_negative_remainder() {
    let mut auras = AuraSubsystem::default();
    auras.insert_runtime_application_like_cpp(runtime_application(
        3,
        vec![amount(1, 25), amount(0, 40), amount(1, 60)],
    ));
    auras.set_runtime_absorb_amount(3, 1, -10);
    let aura = auras.runtime_application_like_cpp(3).unwrap();
    assert_eq!(
        aura.represented_effect_amounts,
        vec![amount(1, 0), amount(0, 40), amount(1, 60)]
    );
    assert_eq!(aura.represented_amount, 99);
    assert_eq!(aura.duration_remaining, 20_000);
    assert_eq!(aura.effect_mask, 3);
}

#[test]
fn runtime_absorb_commit_appends_missing_effect_without_reordering_existing_amounts() {
    let mut auras = AuraSubsystem::default();
    auras.insert_runtime_application_like_cpp(runtime_application(
        3,
        vec![amount(1, 25), amount(0, 40)],
    ));
    auras.set_runtime_absorb_amount(3, 31, 8);
    auras.set_runtime_absorb_amount(3, 32, -1);
    assert_eq!(
        auras
            .runtime_application_like_cpp(3)
            .unwrap()
            .represented_effect_amounts,
        vec![amount(1, 25), amount(0, 40), amount(31, 8), amount(32, 0)],
    );
}

#[test]
fn runtime_absorb_commit_preserves_missing_slot_and_invalidates_even_unchanged_amount() {
    let mut auras = AuraSubsystem::default();
    auras.insert_runtime_application_like_cpp(runtime_application(3, vec![amount(0, 10)]));
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    auras.spell_cast_log_aura_authority_inert_like_cpp = true;
    auras.set_runtime_absorb_amount(99, 0, 10);
    assert!(auras.spell_hit_aura_authority_inert_like_cpp);
    assert!(auras.spell_cast_log_aura_authority_inert_like_cpp);
    assert!(auras.runtime_application_like_cpp(99).is_none());
    auras.set_runtime_absorb_amount(3, 0, 10);
    assert!(!auras.spell_hit_aura_authority_inert_like_cpp);
    assert!(!auras.spell_cast_log_aura_authority_inert_like_cpp);
    assert_eq!(
        auras
            .runtime_application_like_cpp(3)
            .unwrap()
            .represented_effect_amounts,
        vec![amount(0, 10)]
    );
}

#[test]
fn applied_absorb_lookup_preserves_first_reference_mask_and_range_checks() {
    let caster = ObjectGuid::create_player(1, 7);
    let first = AppliedAuraRef::new(20, caster, 3, 1);
    let second = AppliedAuraRef::new(21, caster, 3, 3);
    let high = AppliedAuraRef::new(22, caster, 4, 1_u32 << 31);
    let mut auras = AuraSubsystem::default();
    auras.add_applied(first);
    auras.add_applied(second);
    auras.add_applied(high);
    assert_eq!(auras.find_applied_absorb_effect(3, 0), Some(first));
    assert_eq!(auras.find_applied_absorb_effect(3, 1), Some(second));
    assert_eq!(auras.find_applied_absorb_effect(4, 31), Some(high));
    assert_eq!(auras.find_applied_absorb_effect(4, 32), None);
    assert_eq!(auras.find_applied_absorb_effect(4, u8::MAX), None);
    assert_eq!(auras.find_applied_absorb_effect(99, 0), None);
    assert_eq!(auras.applied_auras, vec![first, second, high]);
}

#[test]
fn surviving_applied_absorb_commit_uses_exact_key_and_never_inserts_missing_amount() {
    let caster = ObjectGuid::create_player(1, 7);
    let combined = AppliedAuraRef::new(20, caster, 3, 3);
    let single = AppliedAuraRef::new(20, caster, 3, 1);
    let mut auras = AuraSubsystem::default();
    auras.add_applied(combined);
    auras.applied_aura_amounts.insert(single, 40);
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    auras.commit_applied_absorb_amount(combined, 12, false);
    assert!(!auras.applied_aura_amounts.contains_key(&combined));
    assert_eq!(auras.applied_aura_amounts.get(&single), Some(&40));
    assert!(auras.spell_hit_aura_authority_inert_like_cpp);
    auras.commit_applied_absorb_amount(single, -10, false);
    assert_eq!(auras.applied_aura_amounts.get(&single), Some(&0));
    assert!(auras.has_applied(combined));
    assert_eq!(auras.removed_count(), 0);
    assert!(auras.spell_hit_aura_authority_inert_like_cpp);
}

#[test]
fn applied_absorb_commit_obeys_supplied_removal_flag_instead_of_remaining_amount() {
    let applied = AppliedAuraRef::new(20, ObjectGuid::create_player(1, 7), 3, 1);
    let mut auras = AuraSubsystem::default();
    auras.register_applied_aura_modifier_like_cpp(applied, 69, 40);
    auras.set_visible(3, applied.aura_ref());
    auras.commit_applied_absorb_amount(applied, 0, false);
    assert!(auras.has_applied(applied));
    assert_eq!(auras.applied_aura_amounts.get(&applied), Some(&0));
    assert_eq!(auras.visible_auras.get(&3), Some(&applied.aura_ref()));
    auras.commit_applied_absorb_amount(applied, 40, true);
    assert!(!auras.has_applied(applied));
    assert!(!auras.applied_aura_amounts.contains_key(&applied));
    assert!(!auras.visible_auras.contains_key(&3));
    assert_eq!(auras.removed_count(), 1);
}

#[test]
fn exhausted_applied_absorb_commit_keeps_group_order_and_clears_only_original_visible_slot() {
    let caster = ObjectGuid::create_player(1, 7);
    let other_caster = ObjectGuid::create_player(1, 8);
    let first = AppliedAuraRef::new(20, caster, 3, 1);
    let covered = AppliedAuraRef::new(20, caster, 7, 2);
    let unrelated_caster = AppliedAuraRef::new(20, other_caster, 3, 4);
    let unrelated_spell = AppliedAuraRef::new(21, caster, 9, 1);
    let mut auras = AuraSubsystem::default();
    for applied in [first, covered, unrelated_caster, unrelated_spell] {
        auras.register_applied_aura_effect_like_cpp(applied, 69, 40, 1);
    }
    auras.register_applied_aura(first, Some(2), 4, 8);
    auras.register_applied_aura(covered, Some(2), 4, 8);
    auras.set_visible(3, first.aura_ref());
    auras.set_visible(7, covered.aura_ref());
    let provenance = AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::new(6, 77),
        spell_visual_id: 9_001,
    };
    auras.set_aura_cast_provenance_like_cpp(3, provenance);
    auras.set_aura_cast_provenance_like_cpp(7, provenance);
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    auras.commit_applied_absorb_amount(first, 0, true);
    assert_eq!(auras.applied_auras, vec![unrelated_caster, unrelated_spell]);
    assert_eq!(
        auras.applied_aura_types.get(&69),
        Some(&vec![unrelated_caster, unrelated_spell])
    );
    assert!(!auras.applied_aura_amounts.contains_key(&first));
    assert!(!auras.applied_aura_amounts.contains_key(&covered));
    assert!(!auras.applied_aura_misc_values.contains_key(&first));
    assert!(!auras.applied_aura_misc_values.contains_key(&covered));
    assert_eq!(auras.applied_aura_amounts.get(&unrelated_caster), Some(&40));
    assert_eq!(auras.applied_aura_amounts.get(&unrelated_spell), Some(&40));
    assert!(!auras.has_interrupt_flag(4));
    assert!(!auras.has_interrupt_flag2(8));
    assert!(!auras.aura_state_auras.contains_key(&2));
    assert_eq!(
        auras.removed_auras,
        vec![first.aura_ref(), covered.aura_ref()]
    );
    assert_eq!(auras.removed_count(), 2);
    assert!(!auras.visible_auras.contains_key(&3));
    // Retain the existing clearing of just the original slot, including the
    // grouped application's visible entry at another slot.
    assert_eq!(auras.visible_auras.get(&7), Some(&covered.aura_ref()));
    assert!(!auras.visible_auras_to_update.contains(&3));
    assert!(auras.visible_auras_to_update.contains(&7));
    assert_eq!(
        auras.aura_cast_provenance_like_cpp(3),
        AuraCastProvenanceLikeCpp::default()
    );
    assert_eq!(
        auras.aura_cast_provenance_like_cpp(7),
        AuraCastProvenanceLikeCpp::default()
    );
    assert!(!auras.spell_hit_aura_authority_inert_like_cpp);
}
