// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime construction and canonical commit regressions, authored without execution.

use std::{cell::RefCell, time::Instant};
use wow_core::ObjectGuid;
use super::{
    AuraCastProvenanceLikeCpp, AuraSubsystem, RepresentedAuraEffectAmountLikeCpp,
    RepresentedAuraEffectLikeCpp,
};
use wow_constants::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE as STAT;

#[test]
fn stat_preparation_calculates_selected_borrowed_rows_before_attribute_difficulty_and_clock() {
    let rows = [(1, STAT, 25, 9, 4), (0, 0, 99, 0, 0), (31, STAT, -10, 8, 0),
                (32, STAT, 77, 0, 0), (1, STAT, 40, 7, 1)];
    let trace = RefCell::new(Vec::new());
    let instant = Instant::now();
    let (aura, amounts, modifies, health) = AuraSubsystem::build_stat_runtime_application(
        20, ObjectGuid::EMPTY, 7, 5_000, 0x101, (1 << 1) | (1 << 31),
        |_| { trace.borrow_mut().push("select"); Some(rows.as_slice()) },
        |r| (r.0, r.1, r.3, r.4),
        |r| {
            assert!(rows.iter().any(|original| std::ptr::eq(original, r)));
            trace.borrow_mut().push("base");
            r.2
        },
        |_| { trace.borrow_mut().push("attribute"); true },
        || { trace.borrow_mut().push("difficulty"); 3 },
        || { trace.borrow_mut().push("clock"); instant },
    );
    assert_eq!(*trace.borrow(), ["select", "base", "base", "base", "attribute", "difficulty", "clock"]);
    assert_eq!(amounts, [
        RepresentedAuraEffectAmountLikeCpp { effect_index: 1, amount: 25 },
        RepresentedAuraEffectAmountLikeCpp { effect_index: 31, amount: -10 },
        RepresentedAuraEffectAmountLikeCpp { effect_index: 1, amount: 40 },
    ]);
    assert_eq!(aura.represented_effect_amounts, amounts);
    assert_eq!(aura.represented_amount, 25);
    assert_eq!(aura.represented_misc_value, Some(4));
    assert_eq!(aura.represented_multiplier, 1.25);
    assert_eq!(aura.difficulty_id, 3);
    assert_eq!(aura.applied_at, instant);
    assert!(modifies && health);
    assert!(AuraSubsystem::default().runtime_applications_like_cpp().is_empty());
}

#[test]
fn stat_preparation_preserves_missing_rows_defaults_and_late_attribute_read() {
    let trace = RefCell::new(Vec::new());
    let (aura, amounts, modifies, health) =
        AuraSubsystem::build_stat_runtime_application::<(u32, i32, i32, i32)>(
            20, ObjectGuid::EMPTY, 0, 0, 0, u32::MAX,
            |_| { trace.borrow_mut().push("select"); None },
            |r| *r,
            |_| panic!("missing rows must not calculate"),
            |_| { trace.borrow_mut().push("attribute"); true },
            || { trace.borrow_mut().push("difficulty"); 2 },
            || { trace.borrow_mut().push("clock"); Instant::now() },
        );
    assert_eq!(*trace.borrow(), ["select", "attribute", "difficulty", "clock"]);
    assert!(amounts.is_empty() && !modifies && !health);
    assert_eq!(aura.represented_effect, None);
    assert_eq!(aura.represented_amount, 0);
    assert_eq!(aura.represented_misc_value, None);
    assert_eq!(aura.represented_multiplier, 1.0);
}

#[test]
fn stat_preparation_retains_negative_multiplier_and_health_stat_mask_rules() {
    for (ability, mask, health) in [(true, 0, true), (true, 4, true),
                                  (true, 1, false), (false, 4, false)] {
        let row = [(0, STAT, 0, mask)];
        let (aura, _, modifies, preserve) = AuraSubsystem::build_stat_runtime_application(
            20, ObjectGuid::EMPTY, 0, 30, 1, 1, |_| Some(row.as_slice()),
            |r| *r, |_| -125, |_| ability, || 0, Instant::now,
        );
        assert!(modifies);
        assert_eq!(preserve, health);
        assert_eq!(aura.represented_multiplier, -0.25);
        assert_eq!(aura.represented_amount, -125);
    }
}

#[test]
fn single_variants_preserve_raw_amounts_calculated_mount_and_variant_metadata() {
    let row = (2, -25, 900);
    let caster = ObjectGuid::create_player(1, 7);
    let instant = Instant::now();
    let focus = AuraSubsystem::build_focus_runtime_application(
        20, caster, 3, &row, |r| *r, || 4, || instant,
    );
    let modifier = AuraSubsystem::build_modifier_runtime_application(
        20, caster, 3, &row, RepresentedAuraEffectLikeCpp::Hover, 8_000,
        |r| *r, || 4, || instant,
    );
    let mounted = AuraSubsystem::build_mounted_runtime_application(
        20, caster, 3, &row, 77, |r| *r, || 4, || instant,
    );
    let xp = AuraSubsystem::build_xp_runtime_application(
        20, caster, 3, &row, |r| *r, || 4, || instant,
    );
    for aura in [&focus, &modifier, &mounted, &xp] {
        assert_eq!(aura.effect_mask, 4);
        assert_eq!(aura.aura_flags, 1);
        assert_eq!(aura.difficulty_id, 4);
        assert_eq!(aura.represented_effect_amounts,
            [RepresentedAuraEffectAmountLikeCpp { effect_index: 2, amount: -25 }]);
        assert_eq!(aura.applied_at, instant);
    }
    assert_eq!((focus.duration_total, focus.represented_misc_value), (30_000, Some(900)));
    assert_eq!(focus.represented_effect, Some(RepresentedAuraEffectLikeCpp::ProvideSpellFocus));
    assert_eq!((modifier.duration_total, modifier.represented_misc_value), (8_000, None));
    assert_eq!(modifier.represented_effect, Some(RepresentedAuraEffectLikeCpp::Hover));
    assert_eq!((mounted.duration_total, mounted.represented_amount), (0, 77));
    assert_eq!(mounted.represented_misc_value, Some(900));
    assert_eq!((xp.duration_total, xp.represented_misc_value), (30_000, None));
    assert_eq!(xp.represented_multiplier, 0.75);
}

#[test]
fn single_variant_difficulty_precedes_row_construction_and_final_clock() {
    let trace = RefCell::new(Vec::new());
    AuraSubsystem::build_focus_runtime_application(
        20, ObjectGuid::EMPTY, 0, &(0, 9, 3),
        |row| { trace.borrow_mut().push("fields"); *row },
        || { trace.borrow_mut().push("difficulty"); 1 },
        || { trace.borrow_mut().push("clock"); Instant::now() },
    );
    assert_eq!(trace.borrow().first(), Some(&"difficulty"));
    assert_eq!(trace.borrow().last(), Some(&"clock"));
    assert_eq!(trace.borrow().iter().filter(|event| **event == "clock").count(), 1);
    trace.borrow_mut().clear();
    AuraSubsystem::build_xp_runtime_application(
        20, ObjectGuid::EMPTY, 0, &(0, 9, 3),
        |row| { trace.borrow_mut().push("fields"); *row },
        || { trace.borrow_mut().push("difficulty"); 1 },
        || { trace.borrow_mut().push("clock"); Instant::now() },
    );
    assert_eq!(&trace.borrow()[..2], ["fields", "difficulty"]);
    assert_eq!(trace.borrow().last(), Some(&"clock"));
}

#[test]
fn runtime_install_replaces_slot_provenance_after_identity_invalidation() {
    let mut auras = AuraSubsystem::default();
    let aura = AuraSubsystem::build_focus_runtime_application(
        20, ObjectGuid::EMPTY, 3, &(0, 9, 3), |r| *r, || 0, Instant::now,
    );
    let old = AuraCastProvenanceLikeCpp { cast_id: ObjectGuid::create_player(1, 1), spell_visual_id: 1 };
    let new = AuraCastProvenanceLikeCpp { cast_id: ObjectGuid::create_player(1, 2), spell_visual_id: 2 };
    auras.install_runtime_application_with_provenance(aura.clone(), old);
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    auras.spell_cast_log_aura_authority_inert_like_cpp = true;
    auras.install_runtime_application_with_provenance(aura, new);
    assert_eq!(auras.aura_cast_provenance_like_cpp(3), new);
    assert_eq!(auras.runtime_applications_like_cpp().len(), 1);
    assert!(!auras.spell_hit_aura_authority_inert_like_cpp);
    assert!(!auras.spell_cast_log_aura_authority_inert_like_cpp);
}

#[test]
fn single_effect_amounts_preserve_negative_base_and_out_of_range_empty_result() {
    assert_eq!(AuraSubsystem::single_effect_amounts(255, -9),
        [RepresentedAuraEffectAmountLikeCpp { effect_index: 255, amount: -9 }]);
    assert!(AuraSubsystem::single_effect_amounts(256, 9).is_empty());
}
