//! Spell-acquisition regressions, part 1 of 2.
//!
//! Moved out of the spell_acquisition_tests.rs root under #683; every test is unchanged.

use super::*;

#[test]
fn player_effect_targets_share_the_cpp_none_caster_and_ally_set() {
    let mut row = effect(1, 100, 0, 0, 36);
    for targets in [[0, 0], [1, 0], [21, 0], [1, 21]] {
        row.implicit_target_raw = targets;
        assert!(row.targets_player_like_cpp());
    }
    row.implicit_target_raw = [1, 5];
    assert!(!row.targets_player_like_cpp());
    row.implicit_target_raw = [6, 0];
    assert!(!row.targets_player_like_cpp());
}

#[test]
fn spell_effect_wdc_hydrates_planner_fields_from_cpp_physical_indices() {
    assert_eq!(
        [
            SPELL_EFFECT_WDC_CHAIN_TARGETS_FIELD,
            SPELL_EFFECT_WDC_POINTS_PER_RESOURCE_FIELD,
            SPELL_EFFECT_WDC_REAL_POINTS_PER_LEVEL_FIELD,
        ],
        [10, 14, 16]
    );

    let path = std::env::temp_dir().join(format!(
        "rustycore-spell-effect-planner-fields-{}.db2",
        std::process::id()
    ));
    std::fs::write(&path, minimal_spell_effect_wdc4()).expect("write SpellEffect WDC4 fixture");
    let reader = Wdc4Reader::open(&path).expect("open SpellEffect WDC4 fixture");
    let row = spell_acquisition_effect_from_wdc_like_cpp(77, 0, &reader);
    std::fs::remove_file(path).expect("remove SpellEffect WDC4 fixture");

    assert_eq!(row.effect_chain_targets_raw, 17);
    assert_eq!(row.effect_aura_raw, 147);
    assert_eq!(row.effect_mechanic_raw, 23);
    assert_eq!(row.effect_attributes_raw, 1);
    assert_eq!(row.effect_points_per_resource_bits, 1.75_f32.to_bits());
    assert_eq!(row.effect_real_points_per_level_bits, (-2.5_f32).to_bits());
    assert_eq!(row.effect_item_type_raw, 12_345);
}

#[test]
fn spell_effect_overlay_source_hydrates_planner_fields_from_projection_columns() {
    assert_eq!(
        [
            SPELL_EFFECT_SQL_CHAIN_TARGETS_COLUMN,
            SPELL_EFFECT_SQL_POINTS_PER_RESOURCE_COLUMN,
            SPELL_EFFECT_SQL_REAL_POINTS_PER_LEVEL_COLUMN,
            SPELL_EFFECT_SQL_ITEM_TYPE_COLUMN,
            SPELL_EFFECT_SQL_AURA_COLUMN,
            SPELL_EFFECT_SQL_MECHANIC_COLUMN,
            SPELL_EFFECT_SQL_ATTRIBUTES_COLUMN,
        ],
        [14, 15, 16, 17, 18, 19, 20]
    );
    let mut source = SentinelSpellEffectSqlSource {
        raw: [0; 21],
        f32_bits: [0; 21],
    };
    source.raw[14] = 23;
    source.f32_bits[15] = 3.25_f32.to_bits();
    source.f32_bits[16] = (-4.5_f32).to_bits();
    source.raw[17] = 54_321;
    source.raw[18] = 147;
    source.raw[19] = 23;
    source.raw[20] = 1;
    let row = spell_acquisition_effect_from_sql_source_like_cpp(88, &mut source);

    assert_eq!(row.effect_chain_targets_raw, 23);
    assert_eq!(row.effect_points_per_resource_bits, 3.25_f32.to_bits());
    assert_eq!(row.effect_real_points_per_level_bits, (-4.5_f32).to_bits());
    assert_eq!(row.effect_item_type_raw, 54_321);
    assert_eq!(row.effect_aura_raw, 147);
    assert_eq!(row.effect_mechanic_raw, 23);
    assert_eq!(row.effect_attributes_raw, 1);
}

#[test]
fn composition_is_base_then_official_then_custom_then_removal() {
    let table_hash = 0xAABB_CCDD;
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (table_hash, 2, 2),
        (table_hash, 3, 2),
        (table_hash, 3, 1),
        (table_hash, 4, 2),
    ]);
    let composed = compose_effective_table_with_removed_like_cpp(
        [(1, "base-1"), (2, "base-2")],
        [(1, "official-1"), (3, "official-sql-only")],
        [(1, "custom-1"), (4, "custom-sql-only")],
        table_hash,
        &removals,
    );
    let effective = &composed.effective_rows;

    assert_eq!(effective.get(&1), Some(&"custom-1"));
    assert!(!effective.contains_key(&2));
    assert_eq!(effective.get(&3), Some(&"official-sql-only"));
    assert!(!effective.contains_key(&4));
    assert_eq!(composed.removed_rows.get(&2), Some(&"base-2"));
    assert_eq!(composed.removed_rows.get(&4), Some(&"custom-sql-only"));
    assert!(!composed.removed_rows.contains_key(&3));
}

#[test]
fn every_source_family_uses_the_complete_overlay_and_removal_lifecycle() {
    fn assert_family<T, Make>(mut make: Make)
    where
        T: Clone + std::fmt::Debug + PartialEq,
        Make: FnMut(u32, i64) -> T,
    {
        let table_hash = 0xAABB_CCDD;
        let base_collision = make(1, 10);
        let removed_base = make(2, 20);
        let official_collision = make(1, 30);
        let official_sql_only = make(3, 40);
        let custom_collision = make(1, 50);
        let removed_custom_sql_only = make(4, 60);
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (table_hash, 2, 2),
            (table_hash, 3, 2),
            (table_hash, 3, 1),
            (table_hash, 4, 2),
        ]);

        let composed = compose_effective_table_with_removed_like_cpp(
            [(1, base_collision), (2, removed_base.clone())],
            [(1, official_collision), (3, official_sql_only.clone())],
            [
                (1, custom_collision.clone()),
                (4, removed_custom_sql_only.clone()),
            ],
            table_hash,
            &removals,
        );

        assert_eq!(composed.effective_rows.get(&1), Some(&custom_collision));
        assert_eq!(
            composed.effective_rows.get(&3),
            Some(&official_sql_only),
            "a later non-removal status must restore the SQL-only row"
        );
        assert_eq!(composed.removed_rows.get(&2), Some(&removed_base));
        assert_eq!(
            composed.removed_rows.get(&4),
            Some(&removed_custom_sql_only)
        );
    }

    assert_family(|record_id, marker| {
        let mut row = effect(record_id, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
        row.effect_base_points_raw = marker;
        row
    });
    assert_family(|record_id, marker| SpellAcquisitionDependencyLikeCpp {
        record_id,
        spell_id_raw: 100,
        learn_spell_id_raw: marker,
        overrides_spell_id_raw: 0,
    });
    assert_family(|record_id, marker| SpellAcquisitionMiscLikeCpp {
        record_id,
        spell_id_raw: 100,
        difficulty_id_raw: 0,
        attributes_raw: [marker, 0],
        show_future_spell_player_condition_id_raw: 0,
    });
    assert_family(|record_id, marker| SpellAcquisitionLevelsLikeCpp {
        record_id,
        spell_id_raw: 100,
        difficulty_id_raw: 0,
        base_level_raw: marker,
        spell_level_raw: 1,
    });
    assert_family(|record_id, marker| SpellAcquisitionTalentLikeCpp {
        record_id,
        spell_rank_raw: [marker, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    assert_family(
        |record_id, marker| SpellAcquisitionSummonPropertiesLikeCpp {
            record_id,
            slot_raw: marker,
            flags_1_raw: 0,
        },
    );
    assert_family(
        |record_id, marker| SpellAcquisitionBattlePetSpeciesLikeCpp {
            species_id: record_id,
            creature_id_raw: marker,
        },
    );
}

#[test]
fn invalid_overlay_replaces_stale_base_and_custom_can_repair_it() {
    let base = effect(7, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    let invalid_official = effect(7, 100, 0, -1, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    let custom = effect(7, 100, 0, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let removals = Db2HotfixRemovalStoreLikeCpp::default();

    let invalid_final = compose_effective_table_like_cpp(
        [(7, base.clone())],
        [(7, invalid_official)],
        [],
        0xAABB,
        &removals,
    );
    let invalid_catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: invalid_final.into_values().collect(),
            ..Default::default()
        },
    );
    assert!(matches!(
        invalid_catalog.acquisition_effects_like_cpp(100),
        SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
    ));

    let repaired_final = compose_effective_table_like_cpp(
        [(7, base)],
        [(7, effect(7, 100, 0, -1, 118))],
        [(7, custom)],
        0xAABB,
        &removals,
    );
    let repaired_catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: repaired_final.into_values().collect(),
            ..Default::default()
        },
    );
    let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
        repaired_catalog.acquisition_effects_like_cpp(100)
    else {
        panic!("custom repair must restore determinate coverage");
    };
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].record_id, 7);
    assert_eq!(
        effects[0].effect_type_checked(),
        Ok(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP)
    );
}

#[test]
fn typed_tombstones_cover_all_source_families_without_changing_final_coverage() {
    let removed_rows = vec![
        SpellAcquisitionRemovedRowLikeCpp::SpellEffect(effect(
            1,
            100,
            0,
            0,
            i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP),
        )),
        SpellAcquisitionRemovedRowLikeCpp::SpellLearnSpell(SpellAcquisitionDependencyLikeCpp {
            record_id: 2,
            spell_id_raw: 100,
            learn_spell_id_raw: 200,
            overrides_spell_id_raw: 0,
        }),
        SpellAcquisitionRemovedRowLikeCpp::SpellMisc(SpellAcquisitionMiscLikeCpp {
            record_id: 3,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            attributes_raw: [0, 0],
            show_future_spell_player_condition_id_raw: 0,
        }),
        SpellAcquisitionRemovedRowLikeCpp::SpellLevels(SpellAcquisitionLevelsLikeCpp {
            record_id: 4,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            base_level_raw: 1,
            spell_level_raw: 1,
        }),
        SpellAcquisitionRemovedRowLikeCpp::Talent(SpellAcquisitionTalentLikeCpp {
            record_id: 5,
            spell_rank_raw: [100, 0, 0, 0, 0, 0, 0, 0, 0],
        }),
        SpellAcquisitionRemovedRowLikeCpp::SummonProperties(
            SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 6,
                slot_raw: 0,
                flags_1_raw: 0,
            },
        ),
        SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(
            SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 7,
                creature_id_raw: 0,
            },
        ),
    ];
    let catalog = catalog_with_removed(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp::default(),
        removed_rows,
    );

    assert_eq!(catalog.removed_rows_like_cpp().len(), 7);
    assert_eq!(
        catalog
            .removed_rows_like_cpp()
            .iter()
            .map(SpellAcquisitionRemovedRowLikeCpp::table_like_cpp)
            .collect::<BTreeSet<_>>(),
        SpellAcquisitionTableLikeCpp::ALL.into_iter().collect()
    );
    assert_eq!(
        catalog.effects_for_spell_difficulty_like_cpp(100, 0),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(&[]),
        "a final removal is evidence, not an implicit indeterminate result"
    );
}

#[test]
fn coverage_distinguishes_zero_effects_missing_and_source_unavailable() {
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
                200,
                0,
                SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
            ),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp::default(),
    );

    assert_eq!(
        catalog.acquisition_effects_like_cpp(100),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(&[])
    );
    assert_eq!(
        catalog.acquisition_effects_like_cpp(300),
        SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
    );
    assert!(matches!(
        catalog.acquisition_effects_like_cpp(200),
        SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
    ));
}

#[test]
fn difficulty_none_slots_are_ordered_and_highest_record_id_wins() {
    let mut lower_slot = effect(10, 100, 0, 1, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    lower_slot.effect_misc_value_raw[0] = 777;
    let mut winner = effect(30, 100, 0, 1, i64::from(SPELL_EFFECT_LEARN_SPELL_LIKE_CPP));
    winner.effect_trigger_spell_raw = 900;
    let first = effect(20, 100, 0, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let mut other_difficulty = effect(40, 100, 2, 0, i64::from(SPELL_EFFECT_SKILL_STEP_LIKE_CPP));
    other_difficulty.effect_misc_value_raw[0] = 777;
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![winner, other_difficulty, lower_slot, first],
            ..Default::default()
        },
    );

    let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
        catalog.acquisition_effects_like_cpp(100)
    else {
        panic!("difficulty-none effects must be covered");
    };
    assert_eq!(
        effects.iter().map(|row| row.record_id).collect::<Vec<_>>(),
        vec![20, 30]
    );
    assert!(catalog.diagnostics_like_cpp().iter().any(|diagnostic| {
        matches!(
            diagnostic.kind,
            SpellAcquisitionDiagnosticKindLikeCpp::EffectSlotCollisionResolved {
                replaced_record_id: 10,
                winning_record_id: 30,
                ..
            }
        )
    }));
    let SpellAcquisitionEffectsLookupLikeCpp::Covered(other) =
        catalog.effects_for_spell_difficulty_like_cpp(100, 2)
    else {
        panic!("other difficulty must remain independently covered");
    };
    assert_eq!(other[0].record_id, 40);
}

#[test]
fn checked_values_do_not_narrow_skill_ids_and_expose_die_domain() {
    let mut row = effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    row.effect_misc_value_raw[0] = 70_000;
    row.effect_base_points_raw = 4;
    row.effect_die_sides_raw = 1;

    assert_eq!(row.misc_value_id_checked(0), Ok(70_000));
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 5,
            maximum: 5,
        })
    );
    row.effect_die_sides_raw = 3;
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 5,
            maximum: 7,
        })
    );
    row.effect_die_sides_raw = 0;
    row.effect_base_points_raw = i64::from(i32::MAX);
    assert!(
        row.base_points_die_sides_domain_checked().is_err(),
        "i32::MAX rounds to 2^31 in f32 and must not use Rust's saturating float cast"
    );
    row.difficulty_id_raw = 256;
    assert!(
        row.difficulty_id_checked().is_err(),
        "C++ Difficulty has an explicit uint8 source domain"
    );
}

#[test]
fn learn_skill_value_domain_honors_legacy_coefficient_and_variance() {
    let mut row = effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    row.effect_misc_value_raw[0] = 777;
    row.effect_base_points_raw = 4;
    row.effect_die_sides_raw = 1;

    row.effect_coefficient_bits = 1.0_f32.to_bits();
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 1,
            maximum: 1,
        }),
        "legacy Scaling.Class=0 makes a nonzero coefficient ignore BasePoints"
    );

    row.effect_coefficient_bits = 0.0_f32.to_bits();
    row.effect_variance_bits = 0.25_f32.to_bits();
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 5,
            maximum: 5,
        }),
        "frand's exclusive upper endpoint keeps the final value singleton"
    );

    row.effect_base_points_raw = 8;
    row.effect_die_sides_raw = 0;
    row.effect_variance_bits = 0.5_f32.to_bits();
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 6,
            maximum: 10,
        }),
        "a variance whose rounded outcomes differ remains explicitly ranged"
    );

    row.effect_base_points_raw = 4;
    row.effect_die_sides_raw = 1;
    row.effect_variance_bits = 0.25_f32.to_bits();
    row.effect_coefficient_bits = 1.0_f32.to_bits();
    assert_eq!(
        row.base_points_die_sides_domain_checked()
            .and_then(|domain| domain
                .deterministic_value()
                .ok_or_else(|| { invalid("test", 0, "deterministic") })),
        Ok(1),
        "variance over the coefficient-forced zero base remains inert"
    );

    row.effect_coefficient_bits = 0.0_f32.to_bits();
    row.effect_base_points_raw = -4;
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: -3,
            maximum: -3,
        }),
        "a negative base reverses which variance endpoint is open"
    );

    row.effect_base_points_raw = 4;
    row.effect_coefficient_bits = 0.0_f32.to_bits();
    row.effect_variance_bits = 0.0_f32.to_bits();
    row.effect_die_sides_raw = -3;
    assert_eq!(
        row.base_points_die_sides_domain_checked(),
        Ok(AcquisitionValueDomainLikeCpp {
            minimum: 1,
            maximum: 5,
        }),
        "negative DieSides uses C++'s inclusive [DieSides, 1] range"
    );

    row.effect_coefficient_bits = f32::NAN.to_bits();
    assert!(row.base_points_die_sides_domain_checked().is_err());
}

#[test]
fn dependencies_metadata_and_talent_membership_use_final_rows() {
    let mut attributes = [0_i64; 2];
    attributes[0] = i64::from(SPELL_ATTR0_PASSIVE_LIKE_CPP);
    attributes[1] = i64::from(SPELL_ATTR1_CAST_WHEN_LEARNED_LIKE_CPP);
    let rows = EffectiveSpellAcquisitionRowsLikeCpp {
        spell_learn_spells: vec![SpellAcquisitionDependencyLikeCpp {
            record_id: 10,
            spell_id_raw: 100,
            learn_spell_id_raw: 200,
            overrides_spell_id_raw: 300,
        }],
        spell_misc: vec![SpellAcquisitionMiscLikeCpp {
            record_id: 11,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            attributes_raw: attributes,
            show_future_spell_player_condition_id_raw: 44,
        }],
        spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
            record_id: 12,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            base_level_raw: 10,
            spell_level_raw: 20,
        }],
        talents: vec![SpellAcquisitionTalentLikeCpp {
            record_id: 13,
            spell_rank_raw: [100, 200, 0, 0, 0, 0, 0, 0, 0],
        }],
        ..Default::default()
    };
    let catalog = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);

    let dependency = &catalog.dependency_rows_from_spell_like_cpp(100)[0];
    assert_eq!(dependency.learned_spell_id_checked(), Ok(200));
    assert_eq!(dependency.overrides_spell_id_checked(), Ok(Some(300)));
    let SpellAcquisitionMetadataLookupLikeCpp::Present(misc) =
        catalog.misc_for_spell_like_cpp(100, 0)
    else {
        panic!("misc metadata missing");
    };
    assert_eq!(misc.is_passive_checked(), Ok(true));
    assert_eq!(misc.cast_when_learned_checked(), Ok(true));
    assert_eq!(misc.is_channeled_checked(), Ok(false));
    assert_eq!(misc.future_player_condition_id_checked(), Ok(Some(44)));
    let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
        catalog.levels_for_spell_like_cpp(100, 0)
    else {
        panic!("levels metadata missing");
    };
    assert_eq!(levels.base_level_checked(), Ok(10));
    assert_eq!(levels.spell_level_checked(), Ok(20));
    assert_eq!(
        catalog.talent_membership_like_cpp(100),
        SpellAcquisitionTalentLookupLikeCpp::Talent
    );
    assert_eq!(
        catalog.talent_membership_like_cpp(200),
        SpellAcquisitionTalentLookupLikeCpp::Talent
    );
    assert_eq!(
        catalog.talent_membership_like_cpp(300),
        SpellAcquisitionTalentLookupLikeCpp::NotTalent
    );
}

#[test]
fn invalidity_is_scoped_to_its_metadata_family() {
    let rows = EffectiveSpellAcquisitionRowsLikeCpp {
        spell_effects: vec![effect(
            1,
            100,
            0,
            0,
            i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP),
        )],
        spell_misc: vec![SpellAcquisitionMiscLikeCpp {
            record_id: 2,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            attributes_raw: [0, 0],
            show_future_spell_player_condition_id_raw: 0,
        }],
        spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
            record_id: 3,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            base_level_raw: i64::from(i16::MAX) + 1,
            spell_level_raw: 1,
        }],
        battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
            species_id: 4,
            creature_id_raw: -1,
        }],
        ..Default::default()
    };
    let scoped_catalog = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);

    assert!(matches!(
        scoped_catalog.effects_for_spell_difficulty_like_cpp(100, 0),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
    ));
    assert!(matches!(
        scoped_catalog.misc_for_spell_like_cpp(100, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Present(_)
    ));
    let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
        scoped_catalog.levels_for_spell_like_cpp(100, 0)
    else {
        panic!("semantic payload invalidity must not erase final metadata");
    };
    assert!(levels.base_level_checked().is_err());
    assert_eq!(levels.spell_level_checked(), Ok(1));

    let mut invalid_effect = effect(5, 200, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    invalid_effect.effect_misc_value_raw[0] = -1;
    let effect_catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![invalid_effect],
            spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                record_id: 6,
                spell_id_raw: 200,
                difficulty_id_raw: 0,
                attributes_raw: [0, 0],
                show_future_spell_player_condition_id_raw: 0,
            }],
            ..Default::default()
        },
    );
    let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
        effect_catalog.effects_for_spell_difficulty_like_cpp(200, 0)
    else {
        panic!("semantic payload invalidity must remain consumer-scoped");
    };
    assert!(effects[0].misc_value_id_checked(0).is_err());
    assert!(matches!(
        effect_catalog.misc_for_spell_like_cpp(200, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Present(_)
    ));
}

#[test]
fn irrelevant_effect_payload_does_not_hide_valid_acquisition_effects() {
    let mut runtime_only = effect(1, 100, 0, 0, 1);
    runtime_only.effect_base_points_raw = i64::from(i32::MAX) + 1;
    runtime_only.effect_die_sides_raw = i64::from(i32::MAX) + 1;
    runtime_only.effect_trigger_spell_raw = -1;
    runtime_only.effect_misc_value_raw = [-1, i64::from(i32::MAX) + 1];
    runtime_only.effect_coefficient_bits = f32::NAN.to_bits();
    runtime_only.effect_variance_bits = f32::INFINITY.to_bits();
    let mut dual_wield = effect(2, 100, 0, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    dual_wield.effect_base_points_raw = i64::from(i32::MAX) + 1;
    dual_wield.effect_coefficient_bits = f32::NAN.to_bits();
    let mut skill = effect(3, 100, 0, 2, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    skill.effect_misc_value_raw[0] = 777;
    skill.effect_base_points_raw = 1;

    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![runtime_only, dual_wield, skill],
            ..Default::default()
        },
    );

    let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
        catalog.acquisition_effects_like_cpp(100)
    else {
        panic!("payload unused by acquisition must not poison the whole spell");
    };
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.record_id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
}

#[test]
fn metadata_payload_validity_is_scoped_to_each_consumed_field() {
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_misc: vec![
                SpellAcquisitionMiscLikeCpp {
                    record_id: 1,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    attributes_raw: [i64::from(SPELL_ATTR0_PASSIVE_LIKE_CPP), 0],
                    show_future_spell_player_condition_id_raw: i64::from(i32::MAX) + 1,
                },
                SpellAcquisitionMiscLikeCpp {
                    record_id: 2,
                    spell_id_raw: 200,
                    difficulty_id_raw: 0,
                    attributes_raw: [i64::MAX, 0],
                    show_future_spell_player_condition_id_raw: 44,
                },
            ],
            spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                record_id: 3,
                spell_id_raw: 100,
                difficulty_id_raw: 0,
                base_level_raw: i64::from(i16::MAX) + 1,
                spell_level_raw: 20,
            }],
            ..Default::default()
        },
    );

    let SpellAcquisitionMetadataLookupLikeCpp::Present(first_misc) =
        catalog.misc_for_spell_like_cpp(100, 0)
    else {
        panic!("final Misc row must remain present");
    };
    assert_eq!(first_misc.is_passive_checked(), Ok(true));
    assert!(first_misc.future_player_condition_id_checked().is_err());

    let SpellAcquisitionMetadataLookupLikeCpp::Present(second_misc) =
        catalog.misc_for_spell_like_cpp(200, 0)
    else {
        panic!("final Misc row must remain present");
    };
    assert!(second_misc.is_passive_checked().is_err());
    assert_eq!(
        second_misc.future_player_condition_id_checked(),
        Ok(Some(44))
    );

    let SpellAcquisitionMetadataLookupLikeCpp::Present(levels) =
        catalog.levels_for_spell_like_cpp(100, 0)
    else {
        panic!("final Levels row must remain present");
    };
    assert!(levels.base_level_checked().is_err());
    assert_eq!(levels.spell_level_checked(), Ok(20));
}

#[test]
fn invalid_difficulty_is_scoped_to_the_related_spell() {
    let invalid_effect = effect(1, 100, 700, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![invalid_effect],
            spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                record_id: 2,
                spell_id_raw: 100,
                difficulty_id_raw: 700,
                attributes_raw: [0, 0],
                show_future_spell_player_condition_id_raw: 0,
            }],
            spell_levels: vec![SpellAcquisitionLevelsLikeCpp {
                record_id: 3,
                spell_id_raw: 100,
                difficulty_id_raw: 700,
                base_level_raw: 1,
                spell_level_raw: 1,
            }],
            ..Default::default()
        },
    );

    assert!(matches!(
        catalog.effects_for_spell_difficulty_like_cpp(100, 0),
        SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        catalog.misc_for_spell_like_cpp(100, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        catalog.levels_for_spell_like_cpp(100, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_)
    ));
    assert_eq!(
        catalog.effects_for_spell_difficulty_like_cpp(200, 0),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(&[])
    );
    assert_eq!(
        catalog.misc_for_spell_like_cpp(200, 0),
        SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow
    );
    assert_eq!(
        catalog.levels_for_spell_like_cpp(200, 0),
        SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow
    );
}

#[test]
fn shadowed_invalid_payload_does_not_poison_the_final_winner() {
    let mut invalid_lower = effect(10, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP));
    invalid_lower.effect_misc_value_raw[0] = -1;
    let winner = effect(20, 100, 0, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![winner, invalid_lower],
            spell_misc: vec![
                SpellAcquisitionMiscLikeCpp {
                    record_id: 10,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    attributes_raw: [i64::MAX, 0],
                    show_future_spell_player_condition_id_raw: 0,
                },
                SpellAcquisitionMiscLikeCpp {
                    record_id: 20,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    attributes_raw: [0, 0],
                    show_future_spell_player_condition_id_raw: 0,
                },
            ],
            spell_levels: vec![
                SpellAcquisitionLevelsLikeCpp {
                    record_id: 10,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    base_level_raw: i64::MAX,
                    spell_level_raw: 1,
                },
                SpellAcquisitionLevelsLikeCpp {
                    record_id: 20,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    base_level_raw: 1,
                    spell_level_raw: 1,
                },
            ],
            ..Default::default()
        },
    );

    let SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
        catalog.effects_for_spell_difficulty_like_cpp(100, 0)
    else {
        panic!("valid higher RecordID must determine the slot");
    };
    assert_eq!(effects[0].record_id, 20);
    assert!(matches!(
        catalog.misc_for_spell_like_cpp(100, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Present(row) if row.record_id == 20
    ));
    assert!(matches!(
        catalog.levels_for_spell_like_cpp(100, 0),
        SpellAcquisitionMetadataLookupLikeCpp::Present(row) if row.record_id == 20
    ));
}
