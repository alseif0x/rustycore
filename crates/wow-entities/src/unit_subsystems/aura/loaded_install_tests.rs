// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restored Aura construction and commit regressions, authored without execution.

use super::{
    AppliedAuraRef, AuraCastProvenanceLikeCpp, AuraRef, AuraSubsystem, AuraThreatSnapshotLikeCpp,
    LoadedAuraStateLikeCpp, RepresentedAuraEffectAmountLikeCpp, VisibleAuraEffectAmountLikeCpp,
};
use std::{cell::RefCell, time::Instant};
use wow_core::ObjectGuid;

#[test]
fn represented_pet_aura_offline_remain_time_matches_cpp_arithmetic() {
    let adjusted_represented_pet_aura_remain_time_like_cpp =
        AuraSubsystem::offline_remaining_duration;
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, true, true),
        Some(2_000)
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(3_000, 3, true, true),
        None,
        "C++ skips when remainTime / IN_MILLISECONDS <= timediff"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(-1, 99, true, true),
        Some(-1),
        "C++ permanent auras do not tick offline"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, true, false),
        Some(5_000),
        "positive auras without SPELL_ATTR4_AURA_EXPIRES_OFFLINE keep their saved remainTime"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, false, false),
        Some(2_000),
        "C++ also ticks negative auras offline; represented SpellInfo::IsPositive is not wired yet"
    );
}

#[test]
fn loaded_effect_join_preserves_normalized_caster_masks_order_and_duplicate_rows() {
    let caster = ObjectGuid::create_player(1, 7);
    let other = ObjectGuid::create_player(1, 8);
    let rows = [
        (ObjectGuid::EMPTY, 20, 3, 1, -10),
        (caster, 20, 3, 0, 20),
        (caster, 20, 3, 1, 40),
        (other, 20, 3, 1, 50),
        (caster, 21, 3, 1, 60),
        (caster, 20, 1, 1, 70),
    ];
    let amounts = AuraSubsystem::collect_loaded_effect_amounts(caster, 20, 3, &rows, |r| {
        (
            if r.0.is_empty() { caster } else { r.0 },
            r.1,
            r.2,
            r.3,
            r.4,
        )
    });
    assert_eq!(
        amounts,
        [
            RepresentedAuraEffectAmountLikeCpp {
                effect_index: 1,
                amount: -10
            },
            RepresentedAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 20
            },
            RepresentedAuraEffectAmountLikeCpp {
                effect_index: 1,
                amount: 40
            },
        ]
    );
    assert!(
        AuraSubsystem::collect_loaded_effect_amounts(caster, 999, 3, &rows, |r| *r,).is_empty()
    );
}

#[test]
fn loaded_runtime_construction_consumes_same_amount_vector_and_calls_clock_last() {
    let trace = RefCell::new(Vec::new());
    let rows = [(ObjectGuid::EMPTY, 20, 1, 0, -9)];
    let amounts =
        AuraSubsystem::collect_loaded_effect_amounts(ObjectGuid::EMPTY, 20, 1, &rows, |r| {
            trace.borrow_mut().push("join");
            *r
        });
    let pointer = amounts.as_ptr();
    trace.borrow_mut().push("threat");
    let instant = Instant::now();
    let aura = AuraSubsystem::build_loaded_runtime_application(
        20,
        4,
        ObjectGuid::EMPTY,
        3,
        LoadedAuraStateLikeCpp::new(-1, -5, 7, 0, 8),
        0x108,
        1,
        amounts,
        || {
            trace.borrow_mut().push("clock");
            instant
        },
    );
    assert_eq!(trace.borrow().last(), Some(&"clock"));
    assert_eq!(
        &trace.borrow()[trace.borrow().len() - 2..],
        ["threat", "clock"]
    );
    assert_eq!(aura.represented_effect_amounts.as_ptr(), pointer);
    assert_eq!(
        (
            aura.duration_total,
            aura.duration_remaining,
            aura.stack_count
        ),
        (0, 0, 1)
    );
    assert_eq!(
        (aura.difficulty_id, aura.aura_flags, aura.effect_mask),
        (4, 0x108, 1)
    );
    assert_eq!(aura.applied_at, instant);
    assert_eq!(aura.represented_effect, None);
    assert_eq!(aura.represented_misc_value, None);
    assert_eq!(aura.represented_multiplier, 1.0);
}

#[test]
fn loaded_runtime_install_preserves_snapshot_runtime_provenance_and_replacement() {
    let mut auras = AuraSubsystem::default();
    let old = AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::create_player(1, 1),
        spell_visual_id: 1,
    };
    let new = AuraCastProvenanceLikeCpp {
        cast_id: ObjectGuid::create_player(1, 2),
        spell_visual_id: 2,
    };
    auras.set_aura_cast_provenance_like_cpp(3, old);
    auras.set_spell_hit_aura_authority_inert_like_cpp(true);
    let snapshot = AuraThreatSnapshotLikeCpp::new([7, 8], vec![(1, 9, -10, 11)]);
    let aura = AuraSubsystem::build_loaded_runtime_application(
        20,
        4,
        ObjectGuid::EMPTY,
        3,
        LoadedAuraStateLikeCpp::new(30, 20, 7, 2, 8),
        0x108,
        1,
        Vec::new(),
        Instant::now,
    );
    auras.install_loaded_runtime_application(3, snapshot.clone(), aura.clone(), new);
    assert_eq!(auras.threat_snapshot_like_cpp(3), Some(&snapshot));
    assert_eq!(auras.runtime_application_like_cpp(3), Some(&aura));
    assert_eq!(auras.aura_cast_provenance_like_cpp(3), new);
    assert!(!auras.spell_hit_aura_authority_inert_like_cpp);
    assert!(
        auras.applied_auras.is_empty(),
        "the loaded runtime installer does not add applied rows"
    );
}

#[test]
fn loaded_pet_install_preserves_whole_mask_raw_state_and_duplicate_effect_amounts() {
    let caster = ObjectGuid::create_player(1, 7);
    let aura_ref = AuraRef::new(20, caster);
    let rows = [
        (ObjectGuid::EMPTY, 20, 3, 1, -10),
        (caster, 20, 3, 0, 20),
        (caster, 20, 3, 1, 40),
        (caster, 20, 1, 0, 99),
    ];
    let state = LoadedAuraStateLikeCpp::new(-1, -1, 7, 0, 8);
    let mut auras = AuraSubsystem::default();
    let old = AuraCastProvenanceLikeCpp {
        cast_id: caster,
        spell_visual_id: 9,
    };
    auras.set_aura_cast_provenance_like_cpp(3, old);
    auras.install_loaded_applied_application(3, aura_ref, 3, state, &rows, |r| {
        (
            if r.0.is_empty() { caster } else { r.0 },
            r.1,
            r.2,
            r.3,
            r.4,
        )
    });
    assert_eq!(auras.applied_auras, [AppliedAuraRef::new(20, caster, 3, 3)]);
    assert_eq!(
        auras.loaded_aura_states_like_cpp.get(&aura_ref),
        Some(&state)
    );
    assert_eq!(
        auras
            .applied_aura_amounts
            .get(&AppliedAuraRef::new(20, caster, 3, 2)),
        Some(&40)
    );
    assert_eq!(
        auras
            .applied_aura_amounts
            .get(&AppliedAuraRef::new(20, caster, 3, 1)),
        Some(&20)
    );
    let visible = auras.visible_aura_applications_like_cpp.get(&3).unwrap();
    assert_eq!(visible.flags, 3);
    assert_eq!(
        visible.effect_amounts,
        [
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 1,
                amount: -10
            },
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 20
            },
            VisibleAuraEffectAmountLikeCpp {
                effect_index: 1,
                amount: 40
            },
        ]
    );
    assert!(
        !auras.visible_auras_to_update.contains(&3),
        "direct restored inserts do not enqueue updates"
    );
    assert_eq!(
        auras.aura_cast_provenance_like_cpp(3),
        old,
        "direct restored inserts retain old slot metadata"
    );
    assert!(auras.applied_aura_types.is_empty());
}

#[test]
fn loaded_pet_duplicate_bases_keep_distinct_slots_and_overwrite_shared_loaded_state() {
    let caster = ObjectGuid::create_player(1, 7);
    let aura_ref = AuraRef::new(20, caster);
    let mut auras = AuraSubsystem::default();
    let rows = [(caster, 20, 1, 0, 9)];
    let first = LoadedAuraStateLikeCpp::new(30, 20, 1, 1, 0);
    let second = LoadedAuraStateLikeCpp::new(40, 10, 2, 3, 8);
    auras.install_loaded_applied_application(0, aura_ref, 1, first, &rows, |r| *r);
    auras.install_loaded_applied_application(1, aura_ref, 1, second, &rows, |r| *r);
    assert_eq!(auras.owned_auras.len(), 1);
    assert_eq!(
        auras.applied_auras,
        [
            AppliedAuraRef::new(20, caster, 0, 1),
            AppliedAuraRef::new(20, caster, 1, 1),
        ]
    );
    assert_eq!(auras.visible_auras.get(&0), Some(&aura_ref));
    assert_eq!(auras.visible_auras.get(&1), Some(&aura_ref));
    assert_eq!(
        auras.loaded_aura_states_like_cpp.get(&aura_ref),
        Some(&second)
    );
    let replacement = AuraRef::new(21, caster);
    auras.install_loaded_applied_application(1, replacement, 1, first, &rows, |r| *r);
    assert_eq!(auras.visible_auras.get(&1), Some(&replacement));
    assert!(
        auras
            .visible_aura_applications_like_cpp
            .get(&1)
            .unwrap()
            .effect_amounts
            .is_empty()
    );
    assert!(
        auras
            .applied_auras
            .contains(&AppliedAuraRef::new(20, caster, 1, 1))
    );
}

#[test]
fn offline_duration_preserves_fractional_expiry_negative_values_and_saturating_elapsed_time() {
    assert_eq!(
        AuraSubsystem::offline_remaining_duration(999, 0, true, true),
        None
    );
    assert_eq!(
        AuraSubsystem::offline_remaining_duration(1_001, 1, true, true),
        None
    );
    assert_eq!(
        AuraSubsystem::offline_remaining_duration(-2, 0, false, false),
        None
    );
    assert_eq!(
        AuraSubsystem::offline_remaining_duration(i32::MAX, u32::MAX, false, false),
        None
    );
    assert_eq!(
        AuraSubsystem::offline_remaining_duration(-1, u32::MAX, false, true),
        Some(-1)
    );
}
