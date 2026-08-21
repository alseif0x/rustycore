// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! collections capability handler tests.

use super::*;
use crate::entity_update_bridge::player_values_update_to_update_object;
use crate::session::RepresentedAuctionPlaceBidLikeCpp;
use wow_constants::InventoryResult;
use wow_constants::unit::NPCFlags1;
use wow_packet::packets::collection::{
    COLLECTION_TYPE_APPEARANCE_LIKE_CPP, COLLECTION_TYPE_TOYBOX_LIKE_CPP,
};
use wow_packet::packets::item::InventoryChangeFailure;
use wow_packet::packets::misc::AuctionPlaceBid;

#[tokio::test]
async fn can_duel_uses_mounted_spell_when_source_is_mounted_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let target_guid = ObjectGuid::create_player(1, 88);
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        target_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_mounted_like_cpp(true);

    session
        .handle_can_duel(can_duel_packet(target_guid, false))
        .await;

    assert_eq!(
        session.represented_can_duel_spell_casts_like_cpp(),
        &[crate::session::RepresentedCanDuelSpellCastLikeCpp {
            target_guid,
            spell_id: crate::session::SPELL_MOUNTED_DUEL_LIKE_CPP,
            to_the_death: false,
        }]
    );
}

#[tokio::test]
async fn mount_set_favorite_updates_known_mount_and_sends_partial_update_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_account_mounts_like_cpp(vec![wow_packet::packets::misc::AccountMount {
        spell_id: 1234,
        flags: 0,
    }]);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
    pkt.write_uint32(1234);
    pkt.write_bit(true);
    pkt.flush_bits();

    session.handle_mount_set_favorite(pkt).await;

    assert_eq!(session.account_mounts_like_cpp().get(&1234), Some(&0x01));
    let bytes = send_rx.try_recv().expect("partial mount update");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::AccountMountUpdate as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(
        i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]),
        1
    );
    assert_eq!(
        i32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
        1234
    );
    assert_eq!(bytes[11], 0x10);
}

#[tokio::test]
async fn mount_set_favorite_ignores_unknown_mount_like_cpp() {
    let (mut session, send_rx) = make_session();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSetFavorite as u16);
    pkt.write_uint32(1234);
    pkt.write_bit(true);
    pkt.flush_bits();

    session.handle_mount_set_favorite(pkt).await;

    assert!(session.account_mounts_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn mount_special_anim_does_not_send_to_source_player_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
    pkt.write_uint32(2);
    pkt.write_int32(-3);
    pkt.write_int32(111);
    pkt.write_int32(222);

    session.handle_mount_special_anim(pkt).await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ MessageDistDeliverer never sends SendMessageToSet packets to i_source"
    );
}

#[tokio::test]
async fn mount_special_anim_fanouts_to_visible_sessions_like_cpp() {
    let (mut source_session, source_send_rx) = make_session();
    let (mut visible_session, visible_send_rx) = make_session();
    let source_guid = ObjectGuid::create_player(1, 77);
    let visible_guid = ObjectGuid::create_player(1, 88);
    source_session.set_player_guid(Some(source_guid));
    source_session.set_player_map_position_like_cpp(571, Position::ZERO);
    visible_session.set_player_guid(Some(visible_guid));
    visible_session.set_player_map_position_like_cpp(571, Position::ZERO);
    visible_session.set_state(crate::session::SessionState::LoggedIn);
    visible_session
        .client_visible_guids_like_cpp
        .insert(source_guid);

    let registry = Arc::new(PlayerRegistry::default());
    let (source_command_tx, source_command_rx) = flume::bounded::<SessionCommand>(2);
    let source_info = broadcast_info_with_command_tx(source_command_tx);
    registry.insert(source_guid, source_info);
    let visible_command_tx = visible_session.session_command_tx();
    let visible_info = broadcast_info_with_command_tx(visible_command_tx);
    registry.insert(visible_guid, visible_info);
    source_session.set_player_registry(Arc::clone(&registry));
    visible_session.set_player_registry(registry);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::MountSpecialAnim as u16);
    pkt.write_uint32(2);
    pkt.write_int32(-3);
    pkt.write_int32(111);
    pkt.write_int32(222);

    source_session.handle_mount_special_anim(pkt).await;

    assert!(
        source_send_rx.try_recv().is_err(),
        "source session must not receive the packet directly"
    );
    assert!(
        source_command_rx.try_recv().is_err(),
        "source registry entry must be skipped like C++ player == i_source"
    );

    visible_session
        .process_represented_session_commands_like_cpp()
        .await;

    let bytes = visible_send_rx
        .try_recv()
        .expect("visible special mount anim");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        ServerOpcodes::SpecialMountAnim as u16
    );
    assert_eq!(&bytes[2..18], &source_guid.to_raw_bytes());
    assert_eq!(
        u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]),
        2
    );
    assert_eq!(
        i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]),
        -3
    );
    assert_eq!(
        i32::from_le_bytes([bytes[26], bytes[27], bytes[28], bytes[29]]),
        111
    );
    assert_eq!(
        i32::from_le_bytes([bytes[30], bytes[31], bytes[32], bytes[33]]),
        222
    );
    assert_eq!(bytes.len(), 34);
}

#[tokio::test]
async fn mount_clear_fanfare_stub_sends_no_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_mount_clear_fanfare(WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn toy_clear_fanfare_clears_known_toy_without_packet_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.load_represented_account_toys_like_cpp([(30_000, true, true)]);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
    pkt.write_uint32(30_000);

    session.handle_toy_clear_fanfare(pkt).await;

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, true, false)]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn toy_clear_fanfare_ignores_unknown_toy_like_cpp() {
    let (mut session, send_rx) = make_session();

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::ToyClearFanfare as u16);
    pkt.write_uint32(40_000);

    session.handle_toy_clear_fanfare(pkt).await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn add_toy_finds_nested_bag_item_by_guid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 55);
    let bag_guid = ObjectGuid::create_item(1, 1_001);
    let toy_guid = ObjectGuid::create_item(1, 1_002);
    let bag_slot = wow_entities::INVENTORY_SLOT_BAG_START;
    let toy_slot = 5;
    let toy_item_id = 30_000_u32;
    let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();

    session.set_player_guid(Some(player_guid));
    install_add_toy_item_templates(&mut session, toy_item_id, 0);
    session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
        wow_data::ToyEntry {
            id: 1,
            source_text: "known".to_string(),
            item_id: toy_item_id_i32,
            flags: 0,
            source_type_enum: 0,
        },
    ])));
    session.load_represented_account_toys_like_cpp([(toy_item_id, false, false)]);
    session.insert_inventory_item_like_cpp(
        bag_slot,
        crate::session::InventoryItem {
            guid: bag_guid,
            entry_id: 101,
            db_guid: bag_guid.counter() as u64,
            inventory_type: Some(wow_constants::InventoryType::Bag as u8),
        },
    );
    let bag_item = session.make_inventory_item_object(
        bag_guid,
        101,
        player_guid,
        1,
        0,
        wow_constants::ItemContext::None,
        bag_slot,
    );
    session.insert_inventory_item_object(bag_item);
    let mut toy_item = session.make_inventory_item_object(
        toy_guid,
        toy_item_id,
        player_guid,
        1,
        0,
        wow_constants::ItemContext::None,
        toy_slot,
    );
    toy_item.set_container_guid_and_slot(bag_guid, bag_slot);
    session.insert_inventory_item_object(toy_item);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::AddToy as u16);
    pkt.write_packed_guid(&toy_guid);

    session.handle_add_toy(pkt).await;

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(toy_item_id, false, false)]
    );
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&toy_guid)
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn add_toy_uses_can_use_item_faction_gate_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 55);
    let toy_guid = ObjectGuid::create_item(1, 1_002);
    let toy_slot = 23;
    let toy_item_id = 30_000_u32;
    let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    install_add_toy_item_templates(
        &mut session,
        toy_item_id,
        wow_constants::ItemFlags2::FactionHorde as u32,
    );
    session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
        wow_data::ToyEntry {
            id: 1,
            source_text: "known".to_string(),
            item_id: toy_item_id_i32,
            flags: 0,
            source_type_enum: 0,
        },
    ])));
    session.insert_inventory_item_like_cpp(
        toy_slot,
        crate::session::InventoryItem {
            guid: toy_guid,
            entry_id: toy_item_id,
            db_guid: toy_guid.counter() as u64,
            inventory_type: Some(wow_constants::InventoryType::NonEquip as u8),
        },
    );
    let toy_item = session.make_inventory_item_object(
        toy_guid,
        toy_item_id,
        player_guid,
        1,
        0,
        wow_constants::ItemContext::None,
        toy_slot,
    );
    session.insert_inventory_item_object(toy_item);

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::AddToy as u16);
    pkt.write_packed_guid(&toy_guid);

    session.handle_add_toy(pkt).await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(
        session
            .inventory_item_objects_like_cpp()
            .contains_key(&toy_guid)
    );
    assert_eq!(
        send_rx.try_recv().unwrap(),
        InventoryChangeFailure::new(InventoryResult::CantEquipEver, toy_guid, ObjectGuid::EMPTY)
            .to_bytes()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn add_toy_rolls_back_without_player_toys_update_when_destroy_fails_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 55);
    let toy_guid = ObjectGuid::create_item(1, 1_003);
    let toy_slot = 5;
    let toy_item_id = 30_000_u32;
    let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "ToyDynamicTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        player_position,
        571,
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    install_add_toy_item_templates(&mut session, toy_item_id, 0);
    session.set_toy_store(Arc::new(wow_data::ToyStore::from_entries([
        wow_data::ToyEntry {
            id: 1,
            source_text: "known".to_string(),
            item_id: toy_item_id_i32,
            flags: 0,
            source_type_enum: 0,
        },
    ])));
    session.insert_inventory_item_like_cpp(
        toy_slot,
        crate::session::InventoryItem {
            guid: toy_guid,
            entry_id: toy_item_id,
            db_guid: toy_guid.counter() as u64,
            inventory_type: Some(wow_constants::InventoryType::NonEquip as u8),
        },
    );
    let toy_item = session.make_inventory_item_object(
        toy_guid,
        toy_item_id,
        player_guid,
        1,
        0,
        wow_constants::ItemContext::None,
        toy_slot,
    );
    session.insert_inventory_item_object(toy_item);

    let (_, _, preflight_item) = session
        .get_inventory_item_by_guid_like_cpp(toy_guid)
        .expect("toy item guid should resolve before AddToy");
    assert!(session.is_toy_item_like_cpp(preflight_item.entry_id));
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&preflight_item.guid)
        .cloned();
    assert_eq!(
        session.can_use_inventory_item_represented_like_cpp(&preflight_item, runtime_item.as_ref()),
        InventoryResult::Ok
    );

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint16(ClientOpcodes::AddToy as u16);
    pkt.write_packed_guid(&toy_guid);

    session.handle_add_toy(pkt).await;

    let first_packet = send_rx.try_recv().ok();
    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.toys_like_cpp().to_vec())
            .unwrap(),
        Vec::<i32>::new()
    );
    assert!(
        first_packet.is_none(),
        "first sent packet: {:?}",
        first_packet
    );
}

#[tokio::test]
async fn add_player_toy_dynamic_field_sends_update_object_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 56);
    let toy_item_id = 30_000_u32;
    let toy_item_id_i32 = i32::try_from(toy_item_id).unwrap();
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);

    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "ToyDynamicTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        player_position,
        571,
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_player_toy_dynamic_field_like_cpp(toy_item_id)
        .expect("canonical current player should receive Player::AddToy dynamic field");
    if let Some(packet) = player_values_update_to_update_object(
        player_guid,
        session.player_map_id_like_cpp(),
        &update,
    ) {
        session.send_packet(&packet);
    }

    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.toys_like_cpp().to_vec())
            .unwrap(),
        vec![toy_item_id_i32]
    );
    let update_packet = send_rx.try_recv().expect("Player::AddToy values update");
    assert_eq!(
        u16::from_le_bytes([update_packet[0], update_packet[1]]),
        ServerOpcodes::UpdateObject as u16
    );
}

#[tokio::test]
async fn collection_item_set_favorite_marks_permanent_appearance_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.represented_item_appearances_like_cpp.insert(65);

    session
        .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
            COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
            65,
            true,
        ))
        .await;

    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(65),
        Some(crate::session::FavoriteAppearanceStateLikeCpp::New)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "AccountTransmogUpdate keeps state only while the legacy opcode is unresolved"
    );
}

#[tokio::test]
async fn collection_item_set_favorite_toggles_known_toy_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.load_represented_account_toys_like_cpp([(30_000, false, true)]);

    session
        .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
            COLLECTION_TYPE_TOYBOX_LIKE_CPP,
            30_000,
            true,
        ))
        .await;

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, true, true)]
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn collection_item_set_favorite_ignores_unknown_toy_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
            COLLECTION_TYPE_TOYBOX_LIKE_CPP,
            40_000,
            true,
        ))
        .await;

    assert!(session.account_toy_rows_like_cpp().is_empty());
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn collection_item_set_favorite_ignores_temporary_or_unknown_appearance_like_cpp() {
    let (mut session, send_rx) = make_session();
    session
        .represented_temporary_item_appearances_like_cpp
        .insert(65, HashSet::from([ObjectGuid::create_item(1, 900)]));

    session
        .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
            COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
            65,
            true,
        ))
        .await;
    session
        .handle_collection_item_set_favorite(collection_item_set_favorite_packet(
            COLLECTION_TYPE_APPEARANCE_LIKE_CPP,
            96,
            true,
        ))
        .await;

    assert!(
        session
            .represented_favorite_item_appearance_state_like_cpp(65)
            .is_none()
    );
    assert!(
        session
            .represented_favorite_item_appearance_state_like_cpp(96)
            .is_none()
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn auction_place_bid_marks_copper_amount_rejected_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager_for_misc_test();
    let player_guid = ObjectGuid::create_player(1, 42);
    let auctioneer = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 90_002, 81);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(crate::session::SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map_for_misc_test(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    add_canonical_auctioneer_for_misc_test(
        &canonical,
        auctioneer,
        Position::new(12.0, 0.0, 0.0, 0.0),
        NPCFlags1::AUCTIONEER.bits(),
    );

    session
        .handle_auction_place_bid(AuctionPlaceBid {
            auctioneer,
            auction_id: 1234,
            bid_amount: 12_301,
            tainted_by: None,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.represented_auction_place_bids_like_cpp(),
        &[RepresentedAuctionPlaceBidLikeCpp {
            auctioneer,
            auction_id: 1234,
            bid_amount: 12_301,
            tainted_by_present: false,
            copper_rejected: true,
        }]
    );
}
