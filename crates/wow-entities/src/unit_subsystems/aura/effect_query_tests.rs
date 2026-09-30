//! Aura query tests; catalog fixtures are development-only.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;
use wow_core::ObjectGuid;
use wow_data::{SpellEffectInfo, SpellStore};
use wow_data_model::aura_effects::AppliedAuraEffectLikeCpp;
use wow_constants::spell::aura_types::{
    SPELL_AURA_SCHOOL_ABSORB, SPELL_AURA_MANA_SHIELD,
    SPELL_AURA_SCHOOL_HEAL_ABSORB, SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
};
use super::{
    AppliedAuraRef, AuraApplicationLikeCpp, AuraSubsystem,
    RepresentedAuraEffectAmountLikeCpp, RepresentedAuraEffectLikeCpp,
};

fn fields(effect: &SpellEffectInfo) -> (u32, u32, i32, i32, i32) {
    (effect.effect_index, effect.effect, effect.effect_aura,
        effect.effect_misc_value_1, effect.effect_misc_value_2)
}

fn creature_effects_from_store(
    applied: &[AppliedAuraRef],
    store: &SpellStore,
    difficulty: u8,
    difficulties: Option<&wow_data::DifficultyStore>,
) -> Vec<AppliedAuraEffectLikeCpp> {
    AuraSubsystem::creature_effects(
        applied, difficulty,
        |id, difficulty| store.effects_for_difficulty_like_cpp(id, difficulty, difficulties),
        fields, SpellEffectInfo::calc_value_no_caster_like_cpp,
    )
}

fn application(slot: u8, spell_id: i32, mask: u32) -> AuraApplicationLikeCpp {
    AuraApplicationLikeCpp {
        spell_id, difficulty_id: 0, caster_guid: ObjectGuid::create_player(1, 7),
        slot, duration_total: 0, duration_remaining: 0, stack_count: 1,
        aura_flags: 0, effect_mask: mask, aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0, represented_effect: None, represented_amount: 0,
        represented_effect_amounts: Vec::new(), represented_misc_value: None,
        represented_multiplier: 1.0, applied_at: Instant::now(),
    }
}

fn row(index: u32, aura_type: i32, misc: i32, amount: i32) -> SpellEffectInfo {
    SpellEffectInfo {
        effect_index: index, effect: 6, effect_aura: aura_type,
        effect_misc_value_1: misc, effect_base_points: amount,
        ..Default::default()
    }
}

#[test]
fn creature_aura_effects_resolve_the_applied_mask_like_cpp() {
    use wow_data::spell::aura_types::{
        SPELL_AURA_MOD_DAMAGE_TAKEN, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
    };

    let caster = ObjectGuid::create_player(1, 91);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_130,
        wow_data::SpellInfo {
            spell_id: 91_130,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(SPELL_AURA_MOD_DAMAGE_TAKEN),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_DAMAGE_TAKEN,
                    effect_misc_value_1: 0x01,
                    effect_base_points: -50,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
                    effect_misc_value_1: 0,
                    effect_base_points: -5,
                    ..Default::default()
                },
            ],
        },
    );

    // Only effect 0 is applied, so only its aura type is resolved.
    let effects = creature_effects_from_store(
        &[AppliedAuraRef::new(91_130, caster, 0, 0b1)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].aura_type, SPELL_AURA_MOD_DAMAGE_TAKEN);
    assert_eq!(effects[0].misc_value, 0x01);
    assert_eq!(effects[0].amount, -50);
    assert_eq!(effects[0].caster_guid, caster);

    // Both effects when the application's mask covers both slots.
    let effects = creature_effects_from_store(
        &[AppliedAuraRef::new(91_130, caster, 0, 0b11)],
        &spell_store,
        0,
        None,
    );
    assert_eq!(effects.len(), 2);
    assert_eq!(effects[1].aura_type, SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN);
    assert_eq!(effects[1].amount, -5);
}


#[test]
fn player_effects_preserve_first_amount_lazy_borrowed_rows_masks_and_slot_order() {
    let rows = [row(0, 5, 10, 100), row(1, 5, 11, 101),
        row(31, 5, 31, 131), row(32, 5, 32, 132), row(2, 0, 12, 102)];
    let mut represented = application(2, 20, 0b11 | (1 << 31));
    represented.represented_effect_amounts = vec![
        RepresentedAuraEffectAmountLikeCpp { effect_index: 0, amount: -9 },
        RepresentedAuraEffectAmountLikeCpp { effect_index: 0, amount: 99 },
    ];
    let auras = HashMap::from([
        (7, application(7, 20, 1)), (2, represented), (1, application(1, 99, u32::MAX)),
    ]);
    let trace = RefCell::new(Vec::new());
    let effects = AuraSubsystem::player_effects(
        &auras, Some(5),
        |id| { trace.borrow_mut().push((0, id)); (id == 20).then_some(rows.as_slice()) },
        fields,
        |effect| {
            assert!(rows.iter().any(|original| std::ptr::eq(original, effect)));
            trace.borrow_mut().push((1, effect.effect_index as i32));
            effect.effect_base_points
        },
        |effect| (effect.slot, effect.misc_value, effect.amount),
    );
    assert_eq!(effects, vec![(2, 10, -9), (2, 11, 101), (2, 31, 131), (7, 10, 100)]);
    assert_eq!(*trace.borrow(), vec![(0, 99), (0, 20), (1, 1), (1, 31), (0, 20), (1, 0)]);
}

#[test]
fn player_unsorted_projection_retains_hashmap_value_iteration() {
    let rows = [row(0, 5, 3, 4)];
    let auras = HashMap::from([
        (7, application(7, 20, 1)), (2, application(2, 21, 1)),
        (1, application(1, 22, 1)),
    ]);
    let expected: Vec<_> = auras.values().map(|aura| (aura.spell_id, 3, 4)).collect();
    let effects = AuraSubsystem::player_effects_in_map_order(
        &auras, 5, |_| Some(rows.as_slice()), fields,
        SpellEffectInfo::calc_value_no_caster_like_cpp,
        |effect| (effect.spell_id, effect.misc_value, effect.amount),
    );
    assert_eq!(effects, expected);
}

#[test]
fn creature_shields_keep_spent_canonical_amount_and_lazy_fallback_order() {
    let rows = [row(0, SPELL_AURA_SCHOOL_ABSORB, 1, 100),
        row(1, SPELL_AURA_SCHOOL_ABSORB, 1, 200)];
    let caster = ObjectGuid::create_player(1, 7);
    let mut auras = AuraSubsystem::default();
    auras.applied_auras = vec![AppliedAuraRef::new(20, caster, 4, 3)];
    auras.applied_aura_amounts.insert(AppliedAuraRef::new(20, caster, 4, 1), 0);
    // A combined mask is not the authoritative per-effect amount key.
    auras.applied_aura_amounts.insert(AppliedAuraRef::new(20, caster, 4, 3), 999);
    let trace = RefCell::new(Vec::new());
    let shields = auras.creature_absorb_shields(
        1,
        |_| { trace.borrow_mut().push("S"); Some(rows.as_slice()) },
        fields,
        |effect| { trace.borrow_mut().push("B"); effect.effect_base_points },
        |_| { trace.borrow_mut().push("C"); 471 },
        |_| { trace.borrow_mut().push("A6"); true },
    );
    assert_eq!(shields.iter().map(|shield| shield.amount).collect::<Vec<_>>(), vec![0, 200]);
    assert_eq!(shields[0].category_id, 471);
    assert!(shields[0].cannot_be_ignored);
    assert_eq!(*trace.borrow(), vec!["S", "C", "A6", "B", "A6"]);
}

#[test]
fn player_shield_callbacks_preserve_category_and_mana_order() {
    let rows = [row(0, SPELL_AURA_MANA_SHIELD, 1, 25)];
    let auras = HashMap::from([(2, application(2, 20, 1))]);
    let trace = RefCell::new(Vec::new());
    let no_absorb = AuraSubsystem::player_absorb_shields(
        &auras, 1,
        |_| { trace.borrow_mut().push("S"); Some(rows.as_slice()) }, fields,
        |_| panic!("non-absorb rows must not calculate amounts"),
        |_| { trace.borrow_mut().push("C"); 0 },
        |_| panic!("non-absorb rows must not query attributes"),
    );
    assert!(no_absorb.is_empty());
    assert_eq!(*trace.borrow(), vec!["S", "C"]);
    trace.borrow_mut().clear();
    let mana = AuraSubsystem::player_mana_shields(
        &auras, 1,
        |_| { trace.borrow_mut().push("S"); Some(rows.as_slice()) }, fields,
        |effect| { trace.borrow_mut().push("B"); effect.effect_base_points },
        |_| { trace.borrow_mut().push("V"); 2.0 },
        |_| { trace.borrow_mut().push("A6"); false },
    );
    assert_eq!(mana[0].amount, 25);
    assert_eq!(mana[0].mana_multiplier, 2.0);
    assert_eq!(*trace.borrow(), vec!["S", "B", "V", "A6"]);
}

#[test]
fn heal_shield_selection_keeps_school_mask_negative_amount_and_slot_order() {
    let rows = [row(0, SPELL_AURA_SCHOOL_HEAL_ABSORB, 2, -10),
        row(1, SPELL_AURA_SCHOOL_HEAL_ABSORB, 1, 40)];
    let auras = HashMap::from([(7, application(7, 20, 3)), (2, application(2, 20, 3))]);
    let shields = AuraSubsystem::player_heal_absorb_shields(
        &auras, 2, |_| Some(rows.as_slice()), fields,
        SpellEffectInfo::calc_value_no_caster_like_cpp,
    );
    assert_eq!(shields.iter().map(|shield| (shield.slot, shield.effect_index, shield.amount))
        .collect::<Vec<_>>(), vec![(2, 0, -10), (7, 0, -10)]);
}

#[test]
fn mechanics_preserve_metadata_first_difficulty_and_effect_zero_gap() {
    let rows = [row(0, 5, 0, 0), row(1, 5, 0, 0), row(31, 5, 0, 0),
        SpellEffectInfo { effect_index: 2, effect: 0, ..Default::default() }];
    let mut aura = application(4, 20, u32::MAX);
    aura.difficulty_id = 3;
    let auras = HashMap::from([(4, aura)]);
    let trace = RefCell::new(Vec::new());
    let mask = AuraSubsystem::application_mechanic_mask(
        &auras,
        |id, difficulty| {
            trace.borrow_mut().push(("M", id, difficulty));
            Some((63, BTreeMap::from([(0, 3), (1, 4), (2, 6), (31, 5), (32, 7)])))
        },
        |id, difficulty| {
            trace.borrow_mut().push(("D", id, difficulty));
            Some(rows.as_slice())
        }, fields,
    );
    assert_eq!(mask, (1 << 63) | (1 << 4) | (1 << 5));
    assert_eq!(*trace.borrow(), vec![("M", 20, 3), ("D", 20, 3)]);
    let no_metadata = AuraSubsystem::applied_mechanic_mask::<SpellEffectInfo>(
        &[AppliedAuraRef::new(u32::MAX, ObjectGuid::EMPTY, 0, u32::MAX)], 9,
        |id, difficulty| { assert_eq!((id, difficulty), (0, 9)); None },
        |_, _| panic!("missing metadata must not select rows"), fields,
    );
    assert_eq!(no_metadata, 0);
    let no_rows = AuraSubsystem::applied_mechanic_mask::<SpellEffectInfo>(
        &[AppliedAuraRef::new(20, ObjectGuid::EMPTY, 0, 2)], 9,
        |_, difficulty| {
            assert_eq!(difficulty, 9);
            Some((0, BTreeMap::from([(1, 4)])))
        }, |_, _| None, fields,
    );
    assert_eq!(no_rows, 0);
}

#[test]
fn creature_effects_keep_difficulty_row_order_and_base_values() {
    let rows = [row(1, 5, 11, -3), row(0, 5, 10, 7), row(32, 5, 32, 100)];
    let applied = [
        AppliedAuraRef::new(20, ObjectGuid::EMPTY, 7, 3),
        AppliedAuraRef::new(99, ObjectGuid::EMPTY, 2, 3),
    ];
    let trace = RefCell::new(Vec::new());
    let effects = AuraSubsystem::creature_effects(
        &applied, 4,
        |id, difficulty| {
            trace.borrow_mut().push((id, difficulty));
            (id == 20).then_some(rows.as_slice())
        }, fields, SpellEffectInfo::calc_value_no_caster_like_cpp,
    );
    assert_eq!(effects.iter().map(|effect| (effect.slot, effect.misc_value, effect.amount))
        .collect::<Vec<_>>(), vec![(7, 11, -3), (7, 10, 7)]);
    assert_eq!(*trace.borrow(), vec![(20, 4), (99, 4)]);
}

#[test]
fn represented_queries_preserve_negative_extrema_empty_defaults_and_misc_filters() {
    let effect = RepresentedAuraEffectLikeCpp::Speed;
    let mut first = application(1, 20, 1);
    first.represented_effect = Some(effect);
    first.represented_amount = -25;
    first.represented_misc_value = Some(2);
    first.represented_multiplier = 0.5;
    let mut second = application(2, 21, 1);
    second.represented_effect = Some(effect);
    second.represented_amount = 50;
    second.represented_misc_value = Some(3);
    second.represented_multiplier = 2.0;
    let auras = HashMap::from([(1, first), (2, second)]);
    assert!(AuraSubsystem::has_represented_effect(&auras, effect));
    assert!(AuraSubsystem::has_represented_effect_with_misc(&auras, effect, 2));
    assert!(!AuraSubsystem::has_represented_effect_with_misc(&auras, effect, 4));
    assert_eq!(AuraSubsystem::represented_modifier(&auras, effect), 25);
    assert_eq!(AuraSubsystem::represented_modifier_by_misc(&auras, effect, 2), -25);
    assert_eq!(AuraSubsystem::maximum_represented_amount(&auras, effect), 50);
    assert_eq!(AuraSubsystem::minimum_represented_amount(&auras, effect), -25);
    assert_eq!(AuraSubsystem::represented_multiplier(&auras, effect), 1.0);
    assert_eq!(AuraSubsystem::represented_amount_multiplier(&auras, effect), 1.5);
    let empty = HashMap::new();
    assert_eq!(AuraSubsystem::represented_modifier(&empty, effect), 0);
    assert_eq!(AuraSubsystem::maximum_represented_amount(&empty, effect), 0);
    assert_eq!(AuraSubsystem::minimum_represented_amount(&empty, effect), 0);
    assert_eq!(AuraSubsystem::represented_multiplier(&empty, effect), 1.0);
    assert_eq!(AuraSubsystem::represented_amount_multiplier(&empty, effect), 1.0);
    assert_eq!(AuraSubsystem::effect_modifier_by_misc(vec![(2, -25), (3, 50)], 2), -25);
    assert_eq!(AuraSubsystem::effect_multiplier_by_misc(vec![(2, -25), (2, 50)], 2), 1.125);
    assert_eq!(AuraSubsystem::effect_modifier_by_misc(Vec::new(), 2), 0);
    assert_eq!(AuraSubsystem::effect_multiplier_by_misc(Vec::new(), 2), 1.0);
}

#[test]
fn stat_health_predicate_queries_ability_before_rows_without_amount_calculation() {
    let rows = [SpellEffectInfo {
        effect_misc_value_2: 1 << 2,
        ..row(0, SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE, 0, 0)
    }];
    let aura = application(1, 20, 1);
    let trace = RefCell::new(Vec::new());
    assert!(AuraSubsystem::total_stat_percentage_preserves_health(
        &aura,
        |_| { trace.borrow_mut().push("A0"); true },
        |_| { trace.borrow_mut().push("S"); Some(rows.as_slice()) }, fields,
    ));
    assert_eq!(*trace.borrow(), vec!["A0", "S"]);
    assert!(!AuraSubsystem::total_stat_percentage_preserves_health::<SpellEffectInfo>(
        &aura, |_| false, |_| panic!("non-ability must not select rows"), fields,
    ));
    let not_stamina = [SpellEffectInfo { effect_misc_value_2: 1, ..rows[0].clone() }];
    assert!(AuraSubsystem::has_total_stat_percentage(
        &aura, |_| Some(not_stamina.as_slice()), fields,
    ));
    assert!(!AuraSubsystem::total_stat_percentage_preserves_health(
        &aura, |_| true, |_| Some(not_stamina.as_slice()), fields,
    ));
}

#[test]
fn duplicate_effect_indices_calculate_each_original_borrowed_row() {
    let rows = [
        SpellEffectInfo { effect_die_sides: 4, ..row(1, 5, 10, 10) },
        SpellEffectInfo { effect_die_sides: 4, ..row(1, 5, 20, 20) },
    ];
    let auras = HashMap::from([(1, application(1, 20, 2))]);
    let mut rolls = 0;
    let effects = AuraSubsystem::player_effects(
        &auras, Some(5), |_| Some(rows.as_slice()), fields,
        |effect| {
            assert!(rows.iter().any(|original| std::ptr::eq(original, effect)));
            effect.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
                assert_eq!((min, max), (1, 4));
                rolls += 1;
                rolls
            })
        },
        |effect| (effect.misc_value, effect.amount),
    );
    assert_eq!(effects, vec![(10, 11), (20, 22)]);
    assert_eq!(rolls, 2);
}
