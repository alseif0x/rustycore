//! Loot scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn loot_unit_new_main_target_releases_existing_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let old_guid = test_creature_guid(19_036);
    let new_guid = test_creature_guid(19_037);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(old_guid);
    insert_allowed_coin_loot_like_cpp(&mut session, old_guid, player_guid, 7);
    register_test_creature_like_cpp(&mut session, test_creature(new_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, new_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(new_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRelease as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), old_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert!(!session.is_active_loot_guid(old_guid));
    assert!(session.is_active_loot_guid(new_guid));
    assert!(session.loot_table.contains_key(&old_guid));
}
#[tokio::test]
async fn loot_unit_response_uses_loot_owner_not_player_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_022);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, owner_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    let response_owner = sent.read_packed_guid().unwrap();
    let response_loot_obj = sent.read_packed_guid().unwrap();
    assert_eq!(response_owner, owner_guid);
    assert_eq!(response_loot_obj.high_type(), HighGuid::LootObject);
    assert_ne!(response_loot_obj, owner_guid);
    assert_ne!(owner_guid, player_guid);
    assert_eq!(
        session.loot_table.get(&owner_guid).unwrap().loot_guid,
        response_loot_obj
    );
    assert!(session.is_active_loot_guid(owner_guid));
}
#[tokio::test]
async fn loot_unit_ae_loot_sends_targets_and_secondary_ack_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    let player_guid = ObjectGuid::create_player(1, 42);
    let main_guid = test_creature_guid(19_031);
    let secondary_guid = test_creature_guid(19_032);
    session.set_player_guid(Some(player_guid));
    session.set_enable_ae_loot_like_cpp(true);
    session.set_player_position_like_cpp(Position::ZERO);
    register_test_creature_like_cpp(&mut session, test_creature(main_guid, false));
    register_test_creature_like_cpp(&mut session, test_creature(secondary_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, main_guid, player_guid, 7);
    insert_allowed_coin_loot_like_cpp(&mut session, secondary_guid, player_guid, 7);

    session.handle_loot_unit(loot_unit_packet(main_guid)).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargets as u16
    );
    assert_eq!(sent.read_uint32().unwrap(), 2);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), main_guid);
    let main_loot_object = sent.read_packed_guid().unwrap();
    assert_eq!(main_loot_object.high_type(), HighGuid::LootObject);
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint32().unwrap();
    sent.read_int32().unwrap();
    sent.read_int32().unwrap();
    assert!(sent.read_bit().unwrap());
    assert!(!sent.read_bit().unwrap());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargetAck as u16
    );
    assert!(sent.read_uint8().is_err());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
    let secondary_loot_object = sent.read_packed_guid().unwrap();
    assert_eq!(secondary_loot_object.high_type(), HighGuid::LootObject);
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint8().unwrap();
    sent.read_uint32().unwrap();
    sent.read_int32().unwrap();
    sent.read_int32().unwrap();
    assert!(sent.read_bit().unwrap());
    assert!(sent.read_bit().unwrap());

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::AeLootTargetAck as u16
    );
    assert!(session.is_active_loot_guid(main_guid));
    assert!(session.active_loot_view_owners.contains(&main_guid));
    assert!(session.active_loot_view_owners.contains(&secondary_guid));
}
#[tokio::test]
async fn new_loot_after_primary_ae_release_closes_secondary_before_replacing_tracking_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, 61_714);
    let primary_guid = test_creature_guid(61_715);
    let secondary_guid = test_creature_guid(61_716);
    let new_guid = test_creature_guid(61_717);
    let secondary_authority =
        open_test_ae_pair_like_cpp(&mut session, player_guid, primary_guid, secondary_guid).await;

    session
        .handle_loot_release(loot_release_packet(primary_guid))
        .await;
    assert!(session.active_loot_guid.is_empty());
    assert!(session.active_loot_view_owners.contains(&secondary_guid));

    session.set_enable_ae_loot_like_cpp(false);
    register_test_creature_like_cpp(&mut session, test_creature(new_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, new_guid, player_guid, 7);
    session.handle_loot_unit(loot_unit_packet(new_guid)).await;

    assert!(session.is_active_loot_guid(new_guid));
    assert_eq!(session.active_loot_view_owners.len(), 1);
    assert!(!session.active_loot_view_owners.contains(&secondary_guid));
    assert!(
        !secondary_authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .players_looting
            .contains(&player_guid),
        "the secondary AE viewer must be released before set_active_loot_guid clears tracking"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&send_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRelease as u16)
            .count(),
        2
    );
}
#[tokio::test]
async fn loot_unit_empty_visible_loot_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_007);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, loot_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_unit_fully_looted_existing_loot_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_017);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: true,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_unit_without_allowed_loot_for_player_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_018);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    install_cached_test_creature_loot_authority_like_cpp(&mut session, loot_guid, player_guid);
    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().items[0].allowed_looters,
        vec![other_guid]
    );
    assert!(!session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_unit_non_tapper_existing_tap_list_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_098);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    tap_test_creature_like_cpp(&mut session, loot_guid, other_guid);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert!(!session.loot_table.contains_key(&loot_guid));
}
#[tokio::test]
async fn loot_unit_existing_coin_loot_without_allowed_looter_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_099);
    session.set_player_guid(Some(player_guid));
    register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

    session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

    assert!(send_rx.try_recv().is_err());
    assert!(!session.is_active_loot_guid(loot_guid));
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
    assert_eq!(
        session.loot_table.get(&loot_guid).unwrap().allowed_looters,
        vec![other_guid]
    );
}
#[tokio::test]
async fn loot_money_stale_active_without_loot_view_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_020);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);

    session.handle_loot_money(loot_money_packet()).await;

    assert!(session.is_active_loot_guid(loot_guid));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn loot_money_zero_money_still_notifies_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_021);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(sent.read_bit().unwrap());
    assert_eq!(session.player_gold_like_cpp(), 0);
    assert!(session.is_active_loot_guid(loot_guid));
}
#[tokio::test]
async fn loot_money_coin_removed_uses_loot_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_024);
    let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_guid);
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object_guid,
            coins: 3,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
    assert!(session.is_active_loot_guid(owner_guid));
}
#[tokio::test]
async fn loot_money_consumes_all_active_loot_views_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_one = test_creature_guid(19_025);
    let owner_two = test_creature_guid(19_026);
    let loot_object_one = represented_loot_object_guid_like_cpp(owner_one);
    let loot_object_two = represented_loot_object_guid_like_cpp(owner_two);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_one);
    session.add_active_loot_view_owner_like_cpp(owner_two);
    session.loot_table.insert(
        owner_one,
        CreatureLoot {
            loot_guid: loot_object_one,
            coins: 3,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );
    session.loot_table.insert(
        owner_two,
        CreatureLoot {
            loot_guid: loot_object_two,
            coins: 7,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_one);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 3);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_two);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 7);
    assert_eq!(session.player_gold_like_cpp(), 10);
    assert_eq!(session.loot_table.get(&owner_one).unwrap().coins, 0);
    assert_eq!(session.loot_table.get(&owner_two).unwrap().coins, 0);
    assert!(session.active_loot_view_owners.contains(&owner_one));
    assert!(session.active_loot_view_owners.contains(&owner_two));
}
#[tokio::test]
async fn loot_money_gain_completes_money_tracking_event_objective_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(5);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let loot_guid = test_creature_guid(19_029);
    let quest_id = 12_530;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 8, // C++ QUEST_OBJECTIVE_MONEY.
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 7,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(loot_guid);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: Vec::new(),
            slot: 0,
        },
    );
    insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, player_guid, 7);

    session.handle_loot_money(loot_money_packet()).await;

    assert_eq!(session.player_gold_like_cpp(), 7);
    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::UpdateObject as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16
    );
    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::QuestUpdateComplete as u16
    );
}
#[tokio::test]
async fn loot_money_splits_corpse_gold_to_near_group_members_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    session.set_loot_money_persistence_test_result_like_cpp(true);
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let loot_guid = test_creature_guid(19_027);
    let (other_tx, other_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let (player_presence_tx, _player_presence_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_presence_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        other_guid,
        broadcast_info(other_guid, other_tx),
        Default::default(),
    );

    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_active_loot_guid(loot_guid);
    session.loot_table.insert(
        loot_guid,
        CreatureLoot {
            loot_guid,
            coins: 9,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid, other_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![player_guid, other_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session.handle_loot_money(loot_money_packet()).await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::CoinRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 4);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(!sent.read_bit().unwrap());

    let sent = other_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootMoneyNotify as u16
    );
    assert_eq!(sent.read_uint64().unwrap(), 4);
    assert_eq!(sent.read_uint64().unwrap(), 0);
    assert!(!sent.read_bit().unwrap());
    assert_eq!(session.player_gold_like_cpp(), 4);
    assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 0);
}
#[tokio::test]
async fn loot_roll_without_canonical_roll_state_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session
        .handle_loot_roll(LootRoll {
            loot_obj: test_creature_guid(19_006),
            loot_list_id: 0,
            roll_type: 1,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn set_loot_specialization_matches_cpp_class_validation() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_class_like_cpp(2);
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 65,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
        ChrSpecializationEntry {
            id: 71,
            class_id: 1,
            order_index: 0,
            role: 0,
        },
    ])));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(65));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 71 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(65));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 999 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(65));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 0 })
        .await;
    assert_eq!(session.loot_specialization_id_like_cpp(), Some(0));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn set_loot_specialization_without_loaded_player_is_ignored_like_cpp_status_guard() {
    let (mut session, _send_rx) = make_session_with_send();
    session.set_player_class_like_cpp(2);
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 65,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
    ])));

    session
        .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
        .await;

    assert_eq!(session.loot_specialization_id_like_cpp(), Some(0));
}
#[tokio::test]
async fn loot_release_ignores_guid_outside_active_view_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_011);
    let spoofed_guid = test_creature_guid(19_012);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);
    session.loot_table.insert(
        spoofed_guid,
        CreatureLoot {
            loot_guid: spoofed_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: Vec::new(),
            looted_by_player: false,
        },
    );

    session
        .handle_loot_release(loot_release_packet(spoofed_guid))
        .await;

    assert!(session.is_active_loot_guid(active_guid));
    assert!(session.loot_table.contains_key(&spoofed_guid));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn loot_release_ignores_active_guid_without_represented_loot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let active_guid = test_creature_guid(19_015);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(active_guid);

    session
        .handle_loot_release(loot_release_packet(active_guid))
        .await;

    assert!(session.is_active_loot_guid(active_guid));
    assert!(send_rx.try_recv().is_err());
}
