//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
