use super::*;

#[test]
fn trainer_store_resolves_selected_default_and_discards_dangling_mappings_like_cpp() {
    assert_eq!(
        trainer_list_required_npc_flags_like_cpp(None),
        TRAINER_LIST_NPC_FLAGS_LIKE_CPP
    );
    assert_eq!(
        trainer_list_required_npc_flags_like_cpp(Some((900, 3))),
        TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP
    );

    let store = trainer_store_from_rows(
        vec![trainer_row(7, 0, "Selected"), trainer_row(8, 2, "Default")],
        Vec::new(),
        Vec::new(),
        vec![
            CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: 8,
                menu_id: 0,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: 7,
                menu_id: 900,
                option_id: 3,
            },
        ],
    );
    assert_eq!(
        resolve_creature_trainer_like_cpp(store.as_ref(), CREATURE_ENTRY, Some((900, 3)))
            .map(TrainerLikeCpp::id_like_cpp),
        Some(7)
    );
    assert_eq!(
        resolve_creature_trainer_like_cpp(store.as_ref(), CREATURE_ENTRY, None)
            .map(TrainerLikeCpp::id_like_cpp),
        Some(8)
    );

    let dangling = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(8, 2, "Default")],
        Vec::<TrainerSpellRowLikeCpp>::new(),
        Vec::<TrainerLocaleRowLikeCpp>::new(),
        [
            CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: 77,
                menu_id: 900,
                option_id: 3,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: 8,
                menu_id: 0,
                option_id: 0,
            },
        ],
        |_| true,
        |_| true,
        |_| true,
        |_, _| true,
    );
    assert_eq!(
        dangling.report.skipped_creature_trainers_missing_trainer,
        vec![(CREATURE_ENTRY, 77, 900, 3)]
    );
    assert_eq!(
        resolve_creature_trainer_like_cpp(&dangling.store, CREATURE_ENTRY, Some((900, 3)))
            .map(TrainerLikeCpp::id_like_cpp),
        Some(8),
        "a selected mapping discarded at load must fall through to the default"
    );
}

#[test]
fn trainer_condition_supported_or_success_outweighs_irrelevant_uncertainty_like_cpp() {
    assert_eq!(
        trainer_condition_admission_proof_like_cpp(true, true),
        TrainerAdmissionProofLikeCpp::Proven(true)
    );
    assert_eq!(
        trainer_condition_admission_proof_like_cpp(false, true),
        TrainerAdmissionProofLikeCpp::Indeterminate
    );
    assert_eq!(
        trainer_condition_admission_proof_like_cpp(false, false),
        TrainerAdmissionProofLikeCpp::Proven(false)
    );
}

#[test]
fn class_race_fit_uses_effective_ability_and_race_class_rows() {
    let (mut session, _) = make_session();
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        ObjectGuid::create_player(1, 42),
        "FitTester".to_string(),
        Position::new(0.0, 0.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let ability = SkillLineAbilityRecord {
        id: 1,
        race_mask: 1,
        skill_line: 164,
        spell: 500,
        min_skill_line_rank: 0,
        class_mask: 1,
        supercedes_spell: 0,
        acquire_method: 0,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        skillup_skill_line_id: 0,
    };
    let race_class = SkillRaceClassInfoRecord {
        id: 2,
        race_mask: 1,
        skill_id: 164,
        class_mask: 1,
        flags: 0,
        availability: 1,
        min_level: 1,
        skill_tier_id: 0,
    };
    session.set_skill_store(Arc::new(
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
            [ability.clone()],
            [race_class],
        ),
    ));
    assert_eq!(
        trainer_spell_class_race_fit_like_cpp(&session, 500),
        TrainerAdmissionProofLikeCpp::Proven(true)
    );

    let wrong_race = SkillLineAbilityRecord {
        race_mask: 2,
        ..ability
    };
    session.set_skill_store(Arc::new(
        SkillStore::from_skill_line_abilities_and_race_class_like_cpp([wrong_race], []),
    ));
    assert_eq!(
        trainer_spell_class_race_fit_like_cpp(&session, 500),
        TrainerAdmissionProofLikeCpp::Proven(false)
    );
}

#[tokio::test]
async fn missing_trainer_mapping_preserves_binding_and_feign_like_cpp() {
    let store = trainer_store_from_rows(
        vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Unused")],
        vec![trainer_spell_row(
            DEFAULT_TRAINER_ID,
            KNOWN_TRAINER_SPELL,
            10,
            1,
        )],
        Vec::new(),
        Vec::new(),
    );
    let mut fixture = trainer_fixture_with_store(store);
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.other_trainer, 17);
    seed_feign_death(&mut fixture.session, 6);

    fixture
        .session
        .handle_trainer_list(wow_packet::packets::gossip::Hello {
            unit: fixture.trainer,
        })
        .await;

    assert!(
        fixture
            .session
            .player_trainer_interaction_matches_like_cpp(fixture.other_trainer, 17),
        "a missing ObjectMgr mapping must not replace a prior published window"
    );
    assert!(
        fixture
            .session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&6)
    );
    assert!(canonical_player_has_died_state(&mut fixture.session));
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn valid_trainer_list_uses_store_states_and_publishes_exact_binding_like_cpp() {
    let mut fixture = trainer_fixture();
    seed_feign_death(&mut fixture.session, 6);

    fixture
        .session
        .handle_trainer_list(wow_packet::packets::gossip::Hello {
            unit: fixture.trainer,
        })
        .await;

    assert!(
        fixture.send_rx.try_recv().is_ok(),
        "C++ removes feign death before publishing the trainer list"
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        TrainerListPacket {
            trainer_guid: fixture.trainer,
            trainer_type: 2,
            trainer_id: DEFAULT_TRAINER_ID as i32,
            spells: vec![
                TrainerListSpell {
                    spell_id: KNOWN_TRAINER_SPELL,
                    money_cost: 10,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    usable: TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
                    req_level: 1,
                },
                TrainerListSpell {
                    spell_id: AVAILABLE_TRAINER_SPELL,
                    money_cost: 20,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    usable: TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP,
                    req_level: 80,
                },
                TrainerListSpell {
                    spell_id: UNAVAILABLE_TRAINER_SPELL,
                    money_cost: 30,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    usable: TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP,
                    req_level: 81,
                },
            ],
            greeting: "Train".to_string(),
        }
        .to_bytes()
    );
    assert_eq!(TRAINER_SPELL_STATE_KNOWN_LIKE_CPP, 0);
    assert_eq!(TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, 1);
    assert_eq!(TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, 2);
    assert!(
        fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        )
    );
    assert!(
        !fixture
            .session
            .resolved_player_visible_auras_like_cpp()
            .expect("canonical Player aura owner")
            .contains_key(&6)
    );
    assert!(!canonical_player_has_died_state(&mut fixture.session));
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn trainer_buy_requires_exact_active_provenance_before_known_spell_like_cpp() {
    for case in ["unbound", "wrong-guid", "wrong-id"] {
        let mut fixture = if case == "wrong-id" {
            let decoy_trainer_id = DEFAULT_TRAINER_ID + 1;
            trainer_fixture_with_store(trainer_store_from_rows(
                vec![
                    trainer_row(DEFAULT_TRAINER_ID, 2, "Default"),
                    trainer_row(decoy_trainer_id, 2, "Decoy"),
                ],
                vec![
                    trainer_spell_row(DEFAULT_TRAINER_ID, KNOWN_TRAINER_SPELL, 10, 1),
                    trainer_spell_row(decoy_trainer_id, KNOWN_TRAINER_SPELL, 10, 1),
                ],
                Vec::new(),
                vec![CreatureTrainerRowLikeCpp {
                    creature_id: CREATURE_ENTRY,
                    trainer_id: DEFAULT_TRAINER_ID,
                    menu_id: 0,
                    option_id: 0,
                }],
            ))
        } else {
            trainer_fixture()
        };
        fixture
            .session
            .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
        match case {
            "unbound" => {}
            "wrong-guid" | "wrong-id" => {
                fixture
                    .session
                    .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
            }
            _ => unreachable!(),
        }
        let (request_guid, request_id) = match case {
            "unbound" => (fixture.trainer, DEFAULT_TRAINER_ID as i32),
            "wrong-guid" => (fixture.other_trainer, DEFAULT_TRAINER_ID as i32),
            "wrong-id" => (fixture.trainer, DEFAULT_TRAINER_ID as i32 + 1),
            _ => unreachable!(),
        };

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                request_guid,
                request_id,
                KNOWN_TRAINER_SPELL,
            ))
            .await;

        assert!(
            fixture.send_rx.try_recv().is_err(),
            "{case} must return before the observable known-spell branch"
        );
        if case != "unbound" {
            assert!(fixture.session.player_trainer_interaction_matches_like_cpp(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32
            ));
        }
    }
}

#[tokio::test]
async fn generic_source_with_zero_trainer_id_is_silent_before_known_spell_like_cpp() {
    let mut fixture = trainer_fixture();
    fixture
        .session
        .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
    fixture
        .session
        .set_player_interaction_source_like_cpp(fixture.trainer);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(fixture.trainer, 0, KNOWN_TRAINER_SPELL))
        .await;

    assert!(
        fixture.send_rx.try_recv().is_err(),
        "C++ GetTrainer(0) rejects a generic gossip source before TeachSpell"
    );
    assert_eq!(
        fixture.session.player_interaction_source_guid_like_cpp(),
        Some(fixture.trainer)
    );
    assert_eq!(fixture.session.player_interaction_trainer_id_like_cpp(), 0);
}

#[tokio::test]
async fn trainer_buy_signed_id_bits_and_binding_survive_repeated_failures_like_cpp() {
    let store = standard_trainer_store(u32::MAX);
    let mut fixture = trainer_fixture_with_store(store);
    fixture
        .session
        .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, u32::MAX);

    let expected = TrainerBuyFailed {
        trainer_guid: fixture.trainer,
        spell_id: KNOWN_TRAINER_SPELL,
        reason: 0,
    }
    .to_bytes();
    for _ in 0..2 {
        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(fixture.trainer, -1, KNOWN_TRAINER_SPELL))
            .await;
        assert_eq!(fixture.send_rx.try_recv().unwrap(), expected);
        assert!(
            fixture
                .session
                .player_trainer_interaction_matches_like_cpp(fixture.trainer, -1)
        );
    }
}
