//! Spell-acquisition regressions, part 2 of 2.
//!
//! Moved out of the spell_acquisition_tests.rs root under #683; every test is unchanged.

use super::*;

#[test]
fn difficulty_fallback_merges_effect_slots_and_uses_first_metadata() {
    let requested_slot = effect(30, 100, 2, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let shadowed_fallback_slot = effect(20, 100, 1, 0, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let fallback_slot = effect(21, 100, 1, 1, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let final_slot = effect(10, 100, 0, 2, i64::from(SPELL_EFFECT_DUAL_WIELD_LIKE_CPP));
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 1),
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![
                final_slot,
                fallback_slot,
                shadowed_fallback_slot,
                requested_slot,
            ],
            spell_misc: vec![
                SpellAcquisitionMiscLikeCpp {
                    record_id: 1,
                    spell_id_raw: 100,
                    difficulty_id_raw: 1,
                    attributes_raw: [0, 0],
                    show_future_spell_player_condition_id_raw: 11,
                },
                SpellAcquisitionMiscLikeCpp {
                    record_id: 2,
                    spell_id_raw: 100,
                    difficulty_id_raw: 0,
                    attributes_raw: [0, 0],
                    show_future_spell_player_condition_id_raw: 22,
                },
            ],
            ..Default::default()
        },
    );

    let SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) =
        catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [2, 1, 0])
    else {
        panic!("complete fallback chain must resolve");
    };
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.record_id)
            .collect::<Vec<_>>(),
        vec![30, 21, 10]
    );
    assert!(matches!(
        catalog.resolved_misc_for_difficulty_chain_like_cpp(100, [2, 1, 0]),
        SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row)
            if row.record_id == 1
    ));
    let SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) =
        catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [2, 9, 1, 0])
    else {
        panic!("absent intermediate fallback must be skipped");
    };
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.record_id)
            .collect::<Vec<_>>(),
        vec![30, 21, 10]
    );
    assert!(matches!(
        catalog.resolved_misc_for_difficulty_chain_like_cpp(100, [2, 9, 1, 0]),
        SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(row)
            if row.record_id == 1
    ));
    assert!(matches!(
        catalog.resolved_effects_for_difficulty_chain_like_cpp(100, [9, 2]),
        SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { difficulty_id: 9 }
    ));
}

#[test]
fn all_final_effects_remain_visible_to_the_planner() {
    let unsupported_runtime_effect = effect(1, 100, 0, 0, 3);
    let learn_effect = {
        let mut row = effect(2, 100, 0, 1, i64::from(SPELL_EFFECT_LEARN_SPELL_LIKE_CPP));
        row.effect_trigger_spell_raw = 200;
        row
    };
    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![unsupported_runtime_effect, learn_effect],
            ..Default::default()
        },
    );

    let SpellAcquisitionEffectsLookupLikeCpp::Covered(all_effects) =
        catalog.difficulty_none_effects_like_cpp(100)
    else {
        panic!("all effects must be available");
    };
    assert_eq!(all_effects.len(), 2);
    assert_eq!(all_effects[0].effect_type_checked(), Ok(3));

    let SpellAcquisitionEffectsLookupLikeCpp::Covered(acquisition_effects) =
        catalog.acquisition_effects_like_cpp(100)
    else {
        panic!("filtered acquisition effects must be available");
    };
    assert_eq!(acquisition_effects.len(), 1);
    assert_eq!(acquisition_effects[0].record_id, 2);
}

#[test]
fn invalid_dependency_remains_visible_and_fails_source_closed() {
    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_learn_spells: vec![SpellAcquisitionDependencyLikeCpp {
                record_id: 1,
                spell_id_raw: 100,
                learn_spell_id_raw: -1,
                overrides_spell_id_raw: 0,
            }],
            ..Default::default()
        },
    );

    assert_eq!(
        catalog.effective_dependency_rows_like_cpp().count(),
        1,
        "invalid final rows remain inspectable"
    );
    assert!(matches!(
        catalog.dependency_rows_lookup_like_cpp(100),
        SpellAcquisitionDependenciesLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        catalog.acquisition_effects_like_cpp(100),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
    ));
}

#[test]
fn unrepresentable_final_effect_relation_marks_all_coverage_indeterminate() {
    let composed = compose_effective_table_like_cpp(
        [(
            1,
            effect(1, 100, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP)),
        )],
        [(
            1,
            effect(1, -1, 0, 0, i64::from(SPELL_EFFECT_SKILL_LIKE_CPP)),
        )],
        [],
        0xAABB,
        &Db2HotfixRemovalStoreLikeCpp::default(),
    );
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: composed.into_values().collect(),
            ..Default::default()
        },
    );

    assert!(matches!(
        catalog.difficulty_none_effects_like_cpp(100),
        SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        catalog.difficulty_none_effects_like_cpp(200),
        SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_)
    ));
}

#[test]
fn invalid_final_talent_rank_is_not_misclassified_as_not_talent() {
    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            talents: vec![SpellAcquisitionTalentLikeCpp {
                record_id: 1,
                spell_rank_raw: [-1, 0, 0, 0, 0, 0, 0, 0, 0],
            }],
            ..Default::default()
        },
    );

    assert!(matches!(
        catalog.talent_membership_like_cpp(100),
        SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        catalog.difficulty_none_effects_like_cpp(100),
        SpellAcquisitionEffectsLookupLikeCpp::Covered(_)
    ));
}

#[test]
fn valid_talent_membership_wins_over_unrelated_invalid_rows() {
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0),
            SpellAcquisitionCoverageSeedLikeCpp::covered(200, 0),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            talents: vec![
                SpellAcquisitionTalentLikeCpp {
                    record_id: 1,
                    spell_rank_raw: [100, 0, 0, 0, 0, 0, 0, 0, 0],
                },
                SpellAcquisitionTalentLikeCpp {
                    record_id: 2,
                    spell_rank_raw: [-1, 0, 0, 0, 0, 0, 0, 0, 0],
                },
            ],
            ..Default::default()
        },
    );

    assert_eq!(
        catalog.talent_membership_like_cpp(100),
        SpellAcquisitionTalentLookupLikeCpp::Talent
    );
    assert!(matches!(
        catalog.talent_membership_like_cpp(200),
        SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_)
    ));
}

#[test]
fn battle_pet_uses_all_difficulties_and_coalesces_same_species() {
    let catalog = catalog(
        [
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 2),
            SpellAcquisitionCoverageSeedLikeCpp::covered(100, 3),
        ],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![
                summon(1, 100, 2, 0, 900, 700),
                summon(2, 100, 3, 1, 900, 700),
            ],
            summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 700,
                slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
            }],
            battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 50,
                creature_id_raw: 900,
            }],
            ..Default::default()
        },
    );

    assert_eq!(
        catalog
            .summon_effects_all_difficulties_like_cpp(100)
            .count(),
        2
    );
    assert_eq!(
        catalog.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Species(50)
    );
}

#[test]
fn battle_pet_requires_exact_coverage_for_each_summon_difficulty() {
    let catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 2, 0, 900, 700)],
            summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 700,
                slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
            }],
            battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 50,
                creature_id_raw: 900,
            }],
            ..Default::default()
        },
    );

    assert!(matches!(
        catalog.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::MissingSpellDifficultyCoverage {
                    spell_id: 100,
                    difficulty_id: 2,
                    effect_record_id: 1,
                }
            ))
    ));
}

#[test]
fn battle_pet_requires_spell_coverage_and_treats_null_properties_as_nonqualifying() {
    let rows = EffectiveSpellAcquisitionRowsLikeCpp {
        spell_effects: vec![summon(1, 100, 0, 0, 900, 0)],
        ..Default::default()
    };
    let missing = catalog([], rows.clone());
    assert!(matches!(
        missing.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::MissingSpellCoverage {
                    spell_id: 100
                }
            ))
    ));

    let unavailable = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
            100,
            700,
            SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
        )],
        rows.clone(),
    );
    assert!(matches!(
        unavailable.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::SpellCoverage {
                    reason: SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
                    ..
                }
            ))
    ));

    let covered = catalog([SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)], rows);
    assert_eq!(
        covered.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::NotBattlePet
    );
}

#[test]
fn battle_pet_distinguishes_removed_properties_and_species() {
    let qualifying_properties = SpellAcquisitionSummonPropertiesLikeCpp {
        record_id: 700,
        slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
        flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
    };
    let removed_properties = catalog_with_removed(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            ..Default::default()
        },
        vec![SpellAcquisitionRemovedRowLikeCpp::SummonProperties(
            qualifying_properties.clone(),
        )],
    );
    assert!(matches!(
        removed_properties.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                    properties_id: 700,
                    ..
                }
            ))
    ));

    let removed_species = catalog_with_removed(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            summon_properties: vec![qualifying_properties],
            ..Default::default()
        },
        vec![SpellAcquisitionRemovedRowLikeCpp::BattlePetSpecies(
            SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 50,
                creature_id_raw: 900,
            },
        )],
    );
    assert!(matches!(
        removed_species.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::RemovedSpeciesForCreature {
                    creature_id: 900,
                    ..
                }
            ))
    ));

    let unknown_removed_properties = catalog_with_removed(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            ..Default::default()
        },
        vec![SpellAcquisitionRemovedRowLikeCpp::Unknown {
            table: SpellAcquisitionTableLikeCpp::SummonProperties,
            record_id: 700,
        }],
    );
    assert!(matches!(
        unknown_removed_properties.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::RemovedSummonProperties {
                    properties_id: 700,
                    ..
                }
            ))
    ));
}

#[test]
fn battle_pet_conflicts_and_missing_references_are_indeterminate() {
    let conflicting = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 700,
                slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
            }],
            battle_pet_species: vec![
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                },
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 51,
                    creature_id_raw: 900,
                },
            ],
            ..Default::default()
        },
    );
    assert!(matches!(
        conflicting.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::ConflictingSpeciesForCreature { .. }
            ))
    ));

    let missing_properties = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            ..Default::default()
        },
    );
    assert!(matches!(
        missing_properties.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(_)
    ));

    let missing_species = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 700,
                slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
            }],
            ..Default::default()
        },
    );
    assert!(matches!(
        missing_species.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(_)
    ));

    // Mutate a raw final row to exercise corrupt difficulty metadata
    // without narrowing it first.
    let mut invalid_rows = EffectiveSpellAcquisitionRowsLikeCpp::default();
    let mut invalid_summon = summon(2, 101, 0, 0, 900, 700);
    invalid_summon.difficulty_id_raw = -1;
    invalid_rows.spell_effects.push(invalid_summon);
    invalid_rows.summon_properties = vec![SpellAcquisitionSummonPropertiesLikeCpp {
        record_id: 700,
        slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
        flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
    }];
    invalid_rows.battle_pet_species = vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
        species_id: 50,
        creature_id_raw: 900,
    }];
    let invalid_catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(101, 0)],
        invalid_rows,
    );
    assert!(matches!(
        invalid_catalog.battle_pet_classification_like_cpp(101),
        BattlePetClassificationLikeCpp::Indeterminate(_)
    ));
}

#[test]
fn species_data_without_qualifying_summon_is_not_authority() {
    let no_summon_catalog = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            // Deliberately no SUMMON effect. BattlePetSpecies.SummonSpellID
            // is not retained or consulted by this catalog.
            battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 50,
                creature_id_raw: 900,
            }],
            ..Default::default()
        },
    );
    assert_eq!(
        no_summon_catalog.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::NotBattlePet
    );

    let invalid_species = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            battle_pet_species: vec![SpellAcquisitionBattlePetSpeciesLikeCpp {
                species_id: 51,
                creature_id_raw: -1,
            }],
            ..Default::default()
        },
    );
    assert_eq!(
        invalid_species.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::NotBattlePet,
        "unreferenced species corruption cannot turn a covered zero-SUMMON spell indeterminate"
    );

    let unreadable_species = catalog(
        [SpellAcquisitionCoverageSeedLikeCpp::covered(100, 0)],
        EffectiveSpellAcquisitionRowsLikeCpp {
            spell_effects: vec![summon(1, 100, 0, 0, 900, 700)],
            summon_properties: vec![SpellAcquisitionSummonPropertiesLikeCpp {
                record_id: 700,
                slot_raw: SUMMON_SLOT_MINIPET_LIKE_CPP,
                flags_1_raw: i64::from(SUMMON_FROM_BATTLE_PET_JOURNAL_LIKE_CPP),
            }],
            battle_pet_species: vec![
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 50,
                    creature_id_raw: 900,
                },
                SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: 51,
                    creature_id_raw: UNREADABLE_SQL_RAW_LIKE_CPP,
                },
            ],
            ..Default::default()
        },
    );
    assert!(matches!(
        unreadable_species.battle_pet_classification_like_cpp(100),
        BattlePetClassificationLikeCpp::Indeterminate(ref reasons)
            if reasons.iter().any(|reason| matches!(
                reason,
                BattlePetIndeterminateReasonLikeCpp::EffectiveTableIncomplete {
                    table: SpellAcquisitionTableLikeCpp::BattlePetSpecies,
                    ..
                }
            ))
    ));
}

#[test]
fn runtime_hash_bundle_is_preserved_without_constants() {
    let hashes = SpellAcquisitionTableHashesLikeCpp {
        spell_effect: 1,
        spell_learn_spell: 2,
        spell_misc: 3,
        spell_levels: 4,
        talent: 5,
        summon_properties: 6,
        battle_pet_species: 7,
    };
    let catalog = SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
        [],
        EffectiveSpellAcquisitionRowsLikeCpp::default(),
        hashes,
        Vec::new(),
    );
    assert_eq!(catalog.table_hashes_like_cpp(), hashes);
}
