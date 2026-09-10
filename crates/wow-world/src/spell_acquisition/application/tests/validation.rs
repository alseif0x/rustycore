//! Validation packets.
//!
//! Separated from tests.rs under #709.

use super::*;

#[test]
fn stable_source_authority_is_exact_and_rejects_unsaved_state() {
    let mut favorite = spell(200, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    favorite.favorite = true;
    let mut dependent = spell(201, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    dependent.dependent = true;
    dependent.favorite = true;
    let mut source = snapshot(vec![favorite, dependent]);
    source.skills = vec![PlayerSkillAcquisitionRowLikeCpp {
        skill_id: 164,
        step: 1,
        value: 75,
        maximum: 150,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(1),
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    }];
    source.occupied_skill_slots = 1;

    assert_eq!(
        stable_source_durable_authority_like_cpp(&source),
        Some(DurablePlayerSpellAcquisitionAuthorityLikeCpp {
            spells: vec![DurablePlayerSpellRowLikeCpp {
                spell_id: 200,
                active: true,
                disabled: false,
            }],
            favorite_spell_ids: vec![200, 201],
            skills: vec![DurablePlayerSkillRowLikeCpp {
                skill_id: 164,
                value: 75,
                maximum: 150,
                profession_slot: 1,
            }],
        })
    );

    let mut unsaved_spell = source.clone();
    unsaved_spell.spells[0].state = PlayerSpellPersistenceStateLikeCpp::Changed;
    assert!(stable_source_durable_authority_like_cpp(&unsaved_spell).is_none());
    assert!(snapshot_has_pending_durable_save_like_cpp(&unsaved_spell));

    let mut with_temporary_spell = source.clone();
    with_temporary_spell
        .spells
        .push(spell(202, PlayerSpellPersistenceStateLikeCpp::Temporary));
    assert_eq!(
        stable_source_durable_authority_like_cpp(&with_temporary_spell),
        stable_source_durable_authority_like_cpp(&source)
    );
    assert!(!snapshot_has_pending_durable_save_like_cpp(
        &with_temporary_spell
    ));

    let mut unsaved_skill = source;
    unsaved_skill.skills[0].state = PlayerSkillPersistenceStateLikeCpp::New;
    assert!(stable_source_durable_authority_like_cpp(&unsaved_skill).is_none());
    assert!(snapshot_has_pending_durable_save_like_cpp(&unsaved_skill));
}

#[test]
fn required_learning_publications_cannot_be_omitted() {
    let (source, plan) = direct_learn_plan();
    prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
        .expect("complete publication tape is valid");

    for omitted_index in 0..plan.post_commit_actions.len() {
        let mut omitted = plan.clone();
        omitted.post_commit_actions.remove(omitted_index);
        assert!(matches!(
            prepare_player_spell_acquisition_like_cpp(&omitted, &no_profession_changes(), &source,),
            Err(
                PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch { .. }
            )
        ));
    }
}

#[test]
fn skill_line_criteria_require_exact_occurrence_identity_and_cardinality() {
    let (source, mut plan) = direct_learn_plan();
    plan.publication_requirements.splice(
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
    prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
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
        prepare_player_spell_acquisition_like_cpp(&wrong_skill, &no_profession_changes(), &source,)
            .is_err()
    );

    let mut duplicated = plan.clone();
    duplicated
        .post_commit_actions
        .splice(2..2, plan.post_commit_actions[..2].iter().cloned());
    assert!(
        prepare_player_spell_acquisition_like_cpp(&duplicated, &no_profession_changes(), &source,)
            .is_err()
    );

    let mut omitted = plan.clone();
    omitted.post_commit_actions.drain(..2);
    assert!(
        prepare_player_spell_acquisition_like_cpp(&omitted, &no_profession_changes(), &source,)
            .is_err()
    );
}

#[test]
fn profession_plan_cannot_omit_an_existing_primary_profession() {
    const EXISTING: u32 = 164;
    const NEW: u32 = 165;
    let existing = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: EXISTING,
        step: 1,
        value: 75,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(0),
        state: PlayerSkillPersistenceStateLikeCpp::Unchanged,
    };
    let learned = PlayerSkillAcquisitionRowLikeCpp {
        skill_id: NEW,
        step: 1,
        value: 1,
        maximum: 75,
        profession_association: ProfessionAssociationInputLikeCpp::Slot(1),
        state: PlayerSkillPersistenceStateLikeCpp::New,
    };
    let mut source = snapshot(Vec::new());
    source.skills = vec![existing];
    source.occupied_skill_slots = 1;
    source.primary_profession_skill_ids = vec![EXISTING];
    let mut resulting = source.clone();
    resulting.skills.push(learned);
    resulting.occupied_skill_slots = 2;
    resulting.primary_profession_skill_ids = vec![EXISTING, NEW];
    let transition = PlannedSkillTransitionLikeCpp {
        skill_id: NEW,
        before: None,
        after: learned,
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
        root_primary_profession_skill_ids: vec![NEW],
        publication_requirements: Vec::new(),
        profession_association_inputs: vec![existing, learned],
        post_commit_actions: Vec::new(),
        diagnostics: Vec::new(),
        resulting_snapshot: resulting,
    };
    let omitted_existing = PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: vec![crate::profession::PlannedPrimaryProfessionLikeCpp {
            skill_id: NEW,
            equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
        }],
        slot_normalizations: Vec::new(),
    };

    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &omitted_existing, &source),
        Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::ProfessionPlanMismatch(
                "complete existing profession membership"
            )
        )
    );
}

#[test]
fn prepared_acquisition_cannot_cross_character_boundaries() {
    let (source, plan) = direct_learn_plan();
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid prepared acquisition")
    else {
        panic!("expected durable acquisition")
    };
    assert!(
        player_spell_acquisition_persistence_request_like_cpp(43, &prepared, 100, 80, [7; 16],)
            .is_err()
    );

    let (mut other_session, send_rx) = make_session();
    other_session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        wow_core::ObjectGuid::create_player(1, 43),
        "OtherAcquisitionPlayer".to_string(),
        wow_core::Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    assert_eq!(
        apply_prepared_player_spell_acquisition_like_cpp(&mut other_session, &prepared),
        Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime)
    );
    assert!(
        other_session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none(),
        "cross-character rejection occurs before installing runtime state"
    );
    assert!(send_rx.is_empty());
}

#[test]
fn prepared_plan_rejects_stale_or_tampered_authority_before_sql() {
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
    let mut plan = SpellAcquisitionPlanLikeCpp {
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
        resulting_snapshot: snapshot(vec![learned]),
    };
    let stale = snapshot(vec![spell(
        99,
        PlayerSpellPersistenceStateLikeCpp::Unchanged,
    )]);
    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &stale),
        Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::StaleSnapshot)
    );

    plan.spell_transitions.clear();
    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
        Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::TypedProjectionMismatch("spell_transitions")
        )
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
    let plan = SpellAcquisitionPlanLikeCpp {
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
    };
    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
        Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
    );
}

#[test]
fn learned_action_requires_a_causal_learning_transition() {
    let unchanged = spell(100, PlayerSpellPersistenceStateLikeCpp::Unchanged);
    let source = snapshot(vec![unchanged]);
    let plan = SpellAcquisitionPlanLikeCpp {
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
    };

    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source,),
        Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
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
    let plan = SpellAcquisitionPlanLikeCpp {
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
    };

    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
        Err(PlayerSpellAcquisitionPrepareErrorLikeCpp::LearnedActionRowMismatch(100))
    );
}

#[test]
fn dual_wield_diagnostic_cannot_replace_live_cast_effect_authority() {
    let source = snapshot(Vec::new());
    let plan = SpellAcquisitionPlanLikeCpp {
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
    };

    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source),
        Err(
            PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch {
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
    let base_plan = SpellAcquisitionPlanLikeCpp {
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
    };
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
                prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source,),
                Err(
                    PlayerSpellAcquisitionPrepareErrorLikeCpp::PostCommitActionCausalityMismatch { .. }
                )
            ),
            "unrelated action reached the prepared boundary: {action:?}"
        );
    }
}
