use super::*;
use crate::forever_spells::custom_source_fixtures as rows;

#[test]
fn unit_condition_uses_final_canonical_rows_ordered_overwrites_and_signed_removals() {
    let make = |flags| {
        let mut row = rows::unit_condition(u32::MAX);
        row.flags = flags;
        row
    };
    let mut baseline = SpellRecords {
        unit_conditions: vec![make(1)],
        ..Default::default()
    };
    baseline.unknown_baseline_records[48] = 9;
    let result = finish(
        baseline,
        SpellRecords {
            unit_conditions: vec![make(2), make(3)],
            ..Default::default()
        },
        SpellRecords {
            unit_conditions: vec![make(4), make(5)],
            ..Default::default()
        },
    )
    .unwrap();
    let row = result.unit_condition(u32::MAX).unwrap();
    assert_eq!(row.flags, 5);
    assert_eq!(row.variable, make(0).variable);
    assert_eq!(row.op, make(0).op);
    assert_eq!(row.value, make(0).value);
    assert_eq!(result.counts()[48], ("UnitCondition", 1, 9));
    assert!(std::ptr::eq(
        row,
        result.unit_condition_records().next().unwrap()
    ));
    let removed = SpellRecords {
        unit_conditions: vec![make(1)],
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        6,
        Default::default(),
        Default::default(),
        &removals([(SPELL_TABLE_HASHES[48], -1, 2)]),
    )
    .unwrap();
    assert!(removed.unit_condition(u32::MAX).is_none());
    assert_eq!(removed.unit_condition_records().count(), 0);
    assert!(
        finish(
            SpellRecords {
                unit_conditions: vec![make(1), make(2)],
                ..Default::default()
            },
            Default::default(),
            Default::default(),
        )
        .is_err()
    );
    let mut official = SpellRecords::default();
    official.unknown_baseline_records[48] = 1;
    assert!(finish(Default::default(), official, Default::default()).is_err());
}

#[test]
fn missile_set_index_borrows_only_final_records_in_ascending_storage_id_order() {
    let missile = |id, set| {
        let mut row = rows::spell_visual_missile(id, 6, b"");
        row.spell_visual_missile_set_id = set;
        row
    };
    let result = SpellRecords {
        spell_visual_missiles: vec![missile(9, 1), missile(3, 1), missile(7, 2)],
        ..Default::default()
    }
    .finish(
        SpellRecords {
            spell_visual_missiles: vec![missile(7, 1)],
            ..Default::default()
        },
        SpellRecords {
            spell_visual_missiles: vec![missile(5, 1)],
            ..Default::default()
        },
        6,
        Default::default(),
        Default::default(),
        &removals([(SPELL_TABLE_HASHES[39], 9, 2)]),
    )
    .unwrap();
    assert_eq!(
        result
            .spell_visual_missiles_for_set(1)
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [3, 5, 7]
    );
    assert_eq!(result.spell_visual_missiles_for_set(2).count(), 0);
    assert_eq!(result.spell_visual_missiles_for_set(u32::MAX).count(), 0);
    for row in result.spell_visual_missiles_for_set(1) {
        assert!(std::ptr::eq(
            row,
            result.spell_visual_missile(row.id).unwrap()
        ));
    }
}

fn batch(id: u32, locale: u8, text: &[u8], marker: i32) -> SpellRecords {
    let mut result = SpellRecords {
        talents: vec![rows::talent(id, locale, text)],
        spell_item_enchantments: vec![rows::spell_item_enchantment(id, locale, text)],
        spell_visuals: vec![rows::spell_visual(id, locale, text)],
        spell_visual_missiles: vec![rows::spell_visual_missile(id, locale, text)],
        spell_visual_effect_names: vec![rows::spell_visual_effect_name(id, locale, text)],
        liquid_types: vec![rows::liquid_type(id, locale, text)],
        ..Default::default()
    };
    result.talents[0].flags = marker;
    result.spell_item_enchantments[0].flags = marker;
    result.spell_visuals[0].flags = marker;
    result.spell_visual_missiles[0].flags = marker;
    result.spell_visual_effect_names[0].flags = marker;
    result.liquid_types[0].flags = marker;
    result
}

#[test]
fn six_dependency_overlays_retain_observed_order_unknown_counts_and_all_fields() {
    let mut baseline = batch(7, 6, b"base", 1);
    for index in 36..42 {
        baseline.unknown_baseline_records[index] = index;
    }
    let mut official = batch(7, 0, b"main", 2);
    let later = batch(7, 0, b"later", 3);
    official.talents.extend(later.talents);
    official
        .spell_item_enchantments
        .extend(later.spell_item_enchantments);
    official.spell_visuals.extend(later.spell_visuals);
    official
        .spell_visual_missiles
        .extend(later.spell_visual_missiles);
    official
        .spell_visual_effect_names
        .extend(later.spell_visual_effect_names);
    official.liquid_types.extend(later.liquid_types);
    let result = finish(baseline, official, batch(7, 0, b"", 4)).unwrap();
    assert_eq!(result.talent(7).unwrap().flags, 4);
    assert_eq!(result.counts()[36], ("Talent", 1, 36));
    assert_eq!(result.spell_item_enchantment(7).unwrap().flags, 4);
    assert_eq!(result.counts()[37], ("SpellItemEnchantment", 1, 37));
    assert_eq!(result.spell_visual(7).unwrap().flags, 4);
    assert_eq!(result.counts()[38], ("SpellVisual", 1, 38));
    assert_eq!(result.spell_visual_missile(7).unwrap().flags, 4);
    assert_eq!(result.counts()[39], ("SpellVisualMissile", 1, 39));
    assert_eq!(result.spell_visual_effect_name(7).unwrap().flags, 4);
    assert_eq!(result.counts()[40], ("SpellVisualEffectName", 1, 40));
    assert_eq!(result.liquid_type(7).unwrap().flags, 4);
    assert_eq!(result.counts()[41], ("LiquidType", 1, 41));
    assert_eq!(
        result.talent(7).unwrap().description.at(0),
        Some(b"later".as_slice())
    );
    assert_eq!(
        result.talent(7).unwrap().description.at(6),
        Some(b"base".as_slice())
    );
    assert_eq!(
        result.spell_item_enchantment(7).unwrap().name.at(0),
        Some(b"later".as_slice())
    );
    assert_eq!(
        result.spell_item_enchantment(7).unwrap().horde_name.at(6),
        Some(b"base".as_slice())
    );
    assert!(result.liquid_type(7).unwrap().name.is_empty()); // Plain strings replace, not localized merge.
    assert_eq!(result.talent(7).unwrap().spell_rank[8], 0x800000B8);
    assert_eq!(
        result.liquid_type(7).unwrap().float_values[37].to_bits(),
        0x7FC00145
    );
}

fn locales(id: u32, text: &[u8]) -> SpellLocaleRecords {
    SpellLocaleRecords {
        talents: vec![TalentLocaleRecord {
            id,
            description: text.to_vec(),
        }],
        spell_item_enchantments: vec![SpellItemEnchantmentLocaleRecord {
            id,
            name: text.to_vec(),
            horde_name: text.to_vec(),
        }],
        ..Default::default()
    }
}

#[test]
fn new_locale_families_update_only_existing_rows_ignore_empty_and_apply_before_removal() {
    let mut custom = locales(7, b"");
    let absent = locales(99, b"absent");
    custom.talents.extend(absent.talents);
    custom
        .spell_item_enchantments
        .extend(absent.spell_item_enchantments);
    let result = batch(7, 6, b"base", 1)
        .finish(
            batch(7, 0, b"main", 2),
            Default::default(),
            6,
            locales(7, b"translated"),
            custom,
            &removals([]),
        )
        .unwrap();
    assert_eq!(
        result.talent(7).unwrap().description.at(6),
        Some(b"translated".as_slice())
    );
    assert_eq!(
        result.talent(7).unwrap().description.at(0),
        Some(b"main".as_slice())
    );
    for text in [
        &result.spell_item_enchantment(7).unwrap().name,
        &result.spell_item_enchantment(7).unwrap().horde_name,
    ] {
        assert_eq!(text.at(6), Some(b"translated".as_slice()));
        assert_eq!(text.at(0), Some(b"main".as_slice()));
    }
    assert!(result.talent(99).is_none());
    assert!(result.spell_item_enchantment(99).is_none());
    let result = batch(u32::MAX, 6, b"base", 1)
        .finish(
            batch(u32::MAX, 0, b"main", 2),
            batch(u32::MAX, 0, b"custom", 3),
            6,
            locales(u32::MAX, b"official"),
            locales(u32::MAX, b"custom"),
            &removals(SPELL_TABLE_HASHES[36..42].iter().map(|&hash| (hash, -1, 2))),
        )
        .unwrap();
    assert!(
        result.counts()[36..42]
            .iter()
            .all(|(_, count, _)| *count == 0)
    );
}

#[test]
fn each_dependency_rejects_duplicate_baselines_false_sql_unknowns_and_non_main_locales() {
    assert!(
        finish(
            SpellRecords {
                talents: vec![rows::talent(7, 6, b""), rows::talent(7, 6, b"")],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                spell_item_enchantments: vec![
                    rows::spell_item_enchantment(7, 6, b""),
                    rows::spell_item_enchantment(7, 6, b"")
                ],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                spell_visuals: vec![rows::spell_visual(7, 6, b""), rows::spell_visual(7, 6, b"")],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                spell_visual_missiles: vec![
                    rows::spell_visual_missile(7, 6, b""),
                    rows::spell_visual_missile(7, 6, b"")
                ],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                spell_visual_effect_names: vec![
                    rows::spell_visual_effect_name(7, 6, b""),
                    rows::spell_visual_effect_name(7, 6, b"")
                ],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        finish(
            SpellRecords {
                liquid_types: vec![rows::liquid_type(7, 6, b""), rows::liquid_type(7, 6, b"")],
                ..Default::default()
            },
            Default::default(),
            Default::default()
        )
        .is_err()
    );
    for index in 36..42 {
        let mut false_coverage = SpellRecords::default();
        false_coverage.unknown_baseline_records[index] = 1;
        assert!(finish(Default::default(), false_coverage, Default::default()).is_err());
    }
    for value in [
        SpellRecords {
            talents: vec![rows::talent(7, 6, b"wrong SQL locale")],
            ..Default::default()
        },
        SpellRecords {
            spell_item_enchantments: vec![rows::spell_item_enchantment(7, 6, b"wrong SQL locale")],
            ..Default::default()
        },
    ] {
        assert!(finish(Default::default(), value, Default::default()).is_err());
    }
}
