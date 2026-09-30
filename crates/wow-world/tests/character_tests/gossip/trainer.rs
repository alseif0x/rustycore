use super::*;
use super::fixtures_state::*;

#[tokio::test]
async fn gossip_hello_trainer_fallback_uses_canonical_access_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 515);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::TRAINER.bits() | NPCFlags1::TRAINER_CLASS.bits(),
    );
    attach_map_manager(&mut session, manager);

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello { unit: guid })
        .await;

    let bytes = send_rx.try_recv().expect("canonical trainer gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 0));
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(guid)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
    assert_eq!(
        player_gossip_options_for_test(&session)[0].option_npc,
        GOSSIP_TRAINER_OPTION_NPC_FOR_TEST
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn gossip_select_trainer_only_source_opens_resolved_trainer_without_close_like_cpp() {
    const TRAINER_ID: u32 = 77;
    const FEIGN_SLOT: u8 = 21;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let guid = creature_guid(2_456, 513);
    insert_banker_creature(&canonical, guid, NPCFlags1::TRAINER.bits());
    session.set_trainer_store_like_cpp(Arc::new(
        wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
            [wow_data::TrainerRowLikeCpp {
                id: TRAINER_ID,
                trainer_type: wow_data::TRAINER_TYPE_TRADESKILL_LIKE_CPP,
                greeting: "Train".to_string(),
            }],
            [],
            [],
            [wow_data::CreatureTrainerRowLikeCpp {
                creature_id: 2_456,
                trainer_id: TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    ));

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello { unit: guid })
        .await;

    let menu_packet = send_rx.try_recv().expect("generated trainer-only menu");
    assert_eq!(
        WorldPacket::from_bytes(&menu_packet).server_opcode(),
        Some(ServerOpcodes::GossipMessage)
    );
    assert_eq!(gossip_message_counts(&menu_packet, guid), (1, 0));
    let option = player_gossip_options_for_test(&session)
        .first()
        .cloned()
        .expect("generated trainer option");
    assert_eq!(option.menu_id, 0);
    assert_eq!(option.order_index, 0);
    assert_eq!(
        option.gossip_option_id,
        GOSSIP_TRAINER_OPTION_ID_FOR_TEST
    );
    assert_eq!(
        option.option_npc,
        GOSSIP_TRAINER_OPTION_NPC_FOR_TEST
    );
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(guid)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);

    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: guid,
            gossip_id: option.menu_id as i32,
            gossip_option_id: option.gossip_option_id,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate, ServerOpcodes::TrainerList],
        "the target fork's trainer-only generated option must be usable and must not pre-send GossipComplete"
    );
    assert!(gossip_trainer_interaction_matches_for_test(&session, guid, TRAINER_ID as i32));
    assert!(!has_gossip_visible_aura_for_test(&session, FEIGN_SLOT));
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
}

#[tokio::test]
async fn quest_giver_hello_trainer_questgiver_sends_mixed_gossip_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 15_513;
    let guid = creature_guid(entry, 513);
    let mut store = store_with_quests(&[9_393]);
    store.starter_quests.entry(entry).or_default().push(9_393);
    session.set_quest_store(Arc::new(store));
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits()
            | NPCFlags1::QUEST_GIVER.bits()
            | NPCFlags1::TRAINER.bits()
            | NPCFlags1::TRAINER_CLASS.bits(),
    );

    session
        .handle_quest_giver_hello(quest_giver_hello_packet(guid))
        .await;

    let bytes = send_rx.try_recv().expect("mixed prepared gossip menu");
    assert_eq!(gossip_message_counts(&bytes, guid), (1, 1));
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(guid)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
    assert_eq!(player_gossip_options_for_test(&session).len(), 1);
    assert_eq!(
        player_gossip_options_for_test(&session)[0].gossip_option_id,
        GOSSIP_TRAINER_OPTION_ID_FOR_TEST
    );
    assert_eq!(
        player_gossip_options_for_test(&session)[0].option_npc,
        GOSSIP_TRAINER_OPTION_NPC_FOR_TEST
    );
    assert!(send_rx.try_recv().is_err());
}
