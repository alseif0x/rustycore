use super::*;

fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
    let (_packet_tx, packet_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = crate::session::WorldSession::new(
        1,
        "AcquisitionTest".to_string(),
        0,
        2,
        2,
        54261,
        vec![0; 40],
        "enUS".to_string(),
        packet_rx,
        send_tx,
    );
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        wow_core::ObjectGuid::create_player(1, 42),
        "AcquisitionPlayer".to_string(),
        wow_core::Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    (session, send_rx)
}

fn snapshot(
    spells: Vec<PlayerSpellAcquisitionRowLikeCpp>,
) -> PlayerSpellAcquisitionSnapshotLikeCpp {
    PlayerSpellAcquisitionSnapshotLikeCpp {
        character_guid: Some(wow_core::ObjectGuid::create_player(1, 42)),
        spells,
        skills: Vec::new(),
        occupied_skill_slots: 0,
        overrides: Vec::new(),
        primary_profession_skill_ids: Vec::new(),
        non_durable_skill_tombstone_ids: Vec::new(),
        race: 1,
        class: 1,
        level: 80,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
        future_player_condition_resolutions: Vec::new(),
        cast_resolutions: BTreeMap::new(),
    }
}

fn spell(
    spell_id: u32,
    state: PlayerSpellPersistenceStateLikeCpp,
) -> PlayerSpellAcquisitionRowLikeCpp {
    PlayerSpellAcquisitionRowLikeCpp {
        spell_id,
        active: true,
        disabled: false,
        dependent: false,
        favorite: false,
        trait_definition_id: None,
        state,
    }
}

fn no_profession_changes() -> PrimaryProfessionCapacityPlanLikeCpp {
    PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: 2,
        used_before: 0,
        free_before: 2,
        existing_professions: Vec::new(),
        new_professions: Vec::new(),
        slot_normalizations: Vec::new(),
    }
}

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

fn direct_learn_actions(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPostCommitActionLikeCpp> {
    vec![
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_requirements(
    spell_id: u32,
    favorite: bool,
    suppress_messaging: bool,
) -> Vec<SpellAcquisitionPublicationRequirementLikeCpp> {
    vec![
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnOrKnowSpellCriteria { spell_id },
        SpellAcquisitionPublicationRequirementLikeCpp::LearnedSpell {
            spell_id,
            favorite,
            suppress_messaging,
        },
        SpellAcquisitionPublicationRequirementLikeCpp::UpdateLearnSpellQuestObjective { spell_id },
    ]
}

fn direct_learn_plan() -> (
    PlayerSpellAcquisitionSnapshotLikeCpp,
    SpellAcquisitionPlanLikeCpp,
) {
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
        publication_requirements: direct_learn_requirements(100, false, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: direct_learn_actions(100, false, false),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    };
    (source, plan)
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
fn runtime_state_is_complete_before_ordered_publication() {
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
    let actions = direct_learn_actions(100, true, false);
    let plan = SpellAcquisitionPlanLikeCpp {
        root: SpellAcquisitionRootLikeCpp::DirectLearn(100),
        source_snapshot: source.clone(),
        mutations: vec![PlannedAcquisitionMutationLikeCpp::Spell(transition.clone())],
        spell_transitions: vec![transition],
        skill_transitions: Vec::new(),
        override_transitions: Vec::new(),
        root_primary_profession_skill_ids: Vec::new(),
        publication_requirements: direct_learn_requirements(100, true, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: actions.clone(),
        diagnostics: Vec::new(),
        resulting_snapshot: snapshot(vec![learned]),
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid plan")
    else {
        panic!("expected ready plan")
    };
    let (mut interrupted_session, interrupted_send_rx) = make_session();
    assert_eq!(
        apply_prepared_player_spell_acquisition_with_fault_like_cpp(
            &mut interrupted_session,
            &prepared,
            |point| (point != PlayerSpellAcquisitionPublicationFaultPointLikeCpp::BeforeAction(0))
                .then_some(())
                .ok_or(()),
        ),
        Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::PublicationInterrupted)
    );
    assert!(
        interrupted_session
            .complete_represented_player_spell_rows_like_cpp()
            .is_some(),
        "committed runtime state is installed before publication can be interrupted"
    );
    assert_eq!(interrupted_send_rx.len(), 0);
    assert!(
        interrupted_session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty()
    );
    let (mut pre_save_session, pre_save_send_rx) = make_session();
    apply_prepared_player_spell_acquisition_before_save_like_cpp(&mut pre_save_session, &prepared)
        .expect("validated C++-timed snapshot applies");
    assert_eq!(
        pre_save_session
            .complete_represented_player_spell_rows_like_cpp()
            .and_then(|rows| rows.get(&100).copied())
            .map(|row| row.state),
        Some(crate::session::RepresentedPlayerSpellStateLikeCpp::New),
        "EffectLearnSpell must publish immediately while leaving _SaveSpells dirty"
    );
    assert_eq!(pre_save_send_rx.len(), 1);
    let (mut session, send_rx) = make_session();

    apply_prepared_player_spell_acquisition_like_cpp(&mut session, &prepared)
        .expect("validated snapshot applies");

    let rows = session
        .complete_represented_player_spell_rows_like_cpp()
        .expect("complete rows installed");
    assert_eq!(rows.get(&100).map(|row| row.favorite), Some(true));
    assert_eq!(
        session
            .complete_represented_spell_trait_definition_ids_like_cpp()
            .and_then(|traits| traits.get(&100).copied()),
        Some(7)
    );
    assert_eq!(
        session.represented_spell_acquisition_post_commit_actions_like_cpp(),
        actions
    );
    assert_eq!(
        send_rx.len(),
        1,
        "the learned result publishes exactly once"
    );
    assert_eq!(
        prepare_player_spell_acquisition_like_cpp(
            &plan,
            &no_profession_changes(),
            &session
                .spell_acquisition_snapshot_like_cpp(
                    PlayerAcquisitionLifecycleLikeCpp::InWorld,
                    Vec::new(),
                    BTreeMap::new(),
                )
                .expect("installed state remains snapshot-complete"),
        ),
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied)
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

#[test]
fn dual_wield_missing_owner_stops_before_any_publication() {
    let mut source = snapshot(Vec::new());
    source.cast_resolutions.insert(
        100,
        PlayerCastAcquisitionResolutionLikeCpp {
            reached_immediate_phase: true,
            executed_hit_target_effect_mask: 1,
            effective_effects: Vec::new(),
            executed_dual_wield_effects: vec![PlayerExecutedDualWieldEffectLikeCpp {
                effect_record_id: 7,
                effect_index: 0,
            }],
        },
    );
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
        publication_requirements: direct_learn_requirements(100, false, false),
        profession_association_inputs: Vec::new(),
        post_commit_actions: vec![
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnOrKnowSpellCriteria {
                spell_id: 100,
            },
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell {
                spell_id: 100,
                favorite: false,
                suppress_messaging: false,
            },
            SpellAcquisitionPostCommitActionLikeCpp::UpdateLearnSpellQuestObjective {
                spell_id: 100,
            },
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
                source_spell_id: 100,
                effect_record_id: 7,
                effect_index: 0,
            },
        ],
        diagnostics: vec![
            SpellAcquisitionDiagnosticLikeCpp::DualWieldEffectProjected {
                spell_id: 100,
                effect_record_id: 7,
                effect_index: 0,
            },
        ],
        resulting_snapshot: PlayerSpellAcquisitionSnapshotLikeCpp {
            spells: vec![learned],
            ..source.clone()
        },
    };
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("valid dual-wield plan")
    else {
        panic!("expected ready plan")
    };
    let (mut session, send_rx) = make_session();
    let before_actions_ran = std::cell::Cell::new(false);

    assert_eq!(
        apply_prepared_player_spell_acquisition_with_before_actions_like_cpp(
            &mut session,
            &prepared,
            |_| before_actions_ran.set(true),
        ),
        Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime)
    );
    assert!(
        !before_actions_ran.get(),
        "money and trainer visuals must not start before every runtime owner preflights"
    );
    assert!(
        session
            .complete_represented_player_spell_rows_like_cpp()
            .is_none_or(|rows| !rows.contains_key(&100)),
        "a missing required runtime owner is rejected before replacing spell authority"
    );
    assert!(
        send_rx.is_empty(),
        "a missing required runtime owner is rejected before any success packet"
    );
    assert!(
        session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .is_empty(),
        "a missing required runtime owner is rejected before recording any action"
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FakeRuntimeEvent {
    Install,
    Begin,
    Record(SpellAcquisitionPostCommitActionLikeCpp),
    GrantDualWield,
    Publish(SpellAcquisitionPostCommitActionLikeCpp, Option<i32>),
}

#[derive(Default)]
struct FakeRuntime {
    character_guid: Option<wow_core::ObjectGuid>,
    canonical_player: bool,
    fail_install: bool,
    dual_wield_grants: usize,
    events: Vec<FakeRuntimeEvent>,
}

impl PlayerSpellAcquisitionRuntimeLikeCpp for FakeRuntime {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid> {
        self.character_guid
    }

    fn has_canonical_player(&self) -> bool {
        self.canonical_player
    }

    fn install_snapshot(
        &mut self,
        _snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        _new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
        if self.fail_install {
            return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
        }
        self.events.push(FakeRuntimeEvent::Install);
        Ok(())
    }

    fn begin_action_batch(&mut self) {
        self.events.push(FakeRuntimeEvent::Begin);
    }

    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp) {
        self.events.push(FakeRuntimeEvent::Record(action));
    }

    fn grant_dual_wield(&mut self) -> bool {
        self.dual_wield_grants += 1;
        self.events.push(FakeRuntimeEvent::GrantDualWield);
        self.canonical_player
    }

    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    ) {
        self.events.push(FakeRuntimeEvent::Publish(
            action.clone(),
            trait_definition_id,
        ));
    }
}

fn prepared_direct_learn_for_fake_runtime() -> PreparedPlayerSpellAcquisitionLikeCpp {
    let (source, plan) = direct_learn_plan();
    let PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared) =
        prepare_player_spell_acquisition_like_cpp(&plan, &no_profession_changes(), &source)
            .expect("direct learn plan is valid")
    else {
        panic!("expected ready direct learn plan");
    };
    prepared
}

#[test]
fn generic_runtime_installs_before_ordered_record_and_publish() {
    let prepared = prepared_direct_learn_for_fake_runtime();
    let mut runtime = FakeRuntime {
        character_guid: prepared.character_guid,
        canonical_player: true,
        ..Default::default()
    };
    let before_actions_called = std::cell::Cell::new(false);

    apply_prepared_player_spell_acquisition_with_before_actions_like_cpp(
        &mut runtime,
        &prepared,
        |_| before_actions_called.set(true),
    )
    .expect("fake runtime accepts the prepared snapshot");

    assert!(before_actions_called.get());
    assert_eq!(
        runtime.events.len(),
        2 + prepared.post_commit_actions.len() * 2
    );
    assert_eq!(runtime.events[0], FakeRuntimeEvent::Install);
    assert_eq!(runtime.events[1], FakeRuntimeEvent::Begin);
    for (index, action) in prepared.post_commit_actions.iter().enumerate() {
        assert_eq!(
            runtime.events[index * 2 + 2],
            FakeRuntimeEvent::Record(action.clone())
        );
        assert_eq!(
            runtime.events[index * 2 + 3],
            FakeRuntimeEvent::Publish(action.clone(), None)
        );
    }
}

#[test]
fn generic_runtime_fault_stops_before_later_record_or_publication() {
    let prepared = prepared_direct_learn_for_fake_runtime();
    let mut runtime = FakeRuntime {
        character_guid: prepared.character_guid,
        canonical_player: true,
        ..Default::default()
    };

    assert_eq!(
        apply_prepared_player_spell_acquisition_with_fault_like_cpp(
            &mut runtime,
            &prepared,
            |point| {
                (point != PlayerSpellAcquisitionPublicationFaultPointLikeCpp::BeforeAction(1))
                    .then_some(())
                    .ok_or(())
            },
        ),
        Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::PublicationInterrupted)
    );
    assert_eq!(runtime.events.len(), 4);
    assert!(matches!(runtime.events[0], FakeRuntimeEvent::Install));
    assert!(matches!(runtime.events[1], FakeRuntimeEvent::Begin));
    assert!(matches!(runtime.events[2], FakeRuntimeEvent::Record(_)));
    assert!(matches!(runtime.events[3], FakeRuntimeEvent::Publish(_, _)));
}

#[test]
fn generic_runtime_applies_dual_wield_once_across_deferred_publication() {
    let mut prepared = prepared_direct_learn_for_fake_runtime();
    prepared
        .post_commit_actions
        .push(SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield {
            source_spell_id: 100,
            effect_record_id: 7,
            effect_index: 0,
        });
    let mut runtime = FakeRuntime {
        character_guid: prepared.character_guid,
        canonical_player: true,
        ..Default::default()
    };

    let actions =
        install_prepared_player_spell_acquisition_runtime_like_cpp(&mut runtime, &prepared)
            .expect("fake runtime installs before deferred publication");
    apply_prepared_player_spell_acquisition_actions_like_cpp(&mut runtime, &actions)
        .expect("deferred action publication succeeds");

    assert_eq!(runtime.dual_wield_grants, 1);
}

#[test]
fn combined_commit_reconciliation_requires_money_and_exact_durable_result() {
    use PlayerSpellAcquisitionMoneyReconciliationLikeCpp::{Committed, Indeterminate};

    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, true, true),
        Committed
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 100, false, true),
        Indeterminate,
        "the old balance cannot prove rollback after an ambiguous COMMIT"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 100, true, true),
        Indeterminate,
        "a later writer may restore money after the acquisition rows committed"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, false, true),
        Indeterminate,
        "money alone must never authorize publication of an incomplete acquisition"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(100, 100, true, true),
        Committed,
        "a free trainer purchase is proven only by its complete durable result"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(100, 100, false, true,),
        Indeterminate,
        "an unchanged balance cannot prove rollback for a free purchase"
    );
    assert_eq!(
        classify_player_spell_acquisition_money_reconciliation_like_cpp(80, 80, true, false,),
        Indeterminate,
        "an identical later operation must not authorize this attempt's publication"
    );
}

#[test]
fn every_durable_fault_boundary_discards_the_whole_operation_prefix() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct DurableState {
        spells: Vec<DurablePlayerSpellRowLikeCpp>,
        favorites: Vec<i32>,
        skills: Vec<DurablePlayerSkillRowLikeCpp>,
    }

    fn execute_atomically(
        original: &DurableState,
        operations: &[PlayerSpellAcquisitionDurableOperationLikeCpp],
        fail_before_operation: Option<usize>,
        fail_before_commit: bool,
    ) -> DurableState {
        let mut transaction = original.clone();
        for (index, operation) in operations.iter().copied().enumerate() {
            if fail_before_operation == Some(index) {
                return original.clone();
            }
            match operation {
                PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter => {}
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells => {
                    transaction.spells.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells => {
                    transaction.favorites.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills => {
                    transaction.skills.clear()
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(row) => {
                    transaction.spells.push(row)
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(id) => {
                    transaction.favorites.push(id)
                }
                PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSkill(row) => {
                    transaction.skills.push(row)
                }
            }
        }
        if fail_before_commit {
            original.clone()
        } else {
            transaction
        }
    }

    let original = DurableState {
        spells: vec![DurablePlayerSpellRowLikeCpp {
            spell_id: 99,
            active: true,
            disabled: false,
        }],
        favorites: vec![99],
        skills: vec![DurablePlayerSkillRowLikeCpp {
            skill_id: 10,
            value: 1,
            maximum: 75,
            profession_slot: -1,
        }],
    };
    let operations = vec![
        PlayerSpellAcquisitionDurableOperationLikeCpp::LockCharacter,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteFavoriteSpells,
        PlayerSpellAcquisitionDurableOperationLikeCpp::DeleteSkills,
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertSpell(DurablePlayerSpellRowLikeCpp {
            spell_id: 100,
            active: true,
            disabled: false,
        }),
        PlayerSpellAcquisitionDurableOperationLikeCpp::InsertFavoriteSpell(100),
    ];

    for index in 0..operations.len() {
        assert_eq!(
            execute_atomically(&original, &operations, Some(index), false),
            original,
            "fault before operation {index} must roll back the entire prefix"
        );
    }
    assert_eq!(
        execute_atomically(&original, &operations, None, true),
        original,
        "fault at commit must not expose the transactional prefix"
    );
    assert_eq!(
        execute_atomically(&original, &operations, None, false),
        DurableState {
            spells: vec![DurablePlayerSpellRowLikeCpp {
                spell_id: 100,
                active: true,
                disabled: false,
            }],
            favorites: vec![100],
            skills: Vec::new(),
        }
    );
}

#[test]
fn trainer_fee_and_acquisition_share_every_rollback_boundary() {
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CombinedState {
        money: u64,
        spells: Vec<i32>,
        skills: Vec<u16>,
    }

    fn execute(original: &CombinedState, fail_before_step: Option<usize>) -> CombinedState {
        let mut transaction = original.clone();
        // Mirrors the combined transaction's destructive replacement,
        // deterministic inserts, guarded money update and COMMIT fence.
        let mut step = 0;
        macro_rules! transaction_step {
            ($body:expr) => {{
                if fail_before_step == Some(step) {
                    return original.clone();
                }
                $body;
                step += 1;
            }};
        }
        transaction_step!(transaction.spells.clear());
        transaction_step!(transaction.skills.clear());
        transaction_step!(transaction.spells.extend([200, 201]));
        transaction_step!(transaction.skills.push(164));
        transaction_step!(transaction.money = 80);
        if fail_before_step == Some(step) {
            return original.clone();
        }
        transaction
    }

    let original = CombinedState {
        money: 100,
        spells: vec![100],
        skills: vec![95],
    };
    for boundary in 0..=5 {
        assert_eq!(
            execute(&original, Some(boundary)),
            original,
            "fault boundary {boundary} must expose neither a fee nor an acquisition prefix"
        );
    }
    assert_eq!(
        execute(&original, None),
        CombinedState {
            money: 80,
            spells: vec![200, 201],
            skills: vec![164],
        }
    );
}

struct RecordingSpellAcquisitionPort {
    attempt: wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp,
    reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    reconciliation_calls: std::sync::atomic::AtomicUsize,
}

impl PlayerSpellAcquisitionPersistencePortLikeCpp for RecordingSpellAcquisitionPort {
    fn attempt_player_spell_acquisition_like_cpp(
        &self,
        _request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp,
    > {
        Box::pin(std::future::ready(self.attempt.clone()))
    }

    fn reconcile_player_spell_acquisition_like_cpp(
        &self,
        _request: PlayerSpellAcquisitionPersistenceRequestLikeCpp,
    ) -> wow_persistence::PersistenceFutureLikeCpp<
        '_,
        PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    > {
        self.reconciliation_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Box::pin(std::future::ready(self.reconciliation))
    }
}

fn empty_persistence_request() -> PlayerSpellAcquisitionPersistenceRequestLikeCpp {
    let authority = DurablePlayerSpellAcquisitionAuthorityLikeCpp {
        spells: Vec::new(),
        favorite_spell_ids: Vec::new(),
        skills: Vec::new(),
    };
    PlayerSpellAcquisitionPersistenceRequestLikeCpp {
        player_guid: 42,
        money_before: 100,
        money_after: 80,
        operation_token: [7; 16],
        source_authority: authority.clone(),
        resulting_authority: authority,
        operations: Vec::new(),
    }
}

#[tokio::test]
async fn port_outcome_reconciles_only_an_unknown_commit_like_cpp() {
    use std::sync::atomic::Ordering;
    use wow_persistence::PlayerSpellAcquisitionPersistenceAttemptLikeCpp as Attempt;

    let applied = RecordingSpellAcquisitionPort {
        attempt: Attempt::Applied,
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Indeterminate,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &applied,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::Applied
    ));
    assert_eq!(applied.reconciliation_calls.load(Ordering::Relaxed), 0);

    let unknown = RecordingSpellAcquisitionPort {
        attempt: Attempt::CommitOutcomeUnknown {
            reason: "lost reply".to_owned(),
        },
        reconciliation: PlayerSpellAcquisitionMoneyReconciliationLikeCpp::Committed,
        reconciliation_calls: std::sync::atomic::AtomicUsize::new(0),
    };
    assert!(matches!(
        persist_player_spell_acquisition_through_port_like_cpp(
            &unknown,
            empty_persistence_request()
        )
        .await,
        PlayerSpellAcquisitionPersistenceOutcomeLikeCpp::ReconciledCommit(reason)
            if reason == "lost reply"
    ));
    assert_eq!(unknown.reconciliation_calls.load(Ordering::Relaxed), 1);
}
