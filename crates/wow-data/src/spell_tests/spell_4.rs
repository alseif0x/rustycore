//! Spell scenarios for [`super`].
//!
//! Split out of spell_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
