use super::*;
use crate::spell_acquisition::*;

#[test]
fn base_learn_spell_fallback_keeps_known_lower_rank_inactive_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let lower_spell_id = 13_344_i32;
    let higher_spell_id = 13_345_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: higher_spell_id as u32,
                supercedes_spell_id: lower_spell_id as u32,
            }],
            |_| true,
        ),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [
            RepresentedPlayerSpellLikeCpp {
                spell_id: lower_spell_id,
                active: false,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: higher_spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, lower_spell_id));

    assert_eq!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .get(&lower_spell_id)
            .map(|row| (row.active, row.state)),
        Some((false, RepresentedPlayerSpellStateLikeCpp::Unchanged)),
        "C++ AddSpell keeps a lower rank inactive when its next rank is known"
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
            spell_id: lower_spell_id as u32,
        }],
        "unchanged AddSpell returns before LearnOrKnow, but non-disabled LearnSpell still advances the quest objective"
    );
}

#[test]
fn base_learn_spell_fallback_rejects_ranked_insertion_before_partial_mutation() {
    let (mut session, _, send_rx) = make_session();
    let lower_spell_id = 13_352_i32;
    let higher_spell_id = 13_353_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: higher_spell_id as u32,
                supercedes_spell_id: lower_spell_id as u32,
            }],
            |_| true,
        ),
    ));
    let lower_row = RepresentedPlayerSpellLikeCpp {
        spell_id: lower_spell_id,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
        state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
    };
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([lower_row]));

    assert!(!apply_base_learning_like_cpp(&mut session, higher_spell_id));

    assert_eq!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .get(&lower_spell_id),
        Some(&lower_row)
    );
    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .contains_key(&higher_spell_id)
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty(),
        "rank insertion must stop before C++ previous-rank and supersession work becomes partial"
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn base_learn_spell_fallback_allows_first_rank_insertion_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let first_rank = 13_354_i32;
    let second_rank = 13_355_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: second_rank as u32,
                supercedes_spell_id: first_rank as u32,
            }],
            |_| true,
        ),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, first_rank));

    assert_eq!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp[&first_rank],
        RepresentedPlayerSpellLikeCpp {
            spell_id: first_rank,
            active: true,
            disabled: false,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::New,
        }
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells]
    );
}

#[test]
fn base_learn_spell_fallback_reactivates_disabled_cpp_closure_in_order() {
    let (mut session, _, send_rx) = make_session();
    let root_spell = 13_346_i32;
    let next_spell = 13_347_i32;
    let requiring_spell = 13_348_i32;
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: next_spell as u32,
                supercedes_spell_id: root_spell as u32,
            }],
            |_| true,
        ),
    ));
    let mut required_store = wow_data::SpellRequiredStoreLikeCpp::default();
    required_store
        .required_by_spell_id
        .insert(requiring_spell as u32, vec![root_spell as u32]);
    required_store
        .requiring_by_required_spell_id
        .insert(root_spell as u32, vec![requiring_spell as u32]);
    session.set_spell_required_store(Arc::new(required_store));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [
            RepresentedPlayerSpellLikeCpp {
                spell_id: root_spell,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: next_spell,
                active: false,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
            RepresentedPlayerSpellLikeCpp {
                spell_id: requiring_spell,
                active: true,
                disabled: true,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ],
        [],
        [],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, root_spell));

    let rows = &session
        .player_spell_test_fixture_like_cpp
        .represented_player_spell_rows_like_cpp;
    assert!(rows.values().all(|row| !row.disabled));
    assert!(rows[&root_spell].active);
    assert!(!rows[&next_spell].active);
    assert!(rows[&requiring_spell].active);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells, ServerOpcodes::LearnedSpells]
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: root_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: root_spell as u32,
                favorite: false,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: next_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: requiring_spell as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: requiring_spell as u32,
                favorite: false,
                suppress_messaging: false,
            },
        ]
    );
}

#[test]
fn base_learn_spell_fallback_clears_trait_override_before_reactivation_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 13_356_i32;
    let overridden_spell_id = 13_357_i32;
    let trait_definition_id = 77_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
    session.set_trait_definition_store(Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::from_entries([
            wow_data::trait_tree::TraitDefinitionEntry {
                id: trait_definition_id as u32,
                override_name: String::new(),
                override_subtext: String::new(),
                override_description: String::new(),
                spell_id: 0,
                override_icon: 0,
                overrides_spell_id: overridden_spell_id,
                visible_spell_id: 0,
            },
        ]),
    ));
    assert!(session.replace_complete_spell_acquisition_runtime_like_cpp(
        [RepresentedPlayerSpellLikeCpp {
            spell_id,
            active: true,
            disabled: true,
            dependent: false,
            favorite: false,
            state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
        }],
        [(spell_id, trait_definition_id)],
        [(overridden_spell_id, spell_id)],
        HashMap::new(),
        0,
        BTreeSet::new(),
    ));

    assert!(apply_base_learning_like_cpp(&mut session, spell_id));

    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_spell_trait_definition_ids_like_cpp
            .contains_key(&spell_id)
    );
    assert!(
        !session
            .represented_override_spells_like_cpp
            .get(&overridden_spell_id)
            .is_some_and(|spells| spells.contains(&spell_id))
    );
    assert!(
        !session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp[&spell_id]
            .disabled
    );
}

#[test]
fn learn_spell_capacity_rejection_cannot_enter_shallow_fallback() {
    assert!(!may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
            configured_max: 2,
            used: 2,
            requested_new: 1,
        },
    ));
    assert!(may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot,
    ));
    assert!(!may_shallow_fallback_after_profession_plan_error_like_cpp(
        crate::profession::PrimaryProfessionCapacityPlanErrorLikeCpp::InvalidConfiguredMaximum {
            configured: 3,
        },
    ));
}

#[test]
fn base_learn_spell_fallback_replaces_temporary_row_with_durable_new_row_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 13_350_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: false,
                favorite: true,
                state: RepresentedPlayerSpellStateLikeCpp::Temporary,
            },
        ])
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));

    assert!(apply_base_learning_like_cpp(&mut session, spell_id));

    assert_eq!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .get(&spell_id),
        Some(&RepresentedPlayerSpellLikeCpp {
            spell_id,
            active: true,
            disabled: false,
            dependent: false,
            favorite: true,
            state: RepresentedPlayerSpellStateLikeCpp::New,
        }),
        "C++ AddSpell removes PLAYERSPELL_TEMPORARY before inserting a durable new row"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::LearnedSpells]
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: spell_id as u32,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: spell_id as u32,
                favorite: true,
                suppress_messaging: false,
            },
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: spell_id as u32,
            },
        ]
    );
}

#[test]
fn base_learn_spell_fallback_rejects_disabled_row_without_dependency_authority() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 13_349_i32;
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    let original_row = RepresentedPlayerSpellLikeCpp {
        spell_id,
        active: true,
        disabled: true,
        dependent: false,
        favorite: false,
        state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
    };
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([original_row]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));

    assert!(!apply_base_learning_like_cpp(&mut session, spell_id));

    assert_eq!(
        session
            .player_spell_test_fixture_like_cpp
            .represented_player_spell_rows_like_cpp
            .get(&spell_id),
        Some(&original_row),
        "without both C++ dependency stores the fallback must fail before enabling the row"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
