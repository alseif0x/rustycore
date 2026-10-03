use super::*;
mod custom_sources;
mod fixtures;
mod value_bounds;
mod value_inputs;
use fixtures::*;
fn removals(rows: impl IntoIterator<Item = (u32, i32, u8)>) -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp(rows)
}
fn finish(
    base: SpellRecords,
    official: SpellRecords,
    custom: SpellRecords,
) -> Result<SpellCatalog> {
    base.finish(
        official,
        custom,
        6,
        SpellLocaleRecords::default(),
        SpellLocaleRecords::default(),
        &removals([]),
    )
}
fn cast(id: u32, base: i32) -> SpellCastTimesRecord {
    SpellCastTimesRecord {
        id,
        base,
        minimum: -1,
    }
}

#[test]
fn numeric_overlays_keep_query_order_and_unknown_coverage_through_final_removal() {
    let mut baseline = SpellRecords {
        spell_cast_times: vec![cast(1, 1), cast(2, 2)],
        ..Default::default()
    };
    baseline.unknown_baseline_records[25] = 7;
    let official = SpellRecords {
        spell_cast_times: vec![cast(1, 3), cast(1, 4), cast(3, 5)],
        ..Default::default()
    };
    let custom = SpellRecords {
        spell_cast_times: vec![cast(1, 6), cast(1, 7), cast(2, 8)],
        ..Default::default()
    };
    let result = baseline
        .finish(
            official,
            custom,
            6,
            Default::default(),
            Default::default(),
            &removals([(SPELL_TABLE_HASHES[25], 2, 2)]),
        )
        .unwrap();
    assert_eq!(result.spell_cast_times(1).unwrap().base, 7);
    assert!(result.spell_cast_times(2).is_none());
    assert_eq!(result.spell_cast_times(3).unwrap().base, 5);
    assert_eq!(result.counts()[25], ("SpellCastTimes", 2, 7));
}

#[test]
fn all_six_localized_families_preserve_other_locales_and_empty_update_semantics() {
    let baseline = SpellRecords {
        spell_names: vec![spell_name(1, 6, b"baseline")],
        difficulties: vec![difficulty(1, 6, b"baseline")],
        spell_ranges: vec![spell_range(1, 6, b"baseline")],
        spell_shapeshift_forms: vec![spell_shapeshift_form(1, 6, b"baseline")],
        battle_pet_species: vec![battle_pet_species(1, 6, b"baseline")],
        spell_category_definitions: vec![spell_category(1, 6, b"baseline")],
        ..Default::default()
    };
    let official = SpellRecords {
        spell_names: vec![spell_name(1, 0, b"main")],
        difficulties: vec![difficulty(1, 0, b"main")],
        spell_ranges: vec![spell_range(1, 0, b"main")],
        spell_shapeshift_forms: vec![spell_shapeshift_form(1, 0, b"main")],
        battle_pet_species: vec![battle_pet_species(1, 0, b"main")],
        spell_category_definitions: vec![spell_category(1, 0, b"main")],
        ..Default::default()
    };
    let custom = SpellRecords {
        spell_names: vec![spell_name(1, 0, b"")],
        difficulties: vec![difficulty(1, 0, b"")],
        spell_ranges: vec![spell_range(1, 0, b"")],
        spell_shapeshift_forms: vec![spell_shapeshift_form(1, 0, b"")],
        battle_pet_species: vec![battle_pet_species(1, 0, b"")],
        spell_category_definitions: vec![spell_category(1, 0, b"")],
        ..Default::default()
    };
    let locale_official = SpellLocaleRecords {
        spell_names: vec![SpellNameLocaleRecord {
            id: 1,
            name: b"locale".to_vec(),
        }],
        difficulties: vec![DifficultyLocaleRecord {
            id: 1,
            name: b"locale".to_vec(),
        }],
        spell_ranges: vec![SpellRangeLocaleRecord {
            id: 1,
            display_name: b"locale".to_vec(),
            display_name_short: b"locale".to_vec(),
        }],
        spell_shapeshift_forms: vec![SpellShapeshiftFormLocaleRecord {
            id: 1,
            name: b"locale".to_vec(),
        }],
        battle_pet_species: vec![BattlePetSpeciesLocaleRecord {
            id: 1,
            description: b"locale".to_vec(),
            source_text: b"locale".to_vec(),
        }],
        spell_category_definitions: vec![SpellCategoryLocaleRecord {
            id: 1,
            name: b"locale".to_vec(),
        }],
        ..Default::default()
    };
    let locale_custom = SpellLocaleRecords {
        spell_names: vec![
            SpellNameLocaleRecord {
                id: 1,
                name: Vec::new(),
            },
            SpellNameLocaleRecord {
                id: 99,
                name: b"absent".to_vec(),
            },
        ],
        difficulties: vec![
            DifficultyLocaleRecord {
                id: 1,
                name: Vec::new(),
            },
            DifficultyLocaleRecord {
                id: 99,
                name: b"absent".to_vec(),
            },
        ],
        spell_ranges: vec![
            SpellRangeLocaleRecord {
                id: 1,
                display_name: Vec::new(),
                display_name_short: Vec::new(),
            },
            SpellRangeLocaleRecord {
                id: 99,
                display_name: b"absent".to_vec(),
                display_name_short: b"absent".to_vec(),
            },
        ],
        spell_shapeshift_forms: vec![
            SpellShapeshiftFormLocaleRecord {
                id: 1,
                name: Vec::new(),
            },
            SpellShapeshiftFormLocaleRecord {
                id: 99,
                name: b"absent".to_vec(),
            },
        ],
        battle_pet_species: vec![
            BattlePetSpeciesLocaleRecord {
                id: 1,
                description: Vec::new(),
                source_text: Vec::new(),
            },
            BattlePetSpeciesLocaleRecord {
                id: 99,
                description: b"absent".to_vec(),
                source_text: b"absent".to_vec(),
            },
        ],
        spell_category_definitions: vec![
            SpellCategoryLocaleRecord {
                id: 1,
                name: Vec::new(),
            },
            SpellCategoryLocaleRecord {
                id: 99,
                name: b"absent".to_vec(),
            },
        ],
        ..Default::default()
    };
    let result = baseline
        .finish(
            official,
            custom,
            6,
            locale_official,
            locale_custom,
            &removals([]),
        )
        .unwrap();
    assert_eq!(result.spell_name(1).unwrap().name.at(0), Some(&b"main"[..]));
    assert_eq!(
        result.spell_name(1).unwrap().name.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.spell_name(99).is_none());
    assert_eq!(result.difficulty(1).unwrap().name.at(0), Some(&b"main"[..]));
    assert_eq!(
        result.difficulty(1).unwrap().name.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.difficulty(99).is_none());
    assert_eq!(
        result.spell_range(1).unwrap().display_name.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.spell_range(1).unwrap().display_name.at(6),
        Some(&b"locale"[..])
    );
    assert_eq!(
        result.spell_range(1).unwrap().display_name_short.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.spell_range(1).unwrap().display_name_short.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.spell_range(99).is_none());
    assert_eq!(
        result.spell_shapeshift_form(1).unwrap().name.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.spell_shapeshift_form(1).unwrap().name.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.spell_shapeshift_form(99).is_none());
    assert_eq!(
        result.battle_pet_species(1).unwrap().description.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.battle_pet_species(1).unwrap().description.at(6),
        Some(&b"locale"[..])
    );
    assert_eq!(
        result.battle_pet_species(1).unwrap().source_text.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.battle_pet_species(1).unwrap().source_text.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.battle_pet_species(99).is_none());
    assert_eq!(
        result.spell_category(1).unwrap().name.at(0),
        Some(&b"main"[..])
    );
    assert_eq!(
        result.spell_category(1).unwrap().name.at(6),
        Some(&b"locale"[..])
    );
    assert!(result.spell_category(99).is_none());
}

#[test]
fn repeated_new_ids_start_fresh_but_existing_ids_keep_prior_nonempty_text() {
    let baseline = SpellRecords {
        spell_names: vec![spell_name(1, 6, b"base")],
        ..Default::default()
    };
    let official = SpellRecords {
        spell_names: vec![
            spell_name(1, 0, b"official"),
            spell_name(1, 0, b""),
            spell_name(2, 0, b"discarded"),
            spell_name(2, 0, b""),
            spell_name(3, 0, b"retained"),
        ],
        ..Default::default()
    };
    let custom = SpellRecords {
        spell_names: vec![spell_name(3, 0, b"custom"), spell_name(3, 0, b"")],
        ..Default::default()
    };
    let result = finish(baseline, official, custom).unwrap();
    assert_eq!(
        result.spell_name(1).unwrap().name.at(0),
        Some(&b"official"[..])
    );
    assert_eq!(result.spell_name(1).unwrap().name.at(6), Some(&b"base"[..]));
    assert_eq!(result.spell_name(2).unwrap().name.at(0), Some(&b""[..]));
    assert_eq!(
        result.spell_name(3).unwrap().name.at(0),
        Some(&b"custom"[..])
    );
}

#[test]
fn table_hashes_are_distinct_and_signed_removal_ids_do_not_alias_families() {
    assert_eq!(
        SPELL_TABLE_HASHES
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        49
    );
    let base = SpellRecords {
        spell_cast_times: vec![cast(u32::MAX, 1)],
        spell_durations: vec![SpellDurationRecord {
            id: u32::MAX,
            duration: -1,
            max_duration: -2,
            duration_per_resource: -3,
        }],
        ..Default::default()
    };
    let result = base
        .finish(
            Default::default(),
            Default::default(),
            6,
            Default::default(),
            Default::default(),
            &removals([(SPELL_TABLE_HASHES[25], -1, 2)]),
        )
        .unwrap();
    assert!(result.spell_cast_times(u32::MAX).is_none());
    assert_eq!(result.spell_duration(u32::MAX).unwrap().duration, -1);
}

#[test]
fn duplicate_baselines_invalid_locales_and_false_sql_unknown_coverage_are_rejected() {
    let duplicates = SpellRecords {
        spell_cast_times: vec![cast(1, 1), cast(1, 2)],
        ..Default::default()
    };
    assert!(finish(duplicates, Default::default(), Default::default()).is_err());
    for locale in [9, 12, 255] {
        assert!(
            SpellRecords::default()
                .finish(
                    Default::default(),
                    Default::default(),
                    locale,
                    Default::default(),
                    Default::default(),
                    &removals([])
                )
                .is_err()
        );
    }
    let wrong_locale = SpellRecords {
        spell_names: vec![spell_name(1, 6, b"not main enUS")],
        ..Default::default()
    };
    assert!(finish(Default::default(), wrong_locale, Default::default()).is_err());
    let mut wrong_unknown = SpellRecords::default();
    wrong_unknown.unknown_baseline_records[0] = 1;
    assert!(finish(Default::default(), Default::default(), wrong_unknown).is_err());
}

#[test]
fn localized_removal_is_final_and_a_later_status_can_revoke_removal() {
    let result = SpellRecords {
        spell_names: vec![spell_name(1, 6, b"base"), spell_name(2, 6, b"base")],
        ..Default::default()
    }
    .finish(
        Default::default(),
        Default::default(),
        6,
        SpellLocaleRecords {
            spell_names: vec![SpellNameLocaleRecord {
                id: 1,
                name: b"locale".to_vec(),
            }],
            ..Default::default()
        },
        Default::default(),
        &removals([
            (SPELL_TABLE_HASHES[0], 1, 2),
            (SPELL_TABLE_HASHES[0], 2, 2),
            (SPELL_TABLE_HASHES[0], 2, 1),
        ]),
    )
    .unwrap();
    assert!(result.spell_name(1).is_none());
    assert_eq!(result.spell_name(2).unwrap().name.at(6), Some(&b"base"[..]));
}
