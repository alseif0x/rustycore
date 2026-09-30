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
    let plan = SpellAcquisitionPlanFixtureLikeCpp {
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
    }
    .build();
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
    let mut plan = SpellAcquisitionPlanFixtureLikeCpp {
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
    }
    .build();
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
