// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Applied preparation/admission/install regressions, authored without execution.

use super::{
    AppliedAuraRef, AuraCastProvenanceLikeCpp, AuraRef, AuraSubsystem, LoadedAuraStateLikeCpp,
    VisibleAuraEffectAmountLikeCpp,
};
use std::cell::Cell;
use wow_core::ObjectGuid;

#[test]
fn live_preparation_preserves_borrowed_row_order_checked_masks_and_base_calculation() {
    let rows = [
        (31, 7, 3, -10),
        (32, 8, 4, 90),
        (0, 9, 5, 20),
        (31, 6, 7, 30),
    ];
    let rolls = Cell::new(0);
    let effects = AuraSubsystem::prepare_live_applied_effects(
        &rows,
        u32::MAX,
        |r| (r.0, r.1, r.2),
        |r| {
            assert!(rows.iter().any(|original| std::ptr::eq(original, r)));
            rolls.set(rolls.get() + 1);
            r.3
        },
    );
    assert_eq!(effects, [(7, -10, 3, 31), (9, 20, 5, 0), (6, 30, 7, 31)]);
    assert_eq!(rolls.get(), 3);
}

#[test]
fn live_install_preserves_first_free_slot_per_effect_amounts_loaded_state_and_provenance() {
    let caster = ObjectGuid::create_player(1, 7);
    let mut auras = AuraSubsystem::default();
    auras.set_visible(0, AuraRef::new(99, caster));
    let provenance = AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::create_player(1, 9),
        spell_visual_id: 42,
    };
    assert_eq!(
        auras.install_live_applied_application(
            20,
            caster,
            u32::MAX,
            provenance,
            &[(7, -10, 3, 2), (8, 30, 5, 0)],
        ),
        Some(())
    );
    let first = AppliedAuraRef::new(20, caster, 1, 4);
    let second = AppliedAuraRef::new(20, caster, 1, 1);
    assert_eq!(auras.applied_auras, [first, second]);
    assert_eq!(auras.applied_aura_types.get(&7).unwrap(), &[first]);
    assert_eq!(auras.applied_aura_types.get(&8).unwrap(), &[second]);
    assert_eq!(auras.applied_aura_amounts.get(&first), Some(&-10));
    assert_eq!(auras.applied_aura_amounts.get(&second), Some(&30));
    assert_eq!(auras.applied_aura_misc_values.get(&first), Some(&3));
    assert_eq!(
        auras
            .loaded_aura_states_like_cpp
            .get(&AuraRef::new(20, caster)),
        Some(&LoadedAuraStateLikeCpp::new(i32::MAX, i32::MAX, 0, 1, 0))
    );
    assert_eq!(auras.visible_auras.get(&1), Some(&AuraRef::new(20, caster)));
    let visible = auras.visible_aura_applications_like_cpp.get(&1).unwrap();
    assert_eq!(visible.flags, 0);
    assert_eq!(
        visible.effect_amounts,
        [
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 2,
                amount: -10
            },
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 30
            },
        ]
    );
    assert_eq!(auras.aura_cast_provenance_like_cpp(1), provenance);
    assert!(auras.visible_auras_to_update.contains(&1));
    assert!(auras.runtime_applications_like_cpp().is_empty());
}

#[test]
fn live_duplicate_rejection_follows_preparation_without_partial_commit() {
    let caster = ObjectGuid::create_player(1, 7);
    let mut auras = AuraSubsystem::default();
    auras.install_live_applied_application(
        20,
        caster,
        30,
        AuraCastProvenanceLikeCpp::default(),
        &[(7, 9, 3, 0)],
    );
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    let before = auras.clone();
    let calculations = Cell::new(0);
    let rows = [(0, 7, 3)];
    let effects = AuraSubsystem::prepare_live_applied_effects(
        &rows,
        1,
        |r| *r,
        |_| {
            calculations.set(calculations.get() + 1);
            90
        },
    );
    assert_eq!(
        calculations.get(),
        1,
        "calculation still precedes duplicate admission"
    );
    assert_eq!(
        auras.install_live_applied_application(
            20,
            caster,
            30,
            AuraCastProvenanceLikeCpp::default(),
            &effects,
        ),
        None
    );
    assert_eq!(auras, before);
    assert_eq!(
        auras.install_live_applied_application(
            20,
            ObjectGuid::create_player(1, 8),
            30,
            AuraCastProvenanceLikeCpp::default(),
            &effects,
        ),
        Some(()),
        "a different caster is not a duplicate"
    );
}

#[test]
fn live_slot_exhaustion_preserves_excluded_slot_255_and_does_not_mutate_owner() {
    let mut auras = AuraSubsystem::default();
    for slot in 0..u8::MAX {
        auras.set_visible(slot, AuraRef::new(99, ObjectGuid::EMPTY));
    }
    let before = auras.clone();
    assert!(!auras.visible_auras.contains_key(&u8::MAX));
    assert_eq!(
        auras.install_live_applied_application(
            20,
            ObjectGuid::EMPTY,
            30,
            AuraCastProvenanceLikeCpp::default(),
            &[(7, 9, 3, 0)],
        ),
        None
    );
    assert_eq!(auras, before);
}

#[test]
fn live_duplicate_effect_rows_keep_visible_multiplicity_and_last_amount_write() {
    let caster = ObjectGuid::create_player(1, 7);
    let mut auras = AuraSubsystem::default();
    auras.install_live_applied_application(
        20,
        caster,
        30,
        AuraCastProvenanceLikeCpp::default(),
        &[(7, 9, 3, 0), (7, -5, 6, 0)],
    );
    let effect = AppliedAuraRef::new(20, caster, 0, 1);
    assert_eq!(auras.applied_auras, [effect]);
    assert_eq!(auras.applied_aura_amounts.get(&effect), Some(&-5));
    assert_eq!(auras.applied_aura_misc_values.get(&effect), Some(&6));
    assert_eq!(
        auras
            .visible_aura_applications_like_cpp
            .get(&0)
            .unwrap()
            .effect_amounts,
        [
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 9
            },
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: -5
            },
        ]
    );
}
