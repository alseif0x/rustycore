//! Ordered failures at the pure plan boundary, without application authority.

use super::*;

#[test]
fn root_rejection_precedes_snapshot_shape() {
    for root in [SpellAcquisitionRootLikeCpp::DirectLearn(0), SpellAcquisitionRootLikeCpp::TrainerWrapperCast(u32::MAX)] {
        let mut plan = unchanged_plan(snapshot(vec![spell(0, PlayerSpellPersistenceStateLikeCpp::Unchanged)]));
        plan.root = root;
        let id = match root {
            SpellAcquisitionRootLikeCpp::DirectLearn(id) | SpellAcquisitionRootLikeCpp::TrainerWrapperCast(id) => id,
        };
        assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::InvalidSpellId(id)));
    }
    let mut valid = unchanged_plan(snapshot(Vec::new()));
    valid.root = SpellAcquisitionRootLikeCpp::TrainerWrapperCast(i32::MAX as u32);
    assert_eq!(validate_acquisition_plan(&valid), Ok(()));
}

#[test]
fn source_shape_precedes_result_shape_and_identity() {
    let mut plan = unchanged_plan(snapshot(vec![spell(0, PlayerSpellPersistenceStateLikeCpp::Unchanged)]));
    plan.resulting_snapshot.spells.clear();
    plan.resulting_snapshot.skills.push(skill(0));
    plan.resulting_snapshot.race = 2;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::InvalidSpellId(0)));
}

#[test]
fn result_shape_precedes_identity_and_identity_precedes_mutations() {
    let (_source, mut plan) = direct_learn_plan();
    plan.resulting_snapshot.race = 2;
    plan.resulting_snapshot.skills.push(skill(0));
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::InvalidSkillId(0)));

    plan.resulting_snapshot.skills.clear();
    plan.resulting_snapshot.character_guid = Some(wow_core::ObjectGuid::create_player(1, 43));
    let PlannedAcquisitionMutationLikeCpp::Spell(transition) = &mut plan.mutations[0] else { panic!("spell mutation") };
    transition.provenance = SpellAcquisitionProvenanceLikeCpp::Root { root: SpellAcquisitionRootLikeCpp::DirectLearn(0) };
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::SnapshotIdentityChanged("character_guid")));
    plan.resulting_snapshot.character_guid = plan.source_snapshot.character_guid;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::SnapshotIdentityChanged("race")));
}

#[test]
fn stream_checks_provenance_then_ids_then_before_rows_then_projection() {
    let (_source, mut plan) = direct_learn_plan();
    let PlannedAcquisitionMutationLikeCpp::Spell(transition) = &mut plan.mutations[0] else { panic!("spell mutation") };
    transition.provenance = SpellAcquisitionProvenanceLikeCpp::Root { root: SpellAcquisitionRootLikeCpp::DirectLearn(200) };
    transition.spell_id = 999;
    transition.before = Some(spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged));
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::ProvenanceMismatch("invalid or root-mismatched transition provenance")));

    let PlannedAcquisitionMutationLikeCpp::Spell(transition) = &mut plan.mutations[0] else { panic!("spell mutation") };
    transition.provenance = SpellAcquisitionProvenanceLikeCpp::Root { root: plan.root };
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TransitionIdMismatch { domain: "spell", id: 999 }));
    let PlannedAcquisitionMutationLikeCpp::Spell(transition) = &mut plan.mutations[0] else { panic!("spell mutation") };
    transition.spell_id = 100;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TransitionBeforeMismatch { domain: "spell", id: 100 }));
    let PlannedAcquisitionMutationLikeCpp::Spell(transition) = &mut plan.mutations[0] else { panic!("spell mutation") };
    transition.before = None;
    plan.spell_transitions.clear();
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TypedProjectionMismatch("spell_transitions")));
}

#[test]
fn typed_projections_precede_result_replay_and_action_checks() {
    let (_source, mut plan) = direct_learn_plan();
    plan.spell_transitions.clear();
    plan.resulting_snapshot.spells.clear();
    plan.post_commit_actions.clear();
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TypedProjectionMismatch("spell_transitions")));

    let mut skill_plan = unchanged_plan(snapshot(Vec::new()));
    skill_plan.skill_transitions.push(PlannedSkillTransitionLikeCpp {
        skill_id: 164,
        before: None,
        after: skill(164),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root { root: skill_plan.root },
    });
    assert_eq!(validate_acquisition_plan(&skill_plan), Err(AcquisitionPlanError::TypedProjectionMismatch("skill_transitions")));
    skill_plan.skill_transitions.clear();
    skill_plan.override_transitions.push(PlannedOverrideTransitionLikeCpp { overridden_spell_id: 100, overriding_spell_id: 200, add: true });
    assert_eq!(validate_acquisition_plan(&skill_plan), Err(AcquisitionPlanError::TypedProjectionMismatch("override_transitions")));
}

#[test]
fn resulting_replay_precedes_actions_and_actions_precede_profession_inputs() {
    let (_source, mut plan) = direct_learn_plan();
    plan.resulting_snapshot.spells[0].favorite = true;
    plan.post_commit_actions.clear();
    plan.profession_association_inputs.push(skill(164));
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::ResultingSnapshotMismatch));
    plan.resulting_snapshot.spells[0].favorite = false;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::PostCommitActionCausalityMismatch { action: "required publication tape", id: 0 }));
    plan.post_commit_actions = direct_learn_actions(100, false, false);
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::ProfessionInputsMismatch));
}

#[test]
fn profession_inputs_compare_exact_rows_after_sorting_ids() {
    let mut source = snapshot(Vec::new());
    source.skills = vec![skill(165), skill(164)];
    source.occupied_skill_slots = 2;
    let mut plan = unchanged_plan(source);
    plan.profession_association_inputs.reverse();
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    plan.profession_association_inputs[0].value = 2;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::ProfessionInputsMismatch));
}

#[test]
fn deleted_slots_and_temporary_spells_remain_represented() {
    let mut source = snapshot(vec![spell(100, PlayerSpellPersistenceStateLikeCpp::Temporary)]);
    let mut deleted = skill(164);
    deleted.step = 0;
    deleted.value = 0;
    deleted.maximum = 0;
    deleted.state = PlayerSkillPersistenceStateLikeCpp::Deleted;
    source.skills.push(deleted);
    source.occupied_skill_slots = 1;
    source.non_durable_skill_tombstone_ids.push(164);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Ok(()));
    source.occupied_skill_slots = 0;
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source)), Err(AcquisitionPlanError::SkillOccupancyMismatch));
}

#[test]
fn tombstone_occupancy_profession_and_trait_checks_keep_their_order() {
    let mut source = snapshot(vec![spell(100, PlayerSpellPersistenceStateLikeCpp::Temporary)]);
    let mut deleted = skill(164);
    deleted.step = 0;
    deleted.value = 0;
    deleted.maximum = 0;
    deleted.state = PlayerSkillPersistenceStateLikeCpp::Deleted;
    source.skills.push(deleted);
    source.occupied_skill_slots = 1;
    source.primary_profession_skill_ids.push(164);
    source.non_durable_skill_tombstone_ids = vec![164, 164];
    source.spells[0].trait_definition_id = Some(0);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::ProfessionPlanMismatch("snapshot primary profession authority")));
    source.primary_profession_skill_ids.clear();
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::InvalidNonDurableSkillTombstone(164)));
    source.non_durable_skill_tombstone_ids.clear();
    source.skills[0].value = 1;
    source.occupied_skill_slots = 0;
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::SkillOccupancyMismatch));
    source.occupied_skill_slots = 1;
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::InvalidDeletedSkill(164)));
    source.skills[0].state = PlayerSkillPersistenceStateLikeCpp::Unchanged;
    source.skills[0].profession_association = ProfessionAssociationInputLikeCpp::Slot(2);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::InvalidProfessionAssociation(2)));
    source.skills[0].profession_association = ProfessionAssociationInputLikeCpp::Unassigned;
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source)), Err(AcquisitionPlanError::InvalidTraitDefinitionId(0)));
}

#[test]
fn individual_row_projections_preserve_id_bounds_and_duplicate_payloads() {
    let mut source = snapshot(vec![spell(i32::MAX as u32, PlayerSpellPersistenceStateLikeCpp::Unchanged)]);
    assert_eq!(acquisition_spell_rows(&source).unwrap().len(), 1);
    source.spells.push(source.spells[0]);
    assert_eq!(acquisition_spell_rows(&source), Err(AcquisitionPlanError::DuplicateSpell(i32::MAX as u32)));
    source.spells[1].spell_id = u32::MAX;
    assert_eq!(acquisition_spell_rows(&source), Err(AcquisitionPlanError::InvalidSpellId(u32::MAX)));
    source.skills = vec![skill(u32::from(u16::MAX))];
    assert_eq!(acquisition_skill_rows(&source).unwrap().len(), 1);
    source.skills.push(source.skills[0]);
    assert_eq!(acquisition_skill_rows(&source), Err(AcquisitionPlanError::DuplicateSkill(u32::from(u16::MAX))));
    source.skills[1].skill_id = u32::from(u16::MAX) + 1;
    assert_eq!(acquisition_skill_rows(&source), Err(AcquisitionPlanError::InvalidSkillId(u32::from(u16::MAX) + 1)));
    source.overrides = vec![(100, 200), (100, 200)];
    assert_eq!(acquisition_override_pairs(&source), Err(AcquisitionPlanError::DuplicateOverride(100, 200)));
    source.overrides = vec![(0, u32::MAX)];
    assert_eq!(acquisition_override_pairs(&source), Err(AcquisitionPlanError::InvalidSpellId(0)));
}

#[test]
fn override_replay_requires_each_ordered_add_and_remove() {
    let mut plan = unchanged_plan(snapshot(Vec::new()));
    let added = PlannedOverrideTransitionLikeCpp { overridden_spell_id: 100, overriding_spell_id: 200, add: true };
    let removed = PlannedOverrideTransitionLikeCpp { add: false, ..added };
    plan.override_transitions = vec![added, removed];
    plan.mutations = vec![PlannedAcquisitionMutationLikeCpp::Override(added), PlannedAcquisitionMutationLikeCpp::Override(removed)];
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    plan.mutations[1] = PlannedAcquisitionMutationLikeCpp::Override(added);
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::DuplicateOverrideMutation { overridden: 100, overriding: 200 }));
    plan.mutations = vec![PlannedAcquisitionMutationLikeCpp::Override(removed)];
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::MissingOverrideMutation { overridden: 100, overriding: 200 }));
}

#[test]
fn skill_stream_preserves_exact_before_and_id_guards() {
    let mut source = snapshot(Vec::new());
    source.skills.push(skill(164));
    source.occupied_skill_slots = 1;
    let mut plan = unchanged_plan(source);
    let mut changed = skill(164);
    changed.value = 2;
    let transition = PlannedSkillTransitionLikeCpp {
        skill_id: 164,
        before: Some(skill(164)),
        after: changed,
        provenance: SpellAcquisitionProvenanceLikeCpp::Root { root: plan.root },
    };
    plan.mutations.push(PlannedAcquisitionMutationLikeCpp::Skill(transition.clone()));
    plan.skill_transitions.push(transition);
    plan.resulting_snapshot.skills[0] = changed;
    plan.profession_association_inputs = vec![changed];
    assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    let PlannedAcquisitionMutationLikeCpp::Skill(transition) = &mut plan.mutations[0] else { panic!("skill mutation") };
    transition.before.as_mut().unwrap().value = 3;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TransitionBeforeMismatch { domain: "skill", id: 164 }));
    let PlannedAcquisitionMutationLikeCpp::Skill(transition) = &mut plan.mutations[0] else { panic!("skill mutation") };
    transition.after.skill_id = 165;
    assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::TransitionIdMismatch { domain: "skill", id: 164 }));
}

#[test]
fn profession_slots_and_traits_remain_snapshot_authority() {
    let mut source = snapshot(vec![spell(100, PlayerSpellPersistenceStateLikeCpp::Temporary)]);
    source.spells[0].trait_definition_id = Some(10);
    source.skills = vec![skill(164), skill(165)];
    source.skills[0].profession_association = ProfessionAssociationInputLikeCpp::Slot(0);
    source.skills[1].profession_association = ProfessionAssociationInputLikeCpp::Slot(1);
    source.primary_profession_skill_ids = vec![164, 165];
    source.occupied_skill_slots = 2;
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Ok(()));
    source.skills[1].profession_association = ProfessionAssociationInputLikeCpp::Slot(0);
    source.spells[0].trait_definition_id = Some(0);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source.clone())), Err(AcquisitionPlanError::ConflictingProfessionAssociation(0)));
    source.skills[1].profession_association = ProfessionAssociationInputLikeCpp::Invalid(-2);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(source)), Err(AcquisitionPlanError::InvalidProfessionAssociation(-2)));

    let mut tombstone = skill(164);
    tombstone.step = 0;
    tombstone.value = 0;
    tombstone.maximum = 0;
    let mut saved = snapshot(Vec::new());
    saved.character_guid = None;
    saved.skills.push(tombstone);
    saved.occupied_skill_slots = 1;
    saved.non_durable_skill_tombstone_ids.push(164);
    assert_eq!(validate_acquisition_plan(&unchanged_plan(saved)), Ok(()));
}

#[test]
fn provenance_keeps_each_existing_id_and_record_gate() {
    let valid = [
        SpellAcquisitionProvenanceLikeCpp::PreviousRank { requested_spell_id: i32::MAX as u32 },
        SpellAcquisitionProvenanceLikeCpp::LearnDependency { source_spell_id: 100 },
        SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank { source_spell_id: 100 },
        SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell { required_spell_id: 100 },
        SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill { source_spell_id: 100 },
        SpellAcquisitionProvenanceLikeCpp::SkillLineAbilityFallback { source_spell_id: 100, record_id: 1 },
        SpellAcquisitionProvenanceLikeCpp::ParentSkill { child_skill_id: u32::from(u16::MAX) },
        SpellAcquisitionProvenanceLikeCpp::RootChildSkill { parent_skill_id: u32::from(u16::MAX) },
        SpellAcquisitionProvenanceLikeCpp::SkillReward { skill_id: 164, record_id: 1 },
        SpellAcquisitionProvenanceLikeCpp::WrapperEffect { wrapper_spell_id: 100, effect_index: u8::MAX, record_id: 1 },
        SpellAcquisitionProvenanceLikeCpp::AutocastEffect { source_spell_id: 100, effect_index: u8::MAX, record_id: 1 },
    ];
    let invalid = [
        SpellAcquisitionProvenanceLikeCpp::PreviousRank { requested_spell_id: u32::MAX },
        SpellAcquisitionProvenanceLikeCpp::LearnDependency { source_spell_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::HigherDisabledRank { source_spell_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::RequiredDisabledSpell { required_spell_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::DirectLearnSkill { source_spell_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::SkillLineAbilityFallback { source_spell_id: 100, record_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::ParentSkill { child_skill_id: u32::from(u16::MAX) + 1 },
        SpellAcquisitionProvenanceLikeCpp::RootChildSkill { parent_skill_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::SkillReward { skill_id: 164, record_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::WrapperEffect { wrapper_spell_id: 100, effect_index: 0, record_id: 0 },
        SpellAcquisitionProvenanceLikeCpp::AutocastEffect { source_spell_id: 100, effect_index: 0, record_id: 0 },
    ];
    for provenance in valid {
        let (_source, mut plan) = direct_learn_plan();
        plan.spell_transitions[0].provenance = provenance;
        plan.mutations[0] = PlannedAcquisitionMutationLikeCpp::Spell(plan.spell_transitions[0].clone());
        assert_eq!(validate_acquisition_plan(&plan), Ok(()));
    }
    for provenance in invalid {
        let (_source, mut plan) = direct_learn_plan();
        plan.spell_transitions[0].provenance = provenance;
        plan.mutations[0] = PlannedAcquisitionMutationLikeCpp::Spell(plan.spell_transitions[0].clone());
        assert_eq!(validate_acquisition_plan(&plan), Err(AcquisitionPlanError::ProvenanceMismatch("invalid or root-mismatched transition provenance")));
    }
}
