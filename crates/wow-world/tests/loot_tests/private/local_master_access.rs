//! Original loot application cases with explicit operation-local fixture permission.
use super::interaction_support::*;

#[tokio::test]
async fn master_loot_item_non_master_loot_view_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_082);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    wow_world::test_fixtures::set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    set_active_loot_guid_for_test(&mut session, loot_owner);
    set_loot_for_test(&mut session, 
        loot_owner,
        CreatureLoot {
            loot_guid: loot_object,
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
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    give_local_master_loot_for_test(&mut session, MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn master_loot_item_remote_target_can_store_error_is_reported_by_target_session_like_cpp() {
    let (mut master_session, master_rx) = make_session_with_send();
    let (mut target_session, _target_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let existing_item_guid = ObjectGuid::create_item(1, 701);
    let loot_owner = test_creature_guid(19_083);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    group.members.push(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
    let mut target_info = broadcast_info(target_guid, target_send_tx);
    target_info.command_tx = target_session.session_command_tx();
    player_registry.register_or_replace(target_guid, target_info, Default::default());

    wow_world::test_fixtures::set_group_guid_for_test_like_cpp(&mut master_session, Some(group_guid));
    master_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    master_session.set_player_registry(Arc::clone(&player_registry));
    master_session.set_player_guid(Some(master_guid));
    set_active_loot_guid_for_test(&mut master_session, loot_owner);
    set_loot_for_test(&mut master_session, 
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
            allowed_looters: vec![target_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 701,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    target_session.set_player_guid(Some(target_guid));
    prepare_money_player_residence_for_test(&mut target_session);
    install_limited_test_item_template(&mut target_session, 701, 1);
    install_loot_inventory_item_for_test(&mut target_session, 35, InventoryItem {
            guid: existing_item_guid,
            entry_id: 701,
            db_guid: 701,
            inventory_type: None,
        }, target_guid, 1, 0, ItemContext::None);

    let master_future = give_local_master_loot_for_test(&mut master_session, MasterLootItem {
        target: target_guid,
        loot: vec![wow_packet::packets::loot::LootItemRequest {
            object: loot_object,
            loot_list_id: 0,
        }],
    });
    let target_future = async {
        for _ in 0..8 {
            process_pending_for_loot_test(&mut target_session).await;
            tokio::task::yield_now().await;
        }
    };
    tokio::join!(master_future, target_future);

    let sent = master_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
    );
    assert!(!loot_for_test(&master_session, loot_owner).unwrap().items[0].taken);
}

#[tokio::test]
async fn master_loot_item_remote_target_unavailable_command_reports_player_not_found_like_cpp() {
    let (mut master_session, master_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let target_guid = ObjectGuid::create_player(1, 77);
    let loot_owner = test_creature_guid(19_084);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    group.members.push(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
    let (command_tx, _command_rx) = flume::bounded(0);
    let mut target_info = broadcast_info(target_guid, target_send_tx);
    target_info.command_tx = command_tx;
    player_registry.register_or_replace(target_guid, target_info, Default::default());

    wow_world::test_fixtures::set_group_guid_for_test_like_cpp(&mut master_session, Some(group_guid));
    master_session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    master_session.set_player_registry(player_registry);
    master_session.set_player_guid(Some(master_guid));
    set_active_loot_guid_for_test(&mut master_session, loot_owner);
    set_loot_for_test(&mut master_session, 
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
            allowed_looters: vec![target_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 702,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![target_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    give_local_master_loot_for_test(&mut master_session, MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = master_rx.try_recv().unwrap();
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
async fn master_loot_item_self_target_can_store_maps_unique_error_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 700);
    let loot_owner = test_creature_guid(19_081);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    wow_world::test_fixtures::set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    prepare_money_player_residence_for_test(&mut session);
    let _ = set_owned_player_group_like_cpp(&mut session, Some((group_guid, 0)));
    set_active_loot_guid_for_test(&mut session, loot_owner);
    install_limited_test_item_template(&mut session, 700, 1);
    install_loot_inventory_item_for_test(&mut session, 35, InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: 700,
            inventory_type: None,
        }, master_guid, 1, 0, ItemContext::None);
    set_loot_for_test(&mut session, 
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
            allowed_looters: vec![master_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 700,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    give_local_master_loot_for_test(&mut session, MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(
        sent.read_uint8().unwrap(),
        LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
    );
}

#[tokio::test]
async fn master_loot_item_self_target_success_marks_removed_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_082);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

    set_loot_for_test(&mut session, 
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
            allowed_looters: vec![master_guid],
            items: vec![LootEntry {
                loot_list_id: 0,
                item_id: 701,
                quantity: 3,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    remove_master_loot_slot_for_test(&mut session, 
        loot_owner,
        loot_object,
        0,
        master_guid,
    );

    let loot = loot_for_test(&session, loot_owner).unwrap();
    assert_eq!(loot.items[0].quantity, 0);
    assert!(loot.items[0].is_looted_for_player_like_cpp(master_guid));
    assert_eq!(loot.unlooted_count, 0);

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootRemoved as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
    assert_eq!(sent.read_uint8().unwrap(), 0);
}

#[tokio::test]
async fn master_loot_item_target_not_allowed_for_loot_sends_master_other_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let master_guid = ObjectGuid::create_player(1, 42);
    let loot_owner = test_creature_guid(19_083);
    let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(master_guid);
    group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
    group.master_looter_guid = master_guid;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    wow_world::test_fixtures::set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    set_active_loot_guid_for_test(&mut session, loot_owner);
    set_loot_for_test(&mut session, 
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
                allowed_looters: vec![master_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    give_local_master_loot_for_test(&mut session, MasterLootItem {
            target: master_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        })
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

