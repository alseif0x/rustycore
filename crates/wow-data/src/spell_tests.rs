//! Behaviour tests for [`super`].
//!
//! Extracted verbatim from `spell.rs`, which was 13,736 lines of which
//! 6,177 — 45% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant.

#![cfg(test)]

use super::*;

fn test_skill_line_like_cpp(
    id: u32,
    category_id: i8,
    parent_skill_line_id: u32,
) -> crate::skill_talent::SkillLineEntry {
    crate::skill_talent::SkillLineEntry {
        id,
        display_name: String::new(),
        alternate_verb: String::new(),
        description: String::new(),
        horde_display_name: String::new(),
        override_source_info_display_name: String::new(),
        category_id,
        spell_icon_file_id: 0,
        can_link: 0,
        parent_skill_line_id,
        parent_tier_index: 0,
        flags: 0,
        spell_book_spell_id: 0,
    }
}

fn test_skill_effect_like_cpp(effect_index: u32, skill_id: i32) -> SpellEffectInfo {
    SpellEffectInfo {
        effect_index,
        effect: spell_effect_types::SPELL_EFFECT_SKILL,
        effect_misc_value_1: skill_id,
        ..Default::default()
    }
}

#[test]
fn primary_profession_spell_classifier_matches_cpp_root_and_rank_rules() {
    let skill_lines = crate::skill_talent::SkillLineStore::from_entries([
        test_skill_line_like_cpp(100, 11, 0),
        test_skill_line_like_cpp(101, 11, 100),
        test_skill_line_like_cpp(200, 9, 0),
        test_skill_line_like_cpp(300, 11, 0),
    ]);
    let mut spell = SpellStore::empty_spell_info_like_cpp(1_000);
    spell.effects = vec![
        test_skill_effect_like_cpp(2, 100),
        test_skill_effect_like_cpp(1, 300),
        test_skill_effect_like_cpp(2, 101),
        test_skill_effect_like_cpp(3, 200),
        test_skill_effect_like_cpp(4, 300),
    ];

    assert_eq!(
        spell
            .primary_profession_skill_effect_ids_like_cpp(&skill_lines)
            .unwrap(),
        vec![300, 100],
        "primary lines follow C++ effect-index order and deduplicate at first appearance"
    );
    assert!(
        spell
            .is_primary_profession_first_rank_like_cpp(
                &skill_lines,
                &SpellChainStoreLikeCpp::default(),
            )
            .unwrap(),
        "C++ SpellInfo::GetRank returns one without a ChainEntry"
    );

    let rank_two = SpellChainStoreLikeCpp {
        chains_by_spell_id: BTreeMap::from([(
            1_000,
            SpellChainNodeLikeCpp {
                prev_spell_id: Some(999),
                next_spell_id: None,
                first_spell_id: 999,
                last_spell_id: 1_000,
                rank: 2,
            },
        )]),
        ..SpellChainStoreLikeCpp::default()
    };
    assert!(
        !spell
            .is_primary_profession_first_rank_like_cpp(&skill_lines, &rank_two)
            .unwrap()
    );

    let mut unhydrated_rank_two = SpellStore::empty_spell_info_like_cpp(1_000);
    unhydrated_rank_two.effects = vec![test_skill_effect_like_cpp(0, 999)];
    let partial_skill_lines =
        crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [test_skill_line_like_cpp(100, 11, 0)],
            [100, 999],
        );
    assert_eq!(
        unhydrated_rank_two
            .is_primary_profession_first_rank_like_cpp(&partial_skill_lines, &rank_two,),
        Ok(false),
        "rank two is decidably false without requiring unrelated partial payload"
    );

    let mut partly_hydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_001);
    partly_hydrated_rank_one.effects = vec![
        test_skill_effect_like_cpp(0, 999),
        test_skill_effect_like_cpp(1, 100),
    ];
    assert_eq!(
        partly_hydrated_rank_one.is_primary_profession_first_rank_like_cpp(
            &partial_skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Ok(true),
        "one hydrated primary effect proves C++'s boolean result"
    );

    let mut only_unhydrated_rank_one = SpellStore::empty_spell_info_like_cpp(1_002);
    only_unhydrated_rank_one.effects = vec![test_skill_effect_like_cpp(0, 999)];
    assert_eq!(
        only_unhydrated_rank_one.is_primary_profession_first_rank_like_cpp(
            &partial_skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                spell_id: 1_002,
                skill_id: 999,
            }
        )
    );
}

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
fn primary_profession_spell_classifier_distinguishes_absent_unhydrated_and_invalid_skill() {
    let skill_lines =
        crate::skill_talent::SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [test_skill_line_like_cpp(100, 11, 0)],
            [100, 999],
        );
    let mut unhydrated = SpellStore::empty_spell_info_like_cpp(1_000);
    unhydrated.effects = vec![test_skill_effect_like_cpp(0, 999)];
    assert_eq!(
        unhydrated.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::MissingSkillLinePayload {
                spell_id: 1_000,
                skill_id: 999,
            }
        )
    );

    let mut absent = SpellStore::empty_spell_info_like_cpp(1_001);
    absent.effects = vec![test_skill_effect_like_cpp(0, 998)];
    assert_eq!(
        absent.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Ok(Vec::new()),
        "a failed C++ LookupEntry is non-primary"
    );

    let mut invalid = SpellStore::empty_spell_info_like_cpp(1_002);
    invalid.effects = vec![test_skill_effect_like_cpp(2, -1)];
    assert_eq!(
        invalid.primary_profession_skill_effect_ids_like_cpp(&skill_lines),
        Err(
            PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSkillId {
                spell_id: 1_002,
                effect_index: 2,
                skill_id: -1,
            }
        )
    );

    let mut invalid_spell = SpellStore::empty_spell_info_like_cpp(1_003);
    invalid_spell.spell_id = -1;
    invalid_spell.effects = vec![test_skill_effect_like_cpp(0, 100)];
    assert_eq!(
        invalid_spell.is_primary_profession_first_rank_like_cpp(
            &skill_lines,
            &SpellChainStoreLikeCpp::default(),
        ),
        Err(PrimaryProfessionSpellClassificationErrorLikeCpp::InvalidSpellId { spell_id: -1 })
    );
}

#[test]
fn effective_effect_store_is_the_only_difficulty_zero_effect_authority_like_cpp() {
    let spell_id = 100u32;
    let build = |effects: Vec<crate::spell_db2::SpellEffectDb2Entry>| {
        SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &crate::spell_db2::SpellMiscStore::from_entries([test_spell_misc_entry_like_cpp(
                1, spell_id, 0, 0,
            )]),
            &crate::spell_db2::SpellEffectDb2Store::from_entries(effects),
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        )
    };
    let effect = |id, effect_index, effect_aura: i32| {
        let mut entry = test_spell_effect_db2_entry_like_cpp(
            id,
            spell_id,
            0,
            effect_index,
            spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            0,
        );
        entry.effect_aura = effect_aura as i16;
        entry
    };

    let before_tombstone = build(vec![
        effect(1, 0, aura_types::SPELL_AURA_MOD_THREAT),
        effect(2, 1, aura_types::SPELL_AURA_MOD_TAUNT),
    ]);
    assert_eq!(
        before_tombstone
            .effects_for_difficulty_like_cpp(spell_id as i32, 0, None)
            .expect("both effective rows")
            .len(),
        2
    );

    // A final `hotfix_data` tombstone leaves the effective store without
    // record 2. No other authority may reintroduce that effect index.
    let after_tombstone = build(vec![effect(1, 0, aura_types::SPELL_AURA_MOD_THREAT)]);
    let effects = after_tombstone
        .effects_for_difficulty_like_cpp(spell_id as i32, 0, None)
        .expect("surviving effective row");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect_index, 0);
    assert_eq!(effects[0].effect_aura, aura_types::SPELL_AURA_MOD_THREAT);

    let spell = after_tombstone
        .get(spell_id as i32)
        .expect("spell from effective SpellMisc");
    assert_eq!(spell.effects.len(), 1);
    assert_eq!(spell.aura_type, Some(aura_types::SPELL_AURA_MOD_THREAT));
}

#[test]
fn casting_requirements_hydrate_requires_spell_focus_from_effective_store_like_cpp() {
    let requirements =
        |id, spell_id: i32, requires_spell_focus| crate::spell_db2::SpellCastingRequirementsEntry {
            id,
            spell_id,
            facing_caster_flags: 0,
            min_faction_id: 0,
            min_reputation: 0,
            required_areas_id: 0,
            required_aura_vision: 0,
            requires_spell_focus,
        };
    let build_spell_store = || {
        SpellStore::from_spell_db2_stores_like_cpp(
            &crate::spell_db2::SpellCategoriesStore::from_entries([]),
            &crate::spell_db2::SpellMiscStore::from_entries([
                test_spell_misc_entry_like_cpp(1, 100, 0, 0),
                test_spell_misc_entry_like_cpp(2, 200, 0, 0),
            ]),
            &crate::spell_db2::SpellEffectDb2Store::from_entries([]),
            &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
        )
    };

    let mut store = build_spell_store();
    store.apply_db2_casting_requirements_like_cpp(
        &crate::spell_db2::SpellCastingRequirementsStore::from_entries([
            requirements(1, 100, 181),
            // A malformed duplicate resolves to the highest record ID, the
            // slot C++'s record-ID ordered DB2 iteration assigns last.
            requirements(2, 100, 23),
        ]),
    );
    assert_eq!(
        store.get(100).map(|spell| spell.requires_spell_focus),
        Some(23)
    );
    assert!(
        store
            .get(100)
            .expect("spell 100")
            .requires_spell_focus_like_cpp()
    );
    assert_eq!(
        store.get(200).map(|spell| spell.requires_spell_focus),
        Some(0),
        "a spell without a requirements row keeps the C++ default"
    );

    // A final tombstone removes the only row, so the spell must fall back to
    // zero instead of keeping a resurrected focus object.
    let mut tombstoned = build_spell_store();
    tombstoned.apply_db2_casting_requirements_like_cpp(
        &crate::spell_db2::SpellCastingRequirementsStore::from_entries([]),
    );
    assert_eq!(
        tombstoned.get(100).map(|spell| spell.requires_spell_focus),
        Some(0)
    );
}

#[test]
fn misc_attributes_resolve_exact_difficulty_then_base_like_cpp() {
    let mut store = SpellStore::new();
    let mut base = [0; 15];
    base[1] = attributes::SPELL_ATTR1_NO_THREAT;
    store.insert_spell_misc_attributes_like_cpp(100, base);
    let mut heroic = [0; 15];
    heroic[4] = attributes::SPELL_ATTR4_NO_HARMFUL_THREAT;
    store.insert_spell_misc_attributes_for_difficulty_like_cpp(100, 2, heroic);

    assert!(store.has_attribute_for_difficulty_like_cpp(
        100,
        2,
        None,
        4,
        attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
    ));
    assert!(!store.has_attribute_for_difficulty_like_cpp(
        100,
        2,
        None,
        1,
        attributes::SPELL_ATTR1_NO_THREAT,
    ));
    assert!(store.has_attribute_for_difficulty_like_cpp(
        100,
        3,
        None,
        1,
        attributes::SPELL_ATTR1_NO_THREAT,
    ));
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
fn real_spell_15691_hit_metadata_matches_db2_when_data_exists() {
    let data_dir = std::env::var("RUSTYCORE_REAL_DATA_DIR")
        .unwrap_or_else(|_| "/home/server/woltk-server-core/Data".to_string());
    let locale = std::env::var("RUSTYCORE_REAL_LOCALE").unwrap_or_else(|_| "enUS".to_string());
    let dbc_dir = std::path::Path::new(&data_dir).join("dbc").join(&locale);
    if ["SpellCategories.db2", "SpellMisc.db2", "SpellEffect.db2"]
        .into_iter()
        .any(|file| !dbc_dir.join(file).is_file())
    {
        eprintln!(
            "Skipping real spell hit-metadata fixture: DB2 files not found at {}",
            dbc_dir.display()
        );
        return;
    }

    let category_store = crate::spell_db2::SpellCategoriesStore::load(&data_dir, &locale)
        .expect("load real SpellCategories.db2");
    let misc_store = crate::spell_db2::SpellMiscStore::load(&data_dir, &locale)
        .expect("load real SpellMisc.db2");
    let effect_store = crate::spell_db2::SpellEffectDb2Store::load(&data_dir, &locale)
        .expect("load real SpellEffect.db2");
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &category_store,
        &misc_store,
        &effect_store,
        &crate::spell_db2::SpellShapeshiftStore::from_entries([]),
    );

    assert_eq!(
        store.hit_metadata_for_difficulty_like_cpp(15_691, 0, None),
        Some(SpellHitMetadataLikeCpp {
            category_id: 0,
            charge_category_id: 0,
            defense_type: 2,
            spell_mechanic: 0,
            school_mask: 1,
            effect_mechanics: BTreeMap::from([(0, 0)]),
        })
    );
}

use crate::{Condition, ConditionEntriesByTypeStore};
use wow_constants::{ConditionSourceType, ConditionType};

#[test]
fn test_spell_store_creation() {
    let store = SpellStore::new();
    assert!(store.is_empty(), "new store should be empty");
}

#[test]
fn exact_spell_info_key_does_not_fabricate_hydrated_payload() {
    let mut store = SpellStore::new();
    store.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(200, 2), (100, 0), (200, 1)],
            &HashSet::from([100, 200]),
        );

    assert!(store.contains_spell_info_exact_like_cpp(100, 0));
    assert!(store.get(100).is_none());
    assert_eq!(
        store.spell_info_keys_in_order_like_cpp(),
        [(100, 0), (200, 1), (200, 2)]
    );
}

#[test]
fn difficulty_none_existence_composes_exact_regular_and_serverside_keys_like_cpp() {
    let mut regular = SpellStore::new();
    regular.spell_info_keys_like_cpp =
        crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::from_candidate_keys_like_cpp(
            [(100, 0), (101, 2), (300, 2)],
            &HashSet::from([100, 101, 300]),
        );
    let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [
            serverside_spell_row(200, 0),
            serverside_spell_row(201, 2),
            serverside_spell_row(400, 3),
        ],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    assert!(serverside.errors.is_empty());
    let no_fallback = crate::DifficultyStore::from_entries([]);

    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 100),
        "an exact regular difficulty-zero key is visible even without hydrated payload"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 101),
        "a regular key that exists only at another difficulty is not a trainer spell"
    );
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 200),
        "an exact server-side difficulty-zero key shares C++ GetSpellInfo authority"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(&serverside.store, &no_fallback, 201),
        "a server-side key that exists only at another difficulty is not a trainer spell"
    );

    let trainer = crate::trainer::TrainerStoreLikeCpp::from_rows_like_cpp(
        [crate::trainer::TrainerRowLikeCpp {
            id: 10,
            trainer_type: crate::trainer::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
            greeting: String::new(),
        }],
        [100, 101, 200, 201].map(|spell_id| crate::trainer::TrainerSpellRowLikeCpp {
            trainer_id: 10,
            spell: crate::trainer::TrainerSpellLikeCpp {
                spell_id,
                money_cost: 0,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_ability: [0; 3],
                req_level: 0,
            },
        }),
        [],
        [],
        |spell_id| {
            regular.contains_spell_info_difficulty_none_like_cpp(
                &serverside.store,
                &no_fallback,
                spell_id,
            )
        },
        |_| true,
        |_| true,
        |_, _| true,
    );
    let loaded = trainer.store.get_trainer_like_cpp(10).unwrap();
    assert!(loaded.get_spell_like_cpp(100).is_some());
    assert!(loaded.get_spell_like_cpp(200).is_some());
    assert!(loaded.get_spell_like_cpp(101).is_none());
    assert!(loaded.get_spell_like_cpp(201).is_none());
    assert_eq!(
        trainer.report.skipped_spells_missing_spell,
        vec![(10, 101), (10, 201)]
    );

    let difficulty_fallbacks = crate::DifficultyStore::from_entries([
        crate::DifficultyEntry {
            id: 0,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 3,
            toggle_difficulty_id: 0,
        },
        crate::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ]);
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            300
        ),
        "a custom Difficulty(0) fallback reaches a regular spell like C++"
    );
    assert!(
        regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            400
        ),
        "the fallback chain reaches a server-side spell like C++"
    );
    assert!(
        !regular.contains_spell_info_difficulty_none_like_cpp(
            &serverside.store,
            &difficulty_fallbacks,
            999
        ),
        "invalid custom fallback cycles terminate instead of hanging startup"
    );
}

#[test]
fn spell_store_db2_loader_keeps_mount_aura_spells_like_cpp() {
    let spell_id = 32_243;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[0] = attributes::SPELL_ATTR0_NO_AURA_CANCEL as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([
        crate::spell_db2::SpellEffectDb2Entry {
            id: 1,
            difficulty_id: 0,
            effect_index: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_amplitude: 0.0,
            effect_attributes: 0,
            effect_aura: aura_types::SPELL_AURA_MOUNTED as i16,
            effect_aura_period: 0,
            effect_base_points: 77,
            effect_bonus_coefficient: 0.0,
            effect_chain_amplitude: 0.0,
            effect_chain_targets: 0,
            effect_die_sides: 0,
            effect_item_type: 0,
            effect_mechanic: 0,
            effect_points_per_resource: 0.0,
            effect_pos_facing: 0.0,
            effect_real_points_per_level: 0.0,
            effect_trigger_spell: 0,
            bonus_coefficient_from_ap: 0.0,
            pvp_multiplier: 0.0,
            coefficient: 0.0,
            variance: 0.0,
            resource_coefficient: 0.0,
            group_size_base_points_coefficient: 0.0,
            effect_misc_value: [23966, 0],
            effect_radius_index: [0, 0],
            effect_spell_class_mask: [0, 0, 0, 0],
            implicit_target: [0, 0],
            spell_id,
        },
    ]);

    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );
    let spell = store.get(spell_id as i32).expect("mount spell loaded");

    assert_eq!(
        spell.effect_type,
        spell_effect_types::SPELL_EFFECT_APPLY_AURA
    );
    assert_eq!(spell.aura_type, Some(aura_types::SPELL_AURA_MOUNTED));
    assert!(
        spell
            .effects
            .iter()
            .any(SpellEffectInfo::is_mounted_aura_like_cpp)
    );
    assert!(store.has_attribute0_like_cpp(spell_id as i32, attributes::SPELL_ATTR0_NO_AURA_CANCEL));
}

#[test]
fn spell_store_db2_loader_keeps_channeled_spell_attr1_like_cpp() {
    let spell_id = 51_588;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[1] = attributes::SPELL_ATTR1_IS_CHANNELLED as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);

    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert!(store.has_attribute1_like_cpp(spell_id as i32, attributes::SPELL_ATTR1_IS_CHANNELLED));
    assert!(store.is_channeled_like_cpp(spell_id as i32));
    assert!(!store.is_channeled_like_cpp(99_999));
}

#[test]
fn spell_store_resolves_interrupt_masks_by_difficulty_and_fallback_like_cpp() {
    let spell_id = 70_101;
    let exact_without_difficulty_entry_spell_id = 70_102;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 1,
            difficulty_id: 0,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x0004_0000, 0],
            channel_interrupt_flags: [0, 0],
            spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 2,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0, 0],
            channel_interrupt_flags: [0x0004_0000, 0],
            spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 3,
            difficulty_id: 9,
            interrupt_flags: 0,
            aura_interrupt_flags: [0, 0x40],
            channel_interrupt_flags: [0, 0x80],
            spell_id: exact_without_difficulty_entry_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    let difficulties = crate::difficulty::DifficultyStore::from_entries([
        crate::difficulty::DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
        crate::difficulty::DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        crate::difficulty::DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
    ]);

    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 2, Some(&difficulties),),
        Some(([0, 0], [0x0004_0000, 0])),
        "the exact row overrides its base row without merging words"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(spell_id as i32, 3, Some(&difficulties),),
        Some(([0x0004_0000, 0], [0, 0])),
        "difficulty 3 walks 3 -> 1 -> 0"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(
            exact_without_difficulty_entry_spell_id as i32,
            9,
            Some(&difficulties),
        ),
        Some(([0, 0x40], [0, 0x80])),
        "an exact SpellInterrupts row wins before Difficulty lookup"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(99_999, 3, Some(&difficulties)),
        None,
        "a fully missing fallback chain stays unknown"
    );
    assert!(store.has_aura_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
    assert!(!store.has_channel_interrupt_flag_like_cpp(spell_id as i32, 0x0004_0000, 0));
}

#[test]
fn spell_store_effective_interrupt_masks_follow_cpp_load_order() {
    let regular_spell_id = 24_314;
    let serverside_spell_id = 70_001;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 1,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x100, 0x200],
            channel_interrupt_flags: [0x300, 0x400],
            spell_id: regular_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x100, 0x200], [0x300, 0x400]))
    );

    assert!(store.store_signed_interrupt_row_by_id_like_cpp(
        1,
        regular_spell_id,
        2,
        [0x10, -1],
        [i32::MIN, 0x40],
    ));
    store.rebuild_interrupt_flags_from_rows_like_cpp();
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x10, u32::MAX], [0x8000_0000, 0x40])),
        "the later row for the same DB2 record ID replaces its masks and preserves signed bit patterns"
    );

    let serverside = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(serverside_spell_id, 2)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );
    assert!(serverside.errors.is_empty());
    store.apply_serverside_spell_interrupts_like_cpp(&serverside.store);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(regular_spell_id as i32, 2, None),
        Some(([0x3c, u32::MAX], [0x8000_0000, 0x40])),
        "the interrupt correction runs after the file/hotfix composition"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(serverside_spell_id as i32, 2, None),
        Some(([43, 44], [45, 46])),
        "server-side masks enter the same effective table before corrections"
    );
}

#[test]
fn spell_store_hotfix_overlay_rekeys_by_db2_record_id_like_cpp() {
    let original_spell_id = 70_201;
    let rekeyed_spell_id = 70_202;
    let interrupts = crate::spell_db2::SpellInterruptsStore::from_entries([
        crate::spell_db2::SpellInterruptsEntry {
            id: 10,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x10, 0],
            channel_interrupt_flags: [0x20, 0],
            spell_id: original_spell_id,
        },
        crate::spell_db2::SpellInterruptsEntry {
            id: 20,
            difficulty_id: 2,
            interrupt_flags: 0,
            aura_interrupt_flags: [0x30, 0],
            channel_interrupt_flags: [0x40, 0],
            spell_id: original_spell_id,
        },
    ]);
    let mut store = SpellStore::new();
    store.apply_db2_interrupts_like_cpp(&interrupts);

    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
        Some(([0x30, 0], [0x40, 0])),
        "the highest DB2 record ID wins when two rows have the same relational key"
    );

    assert!(store.store_signed_interrupt_row_by_id_like_cpp(
        20,
        rekeyed_spell_id,
        3,
        [0x50, 0],
        [0x60, 0],
    ));
    store.rebuild_interrupt_flags_from_rows_like_cpp();
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(original_spell_id as i32, 2, None),
        Some(([0x10, 0], [0x20, 0])),
        "replacing record ID 20 uncovers record ID 10 at its former key"
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(rekeyed_spell_id as i32, 3, None),
        Some(([0x50, 0], [0x60, 0])),
        "the replacement row is indexed by its new spell/difficulty relationship"
    );
}

#[test]
fn spell_store_interrupt_corrections_cover_every_stored_difficulty() {
    let mut store = SpellStore::new();
    for difficulty_id in [0, 2] {
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            29_726,
            difficulty_id,
            [0, 0],
            [0xffff_ffff, 0x20],
        );
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            24_314,
            difficulty_id,
            [0x10, 0x40],
            [0x80, 0x100],
        );
        store.insert_spell_interrupt_flags_for_difficulty_like_cpp(
            99_252,
            difficulty_id,
            [0x200, 0x400],
            [0x800, 0x1000],
        );
    }
    store.insert_spell_interrupt_flags_like_cpp(63_414, [0x10, 0x20], [0xffff_ffff, 0xffff_ffff]);
    store
        .spells
        .insert(61_719, SpellStore::empty_spell_info_like_cpp(61_719));

    store.apply_interrupt_flag_corrections_like_cpp();

    for difficulty_id in [0, 2] {
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(29_726, difficulty_id, None),
            Some(([0, 0], [0xffff_fffb, 0x20]))
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(24_314, difficulty_id, None),
            Some(([0x3c, 0x40], [0x80, 0x100]))
        );
        assert_eq!(
            store.interrupt_flags_for_difficulty_like_cpp(99_252, difficulty_id, None),
            Some(([0x8_0200, 0x400], [0x800, 0x1000]))
        );
    }
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(63_414, 0, None),
        Some(([0x10, 0x20], [0, 0]))
    );
    assert_eq!(
        store.interrupt_flags_for_difficulty_like_cpp(61_719, 0, None),
        Some(([0x3, 0], [0, 0])),
        "a corrected regular spell without a SpellInterrupts row receives a base mask"
    );
}

#[test]
fn db2_cast_times_set_max_base_minimum_like_cpp() {
    use crate::spell_db2::{
        SpellCastTimesEntry, SpellCastTimesStore, SpellMiscEntry, SpellMiscStore,
    };
    let mut store = SpellStore::new();
    store
        .spells
        .insert(100, SpellStore::empty_spell_info_like_cpp(100));
    store
        .spells
        .insert(200, SpellStore::empty_spell_info_like_cpp(200));

    let misc = SpellMiscStore::from_entries([
        SpellMiscEntry {
            id: 1,
            spell_id: 100,
            casting_time_index: 5,
            difficulty_id: 0,
            ..Default::default()
        },
        // casting_time_index 0 → no cast-time row, stays instant.
        SpellMiscEntry {
            id: 2,
            spell_id: 200,
            casting_time_index: 0,
            difficulty_id: 0,
            ..Default::default()
        },
    ]);
    let cast_times = SpellCastTimesStore::from_entries([SpellCastTimesEntry {
        id: 5,
        base: 1500,
        minimum: 1000,
        ..Default::default()
    }]);

    store.apply_db2_cast_times_like_cpp(&misc, &cast_times);

    // C++ CalcCastTime = max(Base, Minimum) = max(1500, 1000) = 1500.
    assert_eq!(store.spells.get(&100).unwrap().cast_time_ms, 1500);
    assert!(store.spells.get(&100).unwrap().has_cast_time());
    // No CastingTimeIndex → untouched (instant).
    assert_eq!(store.spells.get(&200).unwrap().cast_time_ms, 0);
    assert!(!store.spells.get(&200).unwrap().has_cast_time());
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
fn db2_spell_power_sets_power_costs_like_cpp() {
    use crate::spell_db2::{
        SpellPowerDifficultyEntry, SpellPowerDifficultyStore, SpellPowerEntry, SpellPowerStore,
    };

    let mut store = SpellStore::new();
    store
        .spells
        .insert(400, SpellStore::empty_spell_info_like_cpp(400));

    let spell_power = SpellPowerStore::from_entries([
        SpellPowerEntry {
            id: 10,
            order_index: 1,
            mana_cost: 40,
            mana_cost_per_level: 4,
            mana_per_second: 5,
            power_display_id: 0,
            alt_power_bar_id: 0,
            power_cost_pct: 10.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 6.5,
            power_type: PowerType::Mana as i8,
            required_aura_spell_id: 0,
            optional_cost: 0,
            spell_id: 400,
        },
        SpellPowerEntry {
            id: 11,
            order_index: 2,
            mana_cost: 999,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_display_id: 0,
            alt_power_bar_id: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            power_type: PowerType::Mana as i8,
            required_aura_spell_id: 0,
            optional_cost: 0,
            spell_id: 400,
        },
    ]);
    let spell_power_difficulty =
        SpellPowerDifficultyStore::from_entries([SpellPowerDifficultyEntry {
            id: 11,
            difficulty_id: 1,
            order_index: 2,
        }]);

    store.apply_db2_power_costs_like_cpp(&spell_power, &spell_power_difficulty);

    let costs = &store.spells.get(&400).unwrap().power_costs;
    assert_eq!(costs.len(), 1, "non-default difficulty rows are skipped");
    assert_eq!(costs[0].order_index, 1);
    assert_eq!(costs[0].mana_cost, 40);
    assert_eq!(costs[0].mana_cost_per_level, 4);
    assert_eq!(costs[0].mana_per_second, 5);
    assert_eq!(costs[0].power_cost_pct, 10.0);
    assert_eq!(costs[0].power_pct_per_second, 6.5);
    assert_eq!(costs[0].power_type, PowerType::Mana as i8);
}

#[test]
fn spell_info_calc_power_costs_flat_plus_mana_pct_like_cpp() {
    let mut spell = SpellStore::empty_spell_info_like_cpp(500);
    spell.power_costs.push(SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: PowerType::Mana as i8,
        mana_cost: 50,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 12.5,
        power_cost_max_pct: 0.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });

    let costs = spell.calc_power_costs_like_cpp(1000);

    assert_eq!(
        costs,
        vec![SpellPowerCostLikeCpp {
            power_type: PowerType::Mana as i8,
            amount: 175,
        }]
    );
}

#[test]
fn spell_info_calc_power_costs_ignores_mana_max_pct_like_cpp() {
    let mut spell = SpellStore::empty_spell_info_like_cpp(501);
    spell.power_costs.push(SpellPowerCostInfoLikeCpp {
        order_index: 0,
        power_type: PowerType::Mana as i8,
        mana_cost: 0,
        mana_cost_per_level: 0,
        mana_per_second: 0,
        power_cost_pct: 0.0,
        power_cost_max_pct: 18.0,
        power_pct_per_second: 0.0,
        required_aura_spell_id: 0,
        optional_cost: 0,
    });

    let costs = spell.calc_power_costs_like_cpp(1000);

    assert!(costs.is_empty());
}

#[test]
fn spell_store_db2_loader_composes_shapeshift_masks_like_cpp() {
    let spell_id = 70_001;
    let misc_store =
        crate::spell_db2::SpellMiscStore::from_entries([test_spell_misc_entry_like_cpp(
            1, spell_id, 0, 0,
        )]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
        crate::spell_db2::SpellShapeshiftEntry {
            id: 1,
            spell_id: spell_id as i32,
            stance_bar_order: 0,
            shapeshift_exclude: [1 << 2, 0],
            shapeshift_mask: [1 << 4, 0],
        },
    ]);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 3, |_| Some(&form)),
        Some(SpellCastResult::NotShapeshift)
    );
    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 5, |_| Some(&form)),
        Some(SpellCastResult::Success)
    );
    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
        Some(SpellCastResult::OnlyShapeshift)
    );
}

#[test]
fn spell_store_check_shapeshift_uses_spell_misc_attr2_like_cpp() {
    let spell_id = 70_002;
    let mut misc = test_spell_misc_entry_like_cpp(1, spell_id, 0, 0);
    misc.attributes[2] = attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM as i32;
    let misc_store = crate::spell_db2::SpellMiscStore::from_entries([misc]);
    let effect_store = crate::spell_db2::SpellEffectDb2Store::from_entries([]);
    let shapeshift_store = crate::spell_db2::SpellShapeshiftStore::from_entries([
        crate::spell_db2::SpellShapeshiftEntry {
            id: 2,
            spell_id: spell_id as i32,
            stance_bar_order: 0,
            shapeshift_exclude: [0, 0],
            shapeshift_mask: [1 << 4, 0],
        },
    ]);
    let store = SpellStore::from_spell_db2_stores_like_cpp(
        &crate::spell_db2::SpellCategoriesStore::from_entries([]),
        &misc_store,
        &effect_store,
        &shapeshift_store,
    );

    assert_eq!(
        store.check_shapeshift_like_cpp(spell_id as i32, 0, |_| None),
        Some(SpellCastResult::Success)
    );
}

#[test]
fn test_spell_info_effective_cooldown() {
    let spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 1500,
        recovery_time_ms: 8000,
        effect_type: 2,
        effect_base_points: 50,
        effect_bonus_coefficient: 0.5,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    // recovery_time_ms is larger
    assert_eq!(spell.effective_cooldown_ms(), 8000);

    let instant = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 1500,
        recovery_time_ms: 0,
        effect_type: 2,
        effect_base_points: 50,
        effect_bonus_coefficient: 0.5,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    // GCD is the limit
    assert_eq!(instant.effective_cooldown_ms(), 1500);
}

#[test]
fn spell_info_requires_spell_focus_matches_cpp_field() {
    let mut spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    };

    assert!(!spell.requires_spell_focus_like_cpp());
    spell.requires_spell_focus = 181;
    assert!(spell.requires_spell_focus_like_cpp());
}

#[test]
fn spell_implicit_target_effect_mask_normalizes_like_cpp_conditionmgr() {
    let spell = SpellInfo {
        spell_id: 100,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![
            SpellEffectInfo {
                effect_index: 0,
                effect: 0,
                chain_targets: 0,
                implicit_target_1: 6,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 1,
                effect: 0,
                chain_targets: 0,
                implicit_target_1: 7,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 2,
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
                chain_targets: 0,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            },
            SpellEffectInfo {
                effect_index: 3,
                effect: 0,
                chain_targets: 2,
                implicit_target_1: 0,
                implicit_target_2: 0,
                ..Default::default()
            },
        ],
    };

    assert_eq!(
        spell.normalized_implicit_target_effect_mask_like_cpp(0b1111),
        0b1110
    );
    assert_eq!(
        spell.normalized_implicit_target_effect_mask_like_cpp(0b0001),
        0
    );
}

#[test]
fn spell_effect_detects_mounted_aura_like_cpp() {
    let mounted = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 11,
        effect_misc_value_1: 22,
        effect_misc_value_2: 33,
        ..Default::default()
    };
    let other_aura = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_HASTE_SPELLS,
        ..Default::default()
    };

    assert!(mounted.is_mounted_aura_like_cpp());
    assert!(!other_aura.is_mounted_aura_like_cpp());
    assert_eq!(mounted.effect_base_points, 11);
    assert_eq!(mounted.effect_misc_value_1, 22);
    assert_eq!(mounted.effect_misc_value_2, 33);
}

#[test]
fn spell_effect_calc_value_no_caster_rolls_die_sides_like_cpp() {
    let no_die = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 0,
        ..Default::default()
    };
    assert_eq!(
        no_die.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        10
    );

    let one_sided = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 1,
        ..Default::default()
    };
    assert_eq!(
        one_sided.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        11
    );

    let positive_range = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: 7,
        ..Default::default()
    };
    assert_eq!(
        positive_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (1, 7));
            4
        }),
        14
    );

    let negative_range = SpellEffectInfo {
        effect_base_points: 10,
        effect_die_sides: -3,
        ..Default::default()
    };
    assert_eq!(
        negative_range.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (-3, 1));
            -2
        }),
        8
    );
}

#[test]
fn spell_effect_calc_value_no_caster_uses_cpp_double_accumulator() {
    let overflowing_int_add = SpellEffectInfo {
        effect_base_points: i32::MAX,
        effect_die_sides: 1,
        ..Default::default()
    };
    assert_eq!(
        overflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|_, _| unreachable!()),
        i32::MAX
    );

    let underflowing_int_add = SpellEffectInfo {
        effect_base_points: i32::MIN,
        effect_die_sides: -1,
        ..Default::default()
    };
    assert_eq!(
        underflowing_int_add.calc_value_no_caster_with_die_roll_like_cpp(|min, max| {
            assert_eq!((min, max), (-1, 1));
            -1
        }),
        i32::MIN
    );
}

#[test]
fn spell_effect_constants_match_cpp_shared_defines() {
    // C++ `SharedDefines.h`: `SpellEffects` enum.
    assert_eq!(spell_effect_types::SPELL_EFFECT_NONE, 0);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE, 2);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT, 4);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AURA, 6);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE, 7);
    assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_DRAIN, 8);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEALTH_LEECH, 9);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL, 10);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BIND, 11);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PORTAL, 12);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_BASE, 13);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE, 14);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL, 15);
    assert_eq!(spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE, 16);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS, 19);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DODGE, 20);
    assert_eq!(spell_effect_types::SPELL_EFFECT_EVADE, 21);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PARRY, 22);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BLOCK, 23);
    assert_eq!(spell_effect_types::SPELL_EFFECT_WEAPON, 25);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DEFENSE, 26);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE, 30);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY, 35);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SPELL, 36);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE, 37);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LANGUAGE, 39);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DUAL_WIELD, 40);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SKILL, 118);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PLAY_MOVIE, 45);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SPAWN, 46);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TRADE_SKILL, 47);
    assert_eq!(spell_effect_types::SPELL_EFFECT_STEALTH, 48);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DETECT, 49);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT, 51);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT, 52);
    assert_eq!(spell_effect_types::SPELL_EFFECT_POWER_BURN, 62);
    assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT, 63);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID, 65);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH, 67);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DISTRACT, 69);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PULL, 70);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL, 75);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK, 78);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SANCTUARY, 79);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_HOUSE, 81);
    assert_eq!(spell_effect_types::SPELL_EFFECT_BIND_SIGHT, 82);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DUEL, 83);
    assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT, 90);
    assert_eq!(spell_effect_types::SPELL_EFFECT_THREAT_ALL, 91);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_DESELECT, 93);
    assert_eq!(spell_effect_types::SPELL_EFFECT_INEBRIATE, 100);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DISMISS_PET, 102);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION, 103);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SURVEY, 105);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER, 106);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT, 107);
    assert_eq!(spell_effect_types::SPELL_EFFECT_112, 112);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ATTACK_ME, 114);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET, 119);
    assert_eq!(spell_effect_types::SPELL_EFFECT_122, 122);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT, 125);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND, 128);
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY, 129);
    assert_eq!(spell_effect_types::SPELL_EFFECT_KILL_CREDIT2, 134);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CALL_PET, 135);
    assert_eq!(spell_effect_types::SPELL_EFFECT_HEAL_PCT, 136);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT, 137);
    assert_eq!(spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM, 163);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET, 168);
    assert_eq!(spell_effect_types::SPELL_EFFECT_175, 175);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
        177
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_178, 178);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER, 180);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER, 182);
    assert_eq!(spell_effect_types::SPELL_EFFECT_183, 183);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPUTATION_2, 184);
    assert_eq!(spell_effect_types::SPELL_EFFECT_185, 185);
    assert_eq!(spell_effect_types::SPELL_EFFECT_186, 186);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
        187
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
        188
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_LOOT, 189);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS, 190);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE, 191);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET, 192);
    assert_eq!(spell_effect_types::SPELL_EFFECT_START_PET_BATTLE, 193);
    assert_eq!(spell_effect_types::SPELL_EFFECT_194, 194);
    assert_eq!(spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON, 199);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
        202
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
        204
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_ALTER_ITEM, 206);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK, 207);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_REPUTATION, 208);
    assert_eq!(spell_effect_types::SPELL_EFFECT_209, 209);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
        210
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
        211
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_GARRISON, 214);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
        215
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT, 216);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON, 217);
    assert_eq!(spell_effect_types::SPELL_EFFECT_218, 218);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER, 220);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION, 221);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES, 223);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
        224
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL, 225);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET, 226);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
        227
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_228, 228);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY, 229);
    assert_eq!(spell_effect_types::SPELL_EFFECT_230, 230);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
        231
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_PHASE, 232);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
        233
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_234, 234);
    assert_eq!(spell_effect_types::SPELL_EFFECT_235, 235);
    assert_eq!(spell_effect_types::SPELL_EFFECT_INCREASE_SKILL, 238);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
        239
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER, 240);
    assert_eq!(spell_effect_types::SPELL_EFFECT_241, 241);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
        242
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY, 244);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM, 245);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
        246
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
        247
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT, 248);
    assert_eq!(spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM, 249);
    assert_eq!(spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT, 250);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
        251
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS, 252);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GIVE_HONOR, 253);
    assert_eq!(spell_effect_types::SPELL_EFFECT_JUMP_CHARGE, 254);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET, 255);
    assert_eq!(spell_effect_types::SPELL_EFFECT_256, 256);
    assert_eq!(spell_effect_types::SPELL_EFFECT_257, 257);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE, 258);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
        259
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET, 260);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SCRAP_ITEM, 261);
    assert_eq!(spell_effect_types::SPELL_EFFECT_262, 262);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REPAIR_ITEM, 263);
    assert_eq!(spell_effect_types::SPELL_EFFECT_REMOVE_GEM, 264);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
        265
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
        266
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT, 268);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
        269
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_270, 270);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
        271
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_COVENANT, 272);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
        273
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_274, 274);
    assert_eq!(spell_effect_types::SPELL_EFFECT_275, 275);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
        276
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME, 277);
    assert_eq!(spell_effect_types::SPELL_EFFECT_278, 278);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT, 279);
    assert_eq!(spell_effect_types::SPELL_EFFECT_280, 280);
    assert_eq!(spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT, 281);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
        282
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN, 283);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2, 285);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
        286
    );
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
        287
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ITEM, 288);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_AURA_STACKS, 289);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN, 290);
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS, 291);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWNS_BY_CATEGORY,
        292
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES, 293);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_LOOT, 294);
    assert_eq!(spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM, 295);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM, 296);
    assert_eq!(spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM, 297);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
        298
    );
    assert_eq!(spell_effect_types::SPELL_EFFECT_299, 299);
    assert_eq!(spell_effect_types::SPELL_EFFECT_300, 300);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT, 301);
    assert_eq!(spell_effect_types::SPELL_EFFECT_GATHERING, 302);
    assert_eq!(spell_effect_types::SPELL_EFFECT_305, 305);
    assert_eq!(spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS, 306);
    assert_eq!(spell_effect_types::SPELL_EFFECT_307, 307);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD, 308);
    assert_eq!(spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD, 309);
    assert_eq!(spell_effect_types::SPELL_EFFECT_310, 310);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED, 311);
    assert_eq!(spell_effect_types::SPELL_EFFECT_312, 312);
    assert_eq!(spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2, 313);
    assert_eq!(spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS, 314);
    assert_eq!(
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
        315
    );

    // C++ `SpellAuraDefines.h`: selected `AuraType` enum anchors.
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SPEED, 31);
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED, 32);
    assert_eq!(aura_types::SPELL_AURA_MOD_DECREASE_SPEED, 33);
    assert_eq!(aura_types::SPELL_AURA_MOD_SHAPESHIFT, 36);
    assert_eq!(aura_types::SPELL_AURA_TRANSFORM, 56);
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED, 58);
    assert_eq!(aura_types::SPELL_AURA_MOD_SCALE, 61);
    assert_eq!(aura_types::SPELL_AURA_MOUNTED, 78);
    assert_eq!(aura_types::SPELL_AURA_MOD_DETECT_RANGE, 91);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_ALWAYS, 129);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS, 130);
    assert_eq!(aura_types::SPELL_AURA_MOD_DETECTED_RANGE, 152);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK, 171);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK, 172);
    assert_eq!(aura_types::SPELL_AURA_FLY, 201);
    assert_eq!(
        aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
        207
    );
    assert_eq!(aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED, 191);
    assert_eq!(
        aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED,
        206
    );
    assert_eq!(aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED, 208);
    assert_eq!(aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS, 209);
    assert_eq!(aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK, 211);
    assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED, 305);
    assert_eq!(aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL, 373);
    assert_eq!(aura_types::SPELL_AURA_MOD_BATTLE_PET_XP_PCT, 420);
    assert_eq!(aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE, 437);
    assert_eq!(aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION, 499);

    // C++ `SharedDefines.h`: selected SpellAttr0 anchors.
    assert_eq!(attributes::SPELL_ATTR0_ONLY_INDOORS, 0x0000_4000);
    assert_eq!(attributes::SPELL_ATTR0_ONLY_OUTDOORS, 0x0000_8000);
    assert_eq!(attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED, 0x0100_0000);
}

#[test]
fn spell_effect_null_or_unused_classifier_matches_cpp_dispatch_subset() {
    for effect in [
        spell_effect_types::SPELL_EFFECT_NONE,
        spell_effect_types::SPELL_EFFECT_PORTAL_TELEPORT,
        spell_effect_types::SPELL_EFFECT_PORTAL,
        spell_effect_types::SPELL_EFFECT_RITUAL_BASE,
        spell_effect_types::SPELL_EFFECT_RITUAL_SPECIALIZE,
        spell_effect_types::SPELL_EFFECT_RITUAL_ACTIVATE_PORTAL,
        spell_effect_types::SPELL_EFFECT_DODGE,
        spell_effect_types::SPELL_EFFECT_EVADE,
        spell_effect_types::SPELL_EFFECT_WEAPON,
        spell_effect_types::SPELL_EFFECT_DEFENSE,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY,
        spell_effect_types::SPELL_EFFECT_SPELL_DEFENSE,
        spell_effect_types::SPELL_EFFECT_LANGUAGE,
        spell_effect_types::SPELL_EFFECT_SPAWN,
        spell_effect_types::SPELL_EFFECT_STEALTH,
        spell_effect_types::SPELL_EFFECT_DETECT,
        spell_effect_types::SPELL_EFFECT_FORCE_CRITICAL_HIT,
        spell_effect_types::SPELL_EFFECT_GUARANTEE_HIT,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_RAID,
        spell_effect_types::SPELL_EFFECT_ATTACK,
        spell_effect_types::SPELL_EFFECT_CREATE_HOUSE,
        spell_effect_types::SPELL_EFFECT_BIND_SIGHT,
        spell_effect_types::SPELL_EFFECT_THREAT_ALL,
        spell_effect_types::SPELL_EFFECT_SURVEY,
        spell_effect_types::SPELL_EFFECT_SHOW_CORPSE_LOOT,
        spell_effect_types::SPELL_EFFECT_112,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET,
        spell_effect_types::SPELL_EFFECT_122,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY,
        spell_effect_types::SPELL_EFFECT_CALL_PET,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_OWNER,
        spell_effect_types::SPELL_EFFECT_OBLITERATE_ITEM,
        spell_effect_types::SPELL_EFFECT_ALLOW_CONTROL_PET,
        spell_effect_types::SPELL_EFFECT_175,
        spell_effect_types::SPELL_EFFECT_DESPAWN_PERSISTENT_AREA_AURA,
        spell_effect_types::SPELL_EFFECT_178,
        spell_effect_types::SPELL_EFFECT_UPDATE_AREATRIGGER,
        spell_effect_types::SPELL_EFFECT_DESPAWN_AREATRIGGER,
        spell_effect_types::SPELL_EFFECT_183,
        spell_effect_types::SPELL_EFFECT_REPUTATION_2,
        spell_effect_types::SPELL_EFFECT_185,
        spell_effect_types::SPELL_EFFECT_186,
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_ARCHAEOLOGY_DIGSITES,
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET_AS_GUARDIAN,
        spell_effect_types::SPELL_EFFECT_LOOT,
        spell_effect_types::SPELL_EFFECT_CHANGE_PARTY_MEMBERS,
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_DIGSITE,
        spell_effect_types::SPELL_EFFECT_START_PET_BATTLE,
        spell_effect_types::SPELL_EFFECT_194,
        spell_effect_types::SPELL_EFFECT_DESPAWN_SUMMON,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
        spell_effect_types::SPELL_EFFECT_ALTER_ITEM,
        spell_effect_types::SPELL_EFFECT_LAUNCH_QUEST_TASK,
        spell_effect_types::SPELL_EFFECT_SET_REPUTATION,
        spell_effect_types::SPELL_EFFECT_209,
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_BUILDING,
        spell_effect_types::SPELL_EFFECT_LEARN_GARRISON_SPECIALIZATION,
        spell_effect_types::SPELL_EFFECT_CREATE_GARRISON,
        spell_effect_types::SPELL_EFFECT_UPGRADE_CHARACTER_SPELLS,
        spell_effect_types::SPELL_EFFECT_CREATE_SHIPMENT,
        spell_effect_types::SPELL_EFFECT_UPGRADE_GARRISON,
        spell_effect_types::SPELL_EFFECT_218,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_FOLLOWER,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION,
        spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES,
        spell_effect_types::SPELL_EFFECT_ACTIVATE_GARRISON_BUILDING,
        spell_effect_types::SPELL_EFFECT_TRIGGER_ACTION_SET,
        spell_effect_types::SPELL_EFFECT_TELEPORT_TO_LFG_DUNGEON,
        spell_effect_types::SPELL_EFFECT_228,
        spell_effect_types::SPELL_EFFECT_SET_FOLLOWER_QUALITY,
        spell_effect_types::SPELL_EFFECT_230,
        spell_effect_types::SPELL_EFFECT_INCREASE_FOLLOWER_EXPERIENCE,
        spell_effect_types::SPELL_EFFECT_REMOVE_PHASE,
        spell_effect_types::SPELL_EFFECT_RANDOMIZE_FOLLOWER_ABILITIES,
        spell_effect_types::SPELL_EFFECT_234,
        spell_effect_types::SPELL_EFFECT_235,
        spell_effect_types::SPELL_EFFECT_INCREASE_SKILL,
        spell_effect_types::SPELL_EFFECT_END_GARRISON_BUILDING_CONSTRUCTION,
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER,
        spell_effect_types::SPELL_EFFECT_241,
        spell_effect_types::SPELL_EFFECT_GIVE_ARTIFACT_POWER_NO_BONUS,
        spell_effect_types::SPELL_EFFECT_LEARN_FOLLOWER_ABILITY,
        spell_effect_types::SPELL_EFFECT_FINISH_GARRISON_MISSION,
        spell_effect_types::SPELL_EFFECT_ADD_GARRISON_MISSION_SET,
        spell_effect_types::SPELL_EFFECT_FINISH_SHIPMENT,
        spell_effect_types::SPELL_EFFECT_FORCE_EQUIP_ITEM,
        spell_effect_types::SPELL_EFFECT_TAKE_SCREENSHOT,
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_CACHE_SIZE,
        spell_effect_types::SPELL_EFFECT_256,
        spell_effect_types::SPELL_EFFECT_257,
        spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE,
        spell_effect_types::SPELL_EFFECT_RESPEC_AZERITE_EMPOWERED_ITEM,
        spell_effect_types::SPELL_EFFECT_SUMMON_STABLED_PET,
        spell_effect_types::SPELL_EFFECT_SCRAP_ITEM,
        spell_effect_types::SPELL_EFFECT_262,
        spell_effect_types::SPELL_EFFECT_REPAIR_ITEM,
        spell_effect_types::SPELL_EFFECT_REMOVE_GEM,
        spell_effect_types::SPELL_EFFECT_LEARN_AZERITE_ESSENCE_POWER,
        spell_effect_types::SPELL_EFFECT_SET_ITEM_BONUS_LIST_GROUP_ENTRY,
        spell_effect_types::SPELL_EFFECT_APPLY_MOUNT_EQUIPMENT,
        spell_effect_types::SPELL_EFFECT_INCREASE_ITEM_BONUS_LIST_GROUP_STEP,
        spell_effect_types::SPELL_EFFECT_270,
        spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
        spell_effect_types::SPELL_EFFECT_SET_COVENANT,
        spell_effect_types::SPELL_EFFECT_CRAFT_RUNEFORGE_LEGENDARY,
        spell_effect_types::SPELL_EFFECT_274,
        spell_effect_types::SPELL_EFFECT_275,
        spell_effect_types::SPELL_EFFECT_SET_CHROMIE_TIME,
        spell_effect_types::SPELL_EFFECT_278,
        spell_effect_types::SPELL_EFFECT_LEARN_GARR_TALENT,
        spell_effect_types::SPELL_EFFECT_280,
        spell_effect_types::SPELL_EFFECT_LEARN_SOULBIND_CONDUIT,
        spell_effect_types::SPELL_EFFECT_CONVERT_ITEMS_TO_CURRENCY,
        spell_effect_types::SPELL_EFFECT_COMPLETE_CAMPAIGN,
        spell_effect_types::SPELL_EFFECT_MODIFY_KEYSTONE_2,
        spell_effect_types::SPELL_EFFECT_SET_GARRISON_FOLLOWER_LEVEL,
        spell_effect_types::SPELL_EFFECT_CRAFT_ITEM,
        spell_effect_types::SPELL_EFFECT_CRAFT_LOOT,
        spell_effect_types::SPELL_EFFECT_SALVAGE_ITEM,
        spell_effect_types::SPELL_EFFECT_CRAFT_SALVAGE_ITEM,
        spell_effect_types::SPELL_EFFECT_RECRAFT_ITEM,
        spell_effect_types::SPELL_EFFECT_CANCEL_ALL_PRIVATE_CONVERSATIONS,
        spell_effect_types::SPELL_EFFECT_299,
        spell_effect_types::SPELL_EFFECT_300,
        spell_effect_types::SPELL_EFFECT_CRAFT_ENCHANT,
        spell_effect_types::SPELL_EFFECT_GATHERING,
        spell_effect_types::SPELL_EFFECT_305,
        spell_effect_types::SPELL_EFFECT_UPDATE_INTERACTIONS,
        spell_effect_types::SPELL_EFFECT_307,
        spell_effect_types::SPELL_EFFECT_CANCEL_PRELOAD_WORLD,
        spell_effect_types::SPELL_EFFECT_PRELOAD_WORLD,
        spell_effect_types::SPELL_EFFECT_310,
        spell_effect_types::SPELL_EFFECT_ENSURE_WORLD_LOADED,
        spell_effect_types::SPELL_EFFECT_312,
        spell_effect_types::SPELL_EFFECT_CHANGE_ITEM_BONUSES_2,
        spell_effect_types::SPELL_EFFECT_ADD_SOCKET_BONUS,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_APPEARANCE_FROM_ITEM_MOD_APPEARANCE_GROUP,
    ] {
        assert!(
            spell_effect_types::is_cpp_null_or_unused_noop(effect),
            "effect {effect} should mirror C++ EffectNULL/EffectUnused"
        );
    }

    assert!(
        !spell_effect_types::is_cpp_null_or_unused_noop(3),
        "C++ SPELL_EFFECT_DUMMY dispatches EffectDummy and remains script-driven"
    );
    assert!(!spell_effect_types::is_cpp_null_or_unused_noop(
        spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE
    ));
    for real_handler_effect in [
        spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY,
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL,
        243,
        spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM,
        spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
        spell_effect_types::SPELL_EFFECT_JUMP_CHARGE,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET,
        spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
        284,
        spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
        289,
        290,
        291,
        292,
        293,
        303,
        304,
    ] {
        assert!(
            !spell_effect_types::is_cpp_null_or_unused_noop(real_handler_effect),
            "effect {real_handler_effect} has a real C++ dispatch handler in this range"
        );
    }
}

#[test]
fn spell_effect_detects_provide_spell_focus_aura_like_cpp() {
    let focus = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
        effect_misc_value_1: 181,
        ..Default::default()
    };
    let other_effect = SpellEffectInfo {
        effect: spell_effect_types::SPELL_EFFECT_HEAL,
        effect_aura: aura_types::SPELL_AURA_PROVIDE_SPELL_FOCUS,
        ..Default::default()
    };

    assert!(focus.is_provide_spell_focus_aura_like_cpp());
    assert!(!other_effect.is_provide_spell_focus_aura_like_cpp());
    assert_eq!(focus.effect_misc_value_1, 181);
}

#[test]
fn spell_effect_detects_focus_destination_implicit_targets_like_cpp() {
    let mut effect = SpellEffectInfo {
        implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
        ..Default::default()
    };
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_1 = 0;
    effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_2;
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_2 = implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    assert!(effect.has_focus_destination_implicit_target_like_cpp());

    effect.implicit_target_2 = 40;
    assert!(!effect.has_focus_destination_implicit_target_like_cpp());
}

#[test]
fn spell_target_position_store_loads_or_db_targets_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        710,
        SpellInfo {
            spell_id: 710,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 1,
                implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 710,
            effect_index: 1,
            target_map_id: 571,
            x: 100.0,
            y: 200.0,
            z: 30.0,
            orientation: Some(1.25),
        }],
        &spell_store,
        |map_id| map_id == 571,
    );

    assert_eq!(store.load_report_like_cpp().loaded, 1);
    assert_eq!(
        store.get(710, 1).map(|target| target.position),
        Some(wow_core::Position::new(100.0, 200.0, 30.0, 1.25))
    );
}

#[test]
fn spell_target_position_store_uses_effect_facing_when_orientation_is_null_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        9268,
        SpellInfo {
            spell_id: 9268,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                position_facing: 90.0,
                implicit_target_1: implicit_targets::TARGET_DEST_DB,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 9268,
            effect_index: 0,
            target_map_id: 0,
            x: -10.0,
            y: 20.0,
            z: 5.0,
            orientation: None,
        }],
        &spell_store,
        |map_id| map_id == 0,
    );

    let position = store.get(9268, 0).expect("target position").position;
    assert!((position.orientation - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
}

#[test]
fn spell_target_position_store_rejects_wrong_effect_target_like_cpp() {
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        711,
        SpellInfo {
            spell_id: 711,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                implicit_target_1: implicit_targets::TARGET_DEST_NEARBY_ENTRY,
                ..Default::default()
            }],
        },
    );

    let store = SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
        [SpellTargetPositionRowLikeCpp {
            spell_id: 711,
            effect_index: 0,
            target_map_id: 571,
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: Some(0.0),
        }],
        &spell_store,
        |_| true,
    );

    assert!(store.is_empty());
    assert_eq!(store.load_report_like_cpp().skipped_unsupported_target, 1);
}

#[test]
fn spell_implicit_target_conditions_attach_to_effects_like_cpp() {
    let mut store = SpellStore::new();
    store.insert(
        100,
        SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                SpellEffectInfo {
                    effect_index: 0,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 6,
                    implicit_target_2: 0,
                    ..Default::default()
                },
                SpellEffectInfo {
                    effect_index: 1,
                    effect: 0,
                    chain_targets: 0,
                    implicit_target_1: 7,
                    implicit_target_2: 0,
                    ..Default::default()
                },
            ],
        },
    );
    let conditions = ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
        source_type: ConditionSourceType::SpellImplicitTarget,
        source_group: 0b11,
        source_entry: 100,
        condition_type: ConditionType::Aura,
        ..Condition::default()
    }]);

    assert_eq!(
        store.attach_spell_implicit_target_conditions_like_cpp(&conditions),
        2
    );
    assert!(
        store
            .implicit_target_conditions_like_cpp(100, 0)
            .and_then(|reference| reference.upgrade())
            .is_some_and(|conditions| conditions.len() == 1)
    );
    assert!(
        store
            .implicit_target_conditions_like_cpp(100, 1)
            .and_then(|reference| reference.upgrade())
            .is_some_and(|conditions| conditions.len() == 1)
    );
}

#[test]
fn spell_pet_aura_store_loads_first_row_metadata_and_wildcard_like_cpp() {
    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            SpellPetAuraRowLikeCpp {
                spell_id: 10,
                effect_index: 1,
                pet_entry: 0,
                aura_id: 100,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 10,
                effect_index: 1,
                pet_entry: 700,
                aura_id: 200,
            },
        ],
        |spell_id, effect_index| {
            assert_eq!((spell_id, effect_index), (10, 1));
            SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                apply_aura_name: SPELL_AURA_DUMMY_LIKE_CPP,
                target_a: TARGET_UNIT_PET_LIKE_CPP,
                calc_value: 35,
            })
        },
        |aura_id| matches!(aura_id, 100 | 200),
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    let pet_aura = outcome.store.get_pet_aura_like_cpp(10, 1).unwrap();
    assert!(pet_aura.remove_on_change_pet);
    assert_eq!(pet_aura.damage, 35);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(700), 200);
    assert_eq!(
        pet_aura.aura_for_pet_entry_like_cpp(701),
        100,
        "C++ PetAura::GetAura falls back to petEntry 0"
    );
    assert_eq!(
        outcome.store.get_pet_aura_like_cpp(10, 2),
        None,
        "C++ SpellMgr::GetPetAura keys by (spell << 8) + effect index"
    );
}

#[test]
fn spell_pet_aura_store_rejects_invalid_first_rows_like_cpp() {
    let rows = [
        SpellPetAuraRowLikeCpp {
            spell_id: 1,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 10,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 2,
            effect_index: 3,
            pet_entry: 0,
            aura_id: 20,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 3,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 30,
        },
        SpellPetAuraRowLikeCpp {
            spell_id: 4,
            effect_index: 0,
            pet_entry: 0,
            aura_id: 40,
        },
    ];

    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        rows,
        |spell_id, _| match spell_id {
            1 => SpellPetAuraSourceLookupLikeCpp::SpellMissing,
            2 => SpellPetAuraSourceLookupLikeCpp::EffectIndexMissing,
            3 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                apply_aura_name: 73,
                target_a: 0,
                calc_value: 0,
            }),
            4 => SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                apply_aura_name: 0,
                target_a: 0,
                calc_value: 0,
            }),
            _ => unreachable!(),
        },
        |aura_id| aura_id != 40,
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert!(outcome.store.auras_by_spell_effect_key.is_empty());
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellPetAuraLoadErrorKindLikeCpp::SpellMissing,
            SpellPetAuraLoadErrorKindLikeCpp::EffectIndexMissing,
            SpellPetAuraLoadErrorKindLikeCpp::SourceEffectNotDummy,
            SpellPetAuraLoadErrorKindLikeCpp::AuraSpellMissing,
        ]
    );
}

#[test]
fn spell_pet_aura_store_duplicate_keys_add_aura_without_revalidation_like_cpp() {
    let mut source_lookups = 0;
    let mut aura_checks = 0;
    let outcome = SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
        [
            SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 500,
                aura_id: 900,
            },
            SpellPetAuraRowLikeCpp {
                spell_id: 77,
                effect_index: 2,
                pet_entry: 501,
                aura_id: 0,
            },
        ],
        |_, _| {
            source_lookups += 1;
            SpellPetAuraSourceLookupLikeCpp::Found(SpellPetAuraSourceEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_DUMMY,
                apply_aura_name: 0,
                target_a: 0,
                calc_value: -15,
            })
        },
        |aura_id| {
            aura_checks += 1;
            aura_id == 900
        },
    );

    assert_eq!(
        source_lookups, 1,
        "C++ validates only before creating a new SpellPetAuraMap entry"
    );
    assert_eq!(aura_checks, 1);
    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    let pet_aura = outcome.store.get_pet_aura_like_cpp(77, 2).unwrap();
    assert!(!pet_aura.remove_on_change_pet);
    assert_eq!(pet_aura.damage, -15);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(500), 900);
    assert_eq!(pet_aura.aura_for_pet_entry_like_cpp(501), 0);
}

#[test]
fn spell_threat_store_skips_missing_spells_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [
            SpellThreatRowLikeCpp {
                spell_id: 100,
                flat_mod: 7,
                pct_mod: 1.25,
                ap_pct_mod: 0.5,
            },
            SpellThreatRowLikeCpp {
                spell_id: 200,
                flat_mod: 9,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        ],
        |spell_id| spell_id == 100,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(outcome.errors[0].row.spell_id, 200);
    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(100, |_| unreachable!()),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: 7,
            pct_mod: 1.25,
            ap_pct_mod: 0.5,
        })
    );
}

#[test]
fn spell_threat_store_duplicate_rows_last_wins_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [
            SpellThreatRowLikeCpp {
                spell_id: 300,
                flat_mod: 1,
                pct_mod: 1.0,
                ap_pct_mod: 0.0,
            },
            SpellThreatRowLikeCpp {
                spell_id: 300,
                flat_mod: -4,
                pct_mod: 0.75,
                ap_pct_mod: 0.25,
            },
        ],
        |_| true,
    );

    assert_eq!(
        outcome.loaded_row_count, 2,
        "C++ increments count for every valid row before unordered_map overwrite visibility"
    );
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.store.entries_by_spell_id.len(), 1);
    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(300, |_| unreachable!()),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: -4,
            pct_mod: 0.75,
            ap_pct_mod: 0.25,
        })
    );
}

#[test]
fn spell_threat_store_falls_back_to_first_spell_in_chain_like_cpp() {
    let outcome = SpellThreatStoreLikeCpp::from_rows_like_cpp(
        [SpellThreatRowLikeCpp {
            spell_id: 11,
            flat_mod: 40,
            pct_mod: 1.5,
            ap_pct_mod: 0.0,
        }],
        |_| true,
    );

    assert_eq!(
        outcome
            .store
            .get_spell_threat_entry_like_cpp(42, |spell_id| {
                assert_eq!(spell_id, 42);
                11
            }),
        Some(&SpellThreatEntryLikeCpp {
            flat_mod: 40,
            pct_mod: 1.5,
            ap_pct_mod: 0.0,
        })
    );
    assert_eq!(
        outcome.store.get_spell_threat_entry_like_cpp(43, |_| 43),
        None
    );
}

#[test]
fn spell_linked_store_skips_missing_trigger_and_effect_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 100,
                spell_effect: 200,
                link_type: 0,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 300,
                spell_effect: 400,
                link_type: 0,
            },
        ],
        |spell_id| match spell_id {
            100 => Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            }),
            _ => None,
        },
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLinkedLoadErrorKindLikeCpp::EffectSpellMissing,
            SpellLinkedLoadErrorKindLikeCpp::TriggerSpellMissing,
        ]
    );
    assert!(outcome.store.effects_by_type_and_trigger.is_empty());
}

#[test]
fn spell_linked_store_preserves_signed_effects_and_push_order_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 20,
                link_type: 1,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: -30,
                link_type: 1,
            },
        ],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Hit, 10),
        Some([20, -30].as_slice())
    );
}

#[test]
fn spell_linked_store_negative_trigger_forces_remove_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [SpellLinkedRowLikeCpp {
            spell_trigger: -50,
            spell_effect: 60,
            link_type: 1,
        }],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLinkedLoadWarningKindLikeCpp::NegativeTriggerLinkTypeCoercedToRemove
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Remove, 50),
        Some([60].as_slice())
    );
}

#[test]
fn spell_linked_store_invalid_type_and_self_loop_match_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [
            SpellLinkedRowLikeCpp {
                spell_trigger: 10,
                spell_effect: 10,
                link_type: 0,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 20,
                spell_effect: 20,
                link_type: 2,
            },
            SpellLinkedRowLikeCpp {
                spell_trigger: 30,
                spell_effect: 40,
                link_type: 9,
            },
        ],
        |_| {
            Some(SpellLinkedSpellInfoLikeCpp {
                effect_calc_values_by_index: Vec::new(),
            })
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLinkedLoadErrorKindLikeCpp::SelfTriggerLoop,
            SpellLinkedLoadErrorKindLikeCpp::InvalidLinkType,
        ]
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Aura, 20),
        Some([20].as_slice())
    );
}

#[test]
fn spell_linked_store_same_base_point_warning_does_not_skip_like_cpp() {
    let outcome = SpellLinkedStoreLikeCpp::from_rows_like_cpp(
        [SpellLinkedRowLikeCpp {
            spell_trigger: 70,
            spell_effect: 12,
            link_type: 0,
        }],
        |spell_id| {
            if spell_id == 70 {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: vec![(2, 12)],
                })
            } else {
                Some(SpellLinkedSpellInfoLikeCpp {
                    effect_calc_values_by_index: Vec::new(),
                })
            }
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLinkedLoadWarningKindLikeCpp::TriggerEffectSameBasePoint { effect_index: 2 }
    );
    assert_eq!(
        outcome
            .store
            .get_spell_linked_like_cpp(SpellLinkedTypeLikeCpp::Cast, 70),
        Some([12].as_slice())
    );
}

#[test]
fn spell_totem_model_store_skips_missing_dependencies_like_cpp() {
    let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 2,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 20,
                race_id: 2,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 3,
                display_id: 100,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 10,
                race_id: 2,
                display_id: 200,
            },
        ],
        |spell_id| spell_id == 10,
        |race_id| race_id == 2,
        |display_id| display_id == 100,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellTotemModelLoadErrorKindLikeCpp::SpellMissing,
            SpellTotemModelLoadErrorKindLikeCpp::RaceMissing,
            SpellTotemModelLoadErrorKindLikeCpp::DisplayMissing,
        ]
    );
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 2), 100);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(10, 3), 0);
}

#[test]
fn spell_totem_model_store_duplicate_rows_last_wins_like_cpp() {
    let outcome = SpellTotemModelStoreLikeCpp::from_rows_like_cpp(
        [
            SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 1000,
            },
            SpellTotemModelRowLikeCpp {
                spell_id: 50,
                race_id: 8,
                display_id: 2000,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
    );

    assert_eq!(
        outcome.loaded_row_count, 2,
        "C++ increments count for every valid row before std::map overwrite visibility"
    );
    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.store.display_id_by_spell_and_race.len(), 1);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 8), 2000);
    assert_eq!(outcome.store.get_model_for_totem_like_cpp(50, 2), 0);
}

#[test]
fn spell_required_store_skips_missing_and_same_chain_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            SpellRequiredRowLikeCpp {
                spell_id: 10,
                req_spell: 20,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 30,
                req_spell: 40,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 50,
                req_spell: 60,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20 | 30 | 50 | 60),
        |spell_id, req_spell| spell_id == 50 && req_spell == 60,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellRequiredLoadErrorKindLikeCpp::RequiredSpellMissing,
            SpellRequiredLoadErrorKindLikeCpp::SameRankChain,
        ]
    );
    assert_eq!(outcome.store.spells_required_for_spell_like_cpp(10), &[20]);
    assert_eq!(outcome.store.spells_requiring_spell_like_cpp(20), &[10]);
}

#[test]
fn spell_required_store_skips_missing_spell_id_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [SpellRequiredRowLikeCpp {
            spell_id: 70,
            req_spell: 80,
        }],
        |spell_id| spell_id == 80,
        |_, _| false,
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        SpellRequiredLoadErrorKindLikeCpp::SpellMissing
    );
}

#[test]
fn spell_required_store_skips_duplicate_exact_pair_like_cpp() {
    let outcome = SpellRequiredStoreLikeCpp::from_rows_like_cpp(
        [
            SpellRequiredRowLikeCpp {
                spell_id: 90,
                req_spell: 100,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 90,
                req_spell: 100,
            },
            SpellRequiredRowLikeCpp {
                spell_id: 91,
                req_spell: 100,
            },
        ],
        |_| true,
        |_, _| false,
    );

    assert_eq!(outcome.loaded_row_count, 2);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        SpellRequiredLoadErrorKindLikeCpp::Duplicate
    );
    assert!(outcome.store.is_spell_requiring_spell_like_cpp(90, 100));
    assert!(outcome.store.is_spell_requiring_spell_like_cpp(91, 100));
    assert_eq!(outcome.store.spells_required_for_spell_like_cpp(90), &[100]);
    assert_eq!(
        outcome.store.spells_requiring_spell_like_cpp(100),
        &[90, 91]
    );
}

fn learn_skill_source(
    spell_id: u32,
    difficulty_none: bool,
    effects: Vec<SpellLearnSkillEffectLikeCpp>,
) -> SpellLearnSkillSourceSpellInfoLikeCpp {
    SpellLearnSkillSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none,
        effects,
    }
}

#[test]
fn spell_learn_skill_store_derives_skill_effect_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        100,
        true,
        vec![SpellLearnSkillEffectLikeCpp {
            effect: spell_effect_types::SPELL_EFFECT_SKILL,
            misc_value: 755,
            calc_value: 4,
        }],
    )]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(100),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(100),
        SpellLearnSkillLookupLikeCpp::Present(&SpellLearnSkillNodeLikeCpp {
            skill: 755,
            step: 4,
            value: 0,
            maxvalue: 0,
        })
    );
}

#[test]
fn spell_learn_skill_store_derives_dual_wield_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        200,
        true,
        vec![SpellLearnSkillEffectLikeCpp {
            effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
            misc_value: 0,
            calc_value: 0,
        }],
    )]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(200),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
}

#[test]
fn spell_learn_skill_store_skips_non_base_difficulty_and_breaks_after_first_match_like_cpp() {
    let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([
        learn_skill_source(
            300,
            false,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 333,
                calc_value: 3,
            }],
        ),
        learn_skill_source(
            301,
            true,
            vec![
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_NONE,
                    misc_value: 0,
                    calc_value: 0,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                    misc_value: 0,
                    calc_value: 0,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: 755,
                    calc_value: 8,
                },
            ],
        ),
    ]);

    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(outcome.store.get_spell_learn_skill_like_cpp(300).is_none());
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(300),
        SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
    assert_eq!(
        outcome.store.get_spell_learn_skill_like_cpp(301),
        Some(&SpellLearnSkillNodeLikeCpp {
            skill: SKILL_DUAL_WIELD_LIKE_CPP,
            step: 1,
            value: 1,
            maxvalue: 1,
        })
    );
}

#[test]
fn spell_learn_skill_lookup_distinguishes_covered_absence_and_indeterminate() {
    let mut outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
        500,
        true,
        Vec::new(),
    )]);
    outcome.store.mark_spell_learn_skill_indeterminate_like_cpp(
        501,
        SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
            record_id: 9,
            domain: AcquisitionValueDomainLikeCpp {
                minimum: 2,
                maximum: 4,
            },
        },
    );

    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(500),
        SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
    );
    assert!(matches!(
        outcome.store.spell_learn_skill_lookup_like_cpp(501),
        SpellLearnSkillLookupLikeCpp::Indeterminate(
            SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                record_id: 9,
                domain: AcquisitionValueDomainLikeCpp {
                    minimum: 2,
                    maximum: 4,
                },
            }
        )
    ));
    assert_eq!(
        outcome.store.spell_learn_skill_lookup_like_cpp(502),
        SpellLearnSkillLookupLikeCpp::MissingCoverage
    );
}

#[test]
fn spell_learn_skill_store_rejects_duplicate_source_ids_in_every_order() {
    let valid = || {
        learn_skill_source(
            600,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: 755,
                calc_value: 4,
            }],
        )
    };
    let invalid = || {
        learn_skill_source(
            600,
            true,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: spell_effect_types::SPELL_EFFECT_SKILL,
                misc_value: -1,
                calc_value: 4,
            }],
        )
    };

    for sources in [[valid(), invalid()], [invalid(), valid()]] {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp(sources);

        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert!(
            outcome.store.get_spell_learn_skill_like_cpp(600).is_none(),
            "the legacy getter must not leak a node from either duplicate ordering"
        );
        assert_eq!(
            outcome.store.spell_learn_skill_lookup_like_cpp(600),
            SpellLearnSkillLookupLikeCpp::Indeterminate(
                &SpellLearnSkillIndeterminateReasonLikeCpp::DuplicateSourceSpell
            )
        );
        assert!(outcome.errors.iter().any(|error| {
            error.spell_id == 600
                && error.kind == SpellLearnSkillLoadErrorKindLikeCpp::DuplicateSourceSpell
        }));
    }
}

#[test]
fn spell_learn_skill_store_rejects_out_of_range_skill_without_wrapping() {
    for misc_value in [-1, i32::from(u16::MAX) + 1] {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
            400,
            true,
            vec![
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value,
                    calc_value: 1,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                    misc_value: 0,
                    calc_value: 0,
                },
            ],
        )]);

        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert!(outcome.store.get_spell_learn_skill_like_cpp(400).is_none());
        assert!(matches!(
            outcome.store.spell_learn_skill_lookup_like_cpp(400),
            SpellLearnSkillLookupLikeCpp::Indeterminate(
                SpellLearnSkillIndeterminateReasonLikeCpp::SkillOutOfRange { value }
            ) if *value == misc_value
        ));
        assert_eq!(
            outcome.errors,
            vec![SpellLearnSkillLoadErrorLikeCpp {
                spell_id: 400,
                kind: SpellLearnSkillLoadErrorKindLikeCpp::SkillOutOfRange { value: misc_value },
            }]
        );
    }
}

#[test]
fn spell_learn_skill_store_rejects_out_of_range_step_without_wrapping() {
    for calc_value in [-1, i32::from(u16::MAX) + 1] {
        let outcome = SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp([learn_skill_source(
            401,
            true,
            vec![
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_SKILL,
                    misc_value: 755,
                    calc_value,
                },
                SpellLearnSkillEffectLikeCpp {
                    effect: spell_effect_types::SPELL_EFFECT_DUAL_WIELD,
                    misc_value: 0,
                    calc_value: 0,
                },
            ],
        )]);

        assert_eq!(outcome.dbc_loaded_row_count, 0);
        assert!(outcome.store.get_spell_learn_skill_like_cpp(401).is_none());
        assert!(matches!(
            outcome.store.spell_learn_skill_lookup_like_cpp(401),
            SpellLearnSkillLookupLikeCpp::Indeterminate(
                SpellLearnSkillIndeterminateReasonLikeCpp::StepOutOfRange { value }
            ) if *value == calc_value
        ));
        assert_eq!(
            outcome.errors,
            vec![SpellLearnSkillLoadErrorLikeCpp {
                spell_id: 401,
                kind: SpellLearnSkillLoadErrorKindLikeCpp::StepOutOfRange { value: calc_value },
            }]
        );
    }
}

#[test]
fn spell_chain_store_builds_rank_links_from_skill_line_supercedes_like_cpp() {
    let store = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [
            SpellRankEdgeLikeCpp {
                spell_id: 3,
                supercedes_spell_id: 1,
            },
            SpellRankEdgeLikeCpp {
                spell_id: 4,
                supercedes_spell_id: 3,
            },
            SpellRankEdgeLikeCpp {
                spell_id: 5,
                supercedes_spell_id: 4,
            },
            SpellRankEdgeLikeCpp {
                spell_id: 999,
                supercedes_spell_id: 998,
            },
        ],
        |spell_id| matches!(spell_id, 1 | 3 | 4 | 5),
    );

    assert_eq!(store.chains_by_spell_id.len(), 4);
    assert_eq!(
        store.spell_chain_node_like_cpp(1),
        Some(&SpellChainNodeLikeCpp {
            prev_spell_id: None,
            next_spell_id: Some(3),
            first_spell_id: 1,
            last_spell_id: 5,
            rank: 1,
        })
    );
    assert_eq!(
        store.spell_chain_node_like_cpp(4),
        Some(&SpellChainNodeLikeCpp {
            prev_spell_id: Some(3),
            next_spell_id: Some(5),
            first_spell_id: 1,
            last_spell_id: 5,
            rank: 3,
        })
    );
    assert!(store.spell_chain_node_like_cpp(999).is_none());
}

#[test]
fn spell_chain_store_derives_predecessors_after_cpp_last_wins_resolution() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 3,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 4,
                    supercedes_spell_id: 2,
                },
            ],
            |_| true,
        );

    assert!(outcome.diagnostics_in_order_like_cpp.is_empty());
    assert_eq!(
        outcome.store.spell_chain_node_like_cpp(1),
        Some(&SpellChainNodeLikeCpp {
            prev_spell_id: None,
            next_spell_id: Some(3),
            first_spell_id: 1,
            last_spell_id: 3,
            rank: 1,
        })
    );
    assert_eq!(
        outcome.store.spell_chain_node_like_cpp(2),
        Some(&SpellChainNodeLikeCpp {
            prev_spell_id: None,
            next_spell_id: Some(4),
            first_spell_id: 2,
            last_spell_id: 4,
            rank: 1,
        }),
        "the child of an eclipsed edge must remain a root in the final graph"
    );
}

#[test]
fn spell_chain_store_rejects_self_loops_and_pure_or_reachable_cycles() {
    let self_loop =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [SpellRankEdgeLikeCpp {
                spell_id: 30,
                supercedes_spell_id: 30,
            }],
            |_| true,
        );
    assert_eq!(
        self_loop.diagnostics_in_order_like_cpp,
        vec![SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id: 30 }]
    );
    assert!(matches!(
        self_loop.store.spell_chain_lookup_like_cpp(30),
        SpellChainLookupLikeCpp::Indeterminate(diagnostics)
            if diagnostics == [SpellChainLoadDiagnosticLikeCpp::SelfLoop { spell_id: 30 }]
    ));

    let pure_cycle =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 20,
                    supercedes_spell_id: 10,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 10,
                    supercedes_spell_id: 20,
                },
            ],
            |_| true,
        );
    assert_eq!(
        pure_cycle.diagnostics_in_order_like_cpp,
        vec![SpellChainLoadDiagnosticLikeCpp::Cycle {
            spell_ids: vec![10, 20],
        }]
    );
    assert!(matches!(
        pure_cycle.store.spell_chain_lookup_like_cpp(10),
        SpellChainLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        pure_cycle.store.spell_chain_lookup_like_cpp(20),
        SpellChainLookupLikeCpp::Indeterminate(_)
    ));

    let reachable_cycle =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 3,
                    supercedes_spell_id: 2,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 2,
                    supercedes_spell_id: 3,
                },
            ],
            |_| true,
        );
    assert_eq!(
        reachable_cycle.diagnostics_in_order_like_cpp,
        vec![
            SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
                spell_id: 2,
                predecessor_spell_ids: vec![1, 3],
            },
            SpellChainLoadDiagnosticLikeCpp::Cycle {
                spell_ids: vec![2, 3],
            },
        ]
    );
    for spell_id in 1..=3 {
        assert!(matches!(
            reachable_cycle.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
    }
}

#[test]
fn spell_chain_store_rejects_merge_components_without_partial_links() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            [
                SpellRankEdgeLikeCpp {
                    spell_id: 3,
                    supercedes_spell_id: 1,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 3,
                    supercedes_spell_id: 2,
                },
                SpellRankEdgeLikeCpp {
                    spell_id: 4,
                    supercedes_spell_id: 3,
                },
            ],
            |_| true,
        );

    assert_eq!(
        outcome.diagnostics_in_order_like_cpp,
        vec![SpellChainLoadDiagnosticLikeCpp::MultiplePredecessors {
            spell_id: 3,
            predecessor_spell_ids: vec![1, 2],
        }]
    );
    assert!(outcome.store.chains_by_spell_id.is_empty());
    for spell_id in 1..=4 {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
    }
    assert_eq!(
        outcome.store.spell_chain_lookup_like_cpp(99),
        SpellChainLookupLikeCpp::Unranked
    );
}

#[test]
fn spell_chain_store_propagates_invalid_effective_rows_to_the_whole_component() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 1,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 2,
                    spell_id: 3,
                    supercedes_spell_id: 2,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 90,
                    spell_raw: 2,
                    supercedes_spell_raw: i128::from(i32::MAX) + 1,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2 | 3),
        );

    assert!(outcome.store.chains_by_spell_id.is_empty());
    for spell_id in 1..=3 {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(diagnostics)
                if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    record_id: 90,
                    spell_raw: 2,
                    supercedes_spell_raw: i128::from(i32::MAX) + 1,
                    affected_spell_ids: vec![2],
                }]
        ));
    }
    assert_eq!(
        outcome.store.spell_chain_lookup_like_cpp(10),
        SpellChainLookupLikeCpp::Unranked
    );
}

#[test]
fn spell_chain_store_propagates_invalid_spell_endpoint_from_the_predecessor() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 1,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 2,
                    spell_id: 3,
                    supercedes_spell_id: 2,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 91,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: 2,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2 | 3),
        );

    assert!(outcome.store.chains_by_spell_id.is_empty());
    for spell_id in 1..=2 {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
    }
    assert_eq!(
        outcome.store.spell_chain_lookup_like_cpp(3),
        SpellChainLookupLikeCpp::Unranked,
        "the invalid final candidate eclipses the former 2→3 edge before components form"
    );
}

#[test]
fn spell_chain_store_skips_invalid_row_with_a_proven_absent_endpoint() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 1,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 92,
                    spell_raw: 999_999,
                    supercedes_spell_raw: i128::from(i32::MAX) + 1,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2),
        );

    assert!(outcome.diagnostics_in_order_like_cpp.is_empty());
    assert_eq!(outcome.store.chains_by_spell_id.len(), 2);
    assert!(matches!(
        outcome.store.spell_chain_lookup_like_cpp(1),
        SpellChainLookupLikeCpp::Node(node) if node.rank == 1
    ));
    assert!(matches!(
        outcome.store.spell_chain_lookup_like_cpp(2),
        SpellChainLookupLikeCpp::Node(node) if node.rank == 2
    ));
}

#[test]
fn spell_chain_rank_authority_is_last_wins_across_valid_and_invalid_rows() {
    let repaired =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 10,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 20,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2),
        );

    assert!(matches!(
        repaired.store.spell_chain_lookup_like_cpp(1),
        SpellChainLookupLikeCpp::Node(node)
            if node.rank == 1 && node.next_spell_id == Some(2)
    ));
    assert!(matches!(
        repaired.store.spell_chain_lookup_like_cpp(2),
        SpellChainLookupLikeCpp::Node(node)
            if node.rank == 2 && node.prev_spell_id == Some(1)
    ));
    assert_eq!(
        repaired
            .diagnostics_in_order_like_cpp
            .iter()
            .filter(|diagnostic| matches!(
                diagnostic,
                SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    record_id: 10,
                    ..
                }
            ))
            .count(),
        1,
        "an eclipsed malformed source remains observable without poisoning final authority"
    );

    let eclipsed =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 10,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 20,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: 1,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2),
        );

    assert!(matches!(
        eclipsed.store.spell_chain_lookup_like_cpp(1),
        SpellChainLookupLikeCpp::Indeterminate(diagnostics)
            if diagnostics == [SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                record_id: 20,
                spell_raw: i128::from(i32::MAX) + 1,
                supercedes_spell_raw: 1,
                affected_spell_ids: vec![1],
            }]
    ));
    assert_eq!(
        eclipsed.store.spell_chain_lookup_like_cpp(2),
        SpellChainLookupLikeCpp::Unranked,
        "the destination of an eclipsed edge must not remain in the ambiguous component"
    );
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

#[test]
fn spell_chain_store_global_seed_fails_every_lookup_closed() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            [
                SkillLineAbilityRankRowLikeCpp::Edge {
                    record_id: 1,
                    spell_id: 2,
                    supercedes_spell_id: 1,
                },
                SkillLineAbilityRankRowLikeCpp::Indeterminate {
                    record_id: 91,
                    spell_raw: i128::from(i32::MAX) + 1,
                    supercedes_spell_raw: i128::from(i32::MAX) + 2,
                },
            ],
            |spell_id| matches!(spell_id, 1 | 2),
        );

    assert!(outcome.store.chains_by_spell_id.is_empty());
    for spell_id in [1, 2, 999] {
        assert!(matches!(
            outcome.store.spell_chain_lookup_like_cpp(spell_id),
            SpellChainLookupLikeCpp::Indeterminate(_)
        ));
    }
}

#[test]
fn spell_chain_store_rejects_ranks_wider_than_cpp_uint8() {
    let outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_with_diagnostics_like_cpp(
            (1..=u32::from(u8::MAX)).map(|spell_id| SpellRankEdgeLikeCpp {
                spell_id: spell_id + 1,
                supercedes_spell_id: spell_id,
            }),
            |_| true,
        );

    assert_eq!(
        outcome.diagnostics_in_order_like_cpp,
        vec![SpellChainLoadDiagnosticLikeCpp::RankOutOfRange {
            first_spell_id: 1,
            spell_id: 256,
            rank: 256,
        }]
    );
    assert!(outcome.store.chains_by_spell_id.is_empty());
    assert!(matches!(
        outcome.store.spell_chain_lookup_like_cpp(1),
        SpellChainLookupLikeCpp::Indeterminate(_)
    ));
    assert!(matches!(
        outcome.store.spell_chain_lookup_like_cpp(256),
        SpellChainLookupLikeCpp::Indeterminate(_)
    ));
}

#[test]
fn spell_chain_store_accessors_match_cpp_fallbacks() {
    let store = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [
            SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            },
            SpellRankEdgeLikeCpp {
                spell_id: 30,
                supercedes_spell_id: 20,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20 | 30),
    );

    assert_eq!(store.first_spell_in_chain_like_cpp(30), 10);
    assert_eq!(store.last_spell_in_chain_like_cpp(10), 30);
    assert_eq!(store.next_spell_in_chain_like_cpp(10), 20);
    assert_eq!(store.prev_spell_in_chain_like_cpp(30), 20);
    assert_eq!(store.spell_rank_like_cpp(20), 2);
    assert_eq!(store.first_spell_in_chain_like_cpp(99), 99);
    assert_eq!(store.last_spell_in_chain_like_cpp(99), 99);
    assert_eq!(store.next_spell_in_chain_like_cpp(99), 0);
    assert_eq!(store.prev_spell_in_chain_like_cpp(99), 0);
    assert_eq!(store.spell_rank_like_cpp(99), 0);
    assert_eq!(store.spell_with_rank_like_cpp(10, 3, true), 30);
    assert_eq!(store.spell_with_rank_like_cpp(30, 1, true), 10);
    assert_eq!(store.spell_with_rank_like_cpp(99, 2, true), 0);
    assert_eq!(store.spell_with_rank_like_cpp(99, 2, false), 99);
}

fn spell_area_row(spell_id: u32) -> SpellAreaRowLikeCpp {
    SpellAreaRowLikeCpp {
        spell_id,
        area_id: 0,
        quest_start: 0,
        quest_start_status: 0,
        quest_end_status: 0,
        quest_end: 0,
        aura_spell: 0,
        race_mask: 0,
        gender: GENDER_NONE_LIKE_CPP,
        flags: 0,
    }
}

#[test]
fn spell_area_store_populates_primary_and_secondary_indices_like_cpp() {
    let mut row = spell_area_row(100);
    row.area_id = 10;
    row.quest_start = 20;
    row.quest_start_status = 1 << 3;
    row.quest_end = 30;
    row.quest_end_status = 1 << 6;
    row.aura_spell = -40;
    row.race_mask = 1;
    row.gender = GENDER_MALE_LIKE_CPP;
    row.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

    let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
        [row],
        |spell_id| matches!(spell_id, 40 | 100),
        |area_id| area_id == 10,
        |quest_id| matches!(quest_id, 20 | 30),
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.store.spell_area_map_bounds_like_cpp(100).len(), 1);
    assert_eq!(
        outcome
            .store
            .spell_area_for_area_map_bounds_like_cpp(10)
            .len(),
        1
    );
    assert_eq!(
        outcome
            .store
            .spell_area_for_quest_map_bounds_like_cpp(20)
            .len(),
        1
    );
    assert_eq!(
        outcome
            .store
            .spell_area_for_quest_map_bounds_like_cpp(30)
            .len(),
        1
    );
    assert_eq!(
        outcome
            .store
            .spell_area_for_quest_end_map_bounds_like_cpp(30)
            .len(),
        1
    );
    assert_eq!(
        outcome
            .store
            .spell_area_for_aura_map_bounds_like_cpp(40)
            .len(),
        1
    );
    assert_eq!(
        outcome.store.areas_like_cpp()[0],
        SpellAreaLikeCpp {
            spell_id: 100,
            area_id: 10,
            quest_start: 20,
            quest_end: 30,
            aura_spell: -40,
            race_mask: 1,
            gender: GENDER_MALE_LIKE_CPP,
            quest_start_status: 1 << 3,
            quest_end_status: 1 << 6,
            flags: SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP,
        }
    );
}

#[test]
fn spell_area_store_validates_rows_like_cpp() {
    let mut duplicate_first = spell_area_row(100);
    duplicate_first.area_id = 10;
    duplicate_first.quest_start = 20;
    duplicate_first.aura_spell = 40;
    duplicate_first.race_mask = 1;
    duplicate_first.gender = GENDER_FEMALE_LIKE_CPP;

    let duplicate_second = duplicate_first;
    let mut missing_area = spell_area_row(100);
    missing_area.area_id = 999;
    let mut missing_start_quest = spell_area_row(100);
    missing_start_quest.quest_start = 999;
    let mut missing_end_quest = spell_area_row(100);
    missing_end_quest.quest_end = 999;
    let mut missing_aura = spell_area_row(100);
    missing_aura.aura_spell = 999;
    let mut self_aura = spell_area_row(100);
    self_aura.aura_spell = 100;
    let mut invalid_race = spell_area_row(100);
    invalid_race.race_mask = 1_u64 << 62;
    let mut invalid_gender = spell_area_row(100);
    invalid_gender.gender = 3;

    let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
        [
            duplicate_first,
            duplicate_second,
            missing_area,
            missing_start_quest,
            missing_end_quest,
            missing_aura,
            self_aura,
            invalid_race,
            invalid_gender,
        ],
        |spell_id| matches!(spell_id, 40 | 100),
        |area_id| area_id == 10,
        |quest_id| matches!(quest_id, 20 | 30),
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellAreaLoadErrorKindLikeCpp::DuplicateSimilarRequirements,
            SpellAreaLoadErrorKindLikeCpp::AreaMissing,
            SpellAreaLoadErrorKindLikeCpp::QuestStartMissing,
            SpellAreaLoadErrorKindLikeCpp::QuestEndMissing,
            SpellAreaLoadErrorKindLikeCpp::AuraSpellMissing,
            SpellAreaLoadErrorKindLikeCpp::AuraSpellSelfRequirement,
            SpellAreaLoadErrorKindLikeCpp::InvalidRaceMask,
            SpellAreaLoadErrorKindLikeCpp::InvalidGender,
        ]
    );
}

#[test]
fn spell_area_store_rejects_autocast_aura_chains_like_cpp() {
    let mut aura_to_spell = spell_area_row(200);
    aura_to_spell.aura_spell = 100;
    aura_to_spell.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

    let mut spell_to_aura = spell_area_row(100);
    spell_to_aura.aura_spell = 200;
    spell_to_aura.flags = SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP;

    let outcome = SpellAreaStoreLikeCpp::from_rows_like_cpp(
        [aura_to_spell, spell_to_aura],
        |spell_id| matches!(spell_id, 100 | 200),
        |_| true,
        |_| true,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome.errors,
        vec![SpellAreaLoadErrorLikeCpp {
            row: spell_to_aura,
            kind: SpellAreaLoadErrorKindLikeCpp::AuraAutocastChain,
        }]
    );
}

fn custom_attr_source(
    spell_id: u32,
    difficulty: u32,
    effect_type: u32,
) -> SpellCustomAttributeSourceSpellInfoLikeCpp {
    SpellCustomAttributeSourceSpellInfoLikeCpp {
        spell_id,
        difficulty,
        effects: vec![SpellEffectInfo {
            effect_index: 0,
            effect: effect_type,
            ..Default::default()
        }],
    }
}

#[test]
fn spell_custom_attribute_store_applies_sql_rows_per_difficulty_like_cpp() {
    let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [
            SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            },
            SpellCustomAttributeRowLikeCpp {
                spell_id: 100,
                attributes: SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
            },
        ],
        |spell_id| {
            (spell_id == 100)
                .then(|| {
                    vec![
                        custom_attr_source(100, 0, spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE),
                        custom_attr_source(100, 1, spell_effect_types::SPELL_EFFECT_HEAL),
                    ]
                })
                .unwrap_or_default()
        },
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 2);
    assert_eq!(outcome.applied_variant_count, 4);
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 0),
        SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 1),
        SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP
    );
}

#[test]
fn spell_custom_attribute_store_validates_missing_spell_like_cpp() {
    let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [SpellCustomAttributeRowLikeCpp {
            spell_id: 999,
            attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
        }],
        |_| Vec::new(),
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(outcome.applied_variant_count, 0);
    assert_eq!(
        outcome.errors,
        vec![SpellCustomAttributeLoadErrorLikeCpp {
            spell_id: 999,
            difficulty: None,
            attributes: SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            kind: SpellCustomAttributeLoadErrorKindLikeCpp::SpellMissing,
        }]
    );
}

#[test]
fn spell_custom_attribute_store_rejects_share_damage_without_school_damage_like_cpp() {
    let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [SpellCustomAttributeRowLikeCpp {
            spell_id: 100,
            attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
        }],
        |spell_id| {
            (spell_id == 100)
                .then(|| {
                    vec![
                        custom_attr_source(100, 0, spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE),
                        custom_attr_source(100, 1, spell_effect_types::SPELL_EFFECT_HEAL),
                    ]
                })
                .unwrap_or_default()
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.applied_variant_count, 1);
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 0),
        SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP
    );
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 1),
        0
    );
    assert_eq!(
        outcome.errors,
        vec![SpellCustomAttributeLoadErrorLikeCpp {
            spell_id: 100,
            difficulty: Some(1),
            attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
            kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageWithoutSchoolDamage,
        }]
    );
}

#[test]
fn spell_custom_attribute_store_applies_non_effect_attribute_with_unknown_effect_coverage() {
    let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_for_variants_like_cpp(
        [SpellCustomAttributeRowLikeCpp {
            spell_id: 100,
            attributes: SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
        }],
        |spell_id| {
            (spell_id == 100)
                .then_some(vec![SpellCustomAttributeSourceVariantLikeCpp {
                    spell_id,
                    difficulty: 2,
                    effect_types: None,
                }])
                .unwrap_or_default()
        },
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.applied_variant_count, 1);
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 2),
        SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
    );
}

#[test]
fn spell_custom_attribute_store_rejects_share_damage_with_unknown_effect_coverage() {
    let outcome = SpellCustomAttributeStoreLikeCpp::from_sql_rows_for_variants_like_cpp(
        [SpellCustomAttributeRowLikeCpp {
            spell_id: 100,
            attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
        }],
        |spell_id| {
            (spell_id == 100)
                .then_some(vec![SpellCustomAttributeSourceVariantLikeCpp {
                    spell_id,
                    difficulty: 2,
                    effect_types: None,
                }])
                .unwrap_or_default()
        },
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.applied_variant_count, 0);
    assert_eq!(
        outcome
            .store
            .attributes_for_spell_difficulty_like_cpp(100, 2),
        0
    );
    assert_eq!(
        outcome.errors,
        vec![SpellCustomAttributeLoadErrorLikeCpp {
            spell_id: 100,
            difficulty: Some(2),
            attributes: SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
            kind: SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
        }]
    );
}

#[test]
fn spell_custom_attribute_store_queries_attributes_across_exact_difficulties() {
    let store = SpellCustomAttributeStoreLikeCpp {
        attributes_by_spell_and_difficulty: BTreeMap::from([
            (
                SpellCustomAttributeKeyLikeCpp {
                    spell_id: 100,
                    difficulty: 0,
                },
                SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP,
            ),
            (
                SpellCustomAttributeKeyLikeCpp {
                    spell_id: 100,
                    difficulty: 2,
                },
                SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
            ),
            (
                SpellCustomAttributeKeyLikeCpp {
                    spell_id: 101,
                    difficulty: 0,
                },
                SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP,
            ),
        ]),
    };

    assert_eq!(
        store.attributes_for_spell_any_difficulty_like_cpp(100),
        SPELL_ATTR0_CU_CAN_CRIT_LIKE_CPP | SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
    );
    assert!(store.has_attribute_any_difficulty_like_cpp(100, SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP));
    assert!(
        !store.has_attribute_any_difficulty_like_cpp(100, SPELL_ATTR0_CU_DIRECT_DAMAGE_LIKE_CPP)
    );
    assert_eq!(store.attributes_for_spell_any_difficulty_like_cpp(999), 0);
}

#[test]
fn spell_group_store_validates_rows_like_cpp() {
    let outcome = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            SpellGroupRowLikeCpp {
                group_id: 5,
                spell_id: 10,
            },
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 11,
            },
            SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 12,
            },
            SpellGroupRowLikeCpp {
                group_id: 1003,
                spell_id: -1999,
            },
        ],
        |spell_id| matches!(spell_id, 12),
        |spell_id| {
            if spell_id == 12 { 2 } else { 1 }
        },
    );

    assert_eq!(outcome.loaded_row_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellGroupLoadErrorKindLikeCpp::CoreRangeGroupMissing,
            SpellGroupLoadErrorKindLikeCpp::SpellMissing,
            SpellGroupLoadErrorKindLikeCpp::SpellNotFirstRank,
            SpellGroupLoadErrorKindLikeCpp::ReferencedGroupMissing,
        ]
    );
}

#[test]
fn spell_group_store_expands_nested_groups_like_cpp() {
    let outcome = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            },
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: -1002,
            },
            SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 20,
            },
            SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 20,
            },
            SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: -1001,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20),
        |_| 1,
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.spell_group_spell_map_bounds_like_cpp(1001),
        &[10, -1002]
    );
    assert_eq!(
        outcome.store.set_of_spells_in_spell_group_like_cpp(1001),
        BTreeSet::from([10, 20])
    );
    assert_eq!(
        outcome.store.set_of_spells_in_spell_group_like_cpp(1002),
        BTreeSet::from([10, 20])
    );
    assert!(
        outcome
            .store
            .is_spell_member_of_spell_group_like_cpp(20, 1001, |spell_id| spell_id)
    );
    assert_eq!(
        outcome
            .store
            .spell_spell_group_map_bounds_like_cpp(25, |_| 20),
        &[1001, 1002],
        "C++ GetSpellSpellGroupMapBounds first normalizes to GetFirstSpellInChain"
    );
}

#[test]
fn spell_group_stack_rule_store_validates_rows_like_cpp() {
    let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [SpellGroupRowLikeCpp {
            group_id: 1001,
            spell_id: 10,
        }],
        |spell_id| spell_id == 10,
        |_| 1,
    )
    .store;

    let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
        [
            SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: SpellGroupStackRuleLikeCpp::MAX_LIKE_CPP,
            },
            SpellGroupStackRuleRowLikeCpp {
                group_id: 1999,
                stack_rule: SpellGroupStackRuleLikeCpp::Exclusive as u8,
            },
            SpellGroupStackRuleRowLikeCpp {
                group_id: 1001,
                stack_rule: SpellGroupStackRuleLikeCpp::Exclusive as u8,
            },
        ],
        &spell_groups,
        |_| None,
        |_| None,
    );

    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellGroupStackRuleLoadErrorKindLikeCpp::StackRuleMissing,
            SpellGroupStackRuleLoadErrorKindLikeCpp::GroupMissing,
        ]
    );
    assert_eq!(
        outcome.store.spell_group_stack_rule_like_cpp(1001),
        SpellGroupStackRuleLikeCpp::Exclusive
    );
    assert_eq!(
        outcome.store.spell_group_stack_rule_like_cpp(1999),
        SpellGroupStackRuleLikeCpp::Default
    );
}

#[test]
fn spell_group_stack_rule_store_infers_same_effect_aura_group_like_cpp() {
    let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            },
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 20,
            },
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 30,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20 | 30),
        |_| 1,
    )
    .store;
    let spells = BTreeMap::from([
        (
            10,
            test_spell_info_with_aura(10, aura_types::SPELL_AURA_MOD_MELEE_HASTE),
        ),
        (
            20,
            test_spell_info_with_aura(20, aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE),
        ),
        (30, test_spell_info_without_aura(30)),
        (
            31,
            test_spell_info_with_aura(31, aura_types::SPELL_AURA_MOD_RANGED_HASTE),
        ),
    ]);

    let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
        [SpellGroupStackRuleRowLikeCpp {
            group_id: 1001,
            stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveSameEffect as u8,
        }],
        &spell_groups,
        |spell_id| spells.get(&spell_id).cloned(),
        |spell_id| if spell_id == 30 { Some(31) } else { None },
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.same_effect_parsed_count, 1);
    assert_eq!(
        outcome
            .store
            .same_effect_stack_rule_aura_types_like_cpp(1001),
        Some(&BTreeSet::from([
            aura_types::SPELL_AURA_MOD_MELEE_HASTE,
            aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE,
            aura_types::SPELL_AURA_MOD_RANGED_HASTE,
        ])),
        "C++ collapses the melee/ranged haste subgroup to its first aura before expanding it back"
    );
}

#[test]
fn spell_group_stack_rule_store_checks_common_group_rules_like_cpp() {
    let spell_groups = SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 10,
            },
            SpellGroupRowLikeCpp {
                group_id: 1001,
                spell_id: 20,
            },
            SpellGroupRowLikeCpp {
                group_id: 1002,
                spell_id: 30,
            },
        ],
        |spell_id| matches!(spell_id, 10 | 20 | 30),
        |_| 1,
    )
    .store;

    let outcome = SpellGroupStackRuleStoreLikeCpp::from_rows_like_cpp(
        [SpellGroupStackRuleRowLikeCpp {
            group_id: 1001,
            stack_rule: SpellGroupStackRuleLikeCpp::ExclusiveHighest as u8,
        }],
        &spell_groups,
        |_| None,
        |_| None,
    );

    assert_eq!(
        outcome
            .store
            .check_spell_group_stack_rules_like_cpp(&spell_groups, 10, 20),
        SpellGroupStackRuleLikeCpp::ExclusiveHighest
    );
    assert_eq!(
        outcome
            .store
            .check_spell_group_stack_rules_like_cpp(&spell_groups, 10, 30),
        SpellGroupStackRuleLikeCpp::Default
    );
}

#[test]
fn spell_proc_store_expands_negative_spell_id_to_all_ranks_like_cpp() {
    let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: -100,
            proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
            chance: 25.0,
            ..test_spell_proc_row_like_cpp(100)
        }],
        |spell_id| {
            Some(match spell_id {
                100 => test_spell_proc_source_like_cpp(100, 100, Some(101)),
                101 => test_spell_proc_source_like_cpp(101, 100, None),
                _ => return None,
            })
        },
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(100, 0)
            .map(|entry| entry.chance),
        Some(25.0)
    );
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(101, 0)
            .map(|entry| entry.proc_flags),
        Some([PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0])
    );
}

#[test]
fn spell_proc_store_applies_spellinfo_defaults_like_cpp() {
    let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 200,
            ..test_spell_proc_row_like_cpp(200)
        }],
        |spell_id| {
            let mut source = test_spell_proc_source_like_cpp(spell_id, spell_id, None);
            source.proc_flags = [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0];
            source.proc_charges = 3;
            source.proc_chance = 12.5;
            source.proc_cooldown_ms = 1500;
            Some(source)
        },
    );

    let entry = outcome.store.spell_proc_entry_like_cpp(200, 0).unwrap();
    assert_eq!(entry.proc_flags, [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0]);
    assert_eq!(entry.charges, 3);
    assert_eq!(entry.chance, 12.5);
    assert_eq!(entry.cooldown_ms, 1500);
}

#[test]
fn spell_proc_store_validates_and_sanitizes_like_cpp() {
    let outcome = SpellProcStoreLikeCpp::from_rows_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 300,
            school_mask: 0x80,
            proc_flags: [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP],
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP << 1,
            spell_phase_mask: PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP << 1,
            hit_mask: PROC_HIT_MASK_ALL_LIKE_CPP << 1,
            attributes_mask: PROC_ATTR_ALL_ALLOWED_LIKE_CPP | 0x0000_0100,
            disable_effects_mask: 0x1,
            procs_per_minute: -1.0,
            chance: -1.0,
            ..test_spell_proc_row_like_cpp(300)
        }],
        |spell_id| {
            let mut source = test_spell_proc_source_like_cpp(spell_id, spell_id, None);
            source.effects = vec![SpellEffectInfo {
                effect_index: 0,
                effect: spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                effect_aura: 0,
                ..SpellEffectInfo::default()
            }];
            Some(source)
        },
    );

    let entry = outcome.store.spell_proc_entry_like_cpp(300, 0).unwrap();
    assert_eq!(entry.chance, 0.0);
    assert_eq!(entry.procs_per_minute, 0.0);
    assert_eq!(entry.attributes_mask, PROC_ATTR_ALL_ALLOWED_LIKE_CPP);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellProcLoadErrorKindLikeCpp::InvalidSchoolMask,
            SpellProcLoadErrorKindLikeCpp::NegativeChance,
            SpellProcLoadErrorKindLikeCpp::NegativeProcsPerMinute,
            SpellProcLoadErrorKindLikeCpp::InvalidSpellTypeMask,
            SpellProcLoadErrorKindLikeCpp::SpellTypeMaskUnused,
            SpellProcLoadErrorKindLikeCpp::InvalidSpellPhaseMask,
            SpellProcLoadErrorKindLikeCpp::SpellPhaseMaskUnused,
            SpellProcLoadErrorKindLikeCpp::InvalidHitMask,
            SpellProcLoadErrorKindLikeCpp::HitMaskUnused,
            SpellProcLoadErrorKindLikeCpp::DisabledEffectIsNotAura,
            SpellProcLoadErrorKindLikeCpp::ReqSpellmodWithoutSpellmodAura,
            SpellProcLoadErrorKindLikeCpp::InvalidAttributesMask,
        ]
    );
}

#[test]
fn spell_proc_store_lookup_uses_exact_difficulty_before_fallback_like_cpp() {
    let store = test_spell_proc_store_with_entries_like_cpp([
        (400, 1, [PROC_FLAG_DEATH_LIKE_CPP, 0]),
        (400, 2, [PROC_FLAG_KILL_LIKE_CPP, 0]),
    ]);

    let entry = store
        .spell_proc_entry_with_fallback_like_cpp(400, 2, |_| Some(1))
        .unwrap();

    assert_eq!(entry.proc_flags, [PROC_FLAG_KILL_LIKE_CPP, 0]);
}

#[test]
fn spell_proc_store_lookup_walks_difficulty_fallback_chain_like_cpp() {
    let store =
        test_spell_proc_store_with_entries_like_cpp([(500, 1, [PROC_FLAG_DEATH_LIKE_CPP, 0])]);

    let entry = store
        .spell_proc_entry_with_fallback_like_cpp(500, 3, |difficulty| match difficulty {
            3 => Some(2),
            2 => Some(1),
            _ => None,
        })
        .unwrap();

    assert_eq!(entry.proc_flags, [PROC_FLAG_DEATH_LIKE_CPP, 0]);
    assert!(
        store
            .spell_proc_entry_with_fallback_like_cpp(500, 3, |_| None)
            .is_none(),
        "C++ stops when sDifficultyStore.LookupEntry returns null"
    );
}

#[test]
fn spell_proc_store_generates_implicit_entries_after_sql_like_cpp() {
    let mut implicit = test_implicit_spell_proc_source_like_cpp();
    implicit.spell_id = 601;
    implicit.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    implicit.proc_chance = 35.0;
    implicit.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        [0, 0, 0, 0],
    )];

    let outcome = SpellProcStoreLikeCpp::from_rows_and_implicit_sources_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 600,
            proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
            chance: 10.0,
            ..test_spell_proc_row_like_cpp(600)
        }],
        |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
        [implicit],
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.generated_entry_count, 1);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(600, 0)
            .map(|entry| (entry.proc_flags, entry.chance)),
        Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 10.0))
    );
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(601, 0)
            .map(|entry| (entry.proc_flags, entry.chance)),
        Some(([PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0], 35.0))
    );
}

#[test]
fn spell_proc_store_explicit_sql_suppresses_same_key_implicit_like_cpp() {
    let mut duplicate_implicit = test_implicit_spell_proc_source_like_cpp();
    duplicate_implicit.spell_id = 700;
    duplicate_implicit.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    duplicate_implicit.proc_chance = 90.0;
    duplicate_implicit.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        [0, 0, 0, 0],
    )];

    let mut invalid_implicit = duplicate_implicit.clone();
    invalid_implicit.spell_id = 701;
    invalid_implicit.proc_flags = [0, 0];

    let outcome = SpellProcStoreLikeCpp::from_rows_and_implicit_sources_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 700,
            proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
            chance: 11.0,
            ..test_spell_proc_row_like_cpp(700)
        }],
        |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
        [duplicate_implicit, invalid_implicit],
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.generated_entry_count, 0);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(700, 0)
            .map(|entry| (entry.proc_flags, entry.chance)),
        Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 11.0))
    );
    assert!(outcome.store.spell_proc_entry_like_cpp(701, 0).is_none());
}

#[test]
fn spell_proc_source_builds_implicit_source_from_spell_effects_like_cpp() {
    let mut source = test_spell_proc_source_like_cpp(800, 800, None);
    source.spell_family_name = 42;
    source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    source.proc_chance = 30.0;
    source.proc_cooldown_ms = 500;
    source.proc_charges = 2;
    source.proc_base_ppm = 1.5;
    source.attributes3 = attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS;
    source.effects = vec![SpellEffectInfo {
        effect_index: 1,
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        effect_base_points: -100,
        effect_spell_class_mask: [1, 2, 3, 4],
        effect_trigger_spell: 900,
        ..SpellEffectInfo::default()
    }];

    let implicit = source.implicit_proc_source_like_cpp();

    assert_eq!(implicit.spell_id, 800);
    assert_eq!(implicit.difficulty, 0);
    assert_eq!(implicit.spell_family_name, 42);
    assert_eq!(implicit.proc_flags, source.proc_flags);
    assert_eq!(implicit.proc_chance, 30.0);
    assert_eq!(implicit.proc_cooldown_ms, 500);
    assert_eq!(implicit.proc_charges, 2);
    assert_eq!(implicit.proc_base_ppm, 1.5);
    assert_eq!(
        implicit.attributes3,
        attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS
    );
    assert_eq!(implicit.effects.len(), 1);
    assert_eq!(implicit.effects[0].effect_index, 1);
    assert!(implicit.effects[0].is_effect);
    assert!(implicit.effects[0].is_aura);
    assert_eq!(
        implicit.effects[0].aura_type,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
    );
    assert_eq!(implicit.effects[0].spell_class_mask, [1, 2, 3, 4]);
    assert_eq!(implicit.effects[0].calc_value, -100);
    assert_eq!(implicit.effects[0].trigger_spell, 900);
}

#[test]
fn spell_proc_source_builds_from_loaded_spell_and_db2_stores_like_cpp() {
    let mut spells = SpellStore::new();
    spells.insert(
        100,
        SpellInfo {
            spell_id: 100,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(aura_types::SPELL_AURA_PROC_TRIGGER_SPELL),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![SpellEffectInfo {
                effect_index: 0,
                effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
                effect_spell_class_mask: [10, 20, 30, 40],
                ..Default::default()
            }],
        },
    );
    spells.insert(101, test_spell_info_without_aura(101));

    let chains = SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
        [SpellRankEdgeLikeCpp {
            spell_id: 101,
            supercedes_spell_id: 100,
        }],
        |spell_id| spells.get(spell_id as i32).is_some(),
    );
    let aura_options = crate::spell_db2::SpellAuraOptionsStore::from_entries([
        test_spell_aura_options_entry_like_cpp(1, 100, 0, [1, 0], 10, 2, 300, 9),
        test_spell_aura_options_entry_like_cpp(2, 100, 1, [-1, 7], 35, -2, -300, 42),
    ]);
    let misc = crate::spell_db2::SpellMiscStore::from_entries([
        test_spell_misc_entry_like_cpp(1, 100, 0, 0x0100),
        test_spell_misc_entry_like_cpp(2, 100, 1, attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS),
    ]);
    let class_options = crate::spell_db2::SpellClassOptionsStore::from_entries([
        crate::spell_db2::SpellClassOptionsEntry {
            id: 1,
            spell_id: 100,
            modal_next_spell: 0,
            spell_class_set: 8,
            spell_class_mask: [10, 20, 30, 40],
        },
    ]);
    let ppm = crate::spell_db2::SpellProcsPerMinuteStore::from_entries([
        crate::spell_db2::SpellProcsPerMinuteEntry {
            id: 42,
            base_proc_rate: 1.75,
            flags: 0,
        },
    ]);

    let source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        100,
        1,
        &spells,
        &chains,
        &aura_options,
        &misc,
        &class_options,
        &ppm,
    )
    .unwrap();

    assert_eq!(source.spell_id, 100);
    assert_eq!(source.difficulty, 1);
    assert_eq!(source.first_rank_spell_id, 100);
    assert_eq!(source.next_rank_spell_id, Some(101));
    assert_eq!(source.spell_family_name, 8);
    assert_eq!(source.proc_flags, [u32::MAX, 7]);
    assert_eq!(source.proc_chance, 35.0);
    assert_eq!(source.proc_charges, u32::MAX - 1);
    assert_eq!(source.proc_cooldown_ms, (-300_i32) as u32);
    assert_eq!(source.proc_base_ppm, 1.75);
    assert_eq!(
        source.attributes3,
        attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS
    );
    assert_eq!(source.effects.len(), 1);
    assert_eq!(source.effects[0].effect_spell_class_mask, [10, 20, 30, 40]);

    let fallback_source = SpellProcSourceSpellInfoLikeCpp::from_loaded_spell_like_cpp(
        100,
        2,
        &spells,
        &chains,
        &aura_options,
        &misc,
        &class_options,
        &ppm,
    )
    .unwrap();
    assert_eq!(fallback_source.proc_flags, [1, 0]);
    assert_eq!(fallback_source.attributes3, 0x0100);
}

#[test]
fn spell_proc_store_generates_from_spell_infos_after_sql_like_cpp() {
    let mut generated = test_spell_proc_source_like_cpp(901, 901, None);
    generated.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    generated.proc_chance = 45.0;
    generated.effects = vec![SpellEffectInfo {
        effect_index: 0,
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        ..SpellEffectInfo::default()
    }];

    let mut explicit_duplicate = generated.clone();
    explicit_duplicate.spell_id = 900;
    explicit_duplicate.proc_chance = 95.0;

    let outcome = SpellProcStoreLikeCpp::from_rows_and_spell_infos_like_cpp(
        [SpellProcRowLikeCpp {
            spell_id: 900,
            proc_flags: [PROC_FLAG_KILL_LIKE_CPP, 0],
            chance: 12.0,
            ..test_spell_proc_row_like_cpp(900)
        }],
        |spell_id| Some(test_spell_proc_source_like_cpp(spell_id, spell_id, None)),
        [explicit_duplicate, generated],
    );

    assert!(outcome.errors.is_empty());
    assert_eq!(outcome.loaded_row_count, 1);
    assert_eq!(outcome.generated_entry_count, 1);
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(900, 0)
            .map(|entry| (entry.proc_flags, entry.chance)),
        Some(([PROC_FLAG_KILL_LIKE_CPP, 0], 12.0))
    );
    assert_eq!(
        outcome
            .store
            .spell_proc_entry_like_cpp(901, 0)
            .map(|entry| (entry.proc_flags, entry.chance)),
        Some(([PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0], 45.0))
    );
}

#[test]
fn can_spell_trigger_proc_on_event_requires_proc_flag_overlap_like_cpp() {
    let mut entry = test_spell_proc_entry_like_cpp();
    entry.proc_flags = [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP];
    let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP);

    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.type_mask = [0, PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP];
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
}

#[test]
fn can_spell_trigger_proc_on_event_checks_xp_honor_and_power_attrs_like_cpp() {
    let mut entry = test_spell_proc_entry_like_cpp();
    entry.proc_flags = [PROC_FLAG_KILL_LIKE_CPP, 0];
    entry.attributes_mask = PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP;
    let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_KILL_LIKE_CPP);
    event.actor_is_player = true;
    event.action_target_exists = true;
    event.action_target_is_honor_or_xp = false;

    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.action_target_is_honor_or_xp = true;
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    entry.attributes_mask = PROC_ATTR_REQ_POWER_COST_LIKE_CPP;
    event.proc_spell_has_positive_power_cost = None;
    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.proc_spell_has_positive_power_cost = Some(false);
    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.proc_spell_has_positive_power_cost = Some(true);
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
}

#[test]
fn can_spell_trigger_proc_on_event_heartbeat_bypasses_later_masks_like_cpp() {
    let mut entry = test_spell_proc_entry_like_cpp();
    entry.proc_flags = [PROC_FLAG_HEARTBEAT_LIKE_CPP, 0];
    entry.school_mask = 0x04;
    entry.spell_family_name = 7;
    entry.spell_family_mask = [0x10, 0, 0, 0];
    entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
    entry.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
    let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_HEARTBEAT_LIKE_CPP);
    event.school_mask = 0x01;
    event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
        spell_family_name: 8,
        spell_family_mask: [0, 0, 0, 0],
    });
    event.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    event.hit_mask = PROC_HIT_NORMAL_LIKE_CPP;

    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
}

#[test]
fn can_spell_trigger_proc_on_event_matches_school_family_and_type_like_cpp() {
    let mut entry = test_spell_proc_entry_like_cpp();
    entry.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    entry.school_mask = 0x04;
    entry.spell_family_name = 11;
    entry.spell_family_mask = [0x20, 0, 0, 0];
    entry.spell_type_mask = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP;
    entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
    let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP);
    event.school_mask = 0x01;
    event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
        spell_family_name: 11,
        spell_family_mask: [0x20, 0, 0, 0],
    });
    event.spell_type_mask = PROC_SPELL_TYPE_DAMAGE_LIKE_CPP;
    event.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
    event.hit_mask = PROC_HIT_NORMAL_LIKE_CPP;

    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.school_mask = 0x04;
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.spell_info = Some(SpellProcEventSpellInfoLikeCpp {
        spell_family_name: 12,
        spell_family_mask: [0x20, 0, 0, 0],
    });
    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.spell_info = None;
    assert!(
        can_spell_trigger_proc_on_event_like_cpp(&entry, &event),
        "C++ only checks SpellInfo::IsAffected when eventInfo.GetSpellInfo() exists"
    );

    event.spell_type_mask = PROC_SPELL_TYPE_HEAL_LIKE_CPP;
    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));
}

#[test]
fn can_spell_trigger_proc_on_event_matches_phase_and_hit_defaults_like_cpp() {
    let mut entry = test_spell_proc_entry_like_cpp();
    entry.proc_flags = [PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP, 0];
    entry.spell_phase_mask = PROC_SPELL_PHASE_HIT_LIKE_CPP;
    entry.hit_mask = 0;
    let mut event = test_spell_proc_event_like_cpp(PROC_FLAG_TAKE_MELEE_SWING_LIKE_CPP);
    event.spell_phase_mask = 0;
    event.hit_mask = PROC_HIT_ABSORB_LIKE_CPP;

    assert!(!can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    entry.proc_flags = [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0];
    event.type_mask = [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0];
    event.hit_mask = PROC_HIT_ABSORB_LIKE_CPP;
    assert!(can_spell_trigger_proc_on_event_like_cpp(&entry, &event));

    event.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    event.hit_mask = 0;
    assert!(
        can_spell_trigger_proc_on_event_like_cpp(&entry, &event),
        "C++ skips done-hit HitMask checks during PROC_SPELL_PHASE_CAST"
    );
}

#[test]
fn spell_proc_event_spell_info_is_affected_matches_cpp_zero_family_name() {
    let event_spell = SpellProcEventSpellInfoLikeCpp {
        spell_family_name: 3,
        spell_family_mask: [0, 0, 0, 0],
    };

    assert!(event_spell.is_affected_like_cpp(0, [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]));
}

#[test]
fn implicit_proc_aura_info_matches_cpp_trigger_table() {
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_DUMMY),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
            triggered_can_proc: false,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_SCHOOL_ABSORB),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_STEALTH),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOD_CONFUSE),
        Some(ImplicitProcAuraInfoLikeCpp {
            spell_type_mask: PROC_SPELL_TYPE_DAMAGE_LIKE_CPP,
            triggered_can_proc: true,
        })
    );
    assert_eq!(
        implicit_proc_aura_info_like_cpp(aura_types::SPELL_AURA_MOUNTED),
        None
    );
}

#[test]
fn implicit_spell_proc_entry_matches_cpp_default_generation() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [
        PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP | PROC_FLAG_KILL_LIKE_CPP,
        0,
    ];
    source.spell_family_name = 42;
    source.proc_chance = 25.0;
    source.proc_cooldown_ms = 1500;
    source.proc_charges = 3;
    source.effects = vec![
        test_implicit_proc_effect_like_cpp(
            0,
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
            [0x10, 0, 0, 0],
        ),
        test_implicit_proc_effect_like_cpp(1, aura_types::SPELL_AURA_MOUNTED, [0, 0, 0, 0]),
    ];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

    assert_eq!(entry.proc_flags, source.proc_flags);
    assert_eq!(entry.spell_family_name, 42);
    assert_eq!(entry.spell_family_mask, [0x10, 0, 0, 0]);
    assert_eq!(entry.spell_type_mask, PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP);
    assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_HIT_LIKE_CPP);
    assert_eq!(entry.disable_effects_mask, 1 << 1);
    assert_eq!(entry.attributes_mask, PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP);
    assert_eq!(entry.chance, 25.0);
    assert_eq!(entry.cooldown_ms, 1500);
    assert_eq!(entry.charges, 3);
}

#[test]
fn implicit_spell_proc_entry_sets_special_phase_and_hit_masks_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [
        PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP,
        PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP,
    ];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
        [0, 0, 0, 0],
    )];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();

    assert_eq!(entry.spell_phase_mask, PROC_SPELL_PHASE_CAST_LIKE_CPP);
    assert_eq!(entry.hit_mask, PROC_HIT_BLOCK_LIKE_CPP);

    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_REFLECT_SPELLS,
        [0, 0, 0, 0],
    )];
    assert_eq!(
        implicit_spell_proc_entry_like_cpp(&source)
            .unwrap()
            .hit_mask,
        PROC_HIT_REFLECT_LIKE_CPP
    );

    source.effects = vec![test_implicit_proc_effect_with_calc_like_cpp(
        0,
        aura_types::SPELL_AURA_MOD_HIT_CHANCE,
        -100,
    )];
    assert_eq!(
        implicit_spell_proc_entry_like_cpp(&source)
            .unwrap()
            .hit_mask,
        PROC_HIT_MISS_LIKE_CPP
    );
}

#[test]
fn implicit_spell_proc_entry_applies_taken_trigger_attr_and_skips_invalid_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [PROC_FLAG_TAKE_HARMFUL_SPELL_LIKE_CPP, 0];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE,
        [0, 0, 0, 0],
    )];

    let entry = implicit_spell_proc_entry_like_cpp(&source).unwrap();
    assert_eq!(entry.attributes_mask, PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP);

    source.proc_flags = [0, 0];
    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());

    source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    source.effects = vec![test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_MOUNTED,
        [0, 0, 0, 0],
    )];
    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
}

#[test]
fn implicit_spell_proc_entry_rejects_can_proc_from_procs_loop_like_cpp() {
    let mut source = test_implicit_spell_proc_source_like_cpp();
    source.proc_flags = [PROC_FLAG_DEAL_HARMFUL_SPELL_LIKE_CPP, 0];
    source.proc_chance = 100.0;
    source.attributes3 = attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS;
    let mut effect = test_implicit_proc_effect_like_cpp(
        0,
        aura_types::SPELL_AURA_PROC_TRIGGER_SPELL,
        [0, 0, 0, 0],
    );
    effect.trigger_spell = 123;
    source.effects = vec![effect];

    assert!(implicit_spell_proc_entry_like_cpp(&source).is_none());
}

fn learn_source(
    spell_id: u32,
    is_talent: bool,
    is_passive: bool,
    has_skill_step_effect: bool,
    learn_spell_effects: Vec<SpellLearnSpellEffectLikeCpp>,
) -> SpellLearnSourceSpellInfoLikeCpp {
    SpellLearnSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        is_talent,
        is_passive,
        has_skill_step_effect,
        learn_spell_effects,
    }
}

fn test_spell_info_with_aura(spell_id: i32, aura_type: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![SpellEffectInfo {
            effect_index: 0,
            effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            ..SpellEffectInfo::default()
        }],
    }
}

fn test_spell_info_without_aura(spell_id: i32) -> SpellInfo {
    SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: spell_effect_types::SPELL_EFFECT_NONE,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

fn test_spell_proc_entry_like_cpp() -> SpellProcEntryLikeCpp {
    SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0, 0, 0, 0],
        proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
        spell_type_mask: 0,
        spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance: 0.0,
        cooldown_ms: 0,
        charges: 0,
    }
}

fn test_spell_proc_event_like_cpp(type_mask: u32) -> SpellProcEventInfoLikeCpp {
    SpellProcEventInfoLikeCpp {
        type_mask: [type_mask, 0],
        actor_is_player: false,
        action_target_exists: false,
        action_target_is_honor_or_xp: false,
        proc_spell_has_positive_power_cost: None,
        school_mask: SPELL_SCHOOL_MASK_ALL_LIKE_CPP,
        spell_info: None,
        spell_type_mask: PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
        spell_phase_mask: PROC_SPELL_PHASE_CAST_LIKE_CPP,
        hit_mask: PROC_HIT_NORMAL_LIKE_CPP,
    }
}

fn test_spell_proc_store_with_entries_like_cpp(
    entries: impl IntoIterator<Item = (u32, u32, [u32; 2])>,
) -> SpellProcStoreLikeCpp {
    let mut store = SpellProcStoreLikeCpp::default();
    for (spell_id, difficulty, proc_flags) in entries {
        let mut entry = test_spell_proc_entry_like_cpp();
        entry.proc_flags = proc_flags;
        store.proc_entries_by_spell_and_difficulty.insert(
            SpellProcKeyLikeCpp {
                spell_id,
                difficulty,
            },
            entry,
        );
    }
    store
}

fn test_implicit_spell_proc_source_like_cpp() -> ImplicitSpellProcSourceLikeCpp {
    ImplicitSpellProcSourceLikeCpp {
        spell_id: 1000,
        difficulty: 0,
        spell_family_name: 0,
        proc_flags: [PROC_FLAG_DEAL_MELEE_SWING_LIKE_CPP, 0],
        proc_chance: 0.0,
        proc_cooldown_ms: 0,
        proc_charges: 0,
        proc_base_ppm: 0.0,
        attributes3: 0,
        effects: Vec::new(),
    }
}

fn test_implicit_proc_effect_like_cpp(
    effect_index: u32,
    aura_type: i32,
    spell_class_mask: [u32; 4],
) -> ImplicitSpellProcEffectLikeCpp {
    ImplicitSpellProcEffectLikeCpp {
        effect_index,
        is_effect: true,
        is_aura: true,
        aura_type,
        spell_class_mask,
        calc_value: 0,
        trigger_spell: 0,
    }
}

fn test_implicit_proc_effect_with_calc_like_cpp(
    effect_index: u32,
    aura_type: i32,
    calc_value: i32,
) -> ImplicitSpellProcEffectLikeCpp {
    let mut effect = test_implicit_proc_effect_like_cpp(effect_index, aura_type, [0, 0, 0, 0]);
    effect.calc_value = calc_value;
    effect
}

fn test_spell_aura_options_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    proc_type_mask: [i32; 2],
    proc_chance: u8,
    proc_charges: i32,
    proc_category_recovery: i32,
    spell_procs_per_minute_id: u16,
) -> crate::spell_db2::SpellAuraOptionsEntry {
    crate::spell_db2::SpellAuraOptionsEntry {
        id,
        difficulty_id,
        cumulative_aura: 0,
        proc_category_recovery,
        proc_chance,
        proc_charges,
        spell_procs_per_minute_id,
        proc_type_mask,
        spell_id,
    }
}

fn test_spell_misc_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    attributes3: u32,
) -> crate::spell_db2::SpellMiscEntry {
    let mut attributes = [0; 15];
    attributes[3] = attributes3 as i32;
    crate::spell_db2::SpellMiscEntry {
        id,
        attributes,
        difficulty_id,
        casting_time_index: 0,
        duration_index: 0,
        range_index: 0,
        school_mask: 0,
        speed: 0.0,
        launch_delay: 0.0,
        min_duration: 0.0,
        spell_icon_file_data_id: 0,
        active_icon_file_data_id: 0,
        content_tuning_id: 0,
        show_future_spell_player_condition_id: 0,
        spell_id,
    }
}

fn test_spell_effect_db2_entry_like_cpp(
    id: u32,
    spell_id: u32,
    difficulty_id: i32,
    effect_index: i32,
    effect: u32,
    effect_mechanic: i32,
) -> crate::spell_db2::SpellEffectDb2Entry {
    crate::spell_db2::SpellEffectDb2Entry {
        id,
        difficulty_id,
        effect_index,
        effect,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: 0,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id,
    }
}

fn test_spell_proc_row_like_cpp(spell_id: i32) -> SpellProcRowLikeCpp {
    SpellProcRowLikeCpp {
        spell_id,
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0; 4],
        proc_flags: [0; 2],
        spell_type_mask: 0,
        spell_phase_mask: 0,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: 0,
        procs_per_minute: 0.0,
        chance: 0.0,
        cooldown_ms: 0,
        charges: 0,
    }
}

fn test_spell_proc_source_like_cpp(
    spell_id: u32,
    first_rank_spell_id: u32,
    next_rank_spell_id: Option<u32>,
) -> SpellProcSourceSpellInfoLikeCpp {
    SpellProcSourceSpellInfoLikeCpp {
        spell_id,
        difficulty: 0,
        first_rank_spell_id,
        next_rank_spell_id,
        spell_family_name: 0,
        proc_flags: [0; 2],
        proc_charges: 0,
        proc_chance: 0.0,
        proc_cooldown_ms: 0,
        proc_base_ppm: 0.0,
        attributes3: 0,
        effects: Vec::new(),
    }
}

#[test]
fn spell_learn_spell_store_validates_sql_rows_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [
            SpellLearnSpellSqlRowLikeCpp {
                entry: 10,
                spell_id: 20,
                active: false,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 11,
                spell_id: 21,
                active: true,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 12,
                spell_id: 22,
                active: true,
            },
            SpellLearnSpellSqlRowLikeCpp {
                entry: 13,
                spell_id: 23,
                active: true,
            },
        ],
        [],
        [],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            12 => Some(learn_source(12, false, false, false, Vec::new())),
            13 => Some(learn_source(13, true, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 20 | 23),
    );

    assert!(!outcome.sql_result_empty);
    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(outcome.dbc_loaded_row_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceSpellMissing,
            SpellLearnSpellLoadErrorKindLikeCpp::SqlLearnedSpellMissing,
            SpellLearnSpellLoadErrorKindLikeCpp::SqlSourceIsTalent,
        ]
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(10),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 20,
            overrides_spell: 0,
            active: false,
            auto_learned: false,
        }]
    );
    assert!(outcome.store.is_spell_learn_spell_like_cpp(10));
    assert!(outcome.store.is_spell_learn_to_spell_like_cpp(10, 20));
    assert!(!outcome.store.is_spell_learn_to_spell_like_cpp(10, 21));
}

#[test]
fn spell_learn_spell_store_keeps_effect_and_db2_edges_when_world_sql_is_empty() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [],
        [learn_source(
            100,
            false,
            false,
            false,
            vec![SpellLearnSpellEffectLikeCpp {
                trigger_spell: 101,
                target_unit_pet: false,
            }],
        )],
        [crate::spell_db2::SpellLearnSpellEntry {
            id: 1,
            spell_id: 200,
            learn_spell_id: 201,
            overrides_spell_id: 0,
        }],
        |_| None,
        |_| true,
    );

    assert!(outcome.sql_result_empty);
    assert_eq!(outcome.sql_loaded_row_count, 0);
    assert_eq!(outcome.dbc_loaded_row_count, 2);
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(100),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 101,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(200),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 201,
            overrides_spell: 0,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(outcome.errors.is_empty());
    assert!(outcome.warnings.is_empty());
}

#[test]
fn spell_learn_spell_store_adds_spellinfo_effects_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [
            learn_source(
                10,
                false,
                false,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 20,
                    target_unit_pet: false,
                }],
            ),
            learn_source(
                30,
                false,
                true,
                false,
                vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 31,
                    target_unit_pet: false,
                }],
            ),
            SpellLearnSourceSpellInfoLikeCpp {
                spell_id: 40,
                difficulty_none: false,
                is_talent: false,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: vec![SpellLearnSpellEffectLikeCpp {
                    trigger_spell: 41,
                    target_unit_pet: true,
                }],
            },
        ],
        [],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 20 | 31 | 41),
    );

    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(outcome.dbc_loaded_row_count, 1);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForSpellEffect {
            source_spell: 10,
            learned_spell: 20,
        }
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(30),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 31,
            overrides_spell: 0,
            active: true,
            auto_learned: true,
        }]
    );
    assert!(
        outcome
            .store
            .get_spell_learn_spell_map_bounds_like_cpp(40)
            .is_empty()
    );
}

#[test]
fn spell_learn_spell_store_adds_db2_rows_after_sql_and_spell_effects_like_cpp() {
    let outcome = SpellLearnSpellStoreLikeCpp::from_sources_like_cpp(
        [SpellLearnSpellSqlRowLikeCpp {
            entry: 10,
            spell_id: 20,
            active: true,
        }],
        [learn_source(
            30,
            false,
            false,
            false,
            vec![SpellLearnSpellEffectLikeCpp {
                trigger_spell: 31,
                target_unit_pet: true,
            }],
        )],
        [
            crate::spell_db2::SpellLearnSpellEntry {
                id: 1,
                spell_id: 10,
                learn_spell_id: 20,
                overrides_spell_id: 0,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 2,
                spell_id: 30,
                learn_spell_id: 31,
                overrides_spell_id: 0,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 3,
                spell_id: 40,
                learn_spell_id: 41,
                overrides_spell_id: 42,
            },
            crate::spell_db2::SpellLearnSpellEntry {
                id: 4,
                spell_id: 50,
                learn_spell_id: 51,
                overrides_spell_id: 0,
            },
        ],
        |spell_id| match spell_id {
            10 => Some(learn_source(10, false, false, false, Vec::new())),
            _ => None,
        },
        |spell_id| matches!(spell_id, 10 | 20 | 30 | 31 | 40 | 41 | 51),
    );

    assert_eq!(outcome.sql_loaded_row_count, 1);
    assert_eq!(
        outcome.dbc_loaded_row_count, 2,
        "one SpellInfo effect plus one non-redundant SpellLearnSpell.db2 row"
    );
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(
        outcome.warnings[0].kind,
        SpellLearnSpellLoadWarningKindLikeCpp::RedundantSqlRowForDb2 {
            source_spell: 10,
            learned_spell: 20,
        }
    );
    assert_eq!(
        outcome.store.get_spell_learn_spell_map_bounds_like_cpp(40),
        &[SpellLearnSpellNodeLikeCpp {
            spell: 41,
            overrides_spell: 42,
            active: true,
            auto_learned: false,
        }]
    );
    assert!(
        outcome
            .store
            .get_spell_learn_spell_map_bounds_like_cpp(50)
            .is_empty(),
        "C++ silently skips SpellLearnSpell.db2 rows whose source spell is missing"
    );
}

fn serverside_effect_row(spell_id: u32, effect_index: i32) -> ServersideSpellEffectRowLikeCpp {
    ServersideSpellEffectRowLikeCpp {
        spell_id,
        effect_index,
        difficulty_id: 0,
        effect: spell_effect_types::SPELL_EFFECT_APPLY_AURA as i32,
        effect_aura: SPELL_AURA_DUMMY_LIKE_CPP,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura_period: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_base_points: 1.0,
        effect_misc_value_1: 0,
        effect_misc_value_2: 0,
        effect_radius_index_1: 0,
        effect_radius_index_2: 0,
        effect_spell_class_mask: [0, 0, 0, 0],
        implicit_target_1: 0,
        implicit_target_2: 0,
    }
}

#[test]
fn serverside_spell_effect_store_groups_valid_effects_like_cpp() {
    let mut heroic = serverside_effect_row(100, 1);
    heroic.difficulty_id = 2;
    heroic.effect_radius_index_1 = 7;
    heroic.effect_radius_index_2 = 8;
    heroic.effect_spell_class_mask = [1, 2, 3, 4];
    heroic.implicit_target_1 = implicit_targets::TARGET_DEST_DB as i32;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [heroic],
        |_| false,
        |difficulty| difficulty == 2,
        |radius| matches!(radius, 7 | 8),
    );

    assert_eq!(outcome.loaded_effect_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(outcome.warnings.is_empty());
    let effects = outcome
        .store
        .effects_for_spell_difficulty_like_cpp(100, 2)
        .expect("valid serverside effect should be staged");
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].effect_index, 1);
    assert_eq!(effects[0].effect_spell_class_mask, [1, 2, 3, 4]);
    assert_eq!(
        effects[0].implicit_target,
        [implicit_targets::TARGET_DEST_DB as i32, 0]
    );
}

#[test]
fn serverside_spell_effect_store_skips_invalid_rows_like_cpp() {
    let mut regular_spell = serverside_effect_row(10, 0);
    let mut missing_difficulty = serverside_effect_row(20, 0);
    missing_difficulty.difficulty_id = 3;
    let effect_index = serverside_effect_row(30, MAX_SPELL_EFFECTS_LIKE_CPP);
    let mut effect_type = serverside_effect_row(40, 0);
    effect_type.effect = TOTAL_SPELL_EFFECTS_LIKE_CPP;
    let mut aura_type = serverside_effect_row(50, 0);
    aura_type.effect_aura = TOTAL_AURAS_LIKE_CPP;
    let mut target_a = serverside_effect_row(60, 0);
    target_a.implicit_target_1 = TOTAL_SPELL_TARGETS_LIKE_CPP;
    let mut target_b = serverside_effect_row(70, 0);
    target_b.implicit_target_2 = TOTAL_SPELL_TARGETS_LIKE_CPP;
    regular_spell.effect_base_points = 10.0;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [
            regular_spell,
            missing_difficulty,
            effect_index,
            effect_type,
            aura_type,
            target_a,
            target_b,
        ],
        |spell_id| spell_id == 10,
        |_| false,
        |_| true,
    );

    assert_eq!(outcome.loaded_effect_count, 0);
    assert_eq!(
        outcome
            .errors
            .iter()
            .map(|error| error.kind)
            .collect::<Vec<_>>(),
        vec![
            ServersideSpellEffectLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded,
            ServersideSpellEffectLoadErrorKindLikeCpp::DifficultyMissing,
            ServersideSpellEffectLoadErrorKindLikeCpp::EffectIndexOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::EffectTypeOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::AuraTypeOutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget1OutOfRange,
            ServersideSpellEffectLoadErrorKindLikeCpp::ImplicitTarget2OutOfRange,
        ]
    );
}

#[test]
fn serverside_spell_effect_store_preserves_cpp_radius_warning_without_skip() {
    let mut row = serverside_effect_row(100, -1);
    row.effect_radius_index_1 = 77;
    row.effect_radius_index_2 = 88;

    let outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [row],
        |_| false,
        |_| true,
        |_| false,
    );

    assert_eq!(outcome.loaded_effect_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome
            .warnings
            .iter()
            .map(|warning| warning.kind)
            .collect::<Vec<_>>(),
        vec![
            ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius1Missing,
            ServersideSpellEffectLoadWarningKindLikeCpp::EffectRadius2Missing,
        ]
    );
    let effects = outcome
        .store
        .effects_for_spell_difficulty_like_cpp(100, 0)
        .expect("C++ still pushes effects with invalid radius rows");
    assert_eq!(effects[0].effect_index, -1);
    assert_eq!(effects[0].effect_radius_index, [77, 88]);
}

fn serverside_spell_row(spell_id: u32, difficulty_id: u32) -> ServersideSpellRowLikeCpp {
    ServersideSpellRowLikeCpp {
        spell_id,
        difficulty_id,
        category_id: 1,
        dispel: 2,
        mechanic: 3,
        attributes: 4,
        attributes_ex: [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18],
        stances: 19,
        stances_not: 20,
        targets: 21,
        target_creature_type: 22,
        requires_spell_focus: 23,
        facing_caster_flags: 24,
        caster_aura_state: 25,
        target_aura_state: 26,
        exclude_caster_aura_state: 27,
        exclude_target_aura_state: 28,
        caster_aura_spell: 29,
        target_aura_spell: 30,
        exclude_caster_aura_spell: 31,
        exclude_target_aura_spell: 32,
        caster_aura_type: 33,
        target_aura_type: 34,
        exclude_caster_aura_type: 35,
        exclude_target_aura_type: 36,
        casting_time_index: 37,
        recovery_time: 38,
        category_recovery_time: 39,
        start_recovery_category: 40,
        start_recovery_time: 41,
        interrupt_flags: 42,
        aura_interrupt_flags: [43, 44],
        channel_interrupt_flags: [45, 46],
        proc_flags: [47, 48],
        proc_chance: 49,
        proc_charges: 50,
        proc_cooldown: 51,
        proc_base_ppm: 52.0,
        max_level: 53,
        base_level: 54,
        spell_level: 55,
        duration_index: 56,
        range_index: 57,
        speed: 58.0,
        launch_delay: 59.0,
        stack_amount: 60,
        equipped_item_class: -1,
        equipped_item_sub_class_mask: 62,
        equipped_item_inventory_type_mask: 63,
        content_tuning_id: 64,
        spell_name: format!("Serverside {spell_id}"),
        cone_angle: 65.0,
        cone_width: 66.0,
        max_target_level: 67,
        max_affected_targets: 68,
        spell_family_name: 69,
        spell_family_flags: [70, 71, 72, 73],
        dmg_class: 74,
        prevention_type: 75,
        area_group_id: 76,
        school_mask: 77,
        charge_category_id: 78,
    }
}

fn serverside_spell_info_for_shapeshift(
    stances: u64,
    stances_not: u64,
    attributes: u32,
    attributes_ex2: u32,
) -> ServersideSpellInfoLikeCpp {
    let mut row = serverside_spell_row(7000, 0);
    row.attributes = attributes;
    row.attributes_ex = [0; 14];
    row.attributes_ex[1] = attributes_ex2;
    row.stances = stances;
    row.stances_not = stances_not;
    ServersideSpellInfoLikeCpp {
        row,
        effects: Vec::new(),
    }
}

fn shapeshift_form(flags: i32) -> crate::spell_db2::SpellShapeshiftFormEntry {
    crate::spell_db2::SpellShapeshiftFormEntry {
        id: 1,
        name: "Test Form".to_string(),
        creature_type: 0,
        flags,
        attack_icon_file_id: 0,
        bonus_action_bar: 0,
        combat_round_time: 0,
        damage_variance: 0.0,
        mount_type_id: 0,
        creature_display_id: [0; 4],
        preset_spell_id: [0; crate::spell_db2::MAX_SHAPESHIFT_SPELLS],
    }
}

#[test]
fn serverside_spell_check_shapeshift_rejects_excluded_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(0, 1 << 2, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);

    assert_eq!(
        spell.check_shapeshift_like_cpp(3, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}

#[test]
fn serverside_spell_check_shapeshift_allows_explicit_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::STANCE);

    assert_eq!(
        spell.check_shapeshift_like_cpp(5, |_| Some(&form)),
        SpellCastResult::Success
    );
}

#[test]
fn serverside_spell_check_shapeshift_missing_form_allows_like_cpp() {
    let spell =
        serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(7, |_| None),
        SpellCastResult::Success
    );
}

#[test]
fn serverside_spell_check_shapeshift_rejects_not_shapeshifted_attr_like_cpp() {
    let spell =
        serverside_spell_info_for_shapeshift(0, 0, attributes::SPELL_ATTR0_NOT_SHAPESHIFTED, 0);
    let form = shapeshift_form(0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}

#[test]
fn serverside_spell_check_shapeshift_rejects_can_only_cast_shapeshift_spells_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(0, 0, 0, 0);
    let form = shapeshift_form(shapeshift_form_flags::CAN_ONLY_CAST_SHAPESHIFT_SPELLS);

    assert_eq!(
        spell.check_shapeshift_like_cpp(1, |_| Some(&form)),
        SpellCastResult::NotShapeshift
    );
}

#[test]
fn serverside_spell_check_shapeshift_requires_other_shifted_form_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);
    let form = shapeshift_form(0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(2, |_| Some(&form)),
        SpellCastResult::OnlyShapeshift
    );
}

#[test]
fn serverside_spell_check_shapeshift_requires_form_when_unshifted_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(1 << 4, 0, 0, 0);

    assert_eq!(
        spell.check_shapeshift_like_cpp(0, |_| None),
        SpellCastResult::OnlyShapeshift
    );
}

#[test]
fn serverside_spell_check_shapeshift_allows_unshifted_with_attr2_like_cpp() {
    let spell = serverside_spell_info_for_shapeshift(
        1 << 4,
        0,
        0,
        attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
    );

    assert_eq!(
        spell.check_shapeshift_like_cpp(0, |_| None),
        SpellCastResult::Success
    );
}

#[test]
fn serverside_spell_store_composes_rows_with_staged_effects_like_cpp() {
    let effect_outcome = ServersideSpellEffectStoreLikeCpp::from_rows_like_cpp(
        [serverside_effect_row(100, 0)],
        |_| false,
        |_| true,
        |_| true,
    );
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 0)],
        &effect_outcome.store,
        |_| false,
    );

    assert_eq!(outcome.loaded_spell_count, 1);
    assert!(outcome.errors.is_empty());
    assert_eq!(
        outcome.store.serverside_spell_names,
        vec![(100, "Serverside 100".to_string())]
    );
    let info = outcome
        .store
        .get_serverside_spell_like_cpp(100, 0)
        .expect("serverside spell should be represented");
    assert_eq!(info.row.attributes_ex[13], 18);
    assert_eq!(info.row.spell_family_flags, [70, 71, 72, 73]);
    assert_eq!(info.effects.len(), 1);
    assert_eq!(info.effects[0].effect_index, 0);
}

#[test]
fn serverside_spell_store_rejects_regular_db2_spell_like_cpp() {
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 0)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |spell_id| spell_id == 100,
    );

    assert_eq!(outcome.loaded_spell_count, 0);
    assert_eq!(outcome.errors.len(), 1);
    assert_eq!(
        outcome.errors[0].kind,
        ServersideSpellLoadErrorKindLikeCpp::RegularSpellAlreadyLoaded
    );
    assert!(outcome.store.serverside_spell_names.is_empty());
    assert!(outcome.store.spell_infos_by_spell_and_difficulty.is_empty());
}

#[test]
fn serverside_spell_store_does_not_validate_main_row_difficulty_like_cpp() {
    let outcome = ServersideSpellStoreLikeCpp::from_rows_like_cpp(
        [serverside_spell_row(100, 999)],
        &ServersideSpellEffectStoreLikeCpp::default(),
        |_| false,
    );

    assert_eq!(outcome.loaded_spell_count, 1);
    assert!(outcome.errors.is_empty());
    assert!(
        outcome
            .store
            .get_serverside_spell_like_cpp(100, 999)
            .is_some(),
        "C++ LoadSpellInfoServerside validates DifficultyID for effect rows, not for the main serverside_spell row"
    );
}
