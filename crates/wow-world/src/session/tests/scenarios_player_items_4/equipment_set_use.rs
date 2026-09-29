//! Equipment-set use packet scenarios against the private Session owner.

use super::*;

fn use_equipment_set_packet_for_test(
    guid: u64,
    items: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
) -> wow_packet::WorldPacket {
    let mut packet = wow_packet::WorldPacket::new_empty();
    packet.write_bits(0, 2);
    for (slot, item) in items.iter().enumerate() {
        packet.write_guid(item);
        packet.write_uint8(255);
        packet.write_uint8(slot as u8);
    }
    packet.write_uint64(guid);
    packet
}

fn read_use_equipment_set_result_for_test(encoded: Vec<u8>) -> (u64, u8) {
    let mut packet = wow_packet::WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::UseEquipmentSetResult)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let reason = packet.read_uint8().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, reason)
}

#[tokio::test]
async fn use_equipment_set_applies_represented_item_mods_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 66);
    let entry_id = 111;
    session.set_player_guid(Some(player_guid));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (ItemModType::Strength as i8, 11)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    let _ = session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: 66,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let item = session.make_inventory_item_object(
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    let _ = session.insert_inventory_item_object(item);
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[wow_entities::EQUIPMENT_SLOT_MAINHAND as usize] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet_for_test(0x0203, items))
        .await;

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base[0],
        11,
        "C++ HandleUseEquipmentSet reaches SwapItem -> EquipItem -> _ApplyItemMods"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UseEquipmentSetResult
        ]
    );
}

#[tokio::test]
async fn use_equipment_set_empty_slot_unequips_to_backpack_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let item_guid = ObjectGuid::create_item(1, 56);
    let _ = session.insert_inventory_item_like_cpp(
        1,
        InventoryItem {
            guid: item_guid,
            entry_id: 101,
            db_guid: 56,
            inventory_type: Some(InventoryType::Neck as u8),
        },
    );

    session
        .handle_use_equipment_set(use_equipment_set_packet_for_test(
            0x0102_0304_0506_0709,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
        ))
        .await;

    assert_eq!(
        read_use_equipment_set_result_for_test(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0709, 0)
    );
    assert!(session
        .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, 1)
        .is_none());
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}
