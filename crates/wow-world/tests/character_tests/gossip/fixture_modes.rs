use super::*;
use super::fixtures_state::*;

#[tokio::test]
async fn gossip_feature_default_rejects_ownerless_npc_without_clearing_menu() {
    let (mut session, send_rx) = make_quest_status_session_with_fixture(false);
    let source = creature_guid(2456, 801);
    let previous = creature_guid(9306, 802);
    attach_legacy_creature(&mut session, source, 2456, NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits());
    set_player_trainer_interaction_for_test(&mut session, previous, 77);
    push_player_gossip_option_for_test(&mut session, wow_world::session::GossipOptionInfo {
        gossip_option_id: 1, menu_id: 2, order_index: 3, option_npc: 6, action_menu_id: 0,
    });

    session.handle_gossip_hello(Hello { unit: source }).await;

    assert!(send_rx.try_recv().is_err());
    assert!(gossip_trainer_interaction_matches_for_test(&session, previous, 77));
    assert_eq!(player_gossip_options_for_test(&session).len(), 1);
    assert!(gossip_taxi_snapshot_for_test(&session).is_none());
    assert!(gossip_aura_snapshot_for_test(&session).is_none());
}

#[tokio::test]
async fn gossip_feature_explicit_mode_admits_existing_ownerless_npc() {
    let (mut session, send_rx) = make_quest_status_session_with_fixture(true);
    let source = creature_guid(2456, 803);
    attach_legacy_creature(&mut session, source, 2456, NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits());

    session.handle_gossip_hello(Hello { unit: source }).await;

    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::NpcInteractionOpenResult]);
    assert_eq!(player_interaction_source_guid_for_test(&session), Some(source));
    assert!(gossip_taxi_snapshot_for_test(&session).is_some());
}

#[tokio::test]
async fn gossip_feature_mode_rejects_missing_player_guid() {
    let (mut session, send_rx) = make_quest_status_session_with_fixture(true);
    let source = creature_guid(2456, 804);
    attach_legacy_creature(&mut session, source, 2456, NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits());
    session.set_player_guid(None);

    session.handle_gossip_hello(Hello { unit: source }).await;

    assert!(send_rx.try_recv().is_err());
    assert!(gossip_taxi_snapshot_for_test(&session).is_none());
    assert!(gossip_aura_snapshot_for_test(&session).is_none());
}

#[tokio::test]
async fn gossip_feature_mode_does_not_reconstruct_after_handle_retirement() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let source = creature_guid(2456, 805);
    insert_banker_creature(&canonical, source, NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits());
    seed_represented_feign_death_like_cpp(&mut session, 29);
    assert!(adopt_gossip_canonical_player_for_test(&mut session));
    let guid = session.player_guid().expect("fixture GUID");
    assert!(canonical.lock().unwrap().find_map_mut(571, 0).unwrap()
        .map_mut().remove_map_object(guid).is_some());

    session.handle_gossip_hello(Hello { unit: source }).await;

    assert!(send_rx.try_recv().is_err());
    assert!(has_gossip_visible_aura_for_test(&session, 29));
    assert!(gossip_taxi_snapshot_for_test(&session).is_none());
    assert!(gossip_aura_snapshot_for_test(&session).is_none());
}

#[tokio::test]
async fn gossip_feature_mode_keeps_canonical_health_and_aura_priority() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let source = creature_guid(2456, 806);
    insert_banker_creature(&canonical, source, NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits());
    seed_represented_feign_death_like_cpp(&mut session, 30);
    assert!(adopt_gossip_canonical_player_for_test(&mut session));
    assert!(canonical_player_access::configure_canonical_player_vitals_for_test(
        &canonical,
        session.player_guid().unwrap(),
        (0, 100, PowerType::Mana, 100, 100, 100),
    ));

    session.handle_gossip_hello(Hello { unit: source }).await;

    assert!(send_rx.try_recv().is_err());
    assert!(has_gossip_visible_aura_for_test(&session, 30));
    assert!(gossip_aura_snapshot_for_test(&session).unwrap().runtime_applications_like_cpp().is_empty());
    assert!(gossip_taxi_snapshot_for_test(&session).is_some());
}
