//! Original application cases using the feature fixtures and production operations.
use super::support::*;

#[tokio::test]
async fn master_loot_item_without_group_sends_didnt_kill_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    handle_master_loot_item_for_test(
        &mut session,
        MasterLootItem {
            target: ObjectGuid::create_player(1, 77),
            loot: Vec::new(),
        },
    )
    .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_uses_group_master_looter_guid_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader_guid = ObjectGuid::create_player(1, 42);
    let master_guid = ObjectGuid::create_player(1, 43);
    let (leader_tx, _leader_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        leader_guid,
        broadcast_info(leader_guid, leader_tx),
        Default::default(),
    );
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(leader_guid));

    handle_master_loot_item_for_test(
        &mut session,
        MasterLootItem {
            target: master_guid,
            loot: Vec::new(),
        },
    )
    .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
    );

    session.set_player_guid(Some(master_guid));
    handle_master_loot_item_for_test(
        &mut session,
        MasterLootItem {
            target: leader_guid,
            loot: Vec::new(),
        },
    )
    .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn master_loot_item_missing_target_sends_player_not_found_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let missing_target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));

    handle_master_loot_item_for_test(
        &mut session,
        MasterLootItem {
            target: missing_target,
            loot: Vec::new(),
        },
    )
    .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_ineligible_target_sends_master_other_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let loot_owner = test_creature_guid(19_080);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let (target_tx, _target_rx) = flume::bounded::<Vec<u8>>(2);
    let player_registry = Arc::new(PlayerRegistry::default());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        target_guid,
        broadcast_info(target_guid, target_tx),
        Default::default(),
    );
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    set_active_loot_guid_for_test(&mut session, loot_owner);
    set_loot_for_test(
        &mut session,
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
            loot_master: master_guid,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid, target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    handle_master_loot_item_for_test(
        &mut session,
        MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        },
    )
    .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
}
