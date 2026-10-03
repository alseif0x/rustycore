use super::super::Definition;
use super::*;
use crate::forever::spells::SpellLoadPlan;
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, forever_birth::item_records::ItemRecords, forever_spells::*,
};
mod fixtures;
use fixtures::*;

#[test]
fn expected_stat_mapping_covers_every_target_effect_aura_and_mana_condition() {
    // The finite source enum is 361 effects / 665 auras at 02245dcd.
    let aura_rows = [
        (
            &[3, 13, 15, 43, 53, 59, 62, 102, 131, 180][..],
            ExpectedStatType::CreatureSpellDamage,
        ),
        (
            &[8, 14, 34, 69, 84, 97, 115, 135, 161, 230, 250, 301][..],
            ExpectedStatType::PlayerHealth,
        ),
        (&[64][..], ExpectedStatType::PlayerMana),
        (&[29, 99, 124][..], ExpectedStatType::PlayerPrimaryStat),
        (&[189][..], ExpectedStatType::PlayerSecondaryStat),
        (&[22, 83, 123, 465][..], ExpectedStatType::ArmorConstant),
    ];
    for kind in 0..361 {
        for aura in 0..665 {
            for misc in [0, 1, -1, i32::MIN, i32::MAX] {
                let effect = SpellEffectValues {
                    effect: kind,
                    aura,
                    misc_values: [misc, 0],
                    ..Default::default()
                };
                let mut expected = ExpectedStatType::None;
                if [2, 7, 9, 17, 58].contains(&kind) {
                    expected = ExpectedStatType::CreatureSpellDamage;
                }
                if [10, 75].contains(&kind) {
                    expected = ExpectedStatType::PlayerHealth;
                }
                if kind == 8 || ([30, 62].contains(&kind) && misc == 0) {
                    expected = ExpectedStatType::PlayerMana;
                }
                if [6, 27, 35, 65, 119, 128, 129, 143, 174, 202, 271].contains(&kind) {
                    for (auras, stat) in &aura_rows {
                        if auras.contains(&aura) {
                            expected = *stat;
                        }
                    }
                    if misc == 0 && [24, 35, 73, 85, 162, 418].contains(&aura) {
                        expected = ExpectedStatType::PlayerMana;
                    }
                }
                assert_eq!(
                    effect.scaling_expected_stat(),
                    expected,
                    "{kind}/{aura}/{misc}"
                );
            }
        }
    }
}

#[test]
fn rounding_exact_set_includes_persistent_aura_but_excludes_nonrandom_party_aura() {
    for kind in 0..361 {
        for aura in 0..665 {
            let effect = SpellEffectValues {
                effect: kind,
                aura,
                ..Default::default()
            };
            let expected = [2, 7, 9, 10, 17, 31, 58, 67, 75, 121, 8, 30, 62].contains(&kind)
                || ([6, 27, 35, 65, 119, 128, 129, 143, 174, 202].contains(&kind)
                    && [3, 8, 53, 62, 70, 15, 43, 20, 21, 24, 64, 89, 162].contains(&aura));
            assert_eq!(effect.rounds_value(), expected, "{kind}/{aura}");
            for value in [-2.5, 2.5] {
                let result = finish_value(&effect, 0.0, Some(value), &mut no_draw).unwrap();
                assert_eq!(result.value, if expected { value.round() } else { value });
            }
        }
    }
    let party = SpellEffectValues {
        effect: 271,
        aura: 3,
        ..Default::default()
    };
    assert_eq!(
        party.scaling_expected_stat(),
        ExpectedStatType::CreatureSpellDamage
    );
    assert!(!party.rounds_value());
}

#[test]
fn source_level_order_is_spell_then_base_then_min_then_max_not_max_level() {
    let mut fields = SpellConstructorFields {
        spell_level: 2,
        base_level: 4,
        min_scaling_level: 5,
        max_scaling_level: 3,
        max_level: 1,
        ..Default::default()
    };
    fields.attributes[10] = USE_BASE_LEVEL;
    let effect = SpellEffectValues {
        scaling_class: 1,
        scaling_coefficient: 0.5,
        ..Default::default()
    };
    let c = catalog(Default::default());
    let gt = tables(5);
    let i = items(None);
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        155.0
    );
    fields.attributes[12] = FLOAT_AMOUNTS;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        154.5
    );
    fields.min_scaling_level = 0;
    fields.max_scaling_level = 0;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        204.5
    );
    fields.attributes[10] = 0;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        104.5
    );
}

#[test]
fn coefficient_floor_is_positive_only_and_f32_round_preserves_negative_zero() {
    let c = catalog(Default::default());
    let gt = tables(1);
    let i = items(None);
    let mut fields = SpellConstructorFields {
        spell_level: 1,
        ..Default::default()
    };
    let mut effect = SpellEffectValues {
        scaling_class: 1,
        scaling_coefficient: 0.001,
        ..Default::default()
    };
    fields.attributes[12] = FLOAT_AMOUNTS;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        1.0
    );
    effect.scaling_coefficient = -0.001;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
        f64::from(109.0f32 * -0.001f32)
    );
    fields.attributes[12] = 0;
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&gt), &i)
            .unwrap()
            .to_bits(),
        (-0.0f64).to_bits()
    );
}

#[test]
fn zero_level_and_class_early_return_do_not_require_a_scaling_row() {
    let c = catalog(Default::default());
    let i = items(None);
    let mut fields = SpellConstructorFields::default();
    let effect = SpellEffectValues {
        scaling_class: 0,
        scaling_coefficient: f32::NAN,
        ..Default::default()
    };
    assert!(base_value(&fields, &effect, &c, None, &i).unwrap().is_nan());
    fields.spell_level = 1;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 0.0);
    let effect = SpellEffectValues {
        scaling_class: 1,
        scaling_coefficient: 2.0,
        ..Default::default()
    };
    fields.spell_level = 0;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 0.0);
}

#[test]
fn recognized_selector_requires_row_but_unknown_selector_returns_source_zero() {
    let c = catalog(Default::default());
    let i = items(None);
    let fields = SpellConstructorFields {
        spell_level: 2,
        ..Default::default()
    };
    let mut effect = SpellEffectValues {
        scaling_class: 1,
        scaling_coefficient: 2.0,
        ..Default::default()
    };
    assert_eq!(
        base_value(&fields, &effect, &c, None, &i),
        Err(SpellValueError::MissingGameTables)
    );
    assert_eq!(
        base_value(&fields, &effect, &c, Some(&tables(1)), &i),
        Err(SpellValueError::MissingScalingRow)
    );
    for class in [16, i32::MAX, i32::MIN, -11] {
        effect.scaling_class = class;
        assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 0.0);
    }
}

#[test]
fn item_scaled_damage_uses_level_one_else_exact_last_slot_not_spell_or_base_level() {
    let mut fields = SpellConstructorFields {
        spell_level: 4,
        base_level: 9,
        ..Default::default()
    };
    fields.attributes[10] = USE_BASE_LEVEL;
    fields.attributes[11] = SCALES_ITEM_LEVEL;
    fields.attributes[12] = FLOAT_AMOUNTS;
    let mut effect = SpellEffectValues {
        scaling_class: -8,
        scaling_coefficient: 2.0,
        ..Default::default()
    };
    let i = items(None);
    let c = catalog(SpellRecords {
        rand_prop_points: vec![random_points(1, 7.0), random_points(9, 99.0)],
        ..Default::default()
    });
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 14.0);
    effect.scaling_class = -9;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 16.0);
    let c = catalog(SpellRecords {
        rand_prop_points: vec![random_points(9, 99.0)],
        ..Default::default()
    });
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 200.0);
    assert_eq!(
        base_value(&fields, &effect, &catalog(Default::default()), None, &i),
        Err(SpellValueError::InvalidCatalogInputs)
    );
}

#[test]
fn other_item_scaled_classes_use_rare_chest_superior_zero_and_genuine_missing_zero() {
    let mut fields = SpellConstructorFields {
        spell_level: 1,
        ..Default::default()
    };
    fields.attributes[11] = SCALES_ITEM_LEVEL;
    fields.attributes[12] = FLOAT_AMOUNTS;
    let mut effect = SpellEffectValues {
        scaling_coefficient: 2.0,
        ..Default::default()
    };
    let gt = tables(1);
    let i = items(None);
    let mut row = random_points(1, 9.0);
    row.superior_f = [7.5, 20.0, 30.0, 40.0, 50.0];
    row.epic_f = [100.0; 5];
    row.good_f = [200.0; 5];
    let c = catalog(SpellRecords {
        rand_prop_points: vec![row],
        ..Default::default()
    });
    for class in [-10, -7, -6, -5, -4, -3, -2, -1, 1, 15, 16, i32::MIN] {
        effect.scaling_class = class;
        assert_eq!(
            base_value(&fields, &effect, &c, Some(&gt), &i).unwrap(),
            15.0
        );
        assert_eq!(
            base_value(
                &fields,
                &effect,
                &catalog(Default::default()),
                Some(&gt),
                &i
            )
            .unwrap(),
            0.0
        );
    }
}

#[test]
fn rating_and_stamina_multipliers_query_optional_item_zero_with_same_width_inventory_bits() {
    let c = catalog(Default::default());
    let gt = tables(1);
    let mut fields = SpellConstructorFields {
        spell_level: 1,
        ..Default::default()
    };
    fields.attributes[12] = FLOAT_AMOUNTS;
    for class in [-7, -6] {
        let effect = SpellEffectValues {
            scaling_class: class,
            scaling_coefficient: 0.5,
            ..Default::default()
        };
        let (base, multipliers) = if class == -7 {
            (116.0, [2.0, 3.0, 5.0, 7.0])
        } else {
            (121.0, [11.0, 13.0, 17.0, 19.0])
        };
        assert_eq!(
            base_value(&fields, &effect, &c, Some(&gt), &items(None)).unwrap(),
            base * 0.5
        );
        for inventory in 0..=255u8 {
            let multiplier = if [2, 11].contains(&inventory) {
                multipliers[3]
            } else if inventory == 12 {
                multipliers[2]
            } else if [13, 14, 15, 17, 21, 22, 23, 26].contains(&inventory) {
                multipliers[1]
            } else {
                multipliers[0]
            };
            assert_eq!(
                base_value(&fields, &effect, &c, Some(&gt), &items(Some(inventory))).unwrap(),
                base * multiplier * 0.5
            );
        }
        let missing_multiplier_rows = SpellValueGameTables::parse_strs(
            &scaling_text(1),
            "id\tA\tB\tC\tD\n",
            "id\tA\tB\tC\tD\n",
        )
        .unwrap();
        assert_eq!(
            base_value(
                &fields,
                &effect,
                &c,
                Some(&missing_multiplier_rows),
                &items(Some(12))
            )
            .unwrap(),
            base * 0.5
        );
    }
}

#[test]
fn expected_branch_uses_tuning_expansion_but_level_tuning_class_and_season_are_default() {
    let mut fields = SpellConstructorFields {
        content_tuning_id: 7,
        spell_level: 99,
        base_level: 88,
        min_scaling_level: 77,
        max_scaling_level: 66,
        ..Default::default()
    };
    let effect = SpellEffectValues {
        effect: 10,
        base_points: 50.0,
        ..Default::default()
    };
    let c = expected_catalog();
    let i = items(None);
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 300.0);
    fields.attributes[0] = SCALES_CREATURE_LEVEL;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 11.0);
    fields.attributes[12] = FLOAT_AMOUNTS;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 10.5);
    fields.content_tuning_id = 99;
    fields.attributes[0] = 0;
    assert_eq!(base_value(&fields, &effect, &c, None, &i).unwrap(), 150.0);
}

#[test]
fn base_expected_math_is_f32_and_nonexpected_base_keeps_its_fraction_without_float_flag() {
    let mut row = stat(1, -2, 3.1415927);
    row.player_health = 3.1415927;
    let c = catalog(SpellRecords {
        expected_stats: vec![row],
        ..Default::default()
    });
    let mut fields = SpellConstructorFields::default();
    fields.attributes[12] = FLOAT_AMOUNTS;
    let effect = SpellEffectValues {
        effect: 10,
        base_points: 2.7182817,
        ..Default::default()
    };
    assert_eq!(
        base_value(&fields, &effect, &c, None, &items(None)).unwrap(),
        f64::from(f32::from_bits(0x3dae_e4cd))
    );
    fields.attributes[12] = 0;
    let effect = SpellEffectValues {
        effect: 3,
        base_points: 2.5,
        ..Default::default()
    };
    assert_eq!(
        base_value(&fields, &effect, &c, None, &items(None)).unwrap(),
        2.5
    );
}

#[test]
fn variance_draw_follows_base_calculation_uses_base_not_override_and_zero_does_not_draw() {
    let mut calls = Vec::new();
    let mut draw = |lo, hi| {
        calls.push((lo, hi));
        Ok(0.25)
    };
    let mut effect = SpellEffectValues {
        effect: 3,
        scaling_variance: -1.0,
        real_points_per_level: 100.0,
        points_per_resource: 200.0,
        scaling_resource_coefficient: 300.0,
        bonus_coefficient: 400.0,
        ..Default::default()
    };
    let result = finish_value(&effect, 100.0, Some(5.0), &mut draw).unwrap();
    assert_eq!(
        result,
        StartupSpellValue {
            value: 30.0,
            variance: Some(0.25)
        }
    );
    assert_eq!(calls, [(-0.5, 0.5)]);
    effect.scaling_variance = -0.0;
    assert_eq!(
        finish_value(&effect, 2.5, None, &mut no_draw).unwrap(),
        StartupSpellValue {
            value: 2.5,
            variance: None
        }
    );
}

#[test]
fn final_round_and_clamp_use_double_not_base_float_and_as_int_never_fabricates_nan_zero() {
    let mut effect = SpellEffectValues {
        effect: 2,
        ..Default::default()
    };
    assert_eq!(
        finish_value(&effect, 0.0, Some(2.4999999999), &mut no_draw)
            .unwrap()
            .value,
        2.0
    );
    effect.effect = 3;
    for (value, expected) in [
        (f64::INFINITY, MAX_VALUE),
        (f64::NEG_INFINITY, MIN_VALUE),
        (MAX_VALUE + 0.1, MAX_VALUE),
        (MIN_VALUE - 0.1, MIN_VALUE),
        (2.9, 2.9),
        (-2.9, -2.9),
    ] {
        let result = finish_value(&effect, 0.0, Some(value), &mut no_draw).unwrap();
        assert_eq!(result.value, expected);
        assert_eq!(result.as_int().unwrap(), expected as i32);
    }
    let result = finish_value(&effect, f64::NAN, None, &mut no_draw).unwrap();
    assert!(result.value.is_nan());
    assert_eq!(result.as_int(), Err(SpellValueError::UndefinedIntegerCast));
    assert_eq!(
        StartupSpellValue {
            value: f64::from(i32::MAX) + 0.9,
            variance: None
        }
        .as_int()
        .unwrap(),
        i32::MAX
    );
}

#[test]
fn invalid_variance_and_draw_failure_are_explicit_and_never_replace_the_draw_with_zero() {
    let mut effect = SpellEffectValues {
        scaling_variance: f32::NAN,
        ..Default::default()
    };
    assert_eq!(
        finish_value(&effect, 1.0, None, &mut no_draw),
        Err(SpellValueError::InvalidVarianceRange)
    );
    effect.scaling_variance = f32::INFINITY;
    assert_eq!(
        finish_value(&effect, 1.0, None, &mut no_draw),
        Err(SpellValueError::InvalidVarianceRange)
    );
    effect.scaling_variance = 1.0;
    assert_eq!(
        finish_value(&effect, 1.0, None, &mut |_, _| Err(
            SpellValueError::RandomSourceUnavailable
        )),
        Err(SpellValueError::RandomSourceUnavailable)
    );
    for sample in [f32::NAN, f32::INFINITY, 0.5001, -0.5001] {
        assert_eq!(
            finish_value(&effect, 1.0, None, &mut |_, _| Ok(sample)),
            Err(SpellValueError::InvalidVarianceDraw)
        );
    }
}

#[test]
fn public_startup_consumer_borrows_current_definition_and_retains_difficulty_lookup_errors() {
    let c = catalog(SpellRecords {
        difficulties: vec![difficulty(2, 0), difficulty(3, 4), difficulty(4, 3)],
        ..Default::default()
    });
    let mut seeds = SpellLoadPlan::build(Arc::new(c))
        .unwrap()
        .with_server_spells(Default::default())
        .unwrap();
    let mut definition = Definition::empty_server(Vec::new());
    definition.effects.push(SpellEffectValues {
        effect: 3,
        base_points: -2.5,
        ..Default::default()
    });
    seeds.definitions.insert((7, 0), definition);
    let i = items(None);
    assert_eq!(
        seeds.calculate_startup_base_value(7, 2, 0, &i).unwrap(),
        Some(-2.5)
    );
    let result = seeds
        .calculate_startup_value(7, 2, 0, &i, None, &mut no_draw)
        .unwrap()
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -2);
    assert_eq!(
        seeds.calculate_startup_base_value(99, 0, 0, &i).unwrap(),
        None
    );
    assert_eq!(
        seeds.calculate_startup_base_value(7, 0, 32, &i).unwrap(),
        None
    );
    assert_eq!(
        seeds.calculate_startup_base_value(99, 3, 0, &i),
        Err(SpellValueError::DefinitionLookup(
            SpellDefinitionError::DifficultyCycle
        ))
    );
    let effect = &mut seeds.definitions.get_mut(&(7, 0)).unwrap().effects[0];
    effect.scaling_coefficient = 1.0;
    effect.scaling_class = 1;
    seeds
        .definitions
        .get_mut(&(7, 0))
        .unwrap()
        .fields
        .spell_level = 1;
    assert_eq!(
        seeds.calculate_startup_value(7, 0, 0, &i, Some(1.0), &mut no_draw),
        Err(SpellValueError::MissingGameTables)
    );
}
