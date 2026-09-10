//! Item packet regressions.
//!
//! Moved out of item.rs under #685; every test is unchanged.

use super::*;

#[test]
fn inventory_change_failure_serializes() {
    let pkt = InventoryChangeFailure::error(InventoryResult::ItemNotFound);
    let bytes = pkt.to_bytes();
    // opcode(2) + result(4) + guid1(2) + guid2(2) + containerBSlot(1) = 11 bytes
    assert!(bytes.len() >= 11, "Packet too small: {} bytes", bytes.len());
}

#[test]
fn inventory_change_failure_serializes_level_like_cpp_send_equip_error() {
    let pkt = InventoryChangeFailure::error(InventoryResult::CantEquipLevelI).with_level(42);
    let bytes = pkt.to_bytes();

    assert_eq!(&bytes[bytes.len() - 4..], &42u32.to_le_bytes());
}

#[test]
fn inventory_change_failure_serializes_limit_category_like_cpp_send_equip_error() {
    let pkt = InventoryChangeFailure::error(InventoryResult::ItemMaxLimitCategoryCountExceededIs)
        .with_limit_category(777);
    let bytes = pkt.to_bytes();

    assert_eq!(&bytes[bytes.len() - 4..], &777u32.to_le_bytes());
}

#[test]
fn inventory_change_failure_serializes_bind_confirm_context_like_cpp() {
    let pkt = InventoryChangeFailure::error(InventoryResult::EventAutoequipBindConfirm)
        .with_bind_confirm_context(ObjectGuid::new(0, 0x0102), 37, ObjectGuid::new(0, 0x0506));
    let bytes = pkt.to_bytes();

    assert_eq!(
        &bytes[bytes.len() - 12..],
        &[0x03, 0x00, 0x02, 0x01, 37, 0, 0, 0, 0x03, 0x00, 0x06, 0x05,]
    );
}

#[test]
fn swap_inv_item_parses() {
    // Real packet from WoW 3.4.3 client:
    // InvUpdate: 2 bits = 2 (0x80 = 10_000000), flush, then 2x(container, slot)
    // followed by dst_slot=40 src_slot=36
    let mut pkt = WorldPacket::from_bytes(&[
        SwapInvItem::OPCODE as u8,
        (SwapInvItem::OPCODE as u16 >> 8) as u8,
        0x80, // 2 bits: count=2, rest padding
        0xFF,
        0x28, // item[0]: container=255, slot=40
        0xFF,
        0x24, // item[1]: container=255, slot=36
        0x28, // dst_slot=40
        0x24, // src_slot=36
    ]);
    pkt.skip_opcode();
    let swap = SwapInvItem::read(&mut pkt).unwrap();
    assert_eq!(swap.inv_update.items.len(), 2);
    assert_eq!(swap.dst_slot, 40);
    assert_eq!(swap.src_slot, 36);
}

#[test]
fn swap_inv_item_parses_zero_inv_update() {
    // InvUpdate with 0 items (bits 00 = 0x00)
    let mut pkt = WorldPacket::from_bytes(&[
        SwapInvItem::OPCODE as u8,
        (SwapInvItem::OPCODE as u16 >> 8) as u8,
        0x00, // 2 bits: count=0
        15,   // dst
        35,   // src
    ]);
    pkt.skip_opcode();
    let swap = SwapInvItem::read(&mut pkt).unwrap();
    assert_eq!(swap.inv_update.items.len(), 0);
    assert_eq!(swap.dst_slot, 15);
    assert_eq!(swap.src_slot, 35);
}

#[test]
fn auto_equip_item_parses() {
    // InvUpdate with 0 items, then pack_slot=255 slot=35
    let mut pkt = WorldPacket::from_bytes(&[
        AutoEquipItem::OPCODE as u8,
        (AutoEquipItem::OPCODE as u16 >> 8) as u8,
        0x00, // 2 bits: count=0
        255,  // pack_slot (default backpack)
        35,   // slot
    ]);
    pkt.skip_opcode();
    let eq = AutoEquipItem::read(&mut pkt).unwrap();
    assert_eq!(eq.inv_update.items.len(), 0);
    assert_eq!(eq.pack_slot, 255);
    assert_eq!(eq.slot, 35);
}

#[test]
fn auto_equip_item_slot_parses_cpp_shape() {
    let item = ObjectGuid::create_item(1, 55);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(1, 2);
    pkt.write_uint8(255);
    pkt.write_uint8(35);
    pkt.write_guid(&item);
    pkt.write_uint8(15);
    pkt.reset_read();

    let eq = AutoEquipItemSlot::read(&mut pkt).unwrap();

    assert_eq!(eq.inv_update.items, vec![(255, 35)]);
    assert_eq!(eq.item, item);
    assert_eq!(eq.item_dst_slot, 15);
    assert_eq!(pkt.remaining(), 0);
}

#[test]
fn swap_item_parses() {
    // InvUpdate with 0 items, then containerB=255 containerA=255 slotB=15 slotA=35
    let mut pkt = WorldPacket::from_bytes(&[
        SwapItem::OPCODE as u8,
        (SwapItem::OPCODE as u16 >> 8) as u8,
        0x00, // 2 bits: count=0
        255,  // containerSlotB
        255,  // containerSlotA
        15,   // slotB
        35,   // slotA
    ]);
    pkt.skip_opcode();
    let swap = SwapItem::read(&mut pkt).unwrap();
    assert_eq!(swap.inv_update.items.len(), 0);
    assert_eq!(swap.container_slot_b, 255);
    assert_eq!(swap.container_slot_a, 255);
    assert_eq!(swap.slot_b, 15);
    assert_eq!(swap.slot_a, 35);
}

#[test]
fn auto_store_bag_item_parses() {
    // InvUpdate with 0 items, then C++ order: containerB, containerA, slotA.
    let mut pkt = WorldPacket::from_bytes(&[
        AutoStoreBagItem::OPCODE as u8,
        (AutoStoreBagItem::OPCODE as u16 >> 8) as u8,
        0x00, // 2 bits: count=0
        11,   // containerSlotB
        22,   // containerSlotA
        5,    // slotA
    ]);
    pkt.skip_opcode();
    let store = AutoStoreBagItem::read(&mut pkt).unwrap();
    assert_eq!(store.inv_update.items.len(), 0);
    assert_eq!(store.container_slot_a, 22);
    assert_eq!(store.container_slot_b, 11);
    assert_eq!(store.slot_a, 5);
}

#[test]
fn destroy_item_parses() {
    let mut pkt = WorldPacket::from_bytes(&[
        DestroyItemPkt::OPCODE as u8,
        (DestroyItemPkt::OPCODE as u16 >> 8) as u8,
        1,
        0,
        0,
        0,   // count=1
        255, // containerId
        35,  // slotNum
    ]);
    pkt.skip_opcode();
    let destroy = DestroyItemPkt::read(&mut pkt).unwrap();
    assert_eq!(destroy.count, 1);
    assert_eq!(destroy.container_id, 255);
    assert_eq!(destroy.slot_num, 35);
}

#[test]
fn cancel_temp_enchantment_reads_cpp_slot() {
    let mut pkt = WorldPacket::from_bytes(&[
        CancelTempEnchantment::OPCODE as u8,
        (CancelTempEnchantment::OPCODE as u16 >> 8) as u8,
        15,
        0,
        0,
        0,
    ]);
    pkt.skip_opcode();
    let cancel = CancelTempEnchantment::read(&mut pkt).unwrap();
    assert_eq!(cancel.slot, 15);
}

#[test]
fn item_instance_writes_cpp_field_order() {
    let instance = ItemInstance {
        item_id: 0x1122_3344,
        random_properties_seed: -2,
        random_properties_id: 3,
        item_bonus: None,
        modifications: ItemModList::default(),
    };
    let mut pkt = WorldPacket::new_empty();
    instance.write(&mut pkt);

    assert_eq!(
        pkt.data(),
        &[
            0x44, 0x33, 0x22, 0x11, 0xFE, 0xFF, 0xFF, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    );
}

#[test]
fn item_instance_writes_modifications_before_bonus_like_cpp() {
    let instance = ItemInstance {
        item_id: 10,
        random_properties_seed: 20,
        random_properties_id: -30,
        item_bonus: Some(ItemBonuses {
            context: 4,
            bonus_list_ids: vec![100, 200],
        }),
        modifications: ItemModList {
            values: vec![ItemMod::new(-5, 3), ItemMod::new(7, 4)],
        },
    };
    let mut pkt = WorldPacket::new_empty();
    instance.write(&mut pkt);

    assert_eq!(
        pkt.data(),
        &[
            10, 0, 0, 0, 20, 0, 0, 0, 0xE2, 0xFF, 0xFF, 0xFF, 0x80, 0x08, 0xFB, 0xFF, 0xFF, 0xFF,
            3, 7, 0, 0, 0, 4, 4, 2, 0, 0, 0, 100, 0, 0, 0, 200, 0, 0, 0,
        ]
    );
}

#[test]
fn item_push_result_writes_cpp_order_and_bits() {
    let packet = ItemPushResult {
        player_guid: ObjectGuid::new(0, 0x0102),
        slot: 4,
        slot_in_bag: -1,
        item: ItemInstance {
            item_id: 9001,
            random_properties_seed: 12,
            random_properties_id: -77,
            item_bonus: None,
            modifications: ItemModList::default(),
        },
        quest_log_item_id: 777,
        quantity: 3,
        quantity_in_inventory: 9,
        dungeon_encounter_id: 615,
        battle_pet_species_id: 123,
        battle_pet_breed_id: 188,
        battle_pet_breed_quality: 26,
        battle_pet_level: 25,
        item_guid: ObjectGuid::new(0, 0x0506),
        pushed: true,
        display_text: ItemPushResultDisplayType::EncounterLoot,
        created: false,
        is_bonus_roll: false,
        is_encounter_loot: true,
    };
    let mut pkt = WorldPacket::new_empty();
    packet.write(&mut pkt);

    assert_eq!(
        pkt.data(),
        &[
            0x03, 0x00, 0x02, 0x01, 4, 0xFF, 0xFF, 0xFF, 0xFF, 0x09, 0x03, 0x00, 0x00, 3, 0, 0, 0,
            9, 0, 0, 0, 0x67, 0x02, 0x00, 0x00, 123, 0, 0, 0, 188, 0, 0, 0, 26, 0, 0, 0, 25, 0, 0,
            0, 0x03, 0x00, 0x06, 0x05, 0x92, 0x29, 0x23, 0x00, 0x00, 12, 0, 0, 0, 0xB3, 0xFF, 0xFF,
            0xFF, 0x00, 0x00,
        ]
    );
}

#[test]
fn item_time_update_writes_cpp_order() {
    let packet = ItemTimeUpdate {
        item_guid: ObjectGuid::new(0, 0x0102),
        duration_left: 300,
    };
    let mut pkt = WorldPacket::new_empty();
    packet.write(&mut pkt);

    assert_eq!(
        pkt.data(),
        &[0x03, 0x00, 0x02, 0x01, 0x2C, 0x01, 0x00, 0x00,]
    );
}

#[test]
fn item_enchant_time_update_writes_cpp_order() {
    let packet = ItemEnchantTimeUpdate {
        owner_guid: ObjectGuid::new(0, 0x0102),
        item_guid: ObjectGuid::new(0, 0x0506),
        duration_left: 45,
        slot: 2,
    };
    let mut pkt = WorldPacket::new_empty();
    packet.write(&mut pkt);

    assert_eq!(
        pkt.data(),
        &[
            0x03, 0x00, 0x06, 0x05, 45, 0, 0, 0, 2, 0, 0, 0, 0x03, 0x00, 0x02, 0x01,
        ]
    );
}

#[test]
fn item_purchase_data_client_packets_read_cpp_guid() {
    let guid = ObjectGuid::new(0, 0x0102);
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    assert_eq!(
        GetItemPurchaseData::read(&mut pkt).unwrap(),
        GetItemPurchaseData { item_guid: guid }
    );

    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    assert_eq!(
        ItemPurchaseRefund::read(&mut pkt).unwrap(),
        ItemPurchaseRefund { item_guid: guid }
    );
}

#[test]
fn set_item_purchase_data_writes_cpp_order() {
    let mut contents = ItemPurchaseContents {
        money: 0x0807_0605_0403_0201,
        ..Default::default()
    };
    contents.items[0] = ItemPurchaseRefundItem {
        item_id: 11,
        item_count: 2,
    };
    contents.currencies[0] = ItemPurchaseRefundCurrency {
        currency_id: 390,
        currency_count: 5,
    };
    let packet = SetItemPurchaseData {
        item_guid: ObjectGuid::new(0, 0x0102),
        contents,
        flags: 0xAABB_CCDD,
        purchase_time: 0x1122_3344,
    };

    let mut pkt = WorldPacket::new_empty();
    packet.write(&mut pkt);

    assert_eq!(pkt.read_packed_guid().unwrap(), packet.item_guid);
    assert_eq!(pkt.read_uint64().unwrap(), contents.money);
    assert_eq!(pkt.read_int32().unwrap(), 11);
    assert_eq!(pkt.read_int32().unwrap(), 2);
    for _ in 1..5 {
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
    }
    assert_eq!(pkt.read_int32().unwrap(), 390);
    assert_eq!(pkt.read_int32().unwrap(), 5);
    for _ in 1..5 {
        assert_eq!(pkt.read_int32().unwrap(), 0);
        assert_eq!(pkt.read_int32().unwrap(), 0);
    }
    assert_eq!(pkt.read_uint32().unwrap(), 0xAABB_CCDD);
    assert_eq!(pkt.read_uint32().unwrap(), 0x1122_3344);
}

#[test]
fn item_purchase_refund_result_writes_optional_contents_bit() {
    let guid = ObjectGuid::new(0, 0x0102);
    let empty = ItemPurchaseRefundResult {
        item_guid: guid,
        result: 10,
        contents: None,
    };
    let mut pkt = WorldPacket::new_empty();
    empty.write(&mut pkt);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint8().unwrap(), 10);
    assert!(!pkt.read_bit().unwrap());

    let ok = ItemPurchaseRefundResult {
        item_guid: guid,
        result: 0,
        contents: Some(ItemPurchaseContents {
            money: 7,
            ..Default::default()
        }),
    };
    let mut pkt = WorldPacket::new_empty();
    ok.write(&mut pkt);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert!(pkt.read_bit().unwrap());
    assert_eq!(pkt.read_uint64().unwrap(), 7);
}

#[test]
fn item_expire_purchase_refund_writes_guid_only() {
    let guid = ObjectGuid::new(0, 0x0102);
    let mut pkt = WorldPacket::new_empty();
    ItemExpirePurchaseRefund { item_guid: guid }.write(&mut pkt);
    assert_eq!(pkt.read_packed_guid().unwrap(), guid);
    assert!(pkt.is_empty());
}

#[test]
fn equip_slot_mapping() {
    let empty = std::collections::HashMap::new();
    assert_eq!(equip_slot_for_inventory_type(1, &empty), Some(0)); // Head
    assert_eq!(equip_slot_for_inventory_type(5, &empty), Some(4)); // Chest
    assert_eq!(equip_slot_for_inventory_type(16, &empty), Some(14)); // Cloak
    assert_eq!(equip_slot_for_inventory_type(17, &empty), Some(15)); // 2H Weapon
    assert_eq!(equip_slot_for_inventory_type(18, &empty), Some(30)); // Bag
    assert_eq!(equip_slot_for_inventory_type(0, &empty), None); // Non-equippable
}

#[test]
fn equip_slot_for_bag_uses_cpp_bag_slot_range() {
    let occupied = std::collections::HashMap::from([(30, ()), (31, ()), (32, ())]);

    assert_eq!(equip_slot_for_inventory_type(18, &occupied), Some(33));
}
