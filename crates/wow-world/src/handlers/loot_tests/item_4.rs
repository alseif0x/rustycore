//! Item scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    session.loot_table.insert(
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

    session
        .handle_master_loot_item(MasterLootItem {
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
#[test]
fn master_loot_inventory_result_mapping_matches_cpp_errors() {
    assert_eq!(
        super::super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::Ok),
        None
    );
    assert_eq!(
        super::super::master_loot_error_for_inventory_result_like_cpp(
            InventoryResult::ItemMaxCount
        ),
        Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP)
    );
    assert_eq!(
        super::super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::InvFull),
        Some(wow_packet::packets::loot::LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP)
    );
    assert_eq!(
        super::super::master_loot_error_for_inventory_result_like_cpp(
            InventoryResult::CantEquipEver
        ),
        Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
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

    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(master_guid));
    session.set_active_loot_guid(loot_owner);
    install_limited_test_item_template(&mut session, 700, 1);
    session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: 700,
            inventory_type: None,
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        700,
        master_guid,
        1,
        0,
        ItemContext::None,
        35,
    );
    session.insert_inventory_item_object(item);
    session.loot_table.insert(
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

    session
        .handle_master_loot_item(MasterLootItem {
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

    session.loot_table.insert(
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

    session.mark_represented_master_loot_item_removed_like_cpp(
        loot_owner,
        loot_object,
        0,
        master_guid,
    );

    let loot = session.loot_table.get(&loot_owner).unwrap();
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

    master_session.group_guid = Some(group_guid);
    master_session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    master_session.set_player_registry(Arc::clone(&player_registry));
    master_session.set_player_guid(Some(master_guid));
    master_session.set_active_loot_guid(loot_owner);
    master_session.loot_table.insert(
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
    install_limited_test_item_template(&mut target_session, 701, 1);
    target_session.insert_inventory_item_like_cpp(
        35,
        InventoryItem {
            guid: existing_item_guid,
            entry_id: 701,
            db_guid: 701,
            inventory_type: None,
        },
    );
    let item = target_session.make_inventory_item_object(
        existing_item_guid,
        701,
        target_guid,
        1,
        0,
        ItemContext::None,
        35,
    );
    target_session.insert_inventory_item_object(item);

    let master_future = master_session.handle_master_loot_item(MasterLootItem {
        target: target_guid,
        loot: vec![wow_packet::packets::loot::LootItemRequest {
            object: loot_object,
            loot_list_id: 0,
        }],
    });
    let target_future = async {
        for _ in 0..8 {
            target_session.process_pending().await;
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
    assert!(!master_session.loot_table.get(&loot_owner).unwrap().items[0].taken);
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

    master_session.group_guid = Some(group_guid);
    master_session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    master_session.set_player_registry(player_registry);
    master_session.set_player_guid(Some(master_guid));
    master_session.set_active_loot_guid(loot_owner);
    master_session.loot_table.insert(
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

    master_session
        .handle_master_loot_item(MasterLootItem {
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
async fn loot_item_request_uses_loot_object_to_find_active_owner_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let owner_guid = test_creature_guid(19_023);
    let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(owner_guid);
    session.loot_table.insert(
        owner_guid,
        CreatureLoot {
            loot_guid: loot_object_guid,
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
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(loot_object_guid, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!session.loot_table.get(&owner_guid).unwrap().items[0].taken);
    assert!(session.is_active_loot_guid(owner_guid));
}
#[tokio::test]
async fn loot_item_request_can_use_secondary_active_loot_object_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let primary_owner = test_creature_guid(19_027);
    let secondary_owner = test_creature_guid(19_028);
    let secondary_loot_object = represented_loot_object_guid_like_cpp(secondary_owner);
    session.set_player_guid(Some(player_guid));
    session.set_active_loot_guid(primary_owner);
    session.add_active_loot_view_owner_like_cpp(secondary_owner);
    session.loot_table.insert(
        secondary_owner,
        CreatureLoot {
            loot_guid: secondary_loot_object,
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
                allowed_looters: vec![player_guid],
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            }],
            looted_by_player: false,
        },
    );

    session
        .handle_loot_item(loot_item_packet(secondary_loot_object, 0))
        .await;

    let sent = send_rx.try_recv().unwrap();
    let mut sent = WorldPacket::from_bytes(&sent);
    assert_eq!(
        sent.read_uint16().unwrap(),
        wow_constants::ServerOpcodes::LootResponse as u16
    );
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_owner);
    assert_eq!(sent.read_packed_guid().unwrap(), secondary_loot_object);
    assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
    assert!(!session.loot_table.get(&secondary_owner).unwrap().items[0].taken);
    assert!(session.active_loot_view_owners.contains(&primary_owner));
    assert!(session.active_loot_view_owners.contains(&secondary_owner));
}
