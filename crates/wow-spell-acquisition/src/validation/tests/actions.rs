//! Full-boundary publication order and consumption of causal occurrences.

use super::*;

fn supersede_plan() -> SpellAcquisitionPlanLikeCpp {
    let old = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    let new = spell(200, PlayerSpellPersistenceStateLikeCpp::New);
    let mut inactive = old;
    inactive.active = false;
    inactive.state = PlayerSpellPersistenceStateLikeCpp::Changed;
    let root = SpellAcquisitionRootLikeCpp::DirectLearn(200);
    let activation = PlannedSpellTransitionLikeCpp {
        spell_id: 200,
        before: None,
        after: Some(new),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root { root },
    };
    let deactivation = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: Some(old),
        after: Some(inactive),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root { root },
    };
    SpellAcquisitionPlanFixtureLikeCpp {
        root,
        source_snapshot: snapshot(vec![old]),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(activation.clone()), PlannedAcquisitionMutationLikeCpp::Spell(deactivation.clone())],
        spell_transitions: vec![activation, deactivation],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell { old_spell_id: 100, new_spell_id: 200 }],
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![inactive, new]),
    }
    .build()
}

#[test]
fn publication_tape_preserves_order_and_suppression_payload() {
    let (_source, mut plan) = direct_learn_plan();
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    plan.post_commit_actions.swap(0, 1);
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "required publication tape", id: 0 }));
    plan.publication_requirements_for_test_like_cpp().swap(0, 1);
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    let SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell { suppress_messaging, .. } = &mut plan.post_commit_actions[0] else { panic!("learned action") };
    *suppress_messaging = true;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "required publication tape", id: 0 }));
    let SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell { suppress_messaging, .. } = &mut plan.publication_requirements_for_test_like_cpp()[0] else { panic!("learned requirement") };
    *suppress_messaging = true;
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
}

#[test]
fn tradeskill_actions_require_adjacency_even_when_the_tape_matches() {
    let (_source, mut plan) = direct_learn_plan();
    plan.publication_requirements_for_test_like_cpp().splice(0..0, [
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnTradeskillSkillLineCriteria { source_spell_id: 100, skill_id: 164 },
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellFromSkillLineCriteria { source_spell_id: 100, skill_id: 164 },
    ]);
    plan.post_commit_actions.splice(0..0, [
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnTradeskillSkillLineCriteria { source_spell_id: 100, skill_id: 164 },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellFromSkillLineCriteria { source_spell_id: 100, skill_id: 164 },
    ]);
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    plan.post_commit_actions.insert(1, SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: 100 });
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "tradeskill skill-line criteria", id: 100 }));
    plan.post_commit_actions.remove(1);
    plan.post_commit_actions.swap(0, 1);
    plan.publication_requirements_for_test_like_cpp().swap(0, 1);
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "learn-spell skill-line criteria", id: 100 }));
}

#[test]
fn live_dual_wield_occurrences_require_exact_cardinality_and_effect_identity() {
    let mut source = snapshot(Vec::new());
    let effect = PlayerExecutedDualWieldEffectLikeCpp { effect_record_id: 7, effect_index: 2 };
    source.cast_resolutions.insert(100, PlayerCastAcquisitionResolutionLikeCpp {
        reached_immediate_phase: true,
        executed_hit_target_effect_mask: 0,
        effective_effects: Vec::new(),
        executed_dual_wield_effects: vec![effect, effect],
    });
    let mut plan = unchanged_plan(source);
    let grant = SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { source_spell_id: 100, effect_record_id: 7, effect_index: 2 };
    plan.post_commit_actions = vec![grant.clone(), grant.clone()];
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    plan.post_commit_actions.pop();
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "DualWieldEffectProjected", id: 100 }));
    plan.post_commit_actions = vec![grant.clone(), grant.clone(), grant.clone()];
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "GrantDualWield", id: 100 }));
    plan.post_commit_actions = vec![grant.clone(), SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { source_spell_id: 100, effect_record_id: 7, effect_index: 1 }];
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "GrantDualWield", id: 100 }));
    plan.post_commit_actions = vec![grant.clone(), grant];
    plan.source_snapshot.cast_resolutions.get_mut(&100).unwrap().reached_immediate_phase = false;
    plan.resulting_snapshot.cast_resolutions = plan.source_snapshot.cast_resolutions.clone();
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "GrantDualWield", id: 100 }));
}

#[test]
fn supersede_pairing_consumes_deactivation_and_suppresses_learned_evidence() {
    let plan = supersede_plan();
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    let mut duplicate = plan.clone();
    duplicate.post_commit_actions.push(duplicate.post_commit_actions[0].clone());
    assert_eq!(validate_acquisition_plan(&duplicate), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "SupersededSpell", id: 100 }));
    let mut unlearned = plan.clone();
    unlearned.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id: 100 });
    assert_eq!(validate_acquisition_plan(&unlearned), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "UnlearnedSpell", id: 100 }));
    let mut learned = plan.clone();
    learned.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell { spell_id: 200, favorite: false, suppress_messaging: false });
    learned.publication_requirements_for_test_like_cpp().push(SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell { spell_id: 200, favorite: false, suppress_messaging: false });
    assert_eq!(validate_acquisition_plan(&learned), Err(AcquisitionPlanError::LearnedActionRowMismatch(200)));
    let mut reversed = plan;
    reversed.mutations.swap(0, 1);
    reversed.spell_transitions.swap(0, 1);
    assert_eq!(validate_acquisition_plan(&reversed), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "SupersededSpell", id: 100 }));
}

#[test]
fn refresh_and_unlearn_actions_consume_occurrences_once() {
    let (_source, mut learned) = direct_learn_plan();
    learned.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: 100 });
    assert_eq!(validate_acquisition_plan(&learned), Ok(()));
    learned.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::RefreshPassive { spell_id: 100 });
    assert_eq!(validate_acquisition_plan(&learned), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "RefreshPassive", id: 100 }));

    let mut removed = supersede_plan();
    removed.mutations.remove(0);
    removed.spell_transitions.remove(0);
    removed.resulting_snapshot.spells.retain(|row| row.spell_id == 100);
    removed.post_commit_actions = vec![SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id: 100 }];
    assert_eq!(validate_acquisition_plan(&removed), Ok(()));
    removed.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::UnlearnedSpell { spell_id: 100 });
    assert_eq!(validate_acquisition_plan(&removed), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "UnlearnedSpell", id: 100 }));
}

#[test]
fn skill_actions_consume_independent_domain_evidence() {
    let riding_id = u32::from(SKILL_RIDING_LIKE_CPP);
    let mut riding = skill(riding_id);
    riding.state = PlayerSkillPersistenceStateLikeCpp::New;
    let source = snapshot(Vec::new());
    let mut plan = unchanged_plan(source);
    let transition = PlannedSkillTransitionLikeCpp { skill_id: riding_id, before: None, after: riding, provenance: SpellAcquisitionProvenanceLikeCpp::Root { root: plan.root } };
    plan.mutations.push(PlannedAcquisitionMutationLikeCpp::Skill(transition.clone()));
    plan.skill_transitions.push(transition);
    plan.resulting_snapshot.skills.push(riding);
    plan.resulting_snapshot.occupied_skill_slots = 1;
    plan.profession_association_inputs.push(riding);
    plan.post_commit_actions = vec![
        SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id: riding_id },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: riding_id },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateAchieveSkillStepCriteria { skill_id: riding_id },
    ];
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    let mut duplicate = plan.clone();
    duplicate.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: riding_id });
    assert_eq!(validate_acquisition_plan(&duplicate), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "skill action", id: riding_id }));
    duplicate.post_commit_actions.pop();
    duplicate.post_commit_actions.push(SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability { skill_id: riding_id });
    assert_eq!(validate_acquisition_plan(&duplicate), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "UpdateMountCapability", id: riding_id }));
}

#[test]
fn invalid_action_ids_keep_the_original_error_payload() {
    for (action, domain, id) in [
        (SpellAcquisitionPostCommitActionLikeCpp::SupersededSpell { old_spell_id: 0, new_spell_id: u32::MAX }, "spell", 0),
        (SpellAcquisitionPostCommitActionLikeCpp::UpdateSkillRaisedCriteria { skill_id: u32::from(u16::MAX) + 1 }, "skill", u32::from(u16::MAX) + 1),
        (SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { source_spell_id: u32::MAX, effect_record_id: 0, effect_index: 0 }, "spell", u32::MAX),
    ] {
        let mut plan = unchanged_plan(snapshot(Vec::new()));
        plan.post_commit_actions.push(action);
        assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::InvalidPostCommitAction { domain, id }));
    }
}
