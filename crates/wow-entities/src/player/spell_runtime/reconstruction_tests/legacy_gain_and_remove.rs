//! Original Session rule cases, now exercising canonical Player operations.

use super::*;
use super::support::*;

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
                skill_line_id: 755,
                step: 4,
                current_value: 150,
                max_value: 225,
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
            skill_line_id: 755,
            step: 0,
            current_value: 0,
            max_value: 0,
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
        session.player.non_durable_skill_tombstones_like_cpp()
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
            skill_line_id: 755,
            step: 2,
            current_value: 180,
            max_value: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&755),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 755,
            step: 1,
            current_value: 75,
            max_value: 75,
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
            skill_line_id: 755,
            step: 2,
            current_value: 150,
            max_value: 225,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
        },
    )]));
    session.set_known_spells_like_cpp(vec![20]);

    session.remove_known_spell_like_cpp(20);

    assert_eq!(
        session.player_skill_records_like_cpp().get(&755),
        Some(&RepresentedPlayerSkillLikeCpp {
            skill_line_id: 755,
            step: 0,
            current_value: 0,
            max_value: 0,
            profession_slot: 0,
            state: RepresentedPlayerSkillStateLikeCpp::Deleted,
        }),
        "C++ removes the current learned skill when no previous SpellLearnSkill setting is found"
    );
}

