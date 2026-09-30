//! Original causal validation cases moved from the World preparation suite.

use super::*;

#[test]
fn required_learning_publications_cannot_be_omitted() {
    let (_source, plan) = direct_learn_plan();
    validate_acquisition_plan(&plan)
        .expect("complete publication tape is valid");

    for omitted_index in 0..plan.post_commit_actions.len() {
        let mut omitted = plan.clone();
        omitted.post_commit_actions.remove(omitted_index);
        assert!(matches!(
            validate_acquisition_plan(&omitted),
            Err(
                AcquisitionPlanError::PostCommitActionCausalityMismatch { .. }
            )
        ));
    }
}

#[test]
fn skill_line_criteria_require_exact_occurrence_identity_and_cardinality() {
    let (_source, mut plan) = direct_learn_plan();
    plan.publication_requirements_for_test_like_cpp().splice(
        0..0,
        [
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id: 100,
                skill_id: 164,
            },
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id: 100,
                skill_id: 164,
            },
        ],
    );
    plan.post_commit_actions.splice(
        0..0,
        [
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                source_spell_id: 100,
                skill_id: 164,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                source_spell_id: 100,
                skill_id: 164,
            },
        ],
    );
    validate_acquisition_plan(&plan)
        .expect("the exact C++ SkillLineAbility occurrence is valid");

    let mut wrong_skill = plan.clone();
    for action in &mut wrong_skill.post_commit_actions[..2] {
        match action {
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
                skill_id,
                ..
            }
            | SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria {
                skill_id,
                ..
            } => *skill_id = 165,
            _ => unreachable!(),
        }
    }
    assert!(
        validate_acquisition_plan(&wrong_skill)
            .is_err()
    );

    let mut duplicated = plan.clone();
    duplicated
        .post_commit_actions
        .splice(2..2, plan.post_commit_actions[..2].iter().cloned());
    assert!(
        validate_acquisition_plan(&duplicated)
            .is_err()
    );

    let mut omitted = plan.clone();
    omitted.post_commit_actions.drain(..2);
    assert!(
        validate_acquisition_plan(&omitted)
            .is_err()
    );
}

#[test]
fn learned_action_must_match_the_final_favorite_row() {
    let source = snapshot(Vec::new());
    let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: vec![
            SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: true,
                suppress_messaging: false,
            },
        ],
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id: 100,
            favorite: true,
            suppress_messaging: false,
        }],
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    }
    .build();
    assert_eq!(
        validate_acquisition_plan(&plan),
        Err(AcquisitionPlanError::LearnedActionRowMismatch(100))
    );
}

#[test]
fn learned_action_requires_a_causal_learning_transition() {
    let unchanged = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    let source = snapshot(vec![unchanged]);
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: Vec::new(),
        spell_transitions: Vec::new(),
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: vec![
            SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: false,
                suppress_messaging: false,
            },
        ],
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id: 100,
            favorite: false,
            suppress_messaging: false,
        }],
        diagnostics: Vec::new(),
        resulting_snapshot: source.clone(),
    }
    .build();

    assert_eq!(
        validate_acquisition_plan(&plan),
        Err(AcquisitionPlanError::LearnedActionRowMismatch(100))
    );
}

#[test]
fn learned_transition_evidence_is_consumed_once() {
    let source = snapshot(Vec::new());
    let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let duplicate = SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
        spell_id: 100,
        favorite: false,
        suppress_messaging: false,
    };
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: vec![
            SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: false,
                suppress_messaging: false,
            },
            SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: false,
                suppress_messaging: false,
            },
        ],
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![duplicate.clone(), duplicate],
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    }
    .build();

    assert_eq!(
        validate_acquisition_plan(&plan),
        Err(AcquisitionPlanError::LearnedActionRowMismatch(100))
    );
}

#[test]
fn dual_wield_diagnostic_cannot_replace_live_cast_effect_authority() {
    let source = snapshot(Vec::new());
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: Vec::new(),
        spell_transitions: Vec::new(),
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
            source_spell_id: 100,
            effect_record_id: 7,
            effect_index: 0,
        }],
        diagnostics: vec![
            SpellAcquisitionDiagnosticLikeCpp::DualWieldEffectProjected {
                spell_id: 100,
                effect_record_id: 7,
                effect_index: 0,
            },
        ],
        resulting_snapshot: source.clone(),
    }
    .build();

    assert_eq!(
        validate_acquisition_plan(&plan),
        Err(
            AcquisitionPlanError::PostCommitActionCausalityMismatch {
                action: "GrantDualWield",
                id: 100,
            }
        )
    );
}

#[test]
fn unrelated_publication_actions_are_rejected_before_application() {
    let source = snapshot(Vec::new());
    let learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let base_plan = SpellAcquisitionPlanFixtureLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: Vec::new(),
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    }
    .build();
    let unrelated_actions = [
        SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id: 999 },
        SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell {
            old_spell_id: 999,
            new_spell_id: 100,
        },
        SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: 999 },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id: 999 },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id: 999 },
        SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
            source_spell_id: 999,
            effect_record_id: 1,
            effect_index: 0,
        },
        SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
            source_spell_id: 100,
            effect_record_id: 1,
            effect_index: 0,
        },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria {
            source_spell_id: 999,
            skill_id: 164,
        },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id: 164 },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: 164 },
    ];

    for action in unrelated_actions {
        let mut plan = base_plan.clone();
        plan.post_commit_actions = vec![action.clone()];
        assert!(
            matches!(
                validate_acquisition_plan(&plan),
                Err(
                    AcquisitionPlanError::PostCommitActionCausalityMismatch { .. }
                )
            ),
            "unrelated action reached the prepared boundary: {action:?}"
        );
    }
}
