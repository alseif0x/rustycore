//! Misc scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn primary_profession_first_rank_preserves_safe_indeterminate_short_circuits() {
    let skill_lines = crate::skill_talent::SkillLineStore::from_entries([
        test_skill_line_like_cpp(100, 11, 0),
        test_skill_line_like_cpp(200, 9, 0),
    ]);
    let mut primary_spell = SpellStore::empty_spell_info_like_cpp(1_000);
    primary_spell.effects = vec![test_skill_effect_like_cpp(0, 100)];
    let mut non_primary_spell = SpellStore::empty_spell_info_like_cpp(1_001);
    non_primary_spell.effects = vec![test_skill_effect_like_cpp(0, 200)];

    let local_indeterminate =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 90,
                spell_raw: 1_000,
                supercedes_spell_raw: i128::from(i32::MAX) + 1,
            }],
            |spell_id| spell_id == 1_000,
        )
        .store;
    assert_eq!(
        primary_spell
            .is_primary_profession_first_rank_like_cpp(&skill_lines, &local_indeterminate,),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                spell_id: 1_000,
            }
        )
    );

    let global_indeterminate =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [SkillLineAbilityRankRowLikeCpp::Indeterminate {
                record_id: 91,
                spell_raw: i128::from(i32::MAX) + 1,
                supercedes_spell_raw: i128::from(i32::MAX) + 2,
            }],
            |_| false,
        )
        .store;
    assert_eq!(
        primary_spell
            .is_primary_profession_first_rank_like_cpp(&skill_lines, &global_indeterminate,),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::RankChainIndeterminate {
                spell_id: 1_000,
            }
        )
    );
    assert_eq!(
        non_primary_spell
            .is_primary_profession_first_rank_like_cpp(&skill_lines, &global_indeterminate,),
        Ok(false),
        "C++'s false primary-profession operand decides the conjunction without rank"
    );
}
#[test]
fn hit_metadata_composes_each_db2_contributor_and_effect_slot_like_cpp() {
    let spell_id = 90_001;
    let categories = |id, difficulty_id, category, charge_category, defense_type, mechanic| {
        crate::spell_db2::SpellCategoriesEntry {
            id,
            difficulty_id,
            category,
            defense_type,
            dispel_type: 0,
            mechanic,
            prevention_type: 0,
            start_recovery_category: 0,
            charge_category,
            spell_id,
        }
    };
    let category_store = crate::spell_db2::SpellCategoriesStore::from_entries([
        categories(10, 0, 7, 8, 1, 2),
        categories(19, 2, 50, 60, 5, 6),
        categories(20, 2, 30, 40, 3, 4),
    ]);

    let mut base_misc = test_spell_misc_entry_like_cpp(10, spell_id, 0, 0);
    base_misc.school_mask = 1;
    let mut lower_duplicate_misc = test_spell_misc_entry_like_cpp(9, spell_id, 0, 0);
    lower_duplicate_misc.school_mask = 2;
    let misc_store =
        crate::spell_db2::SpellMiscStore::from_entries([base_misc, lower_duplicate_misc]);

    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([
        test_spell_effect_db2_entry_like_cpp(10, spell_id, 0, 0, 2, 7),
        // The row itself exists even though Effect=NONE, so it suppresses
        // the base slot's mechanic during per-effect fallback.
        test_spell_effect_db2_entry_like_cpp(20, spell_id, 2, 0, 0, 0),
        test_spell_effect_db2_entry_like_cpp(11, spell_id, 0, 1, 2, 8),
        test_spell_effect_db2_entry_like_cpp(21, spell_id, 1, 1, 2, 9),
        test_spell_effect_db2_entry_like_cpp(18, spell_id, 2, 2, 2, 5),
        test_spell_effect_db2_entry_like_cpp(22, spell_id, 2, 2, 2, 11),
        test_spell_effect_db2_entry_like_cpp(30, spell_id, 2, MAX_SPELL_EFFECTS_LIKE_CPP, 2, 99),
    ]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &category_store,
        &misc_store,
        &effect_store,
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    );
    let difficulties = crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);

    assert_eq!(
        store.hit_metadata_for_difficulty_like_cpp(spell_id as i32, 2, Some(&difficulties)),
        Some(SpellHitMetadataLikeCpp {
            category_id: 30,
            charge_category_id: 40,
            defense_type: 3,
            spell_mechanic: 4,
            school_mask: 1,
            effect_mechanics: BTreeMap::from([(0, 0), (1, 9), (2, 11)]),
        })
    );
    assert_eq!(
        store.hit_metadata_for_difficulty_like_cpp(spell_id as i32, 3, None),
        Some(SpellHitMetadataLikeCpp {
            category_id: 7,
            charge_category_id: 8,
            defense_type: 1,
            spell_mechanic: 2,
            school_mask: 1,
            effect_mechanics: BTreeMap::from([(0, 7), (1, 8)]),
        })
    );
    assert!(
        store
            .hit_metadata_for_difficulty_like_cpp(99_999, 2, Some(&difficulties))
            .is_none()
    );
}
#[test]
fn synthetic_hit_metadata_insertion_supports_focused_consumers() {
    let mut store = SpellStore::new();
    let metadata = SpellHitMetadataLikeCpp {
        category_id: 13,
        charge_category_id: 17,
        defense_type: 2,
        spell_mechanic: 7,
        school_mask: 4,
        effect_mechanics: BTreeMap::from([(0, 0), (2, 12)]),
    };
    store.insert_spell_hit_metadata_for_difficulty_like_cpp(90_002, 2, metadata.clone());

    assert_eq!(
        store.hit_metadata_for_difficulty_like_cpp(90_002, 2, None),
        Some(metadata)
    );
}
#[test]
fn db2_cooldowns_set_recovery_max_like_cpp() {
    use crate::spell_db2::{SpellCooldownsEntry, SpellCooldownsStore};
    let mut store = SpellStore::new();
    store
        .spells
        .insert(300, SpellStore::empty_spell_info_like_cpp(300));

    let cooldowns = SpellCooldownsStore::from_entries([SpellCooldownsEntry {
        id: 1,
        difficulty_id: 0,
        recovery_time: 3000,
        category_recovery_time: 5000,
        start_recovery_time: 1500,
        spell_id: 300,
    }]);
    store.apply_db2_cooldowns_like_cpp(&cooldowns);

    // C++ GetRecoveryTime = max(RecoveryTime 3000, CategoryRecoveryTime 5000) = 5000.
    assert_eq!(store.spells.get(&300).unwrap().recovery_time_ms, 5000);
    // GCD (cooldown_ms) is a separate mechanic — untouched by this slice.
    assert_eq!(store.spells.get(&300).unwrap().cooldown_ms, 0);
}
#[test]
fn invalid_rank_seed_unites_every_touched_valid_component() {
    let mut outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 11,
                    supercedes_spell_id: 10,
                },
            ],
            |_| true,
        );

    outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
        93,
        i128::from(i32::MAX) + 1,
        i128::from(i32::MAX) + 2,
        &[2, 11],
    );

    assert!(outcome.store.chains_by_spell_id.is_empty());
    for spell_id in [1, 2, 10, 11] {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    record_id: 93,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: i128::from(i32::MAX) + 2,
                    affected_spell_ids: vec![2, 11],
                }]
        ));
    }
}
#[test]
fn invalid_rank_seed_preserves_existing_component_diagnostics() {
    let mut outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 1,
                    supercedes_spell_id: 2,
                },
            ],
            |_| true,
        );

    outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
        94,
        1,
        i128::from(i32::MAX) + 1,
        &[1],
    );

    for spell_id in [1, 2] {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [
                    SpellChainLoadDiagnosticLikeCpp::Cycle {
                        spell_ids: vec![1, 2],
                    },
                    SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                        record_id: 94,
                        spell_raw: 1,
                        supercedes_spell_raw: i128::from(i32::MAX) + 1,
                        affected_spell_ids: vec![1],
                    },
                ]
        ));
    }
}
#[test]
fn global_rank_seed_preserves_existing_component_diagnostics() {
    let mut outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 1,
                    supercedes_spell_id: 2,
                },
            ],
            |_| true,
        );

    outcome.mark_invalid_skill_line_ability_rank_row_like_cpp(
        95,
        i128::from(i32::MAX) + 1,
        i128::from(i32::MAX) + 2,
        &[],
    );

    for spell_id in [1, 2, 999] {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [
                    SpellChainLoadDiagnosticLikeCpp::Cycle {
                        spell_ids: vec![1, 2],
                    },
                    SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                        record_id: 95,
                        spell_raw: i128::from(i32::MAX) + 1,
                        supercedes_spell_raw: i128::from(i32::MAX) + 2,
                        affected_spell_ids: Vec::new(),
                    },
                ]
        ));
    }
    assert!(outcome.store.indeterminate_by_spell_id_like_cpp.is_empty());
}
