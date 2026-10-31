use super::*;
use wow_core::ObjectGuid;
use wow_data::{SpellEffectInfo, SpellEffectPolarityMetadata};

const AURA_SPEED: i32 = 31;
const AURA_TRANSFORM: i32 = 56;
const AURA_SWIM_SPEED: i32 = 58;
const AURA_DAMAGE_PERCENT: i32 = 79;
const AURA_PERCENT_STAT: i32 = 80;
const AURA_GHOST: i32 = 95;
const AURA_NO_PVP_CREDIT: i32 = 159;

fn facts(spell_id: i32) -> SelectedSpellMetadataFacts {
    SelectedSpellMetadataFacts {
        spell_id,
        attributes: [0; 15],
        family_name: Some(0),
        family_flags: Some([0; 4]),
        spell_mechanic: Some(0),
        preexisting_negative_effect_mask: 0,
    }
}

fn effect(index: u32, aura_type: i32) -> SpellEffectInfo {
    SpellEffectInfo {
        effect_index: index,
        effect: 6,
        effect_aura: aura_type,
        implicit_target_1: 1,
        implicit_target_2: 0,
        polarity_metadata: Some(SpellEffectPolarityMetadata::default()),
        ..SpellEffectInfo::default()
    }
}

fn classify(
    spell: SelectedSpellMetadataFacts,
    effects: &[SpellEffectInfo],
) -> Result<SelectedSpellPolarity, SelectedSpellPolarityError> {
    classify_selected_spell_polarity(spell, effects)
}

fn guid(low: u32) -> ObjectGuid {
    ObjectGuid::create_player(1, i64::from(low))
}

fn flags(
    polarity: SelectedSpellPolarity,
    active_mask: u32,
    caster: ObjectGuid,
    owner: ObjectGuid,
    target: ObjectGuid,
    attributes: &[u32; 15],
    friendly: Option<bool>,
) -> Result<u16, SelectedAuraFlagsError> {
    build_selected_aura_application_flags(
        polarity,
        active_mask,
        caster,
        owner,
        target,
        attributes,
        friendly,
    )
}

#[test]
fn selected_flags_stock_spirit_sickness_spells_are_negative_for_partial_masks() {
    for (spell_id, active_mask) in [(8_326, 0b1001), (20_584, 0b0011), (15_007, 0b0001)] {
        let mut spell = facts(spell_id);
        spell.attributes[0] = 0x0400_0000;
        spell.family_name = None;
        spell.family_flags = None;
        spell.spell_mechanic = None;
        let effects = [
            effect(0, AURA_GHOST),
            effect(1, AURA_SPEED),
            effect(3, AURA_NO_PVP_CREDIT),
        ];

        let polarity = classify(spell, &effects).unwrap();
        assert_eq!(polarity.available_effect_mask, 0b1011);
        assert_eq!(
            polarity.negative_effect_mask & polarity.available_effect_mask,
            0b1011
        );
        assert_eq!(polarity.positive_effect_mask, 0);
        assert!(!polarity.whole_spell_positive);
        assert_eq!(
            flags(
                polarity,
                active_mask,
                guid(90_001),
                guid(90_002),
                guid(90_003),
                &spell.attributes,
                None,
            ),
            Ok(0x0010),
        );
    }
}

#[test]
fn selected_flags_global_and_effect_precedence_is_source_ordered() {
    let mut preexisting = facts(99_000);
    preexisting.preexisting_negative_effect_mask = 1;
    preexisting.attributes[0] = 0x0000_0040;
    assert_eq!(
        classify(preexisting, &[effect(0, AURA_NO_PVP_CREDIT)])
            .unwrap()
            .negative_effect_mask
            & 1,
        1,
    );

    let mut spell = facts(99_001);
    spell.preexisting_negative_effect_mask = 1;
    spell.attributes[0] = 0x0000_0040 | 0x0400_0000;
    spell.attributes[4] = 0x0000_1000;
    let mut inactive = effect(0, AURA_NO_PVP_CREDIT);
    inactive.effect = 0;
    let result = classify(spell, &[inactive, effect(1, AURA_NO_PVP_CREDIT)]).unwrap();
    assert_eq!(result.negative_effect_mask & 0b11, 1);
    assert_eq!(result.positive_effect_mask & 0b11, 0b10);

    let mut passive = facts(99_002);
    passive.attributes[0] = 0x0000_0040 | 0x0400_0000;
    passive.attributes[4] = 0x0000_1000;
    let positive = classify(passive, &[effect(0, AURA_GHOST)]).unwrap();
    assert_eq!(positive.positive_effect_mask, 1);

    let mut debuff = facts(99_003);
    debuff.attributes[0] = 0x0400_0000;
    debuff.attributes[4] = 0x0000_1000;
    let negative = classify(debuff, &[effect(0, AURA_NO_PVP_CREDIT)]).unwrap();
    assert_eq!(negative.negative_effect_mask & 1, 1);

    let mut buff = facts(99_004);
    buff.attributes[4] = 0x0000_1000;
    let mut harmful = effect(0, AURA_GHOST);
    harmful.polarity_metadata.as_mut().unwrap().attributes = 0x0000_1000;
    assert_eq!(classify(buff, &[harmful]).unwrap().positive_effect_mask, 1);

    let mut harmful = effect(0, AURA_NO_PVP_CREDIT);
    harmful.polarity_metadata.as_mut().unwrap().attributes = 0x0000_1000;
    let mut family_positive = facts(24_732);
    family_positive.family_name = Some(0);
    family_positive.spell_mechanic = Some(29);
    assert_eq!(
        classify(family_positive, &[harmful])
            .unwrap()
            .negative_effect_mask
            & 1,
        1,
    );
}

#[test]
fn selected_flags_family_and_mechanic_rules_precede_selected_aura_types() {
    let effect = effect(0, AURA_GHOST);

    let mut generic_negative = facts(40_268);
    generic_negative.spell_mechanic = Some(29);
    assert_eq!(
        classify(generic_negative, std::slice::from_ref(&effect))
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut generic_positive = facts(24_732);
    generic_positive.spell_mechanic = None;
    assert_eq!(
        classify(generic_positive, std::slice::from_ref(&effect))
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut envenom = facts(32_645);
    envenom.family_name = Some(8);
    assert_eq!(
        classify(envenom, std::slice::from_ref(&effect))
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut shadow_of_death = facts(40_251);
    shadow_of_death.family_name = Some(8);
    assert_eq!(
        classify(shadow_of_death, std::slice::from_ref(&effect))
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut warrior = facts(99_005);
    warrior.family_name = Some(4);
    warrior.family_flags = Some([0x2020_0000, 0, 0, 0]);
    warrior.spell_mechanic = Some(29);
    assert_eq!(
        classify(warrior, std::slice::from_ref(&effect))
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut immune_shield = facts(99_006);
    immune_shield.spell_mechanic = Some(29);
    assert_eq!(
        classify(immune_shield, &[effect])
            .unwrap()
            .positive_effect_mask,
        1,
    );
}

#[test]
fn selected_flags_require_optional_family_facts_only_when_source_reaches_them() {
    let mut passive = facts(99_007);
    passive.attributes[0] = 0x40;
    passive.family_name = None;
    passive.family_flags = None;
    passive.spell_mechanic = None;
    assert_eq!(
        classify(passive, &[effect(0, AURA_GHOST)])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut missing_family = facts(99_008);
    missing_family.family_name = None;
    assert_eq!(
        classify(missing_family, &[effect(0, AURA_GHOST)]),
        Err(SelectedSpellPolarityError::MissingSpellFamilyName { index: 0 }),
    );

    let mut missing_flags = facts(99_009);
    missing_flags.family_name = Some(4);
    missing_flags.family_flags = None;
    assert_eq!(
        classify(missing_flags, &[effect(0, AURA_GHOST)]),
        Err(SelectedSpellPolarityError::MissingSpellFamilyFlags { index: 0 }),
    );

    let mut missing_mechanic = facts(99_010);
    missing_mechanic.spell_mechanic = None;
    assert_eq!(
        classify(missing_mechanic, &[effect(0, AURA_GHOST)]),
        Err(SelectedSpellPolarityError::MissingSpellMechanic { index: 0 }),
    );
}

#[test]
fn selected_flags_selected_aura_types_use_only_their_source_rules() {
    let mut ghost = effect(0, AURA_GHOST);
    ghost.implicit_target_1 = 700;
    assert_eq!(
        classify(facts(99_011), &[ghost])
            .unwrap()
            .negative_effect_mask,
        1
    );

    let mut speed = effect(0, AURA_SPEED);
    speed.implicit_target_1 = 1;
    speed.polarity_metadata.as_mut().unwrap().scaling_variance = f32::NAN;
    assert_eq!(
        classify(facts(99_012), &[speed])
            .unwrap()
            .positive_effect_mask,
        1
    );

    let mut none_target_speed = effect(0, AURA_SPEED);
    none_target_speed.implicit_target_1 = 0;
    assert_eq!(
        classify(facts(99_047), &[none_target_speed])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut enemy_speed = effect(0, AURA_SPEED);
    enemy_speed.implicit_target_1 = 6;
    assert_eq!(
        classify(facts(99_013), &[enemy_speed])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut unknown_speed = effect(0, AURA_SPEED);
    unknown_speed.implicit_target_1 = 700;
    assert_eq!(
        classify(facts(99_014), &[unknown_speed]),
        Err(SelectedSpellPolarityError::UnsupportedTarget { index: 0 }),
    );

    let mut negative_swim = effect(0, AURA_SWIM_SPEED);
    negative_swim.effect_base_points = -5;
    assert_eq!(
        classify(facts(99_015), &[negative_swim])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let zero_stat = effect(0, AURA_PERCENT_STAT);
    assert_eq!(
        classify(facts(99_016), &[zero_stat])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut negative_level_stat = effect(0, AURA_PERCENT_STAT);
    negative_level_stat.effect_base_points = -2;
    negative_level_stat.effect_die_sides = 2;
    negative_level_stat
        .polarity_metadata
        .as_mut()
        .unwrap()
        .real_points_per_level = -0.5;
    assert_eq!(
        classify(facts(99_017), &[negative_level_stat])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut negative_damage = effect(0, AURA_DAMAGE_PERCENT);
    negative_damage.effect_base_points = -2;
    negative_damage.implicit_target_1 = 1;
    assert_eq!(
        classify(facts(99_018), &[negative_damage])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut enemy_damage = effect(0, AURA_DAMAGE_PERCENT);
    enemy_damage.implicit_target_1 = 6;
    assert_eq!(
        classify(facts(99_019), &[enemy_damage])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let positive_damage = effect(0, AURA_DAMAGE_PERCENT);
    assert_eq!(
        classify(facts(99_051), &[positive_damage])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut no_pvp_credit = effect(0, AURA_NO_PVP_CREDIT);
    no_pvp_credit.effect_base_points = -90;
    no_pvp_credit.effect_trigger_spell = 123_456;
    no_pvp_credit.implicit_target_1 = 700;
    assert_eq!(
        classify(facts(99_020), &[no_pvp_credit])
            .unwrap()
            .positive_effect_mask,
        1,
    );
}

#[test]
fn selected_flags_transform_postpass_uses_only_later_matching_negative_effects() {
    let transform = effect(0, AURA_TRANSFORM);
    let matching_later_ghost = effect(2, AURA_GHOST);
    let polarity = classify(
        facts(99_021),
        &[transform.clone(), matching_later_ghost.clone()],
    )
    .unwrap();
    assert_eq!(polarity.negative_effect_mask & 0b101, 0b101);

    let mut different_target = matching_later_ghost.clone();
    different_target.implicit_target_1 = 6;
    let polarity = classify(facts(99_022), &[transform.clone(), different_target]).unwrap();
    assert_eq!(polarity.positive_effect_mask & 1, 1);

    let earlier_ghost = effect(0, AURA_GHOST);
    let later_transform = effect(2, AURA_TRANSFORM);
    let polarity = classify(facts(99_023), &[earlier_ghost, later_transform]).unwrap();
    assert_eq!(polarity.positive_effect_mask & (1 << 2), 1 << 2);

    let mut missing_hole = facts(99_053);
    missing_hole.preexisting_negative_effect_mask = 1 << 1;
    let mut transform_at_zero = effect(0, AURA_TRANSFORM);
    transform_at_zero.implicit_target_1 = 0;
    transform_at_zero.implicit_target_2 = 0;
    let polarity = classify(
        missing_hole,
        &[transform_at_zero.clone(), effect(2, AURA_NO_PVP_CREDIT)],
    )
    .unwrap();
    assert_eq!(polarity.negative_effect_mask & 0b011, 0b011);

    let mut beyond_vector = facts(99_054);
    beyond_vector.preexisting_negative_effect_mask = 1 << 5;
    let polarity = classify(beyond_vector, &[transform_at_zero]).unwrap();
    assert_eq!(polarity.positive_effect_mask & 1, 1);
    assert_eq!(polarity.negative_effect_mask & (1 << 5), 1 << 5);
}

#[test]
fn selected_flags_aura_unique_uses_only_known_positive_or_enemy_targets() {
    let mut unique_positive = facts(99_048);
    unique_positive.attributes[1] = 0x0000_0800;
    assert_eq!(
        classify(unique_positive, &[effect(0, AURA_NO_PVP_CREDIT)])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut unique_enemy = facts(99_049);
    unique_enemy.attributes[1] = 0x0000_0800;
    let mut enemy = effect(0, AURA_NO_PVP_CREDIT);
    enemy.implicit_target_1 = 6;
    assert_eq!(
        classify(unique_enemy, &[enemy])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut unique_unknown = facts(99_050);
    unique_unknown.attributes[1] = 0x0000_0800;
    let mut unknown = effect(0, AURA_NO_PVP_CREDIT);
    unknown.implicit_target_1 = 700;
    assert_eq!(
        classify(unique_unknown, &[unknown]),
        Err(SelectedSpellPolarityError::UnsupportedTarget { index: 0 }),
    );

    let mut unique_with_unselected_enemy = facts(99_055);
    unique_with_unselected_enemy.attributes[1] = 0x0000_0800;
    let mut unselected_enemy = effect(1, AURA_NO_PVP_CREDIT);
    unselected_enemy.implicit_target_1 = 6;
    let polarity = classify(
        unique_with_unselected_enemy,
        &[effect(0, AURA_NO_PVP_CREDIT), unselected_enemy],
    )
    .unwrap();
    assert_eq!(
        flags(
            polarity,
            1,
            guid(90_401),
            guid(90_402),
            guid(90_403),
            &unique_with_unselected_enemy.attributes,
            None,
        ),
        Ok(0x0010),
    );

    let mut unique_with_inert_enemy = facts(99_056);
    unique_with_inert_enemy.attributes[1] = 0x0000_0800;
    let mut inert_enemy = effect(1, AURA_NO_PVP_CREDIT);
    inert_enemy.effect = 0;
    inert_enemy.implicit_target_1 = 6;
    assert_eq!(
        classify(
            unique_with_inert_enemy,
            &[effect(0, AURA_NO_PVP_CREDIT), inert_enemy],
        )
        .unwrap()
        .positive_effect_mask,
        1,
    );
}

#[test]
fn selected_flags_keep_known_zero_metadata_distinct_from_missing_metadata() {
    let known_zero = effect(0, AURA_SWIM_SPEED);
    let polarity = classify(facts(99_024), &[known_zero]).unwrap();
    assert_eq!(polarity.positive_effect_mask, 1);

    let mut missing = effect(0, AURA_NO_PVP_CREDIT);
    missing.polarity_metadata = None;
    assert_eq!(
        classify(facts(99_025), &[missing]),
        Err(SelectedSpellPolarityError::MissingEffectMetadata { index: 0 }),
    );

    let mut global = facts(99_026);
    global.attributes[0] = 0x0400_0000;
    global.family_name = None;
    global.family_flags = None;
    global.spell_mechanic = None;
    let mut missing_but_global = effect(0, AURA_NO_PVP_CREDIT);
    missing_but_global.polarity_metadata = None;
    assert_eq!(
        classify(global, &[missing_but_global])
            .unwrap()
            .negative_effect_mask,
        1,
    );
}

#[test]
fn selected_flags_accept_sparse_masks_and_preserve_negative_inactive_holes() {
    let mut spell = facts(99_027);
    spell.preexisting_negative_effect_mask = 1 << 1;
    let mut inactive_hole = effect(1, AURA_GHOST);
    inactive_hole.effect = 0;
    let polarity = classify(spell, &[effect(0, AURA_NO_PVP_CREDIT), inactive_hole]).unwrap();
    assert_eq!(polarity.available_effect_mask, 1);
    assert_eq!(polarity.positive_effect_mask, 1);
    assert_eq!(polarity.negative_effect_mask, 1 << 1);
    assert!(!polarity.whole_spell_positive);

    let sparse = classify(
        facts(99_028),
        &[effect(0, AURA_NO_PVP_CREDIT), effect(3, AURA_GHOST)],
    )
    .unwrap();
    assert_eq!(sparse.available_effect_mask, 0b1001);
    assert_eq!(sparse.positive_effect_mask, 1);
    assert_eq!(sparse.negative_effect_mask & 0b1001, 0b1000);
}

#[test]
fn selected_flags_reject_duplicate_out_of_range_empty_and_unsupported_effects() {
    assert_eq!(
        classify(facts(99_029), &[]),
        Err(SelectedSpellPolarityError::EmptyActiveSpell),
    );
    let mut inert = effect(0, AURA_GHOST);
    inert.effect = 0;
    assert_eq!(
        classify(facts(99_030), &[inert]),
        Err(SelectedSpellPolarityError::EmptyActiveSpell),
    );

    assert_eq!(
        classify(
            facts(99_031),
            &[effect(2, AURA_GHOST), effect(2, AURA_NO_PVP_CREDIT)],
        ),
        Err(SelectedSpellPolarityError::DuplicateEffectIndex(2)),
    );
    assert_eq!(
        classify(facts(99_032), &[effect(32, AURA_GHOST)]),
        Err(SelectedSpellPolarityError::InvalidEffectIndex(32)),
    );

    let mut unsupported_effect = effect(0, AURA_GHOST);
    unsupported_effect.effect = 2;
    assert_eq!(
        classify(facts(99_033), &[unsupported_effect]),
        Err(SelectedSpellPolarityError::UnsupportedEffect {
            index: 0,
            effect: 2,
        }),
    );
    assert_eq!(
        classify(facts(99_034), &[effect(0, 1234)]),
        Err(SelectedSpellPolarityError::UnsupportedAuraType {
            index: 0,
            aura_type: 1234,
        }),
    );
}

#[test]
fn selected_flags_calc_value_uses_stable_source_rounded_intervals() {
    let mut negative = effect(0, AURA_SWIM_SPEED);
    negative.effect_base_points = -10;
    negative.effect_die_sides = 1;
    assert_eq!(
        classify(facts(99_035), &[negative])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut rounded_zero = effect(0, AURA_SWIM_SPEED);
    rounded_zero.effect_base_points = -1;
    rounded_zero.effect_die_sides = 1;
    assert_eq!(
        classify(facts(99_036), &[rounded_zero])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut scaled = effect(0, AURA_SWIM_SPEED);
    scaled.effect_base_points = -90;
    scaled
        .polarity_metadata
        .as_mut()
        .unwrap()
        .scaling_coefficient = 0.5;
    assert_eq!(
        classify(facts(99_037), &[scaled])
            .unwrap()
            .positive_effect_mask,
        1,
    );

    let mut stable_variance = effect(0, AURA_SWIM_SPEED);
    stable_variance.effect_base_points = -4;
    stable_variance.effect_die_sides = 1;
    stable_variance
        .polarity_metadata
        .as_mut()
        .unwrap()
        .scaling_variance = 0.5;
    assert_eq!(
        classify(facts(99_038), &[stable_variance])
            .unwrap()
            .negative_effect_mask,
        1,
    );

    let mut ambiguous = effect(0, AURA_SWIM_SPEED);
    ambiguous.effect_base_points = -2;
    ambiguous.effect_die_sides = 2;
    assert_eq!(
        classify(facts(99_039), &[ambiguous]),
        Err(SelectedSpellPolarityError::AmbiguousCalcValueSign { index: 0 }),
    );
}

#[test]
fn selected_flags_calc_value_rejects_nonfinite_and_out_of_range_intervals() {
    let mut nonfinite = effect(0, AURA_SWIM_SPEED);
    nonfinite
        .polarity_metadata
        .as_mut()
        .unwrap()
        .scaling_variance = f32::INFINITY;
    assert_eq!(
        classify(facts(99_040), &[nonfinite]),
        Err(SelectedSpellPolarityError::NonFinitePolarityData { index: 0 }),
    );

    let mut nonfinite_coefficient = effect(0, AURA_SWIM_SPEED);
    nonfinite_coefficient
        .polarity_metadata
        .as_mut()
        .unwrap()
        .scaling_coefficient = f32::NAN;
    assert_eq!(
        classify(facts(99_041), &[nonfinite_coefficient]),
        Err(SelectedSpellPolarityError::NonFinitePolarityData { index: 0 }),
    );

    let mut nonfinite_level = effect(0, AURA_PERCENT_STAT);
    nonfinite_level
        .polarity_metadata
        .as_mut()
        .unwrap()
        .real_points_per_level = f32::NAN;
    assert_eq!(
        classify(facts(99_052), &[nonfinite_level]),
        Err(SelectedSpellPolarityError::NonFinitePolarityData { index: 0 }),
    );

    let mut out_of_range = effect(0, AURA_SWIM_SPEED);
    out_of_range.effect_base_points = i32::MAX;
    out_of_range.effect_die_sides = 1;
    assert_eq!(
        classify(facts(99_042), &[out_of_range]),
        Err(SelectedSpellPolarityError::RoundedValueOutOfRange { index: 0 }),
    );

    let mut source_float_rounding = effect(0, AURA_SWIM_SPEED);
    source_float_rounding.effect_base_points = -16_777_217;
    source_float_rounding.effect_die_sides = 16_777_216;
    assert_eq!(
        classify(facts(99_057), &[source_float_rounding]),
        Err(SelectedSpellPolarityError::AmbiguousCalcValueSign { index: 0 }),
    );
}

#[test]
fn selected_flags_aura_flags_follow_self_hostile_friendly_and_unknown_rules() {
    let polarity = classify(
        facts(99_043),
        &[effect(0, AURA_NO_PVP_CREDIT), effect(1, AURA_GHOST)],
    )
    .unwrap();
    assert!(!polarity.whole_spell_positive);

    let caster = guid(90_101);
    let owner = guid(90_102);
    let target = guid(90_103);
    assert_eq!(
        flags(polarity, 0b01, caster, owner, target, &[0; 15], None),
        Ok(0x0100),
    );
    assert_eq!(
        flags(polarity, 0b10, caster, owner, target, &[0; 15], None),
        Ok(0x0010),
    );
    assert_eq!(
        flags(polarity, 0b11, target, owner, target, &[0; 15], None),
        Ok(0x0011),
    );
    assert_eq!(
        flags(polarity, 0b11, caster, owner, target, &[0; 15], Some(false)),
        Ok(0x0010),
    );
    assert_eq!(
        flags(polarity, 0b11, caster, owner, target, &[0; 15], Some(true)),
        Ok(0x0100),
    );
    assert_eq!(
        flags(polarity, 0b11, caster, owner, target, &[0; 15], None),
        Err(SelectedAuraFlagsError::UnknownForeignCasterRelation),
    );
}

#[test]
fn selected_flags_uniform_active_sign_needs_no_faction_facts() {
    let polarity = classify(
        facts(99_044),
        &[effect(0, AURA_NO_PVP_CREDIT), effect(1, AURA_GHOST)],
    )
    .unwrap();
    let raw_caster = guid(90_201);
    let target = guid(90_202);

    assert_eq!(
        flags(polarity, 1, raw_caster, target, target, &[0; 15], None),
        Ok(0x0102),
    );
    assert_eq!(
        flags(polarity, 2, raw_caster, target, target, &[0; 15], None),
        Ok(0x0010),
    );
    assert_eq!(
        flags(
            polarity,
            0,
            raw_caster,
            target,
            target,
            &[0; 15],
            Some(true)
        ),
        Err(SelectedAuraFlagsError::EmptyActiveEffectMask),
    );
    assert_eq!(
        flags(
            polarity,
            4,
            raw_caster,
            target,
            target,
            &[0; 15],
            Some(true)
        ),
        Err(SelectedAuraFlagsError::ActiveMaskOutsideAvailableEffects {
            requested: 4,
            available: 3,
        }),
    );
}

#[test]
fn selected_flags_exact_cancelable_passive_scalable_and_no_caster_bits() {
    let positive = classify(facts(99_045), &[effect(0, AURA_NO_PVP_CREDIT)]).unwrap();
    let negative = classify(facts(99_046), &[effect(0, AURA_GHOST)]).unwrap();
    let caster = guid(90_301);
    let target = guid(90_302);
    let mut scalable = [0; 15];
    scalable[8] = 0x0000_1000;

    let flags_value = flags(positive, 1, target, target, target, &scalable, None).unwrap();
    assert_eq!(flags_value, 0x010b);
    assert_eq!(flags_value & 0x0004, 0, "DURATION is projected elsewhere");

    assert_eq!(
        flags(positive, 1, caster, target, target, &scalable, None),
        Ok(0x010a),
    );

    let mut passive_scalable = scalable;
    passive_scalable[0] = 0x0000_0040;
    assert_eq!(
        flags(positive, 1, target, target, target, &passive_scalable, None,),
        Ok(0x0309),
    );

    let mut no_icon = scalable;
    no_icon[1] = 0x1000_0000;
    assert_eq!(
        flags(positive, 1, target, target, target, &no_icon, None),
        Ok(0x0109),
    );
    assert_eq!(
        flags(positive, 1, target, caster, target, &scalable, None),
        Ok(0x0109),
    );
    assert_eq!(
        flags(negative, 1, target, target, target, &scalable, None),
        Ok(0x0019),
    );

    assert_eq!(
        flags(positive, 1, target, target, target, &[0; 15], None),
        Ok(0x0103),
        "the seven selected aura types do not independently request amount sending",
    );
}
