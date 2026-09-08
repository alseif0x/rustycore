use super::*;

#[tokio::test]
async fn available_buy_commits_once_then_publishes_cpp_visual_and_learning_order() {
    let mut fixture = trainer_fixture();
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
    let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
    let realm_fence = wow_network::SocketWriteFenceLikeCpp::default();
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    fixture
        .session
        .set_send_write_fence_like_cpp(instance_fence.clone());
    fixture
        .session
        .install_realm_send_write_fence_for_test(realm_fence.clone());

    let (observed_tx, observed_rx) = flume::unbounded::<(&'static str, Vec<u8>)>();
    let instance_rx = fixture.send_rx.clone();
    let instance_observed_tx = observed_tx.clone();
    let instance_writer = tokio::spawn(async move {
        while let Ok(packet) = instance_rx.recv_async().await {
            if instance_fence.acknowledge_marker_like_cpp(&packet) {
                continue;
            }
            if instance_observed_tx.send(("instance", packet)).is_err() {
                break;
            }
        }
    });
    let realm_writer = tokio::spawn(async move {
        while let Ok(packet) = realm_rx.recv_async().await {
            if realm_fence.acknowledge_marker_like_cpp(&packet) {
                continue;
            }
            if observed_tx.send(("realm", packet)).is_err() {
                break;
            }
        }
    });
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);
    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 80);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&AVAILABLE_TRAINER_SPELL)
    );
    let player_guid = fixture.session.player_guid().unwrap();
    assert_eq!(
        observed_rx.recv_async().await.unwrap(),
        (
            "instance",
            wow_packet::packets::update::UpdateObject::player_money_update(
                player_guid,
                fixture.session.player_map_id_like_cpp(),
                80,
                None,
            )
            .to_bytes()
        )
    );
    assert_eq!(
        observed_rx.recv_async().await.unwrap(),
        (
            "realm",
            PlaySpellVisualKit {
                unit: fixture.trainer,
                kit_record_id: 179,
                kit_type: 0,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        )
    );
    assert_eq!(
        observed_rx.recv_async().await.unwrap(),
        (
            "realm",
            PlaySpellVisualKit {
                unit: player_guid,
                kit_record_id: 362,
                kit_type: 1,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        )
    );
    assert_eq!(
        observed_rx.recv_async().await.unwrap(),
        (
            "instance",
            wow_packet::packets::trainer::LearnedSpells::single(AVAILABLE_TRAINER_SPELL).to_bytes()
        )
    );
    assert!(observed_rx.try_recv().is_err());

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;
    assert_eq!(fixture.session.player_gold_like_cpp(), 80);
    assert_eq!(
        observed_rx.recv_async().await.unwrap(),
        (
            "realm",
            TrainerBuyFailed {
                trainer_guid: fixture.trainer,
                spell_id: AVAILABLE_TRAINER_SPELL,
                reason: 0,
            }
            .to_bytes()
        )
    );
    assert!(observed_rx.try_recv().is_err());
    instance_writer.abort();
    realm_writer.abort();
}

#[tokio::test]
async fn stalled_instance_writer_commits_but_never_publishes_realm_visuals_like_cpp() {
    let mut fixture = trainer_fixture();
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
    let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    fixture
        .session
        .set_send_write_fence_like_cpp(instance_fence.clone());
    fixture
        .session
        .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
    fixture.session.set_player_gold_like_cpp(100);
    fixture
        .session
        .set_loot_money_persistence_test_result_like_cpp(true);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            AVAILABLE_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 80);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&AVAILABLE_TRAINER_SPELL)
    );
    assert_eq!(
        fixture.session.state(),
        crate::session::SessionState::Disconnecting
    );
    assert!(realm_rx.try_recv().is_err());

    let player_guid = fixture.session.player_guid().unwrap();
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::player_money_update(
            player_guid,
            fixture.session.player_map_id_like_cpp(),
            80,
            None,
        )
        .to_bytes()
    );
    let marker = fixture.send_rx.try_recv().unwrap();
    assert!(instance_fence.acknowledge_marker_like_cpp(&marker));
    assert!(fixture.send_rx.try_recv().is_err());
}

#[tokio::test]
async fn stalled_instance_writer_keeps_committed_dual_wield_runtime_effect() {
    let mut fixture = trainer_wrapper_fixture();
    let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
    let learned_id = WRAPPER_LEARNED_SPELL as u32;
    let mut dual_wield_effect = player_learn_effect(2, wrapper_id, learned_id);
    dual_wield_effect.effect_index_raw = 1;
    dual_wield_effect.effect_type_raw = 40; // SPELL_EFFECT_DUAL_WIELD
    dual_wield_effect.effect_trigger_spell_raw = 0;
    fixture.session.set_spell_acquisition_catalog(Arc::new(
        SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            [wrapper_id, learned_id]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
            EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects: vec![
                    player_learn_effect(1, wrapper_id, learned_id),
                    dual_wield_effect,
                ],
                ..Default::default()
            },
            SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
    let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
    fixture
        .session
        .install_realm_send_channel_for_test(realm_tx);
    fixture
        .session
        .set_send_write_fence_like_cpp(instance_fence);

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
    assert!(
        fixture
            .session
            .mutate_canonical_player_like_cpp(|player| { player.unit().can_dual_wield_like_cpp() })
            .expect("canonical player"),
        "non-packet runtime actions must install immediately after commit"
    );
    assert!(fixture
        .session
        .represented_spell_acquisition_post_commit_actions_like_cpp()
        .iter()
        .any(|action| matches!(
            action,
            crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        )));
    assert_eq!(
        fixture.session.state(),
        crate::session::SessionState::Disconnecting
    );
    assert!(realm_rx.try_recv().is_err());
}

#[tokio::test]
async fn audited_castable_wrapper_commits_its_projected_target_once() {
    let mut fixture = trainer_wrapper_fixture();

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;

    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert!(
        fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_LEARNED_SPELL)
    );
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&WRAPPER_TRAINER_SPELL),
        "C++ casts a trainer wrapper; it does not learn the wrapper row"
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::player_money_update(
            fixture.session.player_guid().unwrap(),
            fixture.session.player_map_id_like_cpp(),
            75,
            None,
        )
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.trainer,
            kit_record_id: 179,
            kit_type: 0,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        PlaySpellVisualKit {
            unit: fixture.session.player_guid().unwrap(),
            kit_record_id: 362,
            kit_type: 1,
            duration: 0,
            mounted_visual: false,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        wow_packet::packets::trainer::LearnedSpells::single(WRAPPER_LEARNED_SPELL).to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());

    fixture
        .session
        .handle_trainer_buy_spell(trainer_buy_packet(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32,
            WRAPPER_TRAINER_SPELL,
        ))
        .await;
    assert_eq!(fixture.session.player_gold_like_cpp(), 75);
    assert_eq!(
        fixture.send_rx.try_recv().unwrap(),
        TrainerBuyFailed {
            trainer_guid: fixture.trainer,
            spell_id: WRAPPER_TRAINER_SPELL,
            reason: 0,
        }
        .to_bytes()
    );
    assert!(fixture.send_rx.try_recv().is_err());
}
