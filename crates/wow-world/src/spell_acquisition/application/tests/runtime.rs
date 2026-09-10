//! Runtime packets.
//!
//! Separated from tests.rs under #709.

use super::*;

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
