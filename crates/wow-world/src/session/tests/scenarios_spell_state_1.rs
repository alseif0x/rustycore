//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn spell_acquisition_catalog_arc_is_shared_with_session() {
    let (mut session, _, _) = make_session();
    let catalog = Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            std::iter::empty(),
            wow_data::EffectiveSpellAcquisitionRowsLikeCpp::default(),
            wow_data::SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    );

    session.set_spell_acquisition_catalog(Arc::clone(&catalog));

    let catalogs = &session.spell_catalogs;
    let installed = catalogs
        .spell_acquisition_catalog()
        .expect("the process-wide catalog must be installed");
    assert!(Arc::ptr_eq(&catalog, installed));
}
#[test]
fn spell_proc_entry_prefers_exact_difficulty_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = BTreeMap::new();
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 1,
        },
        test_spell_proc_entry_like_cpp(25.0),
    );
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 2,
        },
        test_spell_proc_entry_like_cpp(75.0),
    );
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp {
        proc_entries_by_spell_and_difficulty: entries,
    }));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));

    let entry = session
        .spell_proc_entry_like_cpp(42, 2)
        .expect("exact proc entry");

    assert_eq!(entry.chance, 75.0);
}
#[test]
fn spell_proc_entry_walks_difficulty_fallback_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = BTreeMap::new();
    entries.insert(
        wow_data::SpellProcKeyLikeCpp {
            spell_id: 42,
            difficulty: 1,
        },
        test_spell_proc_entry_like_cpp(25.0),
    );
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp {
        proc_entries_by_spell_and_difficulty: entries,
    }));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([
        DifficultyEntry {
            id: 3,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 2,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 2,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        },
        DifficultyEntry {
            id: 1,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        },
    ])));

    let entry = session
        .spell_proc_entry_like_cpp(42, 3)
        .expect("fallback proc entry");

    assert_eq!(entry.chance, 25.0);
}
#[test]
fn next_spell_in_chain_returns_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.next_spell_in_chain_like_cpp(10), 0);
}
#[test]
fn prev_spell_in_chain_returns_zero_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.prev_spell_in_chain_like_cpp(10), 0);
}
#[test]
fn first_spell_in_chain_returns_input_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert_eq!(session.first_spell_in_chain_like_cpp(10), 10);
}
#[test]
fn remove_known_spell_removes_non_talent_higher_ranks_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 20,
                    supercedes_spell_id: 10,
                },
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 30,
                    supercedes_spell_id: 20,
                },
            ],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20, 30, 40]);

    session.remove_known_spell_like_cpp(10);

    assert_eq!(
        session.known_spells_like_cpp(),
        &[40],
        "C++ Player::RemoveSpell recursively removes known non-talent higher ranks before removing the current spell"
    );
    assert!(
        session
            .represented_player_spell_rows_like_cpp()
            .into_iter()
            .all(|spell| spell.spell_id == 40
                || spell.state == RepresentedPlayerSpellStateLikeCpp::Removed)
    );
}
#[test]
fn login_known_spells_deactivate_lower_ranks_like_cpp_addspell() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 20,
                    supercedes_spell_id: 10,
                },
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 30,
                    supercedes_spell_id: 20,
                },
                wow_data::SpellRankEdgeLikeCpp {
                    spell_id: 200,
                    supercedes_spell_id: 100,
                },
            ],
            |_| true,
        ),
    ));
    let mut known_spells = vec![10, 40, 20, 100, 30];

    assert_eq!(
        session.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells),
        2
    );
    assert_eq!(
        known_spells,
        vec![40, 100, 30],
        "C++ Player::AddSpell keeps lower ranks in PlayerSpellMap but marks them inactive when a higher known rank supersedes them, so SendKnownSpells skips them"
    );
}
#[test]
fn remove_known_spell_preserves_talent_higher_rank_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    let outcome = wow_data::SpellCustomAttributeStoreLikeCpp::from_sql_rows_like_cpp(
        [wow_data::SpellCustomAttributeRowLikeCpp {
            spell_id: 20,
            attributes: wow_data::SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
        }],
        |spell_id| {
            (spell_id == 20)
                .then_some(vec![wow_data::SpellCustomAttributeSourceSpellInfoLikeCpp {
                    spell_id,
                    difficulty: 0,
                    effects: Vec::new(),
                }])
                .unwrap_or_default()
        },
    );
    session.set_spell_custom_attribute_store(Arc::new(outcome.store));
    session.set_known_spells_like_cpp(vec![10, 20]);

    session.remove_known_spell_like_cpp(10);

    assert_eq!(
        session.known_spells_like_cpp(),
        &[20],
        "C++ skips recursive higher-rank removal when the next spell has SPELL_ATTR0_CU_IS_TALENT"
    );
}
#[test]
fn spell_required_queries_return_empty_without_store_like_cpp() {
    let (session, _, _) = make_session();

    assert!(session.spells_required_for_spell_like_cpp(100).is_empty());
    assert!(session.spells_requiring_spell_like_cpp(10).is_empty());
    assert!(!session.is_spell_requiring_spell_like_cpp(100, 10));
}
#[test]
fn spell_required_queries_match_forward_reverse_maps_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_required_store(Arc::new(test_spell_required_store_like_cpp()));

    assert_eq!(session.spells_required_for_spell_like_cpp(100), &[10, 11]);
    assert_eq!(session.spells_requiring_spell_like_cpp(10), &[100, 101]);
    assert!(session.is_spell_requiring_spell_like_cpp(100, 10));
    assert!(!session.is_spell_requiring_spell_like_cpp(11, 100));
}
#[test]
fn remove_known_spell_removes_spells_requiring_it_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_required_store(Arc::new(test_spell_required_store_like_cpp()));
    session.set_known_spells_like_cpp(vec![10, 100, 101, 200]);

    session.remove_known_spell_like_cpp(10);

    assert_eq!(
        session.known_spells_like_cpp(),
        &[200],
        "C++ Player::RemoveSpell removes spells returned by GetSpellsRequiringSpellBounds recursively"
    );
    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp()
            .into_iter()
            .filter(|spell| spell.state == RepresentedPlayerSpellStateLikeCpp::Removed)
            .map(|spell| spell.spell_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([10, 100, 101]),
        "C++ marks the removed required spell and its non-dependent known dependants as PLAYERSPELL_REMOVED"
    );

    session.remove_known_spell_like_cpp(11);
    assert_eq!(
        session.known_spells_like_cpp(),
        &[200],
        "C++ returns before recursive required-spell cleanup when the removed spell is not known"
    );
}
#[test]
fn remove_known_spell_removes_learned_dependent_spells_and_overrides_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp {
        learned_by_spell_id: BTreeMap::from([(
            10,
            vec![wow_data::SpellLearnSpellNodeLikeCpp {
                spell: 20,
                overrides_spell: 100,
                active: true,
                auto_learned: false,
            }],
        )]),
    }));
    session.set_known_spells_like_cpp(vec![10, 20, 30]);
    session.add_represented_override_spell_like_cpp(100, 20);

    session.remove_known_spell_like_cpp(10);

    assert_eq!(
        session.known_spells_like_cpp(),
        &[30],
        "C++ Player::RemoveSpell removes spells returned by GetSpellLearnSpellMapBounds"
    );
    assert!(
        session.represented_override_spells_like_cpp().is_empty(),
        "C++ removes OverridesSpell pairs for learned dependent spells"
    );
    assert_eq!(
        session
            .represented_player_spell_rows_like_cpp()
            .into_iter()
            .filter(|spell| spell.state == RepresentedPlayerSpellStateLikeCpp::Removed)
            .map(|spell| spell.spell_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([10, 20])
    );
}
#[test]
fn loaded_known_spell_dependencies_rebuild_all_active_override_edges_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp {
        learned_by_spell_id: BTreeMap::from([(
            10,
            vec![
                wow_data::SpellLearnSpellNodeLikeCpp {
                    spell: 20,
                    overrides_spell: 100,
                    active: true,
                    auto_learned: false,
                },
                wow_data::SpellLearnSpellNodeLikeCpp {
                    spell: 30,
                    overrides_spell: 200,
                    active: true,
                    auto_learned: true,
                },
            ],
        )]),
    }));
    session.add_represented_override_spell_like_cpp(999, 888);
    session.reset_represented_talents_like_cpp();

    let mut known_spells = vec![10, 20];
    assert_eq!(
        session.apply_loaded_known_spell_dependencies_like_cpp(&mut known_spells),
        0,
        "an already-persisted dependency and an auto-learned dependency do not append spell rows"
    );
    assert_eq!(known_spells, vec![10, 20]);
    assert_eq!(
        session.represented_override_spells_like_cpp(),
        HashMap::from([(100, BTreeSet::from([20])), (200, BTreeSet::from([30])),]),
        "C++ AddSpell rebuilds active OverridesSpell edges outside the AutoLearned branch"
    );

    session.reset_represented_talents_like_cpp();
    let mut active_projection = Vec::new();
    assert_eq!(
        session.apply_loaded_spell_dependencies_from_roots_like_cpp(&[10], &mut active_projection,),
        1,
        "an inactive non-disabled loaded root still runs AddSpell dependency expansion"
    );
    assert_eq!(active_projection, vec![20]);
    assert_eq!(
        session.represented_override_spells_like_cpp(),
        HashMap::from([(100, BTreeSet::from([20])), (200, BTreeSet::from([30])),])
    );
}
#[test]
fn loaded_dependency_spells_apply_learn_skill_before_authority_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp {
        learned_by_spell_id: BTreeMap::from([(
            30,
            vec![wow_data::SpellLearnSpellNodeLikeCpp {
                spell: 10,
                overrides_spell: 0,
                active: true,
                auto_learned: false,
            }],
        )]),
    }));
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([(
            10,
            wow_data::SpellLearnSkillNodeLikeCpp {
                skill: 755,
                step: 4,
                value: 75,
                maxvalue: 150,
            },
        )]),
        covered_spell_ids: BTreeSet::from([10, 30]),
        ..Default::default()
    }));
    session.replace_player_skill_records_like_cpp(HashMap::new(), true, false);

    let mut known_spells = vec![30];
    let mut side_effect_spells = vec![30];
    assert_eq!(
        session.apply_loaded_spell_dependency_skills_like_cpp(
            &mut known_spells,
            &mut side_effect_spells,
        ),
        (1, true)
    );
    assert_eq!(known_spells, vec![30, 10]);
    assert_eq!(side_effect_spells, vec![30, 10]);
    assert_eq!(session.player_skill_value_like_cpp(755), 75);
    assert_eq!(session.player_skill_max_value_like_cpp(755), 150);
}
#[test]
fn remove_known_spell_removes_first_rank_learned_skill_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_learn_skill_store(Arc::new(test_spell_learn_skill_store_like_cpp()));
    session.replace_player_skill_records_like_cpp(
        HashMap::from([(
            755,
            RepresentedPlayerSkillLikeCpp {
                skill_id: 755,
                step: 4,
                value: 150,
                max: 225,
                profession_slot: 0,
                state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
            },
        )]),
        false,
        false,
    );
    assert!(!session.player_skill_records_loaded_like_cpp());
    session.set_known_spells_like_cpp(vec![10, 30]);

    session.remove_known_spell_like_cpp(10);

    assert_eq!(
        session.known_spells_like_cpp(),
        &[30],
        "C++ Player::RemoveSpell removes the known first-rank spell itself"
    );
    assert_eq!(
        session.player_skill_records_like_cpp().get(&755),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 755,
            step: 0,
            value: 0,
            max: 0,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Deleted,
        }),
        "C++ SetSkill(skill, 0, 0, 0) resets the learned skill for first-rank SpellLearnSkill nodes"
    );
    assert!(
        session.player_skill_records_loaded_like_cpp(),
        "the pre-#164 mutation path always made the represented map eligible for persistence"
    );
    assert!(
        session.complete_player_skill_records_like_cpp().is_none(),
        "a runtime mutation does not manufacture exact slot-occupancy authority"
    );

    assert!(
        session
            .player_skill_non_durable_tombstones_like_cpp
            .contains(&755)
    );
}
#[test]
fn remove_known_spell_downgrades_to_previous_learned_skill_with_explicit_max_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([
            (
                10,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: 755,
                    step: 1,
                    value: 75,
                    maxvalue: 75,
                },
            ),
            (
                20,
                wow_data::SpellLearnSkillNodeLikeCpp {
                    skill: 755,
                    step: 2,
                    value: 150,
                    maxvalue: 150,
                },
            ),
        ]),
        ..Default::default()
    }));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        755,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 755,
            step: 2,
            value: 180,
            max: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&755),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 755,
            step: 1,
            value: 75,
            max: 75,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Changed,
        }),
        "C++ clamps GetPureSkillValue/GetPureMaxSkillValue to the previous SpellLearnSkill explicit value/maxvalue before SetSkill"
    );
}
#[test]
fn remove_known_spell_resets_skill_when_previous_learned_skill_missing_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_spell_learn_skill_store(Arc::new(wow_data::SpellLearnSkillStoreLikeCpp {
        skill_by_spell_id: BTreeMap::from([(
            20,
            wow_data::SpellLearnSkillNodeLikeCpp {
                skill: 755,
                step: 2,
                value: 150,
                maxvalue: 150,
            },
        )]),
        ..Default::default()
    }));
    session.set_player_skill_records_like_cpp(HashMap::from([(
        755,
        RepresentedPlayerSkillLikeCpp {
            skill_id: 755,
            step: 2,
            value: 150,
            max: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&755),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_id: 755,
            step: 0,
            value: 0,
            max: 0,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Deleted,
        }),
        "C++ removes the current learned skill when no previous SpellLearnSkill setting is found"
    );
}
#[test]
fn remove_known_spell_marks_previous_rank_dependent_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20]);
    session.learn_dependent_known_spell_like_cpp(20);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(session.known_spells_like_cpp(), &[10]);
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&10),
        "C++ RemoveSpell copies cur_dependent to the previous rank before AddSpell reactivates it"
    );
    assert!(
        !session
            .represented_player_spell_rows_like_cpp()
            .iter()
            .any(|spell| {
                spell.spell_id == 20 && spell.state == RepresentedPlayerSpellStateLikeCpp::Removed
            }),
        "represented dependent spells are still skipped from normal removed-spell save evidence"
    );
}
#[test]
fn remove_known_spell_clears_previous_rank_dependent_when_current_is_independent_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20]);
    session.learn_dependent_known_spell_like_cpp(10);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(session.known_spells_like_cpp(), &[10]);
    assert!(
        !session
            .represented_dependent_known_spells_like_cpp()
            .contains(&10),
        "C++ RemoveSpell updates previous-rank dependent state when it differs from the removed rank"
    );
    assert!(
        session
            .represented_player_spell_rows_like_cpp()
            .iter()
            .any(|spell| {
                spell.spell_id == 20 && spell.state == RepresentedPlayerSpellStateLikeCpp::Removed
            }),
        "non-dependent removed current rank still carries represented _SaveSpells delete evidence"
    );
}
#[test]
fn remove_known_spell_does_not_create_missing_previous_rank_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert!(
        session.known_spells_like_cpp().is_empty(),
        "C++ RemoveSpell only reactivates a previous rank when prev_id already exists in PlayerSpellMap; Rust does not invent an absent previous row in the represented model"
    );
}
#[test]
fn remove_known_spell_erases_override_source_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_known_spells_like_cpp(vec![10, 30]);
    session.add_represented_override_spell_like_cpp(10, 20);
    session.add_represented_override_spell_like_cpp(30, 40);

    session.remove_known_spell_like_cpp(10);

    assert!(
        !session
            .represented_override_spells_like_cpp()
            .contains_key(&10),
        "C++ Player::RemoveSpell erases m_overrideSpells[spell_id] after removing the spell"
    );
    assert_eq!(
        session
            .represented_override_spells_like_cpp()
            .get(&30)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![40],
        "unrelated override spell entries are not removed by m_overrideSpells.erase(spell_id)"
    );
}
#[test]
fn remove_known_spell_removes_trait_definition_override_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_known_spells_like_cpp(vec![20, 40]);
    session.set_trait_definition_store(Arc::new(TraitDefinitionStore::from_entries([
        wow_data::trait_tree::TraitDefinitionEntry {
            id: 7,
            override_name: String::new(),
            override_subtext: String::new(),
            override_description: String::new(),
            spell_id: 20,
            override_icon: 0,
            overrides_spell_id: 10,
            visible_spell_id: 0,
        },
    ])));
    session.add_represented_override_spell_like_cpp(10, 20);
    session.add_represented_override_spell_like_cpp(30, 40);
    session.set_represented_spell_trait_definition_id_like_cpp(20, 7);

    session.remove_known_spell_like_cpp(20);

    assert!(
        !session
            .represented_override_spells_like_cpp()
            .get(&10)
            .is_some_and(|spells| spells.contains(&20)),
        "C++ Player::RemoveSpell removes TraitDefinition OverridesSpellID -> spell_id pairs"
    );
    assert_eq!(
        session
            .represented_override_spells_like_cpp()
            .get(&30)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![40],
        "trait-definition cleanup must not remove unrelated override mappings"
    );
    assert!(
        !session
            .represented_spell_trait_definition_ids_like_cpp
            .contains_key(&20),
        "removed PlayerSpell no longer owns a represented TraitDefinitionId"
    );
}
#[test]
fn remove_known_spell_sends_unlearned_spells_with_suppress_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_known_spells_like_cpp(vec![20, 40]);

    session.remove_known_spell_with_suppress_messaging_like_cpp(20, true);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::UnlearnedSpells as u16
    );
    assert_eq!(packet.read_uint32().expect("count"), 1);
    assert_eq!(packet.read_uint32().expect("spell id"), 20);
    assert!(packet.read_bit().expect("SuppressMessaging"));
    packet.flush_bits();
    assert!(packet.is_empty());
}
#[test]
fn remove_known_spell_sends_superceded_packet_when_previous_rank_reactivates_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 20,
                supercedes_spell_id: 10,
            }],
            |_| true,
        ),
    ));
    session.set_known_spells_like_cpp(vec![10, 20]);

    session.remove_known_spell_with_suppress_messaging_like_cpp(20, true);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::SupercededSpells as u16
    );
    assert_eq!(packet.read_uint32().expect("count"), 1);
    assert_eq!(packet.read_int32().expect("new spell id"), 10);
    assert!(!packet.read_bit().expect("IsFavorite"));
    assert!(!packet.read_bit().expect("field_8.HasValue"));
    assert!(packet.read_bit().expect("Superceded.HasValue"));
    assert!(!packet.read_bit().expect("TraitDefinitionID.HasValue"));
    packet.flush_bits();
    assert_eq!(packet.read_int32().expect("old spell id"), 20);
    assert!(
        packet.is_empty(),
        "C++ sends SendSupercededSpell instead of UnlearnedSpells when the lower rank reactivates"
    );
}
