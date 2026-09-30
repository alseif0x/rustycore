use super::*;
use super::fixtures_state::*;

#[tokio::test]
async fn valid_direct_service_hello_replaces_stale_trainer_provenance_like_cpp() {
    const FEIGN_SLOT: u8 = 24;
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    insert_bank_test_player_in_world(&session, &canonical);
    let old_trainer = creature_guid(9306, 304);
    let vendor = creature_guid(2456, 305);
    insert_banker_creature(
        &canonical,
        vendor,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::VENDOR.bits(),
    );
    set_player_trainer_interaction_for_test(&mut session, old_trainer, 77);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 21,
            menu_id: 22,
            order_index: 23,
            option_npc: GOSSIP_TRAINER_OPTION_NPC_FOR_TEST,
            action_menu_id: 0,
        },
    );
    seed_represented_feign_death_like_cpp(&mut session, FEIGN_SLOT);

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello { unit: vendor })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes fake death before dispatching the selected direct service"
    );
    assert!(
        !has_gossip_visible_aura_for_test(&session, FEIGN_SLOT),
        "the direct-service shortcut represents a successful C++ gossip selection"
    );
    assert!(!canonical_player_has_died_state_like_cpp(&mut session));
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(vendor),
        "Rust's direct-service shortcut must preserve C++ SendGossipMenu source ownership"
    );
    assert_eq!(
        player_interaction_trainer_id_for_test(&session),
        0,
        "opening another valid service invalidates an earlier trainer window"
    );
    assert!(player_gossip_options_for_test(&session).is_empty());
}

#[tokio::test]
async fn gossip_hello_mixed_direct_service_keeps_service_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9309;
    let guid = creature_guid(entry, 309);
    let stale_source = creature_guid(entry, 999);
    let mut store = store_with_quests(&[3009]);
    store.starter_quests.entry(entry).or_default().push(3009);
    session.set_quest_store(Arc::new(store));
    set_player_trainer_interaction_for_test(&mut session, stale_source, 77);
    push_player_gossip_option_for_test(
        &mut session,
        wow_world::session::GossipOptionInfo {
            gossip_option_id: 91,
            menu_id: 92,
            order_index: 93,
            option_npc: 94,
            action_menu_id: 95,
        },
    );
    attach_legacy_creature(
        &mut session,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::QUEST_GIVER.bits() | NPCFlags1::BANKER.bits(),
    );

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello { unit: guid })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult]
    );
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(guid)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
    assert!(
        player_gossip_options_for_test(&session).is_empty(),
        "C++ HandleGossipHelloOpcode clears the prior menu before opening a direct service"
    );
}

#[tokio::test]
async fn gossip_hello_canonical_only_direct_fallback_uses_resolved_flags_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let entry = 9311;
    let guid = creature_guid(entry, 311);
    let mut manager = wow_map::MapManager::default();
    insert_canonical_creature_with_npc_flags(
        &mut manager,
        guid,
        entry,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    attach_map_manager(&mut session, manager);

    session
        .handle_gossip_hello(wow_packet::packets::gossip::Hello { unit: guid })
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::NpcInteractionOpenResult],
        "C++-resolved canonical NPC flags must drive fallback interactions when the legacy mirror has no creature"
    );
    assert_eq!(
        player_interaction_source_guid_for_test(&session),
        Some(guid)
    );
    assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
}

#[tokio::test]
async fn gossip_select_requires_exact_active_source_and_routes_exact_match_like_cpp() {
    let requested = creature_guid(15_513, 520);
    let other = creature_guid(15_513, 521);
    for (active_source, expected_opcodes) in [
        (None, Vec::new()),
        (Some(other), Vec::new()),
        (
            Some(requested),
            vec![ServerOpcodes::NpcInteractionOpenResult],
        ),
    ] {
        let (mut session, send_rx) = make_quest_status_session();
        attach_legacy_creature(
            &mut session,
            requested,
            15_513,
            NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
        );
        if let Some(active_source) = active_source {
            set_player_trainer_interaction_for_test(&mut session, active_source, 77);
        }
        push_player_gossip_option_for_test(
            &mut session,
            wow_world::session::GossipOptionInfo {
                gossip_option_id: 41,
                menu_id: 42,
                order_index: 43,
                option_npc: 6, // Banker gives an observable packet if routed.
                action_menu_id: 0,
            },
        );

        session
            .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
                gossip_unit: requested,
                gossip_id: 42,
                gossip_option_id: 41,
                promotion_code: String::new(),
            })
            .await;

        assert_eq!(drain_server_opcodes(&send_rx), expected_opcodes);
        assert_eq!(player_gossip_options_for_test(&session).len(), 1);
        if active_source == Some(requested) {
            assert_eq!(
                player_interaction_source_guid_for_test(&session),
                Some(requested)
            );
            assert_eq!(player_interaction_trainer_id_for_test(&session), 0);
        } else {
            assert_eq!(
                player_interaction_source_guid_for_test(&session),
                active_source
            );
            assert_eq!(
                player_interaction_trainer_id_for_test(&session),
                if active_source.is_some() { 77 } else { 0 }
            );
        }
    }
}
