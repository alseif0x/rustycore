use std::sync::Arc;

use wow_constants::InventoryType;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_packet::WorldPacket;
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    handle_save_equipment_set_for_test, insert_inventory_item_for_test,
    insert_represented_equipment_set_for_test, represented_equipment_set_for_test,
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

fn save_equipment_set_packet(
    set_type: i32,
    guid: u64,
    set_id: u32,
    ignore_mask: u32,
    pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    appearances: [i32; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    enchants: [i32; 2],
    assigned_spec_index: Option<i32>,
    name: &str,
    icon: &str,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_int32(set_type);
    pkt.write_uint64(guid);
    pkt.write_uint32(set_id);
    pkt.write_uint32(ignore_mask);
    for i in 0..wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP {
        pkt.write_guid(&pieces[i]);
        pkt.write_int32(appearances[i]);
    }
    pkt.write_int32(enchants[0]);
    pkt.write_int32(enchants[1]);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_int32(0);
    pkt.write_bit(assigned_spec_index.is_some());
    pkt.write_bits(name.len() as u32, 8);
    pkt.write_bits(icon.len() as u32, 9);
    if let Some(spec_index) = assigned_spec_index {
        pkt.write_int32(spec_index);
    }
    pkt.write_string(name);
    pkt.write_string(icon);
    pkt
}

fn read_equipment_set_id(encoded: Vec<u8>) -> (u64, i32, u32) {
    let mut packet = WorldPacket::new_client(encoded.as_slice().into());
    assert_eq!(
        packet.server_opcode(),
        Some(wow_constants::ServerOpcodes::EquipmentSetId)
    );
    packet.skip_opcode();
    let guid = packet.read_uint64().unwrap();
    let set_type = packet.read_int32().unwrap();
    let set_id = packet.read_uint32().unwrap();
    assert_eq!(packet.remaining(), 0);
    (guid, set_type, set_id)
}

#[tokio::test]
async fn save_equipment_set_new_equipment_normalizes_and_sends_id_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 55);
    insert_inventory_item_for_test(
        &mut session,
        0,
        InventoryItem {
            guid: item_guid,
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = item_guid;
    let appearances = [77; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];

    handle_save_equipment_set_for_test(
        &mut session,
        save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            appearances,
            [12, 34],
            Some(2),
            "Tank",
            "INV_Helmet_01",
        ),
    )
    .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, 0, 7));
    let saved = represented_equipment_set_for_test(&session, generated_guid).unwrap();
    assert_eq!(saved.guid, generated_guid);
    assert_eq!(saved.set_id, 7);
    assert_eq!(saved.set_name, "Tank");
    assert_eq!(saved.set_icon, "INV_Helmet_01");
    assert_eq!(saved.pieces[0], item_guid);
    assert_eq!(saved.appearances[0], 0);
    assert_eq!(saved.appearances[1], 0);
    assert_eq!(saved.enchants, [0, 0]);
    assert_eq!(saved.assigned_spec_index, 2);
    assert_eq!(
        saved.state,
        wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::New
    );
    assert_ne!(saved.ignore_mask & (1 << 1), 0);
}
#[tokio::test]
async fn save_equipment_set_requires_process_wide_guid_allocator() {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(1);
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
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    handle_save_equipment_set_for_test(
        &mut session,
        save_equipment_set_packet(
            0,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "No allocator",
            "INV_Misc_QuestionMark",
        ),
    )
    .await;

    assert!(send_rx.try_recv().is_err());
    assert!(represented_equipment_set_for_test(&session, 1).is_none());
}
#[tokio::test]
async fn concurrent_sessions_share_equipment_and_transmog_set_guid_namespace_like_cpp() {
    let (mut equipment_session, equipment_rx) = make_session_with_send_capacity(1);
    let (mut transmog_session, transmog_rx) = make_session_with_send_capacity(1);
    let generator = Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(400));
    set_equipment_set_guid_generator_for_test(&mut equipment_session, Arc::clone(&generator));
    set_equipment_set_guid_generator_for_test(&mut transmog_session, Arc::clone(&generator));
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    tokio::join!(
        handle_save_equipment_set_for_test(
            &mut equipment_session,
            save_equipment_set_packet(
                0,
                0,
                7,
                ignore_mask,
                [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
                [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
                [0, 0],
                None,
                "Equipment",
                "INV_Sword_01",
            ),
        ),
        handle_save_equipment_set_for_test(
            &mut transmog_session,
            save_equipment_set_packet(
                1,
                0,
                8,
                ignore_mask,
                [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
                [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
                [0, 0],
                None,
                "Transmog",
                "INV_Chest_Cloth_01",
            ),
        ),
    );

    let equipment = read_equipment_set_id(equipment_rx.try_recv().unwrap());
    let transmog = read_equipment_set_id(transmog_rx.try_recv().unwrap());
    let mut guids = [equipment.0, transmog.0];
    guids.sort_unstable();
    assert_eq!(guids, [400, 401]);
    assert_eq!(equipment.1, 0);
    assert_eq!(transmog.1, 1);
    assert_eq!(generator.next_after_max_used(), 402);
}
#[tokio::test]
async fn save_equipment_set_existing_marks_changed_without_id_packet_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    insert_represented_equipment_set_for_test(
        &mut session,
        100,
        wow_entities::PlayerEquipmentSetLikeCpp::equipment(
            7,
            -1,
            wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
        ),
    );
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    handle_save_equipment_set_for_test(
        &mut session,
        save_equipment_set_packet(
            0,
            100,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Dps",
            "INV_Sword_01",
        ),
    )
    .await;

    assert!(send_rx.try_recv().is_err());
    let saved = represented_equipment_set_for_test(&session, 100).unwrap();
    assert_eq!(saved.set_name, "Dps");
    assert_eq!(saved.assigned_spec_index, -1);
    assert_eq!(
        saved.state,
        wow_entities::PlayerEquipmentSetUpdateStateLikeCpp::Changed
    );
}
#[tokio::test]
async fn save_equipment_set_negative_type_follows_cpp_non_equipment_branch() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let ignore_mask = (1_u32 << wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP) - 1;

    handle_save_equipment_set_for_test(
        &mut session,
        save_equipment_set_packet(
            -1,
            0,
            7,
            ignore_mask,
            [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Odd",
            "INV_Odd",
        ),
    )
    .await;

    let (generated_guid, set_type, set_id) = read_equipment_set_id(send_rx.try_recv().unwrap());
    assert_eq!((generated_guid, set_type, set_id), (1, -1, 7));
    let saved = represented_equipment_set_for_test(&session, generated_guid).unwrap();
    assert_eq!(saved.raw_set_type, -1);
    assert_eq!(
        saved.set_type,
        wow_entities::PlayerEquipmentSetTypeLikeCpp::Transmog
    );
}
#[tokio::test]
async fn save_equipment_set_rejects_equipment_guid_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    insert_inventory_item_for_test(
        &mut session,
        0,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 55),
            entry_id: 100,
            db_guid: 55,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    let mut pieces = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    pieces[0] = ObjectGuid::create_item(1, 99);

    handle_save_equipment_set_for_test(
        &mut session,
        save_equipment_set_packet(
            0,
            0,
            7,
            0,
            pieces,
            [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            [0, 0],
            None,
            "Bad",
            "INV_Bad",
        ),
    )
    .await;

    assert!(send_rx.try_recv().is_err());
    assert!(represented_equipment_set_for_test(&session, 1).is_none());
}
