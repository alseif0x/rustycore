use std::sync::Arc;

use wow_constants::InventoryType;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_entities::{INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START};
use wow_packet::WorldPacket;
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    get_inventory_item_by_pos_for_test, insert_inventory_item_for_test,
    set_equipment_set_guid_generator_for_test,
};

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    set_equipment_set_guid_generator_for_test(
        &mut session,
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    (session, send_rx)
}

fn use_equipment_set_packet(
    guid: u64,
    items: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bits(0, 2);
    for (slot, item) in items.iter().enumerate() {
        pkt.write_guid(item);
        pkt.write_uint8(255);
        pkt.write_uint8(slot as u8);
    }
    pkt.write_uint64(guid);
    pkt
}

fn read_use_equipment_set_result(encoded: Vec<u8>) -> (u64, u8) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::UseEquipmentSetResult)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let reason = packet.read_uint8().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, reason)
}

#[tokio::test]
async fn use_equipment_set_moves_direct_inventory_item_and_sends_result_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = item_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102_0304_0506_0708, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102_0304_0506_0708, 0)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, 0)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START
        )
        .is_none()
    );
}
