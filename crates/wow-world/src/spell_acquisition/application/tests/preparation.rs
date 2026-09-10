//! Preparation packets.
//!
//! Separated from tests.rs under #709.

use super::*;

#[test]
fn prepared_plan_replays_causal_stream_and_normalizes_post_save_state() {
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
    let resulting = snapshot(vec![learned]);
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: direct_learn_requirements(100, false, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: direct_learn_actions(100, false, false),
        diagnostics: Vec::new(),
        resulting_snapshot: resulting,
    };

    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid plan")
    else {
        panic!("expected a prepared mutation")
    };
    assert_eq!(prepared.durable_spells.len(), 1);
    assert_eq!(
        prepared.pending_save_runtime_snapshot.spells[0].state,
        PlayerSpellPersistenceStateLikeCpp::New,
        "C++ LearnSpell keeps the new row dirty until Player::SaveToDB"
    );
    assert_eq!(
        prepared.runtime_snapshot.spells[0].state,
        PlayerSpellPersistenceStateLikeCpp::Unchanged
    );
    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(
            &plan,
            &no_profession_changes(),
            &prepared.runtime_snapshot,
        ),
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied)
    );
}

#[test]
fn prepared_plan_builds_one_deterministic_full_replacement_transaction() {
    let source = snapshot(Vec::new());
    let mut learned = spell(100, PlayerSpellPersistenceStateLikeCpp::New);
    learned.favorite = true;
    learned.trait_definition_id = Some(7);
    let transition = PlannedSpellTransitionLikeCpp {
        spell_id: 100,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: direct_learn_requirements(100, true, true),
        profession_association_inputs: Vec::new(),
        post_commit_actions: direct_learn_actions(100, true, true),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid plan")
    else {
        panic!("expected ready plan")
    };

    assert_eq!(
        prepared.durable_operations,
        vec![
            PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
            PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
            PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(
                DurablePlayerSpellRowLikeCpp {
                    spell_id: 100,
                    active: true,
                    disabled: false,
                }
            ),
            PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(100),
        ]
    );
    let request =
        player_spell_acquisition_persistence_request_like_cpp(42, &prepared, 100, 80, [9; 16])
            .expect("the validated plan has one complete SQLx-free request");
    assert_eq!(request.player_guid, 42);
    assert_eq!(request.money_before, 100);
    assert_eq!(request.money_after, 80);
    assert_eq!(request.operation_token, [9; 16]);
    assert_eq!(request.operations, prepared.durable_operations);
    assert_eq!(request.resulting_authority.spells, prepared.durable_spells);
}

#[test]
fn action_only_plan_publishes_without_preparing_a_durable_rewrite() {
    let unchanged = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    let source = snapshot(vec![unchanged]);
    let action =
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id: 100 };
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: Vec::new(),
        spell_transitions: Vec::new(),
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: vec![
            SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: 100,
            },
        ],
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![action.clone()],
        diagnostics: Vec::new(),
        resulting_snapshot: source.clone(),
    };

    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid action-only plan")
    else {
        panic!("an action-only learn must not prepare durable replacement operations")
    };
    assert_eq!(prepared.runtime_snapshot, source);

    let (mut session, send_rx) = make_session();
    session.record_spell_acquisition_post_commit_action_like_cpp(
        SpellAcquisitionPostCommitActionLikeCpp::UpdateMountCapability {
            skill_id: u32::from(crate::session::SKILL_RIDING_LIKE_CPP),
        },
    );
    apply_prepared_player_spell_acquisition_actions_like_cpp(&mut session, &prepared)
        .expect("publish action-only plan");
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        &[action],
        "the current acquisition batch replaces earlier retained intent instead of growing forever"
    );
    assert!(send_rx.is_empty());
}

#[test]
fn profession_capacity_assignment_is_applied_to_durable_and_runtime_rows() {
    const PROFESSION: u32 = 164;
    let source = snapshot(Vec::new());
    let learned_skill = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: PROFESSION,
        step: 1,
        value: 1,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::New,
    };
    let transition = PlannedSkillTransitionLikeCpp {
        skill_id: PROFESSION,
        before: None,
        after: learned_skill,
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let mut resulting = source.clone();
    resulting.skills.push(learned_skill);
    resulting.occupied_skill_slots = 1;
    resulting.primary_profession_skill_ids = vec![PROFESSION];
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
        spell_transitions: Vec::new(),
        skill_transitions: vec![transition],
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: vec![PROFESSION],
        publication_requirements: Vec::new(),
        profession_association_inputs: vec![learned_skill],
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: resulting,
    };
    let profession_plan = PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
            skill_id: PROFESSION,
            equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
        }],
        slot_normalizations: Vec::new(),
    };

    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source)
            .expect("valid profession assignment")
    else {
        panic!("expected ready profession plan")
    };
    assert_eq!(prepared.durable_skills[0].profession_slot, 0);
    assert_eq!(
        prepared.runtime_snapshot.skills[0].profession_association,
        ProfessionAssociationInputLikeCpp::Slot(0)
    );
}

#[test]
fn deleted_skill_tombstone_is_retained_only_in_runtime_after_save() {
    let existing = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: 95,
        step: 1,
        value: 75,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    };
    let deleted = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: 95,
        step: 0,
        value: 0,
        maximum: 0,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Deleted,
    };
    let mut source = snapshot(Vec::new());
    source.skills.push(existing);
    source.occupied_skill_slots = 1;
    let mut resulting = source.clone();
    resulting.skills[0] = deleted;
    let transition = PlannedSkillTransitionLikeCpp {
        skill_id: 95,
        before: Some(existing),
        after: deleted,
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
        spell_transitions: Vec::new(),
        skill_transitions: vec![transition],
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: vec![deleted],
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: resulting,
    };

    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid deleted skill")
    else {
        panic!("expected ready deleted-skill plan")
    };
    assert!(prepared.durable_skills.is_empty());
    assert_eq!(
        prepared.non_durable_skill_tombstone_ids,
        BTreeSet::from([95])
    );
    assert_eq!(
        prepared.runtime_snapshot.skills[0].state,
        PlayerSkillPersistenceStateLikeCpp::Unchanged
    );
    assert_eq!(prepared.runtime_snapshot.skills[0].value, 0);
    assert_eq!(
        prepared.runtime_snapshot.non_durable_skill_tombstone_ids,
        vec![95]
    );

    let (mut session, _) = make_session();
    apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
        .expect("apply committed deleted-skill snapshot");

    let saved_tombstone_source = prepared.runtime_snapshot.clone();
    let learned = spell(200, PlayerSpellPersistenceStateLikeCpp::New);
    let spell_transition = PlannedSpellTransitionLikeCpp {
        spell_id: 200,
        before: None,
        after: Some(learned),
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(200),
        },
    };
    let mut saved_tombstone_resulting = saved_tombstone_source.clone();
    saved_tombstone_resulting.spells.push(learned);
    let saved_tombstone_plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(200),
        source_snapshot: saved_tombstone_source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(
            spell_transition.clone(),
        )],
        spell_transitions: vec![spell_transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: saved_tombstone_source.skills.clone(),
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: saved_tombstone_resulting,
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(saved_tombstone_prepared) =
        prepare_player_spell_acquisition_like_cpp(
            &saved_tombstone_plan,
            &no_profession_changes(),
            &saved_tombstone_source,
        )
        .expect("an unrelated acquisition preserves the saved tombstone")
    else {
        panic!("expected ready plan with saved tombstone")
    };
    assert!(
        saved_tombstone_prepared.durable_skills.is_empty(),
        "an unrelated acquisition must not resurrect an already-saved zero skill row"
    );
    assert_eq!(
        saved_tombstone_prepared.non_durable_skill_tombstone_ids,
        BTreeSet::from([95])
    );

    let relearned = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: 95,
        step: 1,
        value: 1,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Unassigned,
        state: PlayerSkillPersistenceStateLikeCpp::Changed,
    };
    let relearn_source = prepared.runtime_snapshot.clone();
    let mut relearn_resulting = relearn_source.clone();
    relearn_resulting.skills[0] = relearned;
    relearn_resulting.non_durable_skill_tombstone_ids.clear();
    let relearn_transition = PlannedSkillTransitionLikeCpp {
        skill_id: 95,
        before: Some(relearn_source.skills[0]),
        after: relearned,
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(101),
        },
    };
    let relearn_plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(101),
        source_snapshot: relearn_source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(
            relearn_transition.clone(),
        )],
        spell_transitions: Vec::new(),
        skill_transitions: vec![relearn_transition],
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: Vec::new(),
        profession_association_inputs: vec![relearned],
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: relearn_resulting,
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(relearn_prepared) =
        prepare_player_spell_acquisition_like_cpp(
            &relearn_plan,
            &no_profession_changes(),
            &relearn_source,
        )
        .expect("C++ SetSkill reactivates a saved deleted skill")
    else {
        panic!("expected ready relearn plan")
    };

    apply_prepared_player_spell_acquisition_before_save_like_cpp(&mut session, &relearn_prepared)
        .expect("saved tombstone must not block relearning the skill");
    assert!(
        !session
            .player_skill_non_durable_tombstones_like_cpp()
            .contains(&95)
    );
    assert_eq!(session.player_skill_records_like_cpp()[&95].value, 1);
    assert_eq!(
        session.player_skill_records_like_cpp()[&95].state,
        crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
    );
}

#[test]
fn committed_skill_snapshot_refreshes_enchanting_runtime_projection() {
    let enchanting = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: crate::session::SKILL_ENCHANTING_LIKE_CPP.into(),
        step: 2,
        value: 150,
        maximum: 225,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(0),
        state: PlayerSkillPersistenceStateLikeCpp::New,
    };
    let source = snapshot(Vec::new());
    let mut resulting = source.clone();
    resulting.skills.push(enchanting);
    resulting.occupied_skill_slots = 1;
    resulting.primary_profession_skill_ids = vec![enchanting.skill_id];
    let transition = PlannedSkillTransitionLikeCpp {
        skill_id: enchanting.skill_id,
        before: None,
        after: enchanting,
        provenance: SpellAcquisitionProvenanceLikeCpp::Root {
            root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        },
    };
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Skill(transition.clone())],
        spell_transitions: Vec::new(),
        skill_transitions: vec![transition],
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: vec![enchanting.skill_id],
        publication_requirements: Vec::new(),
        profession_association_inputs: vec![enchanting],
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: resulting,
    };
    let profession_plan = PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
            skill_id: enchanting.skill_id,
            equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
        }],
        slot_normalizations: Vec::new(),
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &profession_plan, &source)
            .expect("valid enchanting acquisition")
    else {
        panic!("expected ready enchanting plan")
    };

    let (mut session, _) = make_session();
    apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
        .expect("apply committed enchanting snapshot");
    assert_eq!(session.represented_enchanting_skill, 150);
}
