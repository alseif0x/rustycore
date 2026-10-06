//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn registered_item_text_query_preserves_payload_failure_invalid_and_valid_bytes() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp(&mut builder).unwrap();
    let registry = builder.build();
    let entry = *registry.get(ClientOpcodes::ItemTextQuery).unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;
    assert!(send_rx.try_recv().is_err(), "parse failure sends nothing");
    let owner = ObjectGuid::create_player(1, 700);
    let guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 2);
    let mut request = WorldPacket::new_empty();
    request.write_guid(&guid);
    (entry.handler)(&mut session, &catalogs, request).await;
    let bytes = send_rx.try_recv().expect("invalid item response");
    assert_eq!(
        WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QueryItemTextResponse)
    );
    assert_eq!(&bytes[2..5], &[0, 0, 0]);
    assert_eq!(&bytes[5..21], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);

    let mut item =
        session.make_inventory_item_object(guid, 8000, owner, 1, 0, ItemContext::None, 0);
    item.set_text("abc");
    session.insert_inventory_item_object(item);
    let mut request = WorldPacket::new_empty();
    request.write_guid(&guid);
    (entry.handler)(&mut session, &catalogs, request).await;
    let bytes = send_rx.try_recv().expect("valid item response");
    assert_eq!(
        WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QueryItemTextResponse)
    );
    assert_eq!(&bytes[2..5], &[0x80, 0, 0x18]);
    assert_eq!(&bytes[5..8], b"abc");
    assert_eq!(&bytes[8..24], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 24);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn registered_item_text_query_does_not_read_fixture_item_after_same_guid_owner_replacement() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let owner = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(owner));
    let guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 3);
    let mut item =
        session.make_inventory_item_object(guid, 8000, owner, 1, 0, ItemContext::None, 0);
    item.set_text("fixture text must not leak");
    session.insert_inventory_item_object(item);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let mut first = Box::new(wow_entities::Player::new(Some(1), false));
    first.unit_mut().world_mut().object_mut().create(owner);
    let stale = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(first)
        .unwrap();
    session.core.player_handle_like_cpp = Some(stale);
    let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_ne!(stale, replacement_handle);
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp(&mut builder).unwrap();
    let registry = builder.build();
    let entry = *registry.get(ClientOpcodes::ItemTextQuery).unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    let mut request = WorldPacket::new_empty();
    request.write_guid(&guid);
    (entry.handler)(&mut session, &catalogs, request).await;
    let bytes = send_rx
        .try_recv()
        .expect("invalid response for stale owner");
    assert_eq!(
        WorldPacket::from_bytes(&bytes).server_opcode(),
        Some(ServerOpcodes::QueryItemTextResponse)
    );
    assert_eq!(&bytes[2..5], &[0, 0, 0]);
    assert_eq!(&bytes[5..21], &guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .inventory_runtime_like_cpp()
                .item_objects()
                .is_empty()),
        Some(true)
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn bank_domain_registration_exact_set_preserves_slot_flag_metadata() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_bank_handlers_like_cpp(&mut builder).unwrap();
    let registry = builder.build();
    assert_eq!(registry.len(), 1);
    let entry = registry.get(ClientOpcodes::ChangeBankBagSlotFlag).unwrap();
    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_change_bank_bag_slot_flag");
    assert_eq!(
        crate::session::registry::registered_handler_entries_like_cpp()
            .filter(|entry| entry.opcode == ClientOpcodes::ChangeBankBagSlotFlag)
            .count(),
        1
    );
}

#[tokio::test]
async fn registered_bank_slot_flag_preserves_self_admission_and_rejects_invalid_payload_and_bounds()
{
    let (mut session, send_rx, _canonical) = make_bank_slot_session(8);
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_bank_handlers_like_cpp(&mut builder).unwrap();
    let registry = builder.build();
    let entry = *registry.get(ClientOpcodes::ChangeBankBagSlotFlag).unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(send_rx.try_recv().is_err());
    let guid = session.player_guid().unwrap();
    assert!(session.set_player_interaction_source_like_cpp(guid));
    for (slot, flag) in [(7, 4), (2, 32)] {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(slot);
        packet.write_uint32(flag);
        packet.write_bit(true);
        packet.flush_bits();
        (entry.handler)(&mut session, &catalogs, packet).await;
        assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
        assert!(send_rx.try_recv().is_err());
    }
    for enabled in [true, false] {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint32(2);
        packet.write_uint32(4);
        packet.write_bit(enabled);
        packet.flush_bits();
        (entry.handler)(&mut session, &catalogs, packet).await;
        assert_eq!(
            session.represented_bank_bag_slot_flag_like_cpp(2),
            Some(if enabled { 16 } else { 0 })
        );
        let bytes = send_rx.try_recv().expect("complete VALUES publication");
        assert_eq!(
            WorldPacket::from_bytes(&bytes).server_opcode(),
            Some(ServerOpcodes::UpdateObject)
        );
    }
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn registered_bank_slot_flag_rejects_stale_owner_without_fixture_fallback() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let guid = session.player_guid().unwrap();
    assert!(session.set_player_interaction_source_like_cpp(guid));
    let mut first = Box::new(wow_entities::Player::new(Some(1), false));
    first.unit_mut().world_mut().object_mut().create(guid);
    first.set_interaction_source_like_cpp(guid);
    let stale = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(first)
        .expect("first owner");
    session.core.player_handle_like_cpp = Some(stale);
    let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");
    assert_ne!(stale, replacement_handle);
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_bank_handlers_like_cpp(&mut builder).unwrap();
    let registry = builder.build();
    let entry = *registry.get(ClientOpcodes::ChangeBankBagSlotFlag).unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    let mut packet = WorldPacket::new_empty();
    packet.write_uint32(2);
    packet.write_uint32(4);
    packet.write_bit(true);
    packet.flush_bits();
    (entry.handler)(&mut session, &catalogs, packet).await;
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .bank_bag_slot_flag_value_like_cpp(2)),
        Some(Some(0))
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn inventory_domain_registration_exact_set_includes_cancel_temp_enchantment() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_inventory::register_inventory_handlers_like_cpp(&mut builder)
        .expect("Inventory registrations");
    let registry = builder.build();
    // Exact Inventory registrar set: equipment sets, temporary enchantment,
    // item text and the auction/commerce-token family moved by #1263 F5.
    let inplace = [
        (
            ClientOpcodes::AssignEquipmentSetSpec,
            "handle_assign_equipment_set_spec",
        ),
        (
            ClientOpcodes::CancelTempEnchantment,
            "handle_cancel_temp_enchantment",
        ),
        (ClientOpcodes::ItemTextQuery, "handle_item_text_query"),
    ];
    let thread_unsafe = [
        (ClientOpcodes::SaveEquipmentSet, "handle_save_equipment_set"),
        (
            ClientOpcodes::DeleteEquipmentSet,
            "handle_delete_equipment_set",
        ),
        (
            ClientOpcodes::AuctionListBidderItems,
            "handle_auction_list_bidder_items",
        ),
        (ClientOpcodes::AuctionListItems, "handle_auction_list_items"),
        (ClientOpcodes::AuctionPlaceBid, "handle_auction_place_bid"),
        (
            ClientOpcodes::AuctionRemoveItem,
            "handle_auction_remove_item",
        ),
        (ClientOpcodes::AuctionSellItem, "handle_auction_sell_item"),
        (
            ClientOpcodes::AuctionReplicateItems,
            "handle_auction_replicate_items",
        ),
        (
            ClientOpcodes::AuctionListOwnerItems,
            "handle_auction_list_owner_items",
        ),
        (
            ClientOpcodes::AuctionListPendingSales,
            "handle_auction_list_pending_sales",
        ),
        (
            ClientOpcodes::AuctionableTokenSell,
            "handle_auctionable_token_sell",
        ),
        (
            ClientOpcodes::AuctionableTokenSellAtMarketPrice,
            "handle_auctionable_token_sell_at_market_price",
        ),
        (
            ClientOpcodes::CommerceTokenGetLog,
            "handle_commerce_token_get_log",
        ),
    ];
    assert_eq!(registry.len(), inplace.len() + thread_unsafe.len());
    let expected = inplace
        .map(|(opcode, name)| (opcode, PacketProcessing::Inplace, name))
        .into_iter()
        .chain(thread_unsafe.map(|(opcode, name)| (opcode, PacketProcessing::ThreadUnsafe, name)));
    for (opcode, processing, name) in expected {
        let entry = registry.get(opcode).expect("expected Inventory opcode");
        assert_eq!(entry.status, SessionStatus::LoggedIn);
        assert_eq!(entry.processing, processing);
        assert_eq!(entry.handler_name, name);
    }
    assert_eq!(
        crate::session::registry::registered_handler_entries_like_cpp()
            .filter(|entry| entry.opcode == ClientOpcodes::CancelTempEnchantment)
            .count(),
        1
    );
    assert_eq!(
        crate::session::registry::registered_handler_entries_like_cpp()
            .filter(|entry| entry.opcode == ClientOpcodes::ItemTextQuery)
            .count(),
        1
    );
}

#[tokio::test]
async fn registered_cancel_temp_enchantment_clears_only_after_valid_payload_and_equipment_slot() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, guid, 15, 903);
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == ClientOpcodes::CancelTempEnchantment)
        .unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(&mut session, &catalogs, WorldPacket::new_empty()).await;
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .unwrap()
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        903
    );
    assert!(send_rx.try_recv().is_err());
    for slot in [-1, 256, 36] {
        let mut packet = WorldPacket::new_empty();
        packet.write_int32(slot);
        (entry.handler)(&mut session, &catalogs, packet).await;
        assert_eq!(
            session
                .inventory_item_objects_like_cpp()
                .get(&item_guid)
                .unwrap()
                .data()
                .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
                .id,
            903
        );
    }
    let mut packet = WorldPacket::new_empty();
    packet.write_int32(15);
    (entry.handler)(&mut session, &catalogs, packet).await;
    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .unwrap()
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        0
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn registered_cancel_temp_enchantment_does_not_fall_back_when_owner_handle_is_stale() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, guid, 15, 904);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 10)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let mut first = Box::new(wow_entities::Player::new(Some(1), false));
    first.unit_mut().world_mut().object_mut().create(guid);
    let stale_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(first)
        .expect("first owner");
    session.core.player_handle_like_cpp = Some(stale_handle);
    let mut replacement = Box::new(wow_entities::Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");
    assert_ne!(stale_handle, replacement_handle);
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == ClientOpcodes::CancelTempEnchantment)
        .unwrap();
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    let mut packet = WorldPacket::new_empty();
    packet.write_int32(15);
    (entry.handler)(&mut session, &catalogs, packet).await;

    assert_eq!(
        session
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .unwrap()
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        904
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player
                    .inventory_runtime_like_cpp()
                    .item_objects()
                    .is_empty()
            }),
        Some(true)
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn change_bank_bag_slot_flag_toggles_flag_after_banker_activation_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(4);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 4);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());

    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(16));
    assert!(send_rx.try_recv().is_ok(), "flag update should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: false,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(
        send_rx.try_recv().is_ok(),
        "flag clear update should be sent"
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_without_current_bank_like_cpp() {
    let (mut session, send_rx, _canonical) = make_bank_slot_session(1);

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 2,
            flag: 4,
            enabled: true,
        })
        .await;

    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(2), Some(0));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn change_bank_bag_slot_flag_rejects_invalid_slot_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let banker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 5);
    insert_banker_creature(&canonical, banker, NPCFlags1::BANKER.bits());
    session.handle_banker_activate(Hello { unit: banker }).await;
    assert!(send_rx.try_recv().is_ok(), "bank open should be sent");

    session
        .handle_change_bank_bag_slot_flag(ChangeBankBagSlotFlag {
            slot: 7,
            flag: 4,
            enabled: true,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(session.represented_bank_bag_slot_flag_like_cpp(6), Some(0));
}
#[tokio::test]
async fn gossip_banker_selection_replaces_trainer_provenance_like_cpp() {
    let (mut session, send_rx) = make_quest_status_session();
    let banker = creature_guid(15_513, 522);
    attach_legacy_creature(
        &mut session,
        banker,
        15_513,
        NPCFlags1::GOSSIP.bits() | NPCFlags1::BANKER.bits(),
    );
    session.set_player_trainer_interaction_like_cpp(banker, 77);
    session
        .interaction
        .gossip_options_for_test_mut_like_cpp()
        .push(crate::session::GossipOptionInfo {
            gossip_option_id: 51,
            menu_id: 52,
            order_index: 53,
            option_npc: 6,
            action_menu_id: 0,
        });

    session
        .handle_gossip_select_option(wow_packet::packets::gossip::GossipSelectOption {
            gossip_unit: banker,
            gossip_id: 52,
            gossip_option_id: 51,
            promotion_code: String::new(),
        })
        .await;

    assert_eq!(
        WorldPacket::from_bytes(&send_rx.try_recv().unwrap()).server_opcode(),
        Some(ServerOpcodes::NpcInteractionOpenResult)
    );
    assert_eq!(
        session.player_interaction_source_guid_like_cpp(),
        Some(banker)
    );
    assert_eq!(session.player_interaction_trainer_id_like_cpp(), 0);
}
#[tokio::test]
async fn item_text_query_missing_item_sends_cpp_invalid_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 1);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x00);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x00);
    assert_eq!(&bytes[5..21], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 21);
}
#[tokio::test]
async fn item_text_query_inventory_item_sends_text_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let owner_guid = ObjectGuid::create_player(1, 700);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 0, 0, 700, 2);
    let mut item =
        session.make_inventory_item_object(item_guid, 8000, owner_guid, 1, 0, ItemContext::None, 0);
    item.set_text("abc");
    session.insert_inventory_item_object(item);

    session
        .handle_item_text_query(ItemTextQuery { id: item_guid })
        .await;

    let bytes = send_rx.try_recv().expect("item text response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryItemTextResponse as u16
    );
    assert_eq!(bytes[2], 0x80);
    assert_eq!(bytes[3], 0x00);
    assert_eq!(bytes[4], 0x18);
    assert_eq!(&bytes[5..8], b"abc");
    assert_eq!(&bytes[8..24], &item_guid.to_raw_bytes());
    assert_eq!(bytes.len(), 24);
}
#[test]
fn parse_equipment_cache_empty() {
    let eq = parse_equipment_cache("");
    for slot in &eq {
        assert_eq!(slot.display_id, 0);
        assert_eq!(slot.inv_type, 0);
    }
}
#[tokio::test]
async fn cancel_temp_enchantment_clears_equipped_temporary_enchant_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 901);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn cancel_temp_enchantment_ignores_non_equipment_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 36, 902);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 36 })
        .await;

    let item = session
        .inventory_item_objects_like_cpp()
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        902
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn extended_cost_item_turnin_plan_matches_cpp_destroy_order() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
        crate::session::registry::build_dispatch_table(),
    );
    let player_guid = ObjectGuid::create_player(1, 1);
    session.set_player_guid(Some(player_guid));

    for (slot, db_guid, count) in [(35, 10_u64, 4_u32), (36, 11_u64, 5_u32)] {
        let item_guid = ObjectGuid::create_item(1, db_guid as i64);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            player_guid,
            count,
            0,
            ItemContext::Vendor,
            slot,
        );
        session.insert_inventory_item_object(item);
    }

    assert!(session.has_item_count_direct_inventory(700, 9));
    assert!(!session.has_item_count_direct_inventory(700, 10));
    assert_eq!(
        session.plan_destroy_item_count_direct_inventory(700, 6),
        Some(vec![
            ExtendedCostItemTurninChange::Delete {
                slot: 35,
                item_guid: ObjectGuid::create_item(1, 10),
                db_guid: 10,
            },
            ExtendedCostItemTurninChange::Update {
                slot: 36,
                item_guid: ObjectGuid::create_item(1, 11),
                db_guid: 11,
                new_count: 3,
            },
        ])
    );
}
#[test]
fn vendor_item_current_count_updates_like_cpp() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<wow_packet::WorldPacket>(8);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(8);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
        crate::session::registry::build_dispatch_table(),
    );
    let vendor_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 7, 1);

    assert_eq!(
        session
            .interaction
            .vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert_eq!(
        session.update_vendor_item_current_count(vendor_guid, 700, 5, 60, 1, 2),
        3
    );
    assert_eq!(
        session
            .interaction
            .vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        3
    );

    let _ = session
        .interaction
        .vendor_item_last_increment_time_for_test_like_cpp(
            vendor_guid,
            700,
            (wow_entities::game_time_secs_like_cpp().max(0) as u64).saturating_sub(120),
        );

    assert_eq!(
        session
            .interaction
            .vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert!(
        !session
            .interaction
            .has_vendor_item_count_for_test_like_cpp(vendor_guid, 700)
    );
}
#[test]
fn destroy_item_count_action_matches_cpp_direct_item_branch() {
    assert_eq!(
        destroy_item_count_action(5, 0),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 5),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 7),
        DestroyItemCountAction::FullStack
    );
    assert_eq!(
        destroy_item_count_action(5, 2),
        DestroyItemCountAction::PartialStack { new_count: 3 }
    );
}
#[test]
fn item_currently_looted_guard_uses_runtime_loot_generated_state() {
    let mut item = wow_entities::Item::default();
    assert!(!item_is_currently_looted_like_cpp(&item));

    item.set_loot_generated(true);
    assert!(item_is_currently_looted_like_cpp(&item));
}
#[test]
fn sell_non_empty_bag_guard_matches_cpp_is_not_empty_bag() {
    assert!(item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        false
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Chest),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(None, true));
}
#[test]
fn parse_equipment_cache_real_data() {
    // Real data from DB: first slot has inv_type=0, next few slots have gear
    let cache = "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 2470 0 0 0 20 33257 0 1 0";
    let eq = parse_equipment_cache(cache);
    // Slot 0: all zeros
    assert_eq!(eq[0].display_id, 0);
    // Slot 3: inv_type=4, display_id=2470
    assert_eq!(eq[3].inv_type, 4);
    assert_eq!(eq[3].display_id, 2470);
    // Slot 4: inv_type=20, display_id=33257, subclass=1
    assert_eq!(eq[4].inv_type, 20);
    assert_eq!(eq[4].display_id, 33257);
    assert_eq!(eq[4].subclass, 1);
}
