// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Runtime removal selection regressions, authored without execution.

use super::{AuraApplicationLikeCpp, AuraSubsystem, RepresentedAuraEffectLikeCpp};
use std::{cell::RefCell, collections::HashMap, time::Instant};
use wow_core::ObjectGuid;

fn application(
    slot: u8,
    spell_id: i32,
    caster_guid: ObjectGuid,
    represented_effect: Option<RepresentedAuraEffectLikeCpp>,
) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id,
        difficulty_id: 0,
        caster_guid,
        slot,
        duration_total: 30_000,
        duration_remaining: 20_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect,
        represented_amount: 9,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: Instant::now(),
    }
}

#[test]
fn owned_cancel_gate_short_circuits_each_catalog_operation_in_original_order() {
    for stop in 0..=4 {
        let trace = RefCell::new(Vec::new());
        let permitted = AuraSubsystem::owned_spell_is_cancelable(
            -20,
            |spell| {
                assert_eq!(spell, -20);
                trace.borrow_mut().push("exists");
                stop != 0
            },
            |spell| {
                assert_eq!(spell, -20);
                trace.borrow_mut().push("no_cancel");
                stop == 1
            },
            |spell| {
                assert_eq!(spell, -20);
                trace.borrow_mut().push("channel");
                stop == 2
            },
            |spell| {
                assert_eq!(spell, -20);
                trace.borrow_mut().push("passive");
                stop == 3
            },
        );
        let stages = ["exists", "no_cancel", "channel", "passive"];
        assert_eq!(*trace.borrow(), stages[..(stop + 1).min(4)]);
        assert_eq!(permitted, stop == 4);
    }
}

#[test]
fn owned_cancel_gate_rejects_before_a_snapshot_is_obtained() {
    let trace = RefCell::new(Vec::new());
    if AuraSubsystem::owned_spell_is_cancelable(
        20,
        |_| {
            trace.borrow_mut().push("exists");
            true
        },
        |_| {
            trace.borrow_mut().push("no_cancel");
            true
        },
        |_| panic!("channel must remain lazy"),
        |_| panic!("passive must remain lazy"),
    ) {
        trace.borrow_mut().push("snapshot");
    }
    assert_eq!(*trace.borrow(), ["exists", "no_cancel"]);
}

#[test]
fn effect_cancel_reads_attributes_even_for_a_different_effect_in_hashmap_order() {
    let auras = HashMap::from([
        (
            99,
            application(
                7,
                20,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::ModScale),
            ),
        ),
        (
            98,
            application(
                8,
                21,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::Hover),
            ),
        ),
        (
            97,
            application(
                9,
                22,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::ModScale),
            ),
        ),
    ]);
    let expected_reads: Vec<_> = auras.values().map(|aura| aura.spell_id).collect();
    let mut reads = Vec::new();
    let slots = AuraSubsystem::runtime_cancelable_slots_for_effect(
        &auras,
        RepresentedAuraEffectLikeCpp::ModScale,
        |spell| {
            reads.push(spell);
            spell == 22
        },
    );
    assert_eq!(
        reads, expected_reads,
        "attribute selection precedes the effect predicate"
    );
    assert_eq!(slots, [7]);
}

#[test]
fn attribute_selector_forwards_attribute_and_retains_duplicate_value_slots() {
    let auras = HashMap::from([
        (99, application(7, 20, ObjectGuid::EMPTY, None)),
        (98, application(7, 21, ObjectGuid::EMPTY, None)),
        (97, application(8, 22, ObjectGuid::EMPTY, None)),
    ]);
    let mut reads = Vec::new();
    let slots = AuraSubsystem::runtime_slots_with_attribute(&auras, 0x8000_0000, |spell, attr| {
        assert_eq!(attr, 0x8000_0000);
        reads.push(spell);
        spell != 22
    });
    assert_eq!(
        reads,
        auras.values().map(|aura| aura.spell_id).collect::<Vec<_>>()
    );
    assert_eq!(slots, [7, 7]);
}

#[test]
fn spell_selector_preserves_duplicate_slots_discrepant_keys_and_negative_spell_ids() {
    let auras = HashMap::from([
        (99, application(7, -20, ObjectGuid::EMPTY, None)),
        (98, application(7, -20, ObjectGuid::EMPTY, None)),
        (97, application(8, 20, ObjectGuid::EMPTY, None)),
    ]);
    assert_eq!(AuraSubsystem::runtime_slots_for_spell(&auras, -20), [7, 7]);
    assert!(AuraSubsystem::runtime_slots_for_spell(&auras, 0).is_empty());
    assert_eq!(
        auras.len(),
        3,
        "selection does not commit or change the owner"
    );
}

#[test]
fn owned_cancel_selection_keeps_empty_caster_wildcard_and_only_represented_positive_shapes() {
    let first = ObjectGuid::create_player(1, 7);
    let other = ObjectGuid::create_player(1, 8);
    let auras = HashMap::from([
        (99, application(0, 20, first, None)),
        (
            98,
            application(1, 20, other, Some(RepresentedAuraEffectLikeCpp::Mounted)),
        ),
        (
            97,
            application(2, 20, first, Some(RepresentedAuraEffectLikeCpp::ModScale)),
        ),
        (
            96,
            application(
                3,
                20,
                first,
                Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl),
            ),
        ),
        (
            95,
            application(4, 20, first, Some(RepresentedAuraEffectLikeCpp::Hover)),
        ),
        (94, application(5, 21, first, None)),
    ]);
    let mut any_caster =
        AuraSubsystem::runtime_cancelable_owned_slots(&auras, 20, ObjectGuid::EMPTY);
    any_caster.sort_unstable();
    assert_eq!(any_caster, [0, 1, 2, 3]);
    let mut exact_caster = AuraSubsystem::runtime_cancelable_owned_slots(&auras, 20, first);
    exact_caster.sort_unstable();
    assert_eq!(exact_caster, [0, 2, 3]);
    assert!(
        AuraSubsystem::runtime_cancelable_owned_slots(&auras, 20, ObjectGuid::create_player(1, 9),)
            .is_empty()
    );
}

#[test]
fn interrupt_selection_preserves_zero_masks_and_either_flag_family() {
    let mut first = application(7, 20, ObjectGuid::EMPTY, None);
    first.aura_interrupt_flags = 4;
    let mut second = application(8, 21, ObjectGuid::EMPTY, None);
    second.aura_interrupt_flags2 = 8;
    let third = application(9, 22, ObjectGuid::EMPTY, None);
    let auras = HashMap::from([(99, first), (98, second), (97, third)]);
    assert!(AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 0, 0).is_empty());
    assert_eq!(
        AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 4, 0),
        [7]
    );
    assert_eq!(
        AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 0, 8),
        [8]
    );
    assert!(AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 2, 16).is_empty());
    let mut both = AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 4, 8);
    both.sort_unstable();
    assert_eq!(both, [7, 8]);
    let mut overlap = application(7, 20, ObjectGuid::EMPTY, None);
    overlap.aura_interrupt_flags = 4;
    overlap.aura_interrupt_flags2 = 8;
    assert_eq!(
        AuraSubsystem::runtime_slots_with_interrupt_flags(&HashMap::from([(99, overlap)]), 4, 8,),
        [7],
        "matching both families still selects the value once"
    );
}

#[test]
fn expiry_reads_each_nonpermanent_timestamp_by_reference_in_hashmap_order() {
    let mut permanent = application(9, 20, ObjectGuid::EMPTY, None);
    permanent.duration_total = 0;
    let mut first = application(7, 21, ObjectGuid::EMPTY, None);
    first.duration_total = 2;
    let mut second = application(8, 22, ObjectGuid::EMPTY, None);
    second.duration_total = 2;
    let auras = HashMap::from([(99, permanent), (98, first), (97, second)]);
    let expected: Vec<_> = auras
        .values()
        .filter(|aura| aura.duration_total != 0)
        .map(|aura| &aura.applied_at as *const Instant)
        .collect();
    let permanent_time = &auras.get(&99).unwrap().applied_at as *const Instant;
    let mut reads = Vec::new();
    let slots = AuraSubsystem::expired_runtime_slots(&auras, |time| {
        let pointer = time as *const Instant;
        assert_ne!(
            pointer, permanent_time,
            "permanent entries must not read the clock"
        );
        reads.push(pointer);
        if reads.len() == 1 { 1 } else { 2 }
    });
    assert_eq!(
        reads, expected,
        "no timestamp copies or single shared clock read"
    );
    assert_eq!(slots.len(), 1);
    let expected_slot = auras
        .values()
        .find(|aura| &aura.applied_at as *const Instant == expected[1])
        .unwrap()
        .slot;
    assert_eq!(slots, [expected_slot]);
}

#[test]
fn expiry_preserves_u128_to_u32_truncation_and_exact_duration_boundary() {
    let mut aura = application(7, 20, ObjectGuid::EMPTY, None);
    aura.duration_total = 1;
    let auras = HashMap::from([(99, aura)]);
    let wrapped_zero = u128::from(u32::MAX) + 1;
    assert!(AuraSubsystem::expired_runtime_slots(&auras, |_| wrapped_zero).is_empty());
    assert_eq!(
        AuraSubsystem::expired_runtime_slots(&auras, |_| wrapped_zero + 1),
        [7]
    );
    assert!(AuraSubsystem::expired_runtime_slots(&auras, |_| 1u128 << 64).is_empty());
    assert_eq!(AuraSubsystem::expired_runtime_slots(&auras, |_| 1), [7]);
    assert!(AuraSubsystem::expired_runtime_slots(&auras, |_| 0).is_empty());
}

#[test]
fn expiry_retains_duplicate_slots_for_the_unchanged_application_removal_loop() {
    let aura = application(7, 20, ObjectGuid::EMPTY, None);
    let auras = HashMap::from([(99, aura.clone()), (98, aura)]);
    let mut reads = 0;
    let slots = AuraSubsystem::expired_runtime_slots(&auras, |_| {
        reads += 1;
        30_000
    });
    assert_eq!(slots, [7, 7]);
    assert_eq!(reads, 2);
    assert_eq!(
        slots.len(),
        2,
        "the caller retains the selected count even if a later removal fails"
    );
}

#[test]
fn stealth_selection_uses_map_keys_while_other_selectors_use_aura_slots() {
    let auras = HashMap::from([
        (
            99,
            application(
                7,
                20,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::Stealth),
            ),
        ),
        (
            98,
            application(
                7,
                20,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::Invisibility),
            ),
        ),
        (
            97,
            application(
                8,
                21,
                ObjectGuid::EMPTY,
                Some(RepresentedAuraEffectLikeCpp::Hover),
            ),
        ),
    ]);
    let mut keys = AuraSubsystem::runtime_stealth_or_invisibility_slots(&auras);
    keys.sort_unstable();
    assert_eq!(keys, [98, 99]);
    assert_eq!(AuraSubsystem::runtime_slots_for_spell(&auras, 20), [7, 7]);
}

#[test]
fn empty_snapshot_does_not_invoke_catalog_or_elapsed_callbacks() {
    let auras = HashMap::new();
    assert!(AuraSubsystem::runtime_slots_for_spell(&auras, 20).is_empty());
    assert!(
        AuraSubsystem::runtime_slots_with_attribute(&auras, 1, |_, _| panic!("no row")).is_empty()
    );
    assert!(
        AuraSubsystem::runtime_cancelable_slots_for_effect(
            &auras,
            RepresentedAuraEffectLikeCpp::ModScale,
            |_| panic!("no row"),
        )
        .is_empty()
    );
    assert!(
        AuraSubsystem::runtime_cancelable_owned_slots(&auras, 20, ObjectGuid::EMPTY).is_empty()
    );
    assert!(AuraSubsystem::runtime_slots_with_interrupt_flags(&auras, 1, 1).is_empty());
    assert!(AuraSubsystem::expired_runtime_slots(&auras, |_| panic!("no row")).is_empty());
    assert!(AuraSubsystem::runtime_stealth_or_invisibility_slots(&auras).is_empty());
}
