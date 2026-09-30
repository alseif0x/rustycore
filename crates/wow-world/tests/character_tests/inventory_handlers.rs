use super::item_fixture_rows::{basic_item_record, inventory_sparse_template};
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    enable_ownerless_inventory_snapshots_for_test, set_equipment_set_guid_generator_for_test,
    swap_inventory_item_for_test, auto_equip_item_slot_for_test, swap_item_for_test,
    get_inventory_item_by_pos_for_test, insert_inventory_item_for_test,
    insert_inventory_item_object_for_test, make_inventory_item_object_for_test,
    set_failing_player_inventory_persistence_port_for_test,
};
use std::sync::Arc;
use wow_constants::{
    InventoryResult, InventoryType, ItemClass, ItemContext, ServerOpcodes,
};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_data::item::stats::{
    ItemModType, ItemSparseTemplateEntry, ItemStatEntry, ItemStatsStore,
};
use wow_entities::{
    EQUIPMENT_SLOT_MAINHAND, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START,
};
use wow_packet::packets::item::{
    AutoEquipItemSlot, InvUpdate, SwapInvItem, SwapItem,
};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    set_equipment_set_guid_generator_for_test(&mut session, Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    enable_ownerless_inventory_snapshots_for_test(&mut session);
    (session, send_rx)
}

fn make_session_with_realm_send_capacity(
    capacity: usize,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, instance_rx) = make_session_with_send_capacity(capacity);
    let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(capacity);
    wow_world::test_fixtures::install_realm_send_channel_for_test(&mut session, realm_tx);
    (session, instance_rx, realm_rx)
}

fn inventory_failure_result(packet: &[u8]) -> i32 {
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::InventoryChangeFailure as u16
    );
    i32::from_le_bytes(packet[2..6].try_into().expect("inventory result bytes"))
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

fn install_bank_move_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    max_stack_size: i32,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([basic_item_record(
        entry_id,
        ItemClass::Miscellaneous as u8,
        0,
        InventoryType::NonEquip as i8,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        entry_id,
        ItemSparseTemplateEntry {
            stackable: max_stack_size,
            ..inventory_sparse_template(InventoryType::NonEquip as i8)
        },
    )])));
}

fn insert_bank_move_test_item(
    session: &mut WorldSession,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    count: u32,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    insert_inventory_item_for_test(
        session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id,
            db_guid,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let item = make_inventory_item_object_for_test(
        session,
        item_guid,
        entry_id,
        player_guid,
        count,
        0,
        ItemContext::None,
        slot,
    );
    insert_inventory_item_object_for_test(session, item);
    item_guid
}

fn install_equippable_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    inventory_type: InventoryType,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([basic_item_record(
        entry_id,
        ItemClass::Weapon as u8,
        wow_constants::ItemSubClassWeapon::Sword as u8,
        inventory_type as i8,
    )])));
    let sparse = ItemSparseTemplateEntry {
        max_durability: 100,
        ..inventory_sparse_template(inventory_type as i8)
    };
    let stats = std::iter::empty::<(u32, ItemStatEntry)>();
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            stats,
            [(entry_id, sparse)],
            [],
        ),
    ));
}

fn insert_equippable_test_item(
    session: &mut WorldSession,
    bag: u8,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    inventory_type: InventoryType,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    if bag == INVENTORY_SLOT_BAG_0 {
        insert_inventory_item_for_test(
            session,
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid,
                inventory_type: Some(inventory_type as u8),
            },
        );
    }
    let mut item = make_inventory_item_object_for_test(
        session,
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if bag != INVENTORY_SLOT_BAG_0 {
        let bag_guid = session
            .inventory_items_like_cpp()
            .get(&bag)
            .expect("represented bag")
            .guid;
        item.set_container_guid_and_slot(bag_guid, bag);
    }
    insert_inventory_item_object_for_test(session, item);
    item_guid
}

#[tokio::test]
async fn swap_inv_item_empty_source_returns_without_moving_destination_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let destination_guid = ObjectGuid::create_item(1, 62);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: destination_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: None,
        },
    );

    swap_inventory_item_for_test(&mut session, SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1,)
            .map(|item| item.guid),
        Some(destination_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ Player::SwapItem silently returns when the source is empty"
    );
}

#[tokio::test]
async fn swap_inv_item_commit_failure_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 106, 1);
    let item_guid = insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 106, 61, 1);
    set_failing_player_inventory_persistence_port_for_test(&mut session);

    swap_inventory_item_for_test(&mut session, SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(item_guid)
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::InventoryChangeFailure]
    );
}

#[tokio::test]
async fn auto_equip_item_slot_without_persistence_keeps_runtime_unchanged_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_equippable_item_fixture(&mut session, 105, InventoryType::Weapon);
    let item_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        105,
        60,
        InventoryType::Weapon,
    );

    auto_equip_item_slot_for_test(&mut session, AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    let error = send_rx
        .try_recv()
        .expect("missing persistence should report an inventory failure");
    assert_eq!(
        u16::from_le_bytes([error[0], error[1]]),
        ServerOpcodes::InventoryChangeFailure as u16,
    );
}






#[tokio::test]
async fn swap_item_validates_source_and_destination_positions_like_cpp() {
    let (mut session, instance_rx, realm_rx) = make_session_with_realm_send_capacity(2);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    swap_item_for_test(&mut session, SwapItem {
            inv_update: InvUpdate {
                items: vec![(200, 0), (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: 200,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
            slot_b: INVENTORY_SLOT_ITEM_START,
        })
        .await;
    assert_eq!(
        inventory_failure_result(&realm_rx.try_recv().expect("invalid source realm error")),
        InventoryResult::ItemNotFound as i32
    );
    assert!(instance_rx.try_recv().is_err());

    swap_item_for_test(&mut session, SwapItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START), (200, 0)],
            },
            container_slot_a: INVENTORY_SLOT_BAG_0,
            container_slot_b: 200,
            slot_a: INVENTORY_SLOT_ITEM_START,
            slot_b: 0,
        })
        .await;
    assert_eq!(
        inventory_failure_result(
            &realm_rx
                .try_recv()
                .expect("invalid destination realm error")
        ),
        InventoryResult::WrongSlot as i32
    );
    assert!(instance_rx.try_recv().is_err());
}

#[tokio::test]
async fn swap_inv_item_rejects_bank_positions_without_bank_access_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_bank_move_item_fixture(&mut session, 120, 1);
    let source_guid =
        insert_bank_move_test_item(&mut session, INVENTORY_SLOT_ITEM_START, 120, 70, 1);

    swap_inventory_item_for_test(&mut session, SwapInvItem {
            inv_update: InvUpdate {
                items: vec![
                    (INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                    (INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START),
                ],
            },
            src_slot: INVENTORY_SLOT_ITEM_START,
            dst_slot: wow_entities::BANK_SLOT_ITEM_START,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(source_guid)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "C++ silently rejects a bank swap when WorldSession::CanUseBank fails"
    );
}
