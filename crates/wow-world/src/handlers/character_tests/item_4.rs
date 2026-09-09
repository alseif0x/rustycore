//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

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
        .gossip_options
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
#[test]
fn vendor_stored_new_item_keeps_cpp_new_and_bonding_flags() {
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(None, INVENTORY_SLOT_BAG_0, 23),
        ItemFieldFlags::NEW_ITEM.bits()
    );

    let mut template = wow_entities::ItemStorageTemplate::regular_item(700, 1);
    template.bonding = ItemBondingType::OnAcquire;
    assert_eq!(
        vendor_stored_new_item_flags_like_cpp(Some(&template), INVENTORY_SLOT_BAG_0, 23),
        (ItemFieldFlags::NEW_ITEM | ItemFieldFlags::SOULBOUND).bits()
    );
}
#[test]
fn vendor_list_item_limit_matches_cpp_cap() {
    assert!(!vendor_list_reaches_cpp_item_limit(149));
    assert!(vendor_list_reaches_cpp_item_limit(150));
    assert!(vendor_list_reaches_cpp_item_limit(151));
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
    );
    let vendor_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 7, 1);

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert_eq!(
        session.update_vendor_item_current_count(vendor_guid, 700, 5, 60, 1, 2),
        3
    );
    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        3
    );

    if let Some(count) = session.vendor_item_counts.get_mut(&(vendor_guid, 700)) {
        count.last_increment_time = WorldSession::vendor_stock_now_secs().saturating_sub(120);
    }

    assert_eq!(
        session.vendor_item_current_count(vendor_guid, 700, 5, 60, 1),
        5
    );
    assert!(!session.vendor_item_counts.contains_key(&(vendor_guid, 700)));
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
fn sell_item_amount_action_matches_cpp_amount_branch() {
    assert_eq!(
        sell_item_amount_action(5, 0),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 5),
        SellItemAmountAction::FullStack { amount: 5 }
    );
    assert_eq!(
        sell_item_amount_action(5, 2),
        SellItemAmountAction::PartialStack {
            amount: 2,
            remaining: 3,
        }
    );
    assert_eq!(sell_item_amount_action(5, 6), SellItemAmountAction::Invalid);
    assert_eq!(
        sell_item_amount_action(5, -1),
        SellItemAmountAction::Invalid
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
fn vendor_buy_destination_rejects_cpp_slot_over_max_bag_size() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: (MAX_BAG_SIZE + 1) as i32,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        None
    );
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
