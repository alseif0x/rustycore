//! Inventory packet admission cases which return before owner-dependent planning.

use std::sync::Arc;
use wow_constants::InventoryType;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_entities::{
    EQUIPMENT_SLOT_MAINHAND, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START,
    INVENTORY_SLOT_ITEM_START,
};
use wow_packet::packets::item::{
    AutoEquipItem, AutoEquipItemSlot, AutoStoreBagItem, InvUpdate, SwapInvItem, SwapItem,
};
use wow_packet::WorldPacket;
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    auto_equip_item_for_test, auto_equip_item_slot_for_test, auto_store_bag_item_for_test,
    get_inventory_item_by_pos_for_test, insert_inventory_item_for_test,
    set_equipment_set_guid_generator_for_test, swap_inventory_item_for_test, swap_item_for_test,
};

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
    (session, send_rx)
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_bad_inv_count_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 61);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 106,
            db_guid: 61,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    auto_equip_item_slot_for_test(&mut session, AutoEquipItemSlot {
            inv_update: InvUpdate { items: Vec::new() },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}

#[tokio::test]
async fn swap_inv_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    swap_inventory_item_for_test(&mut session, SwapInvItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            src_slot: 250,
            dst_slot: 251,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before slot validation or equip errors"
    );
}

#[tokio::test]
async fn swap_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    swap_item_for_test(&mut session, SwapItem {
            inv_update: InvUpdate { items: Vec::new() },
            container_slot_a: INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_START,
            slot_a: 0,
            slot_b: 1,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before container validation"
    );
}

#[tokio::test]
async fn auto_equip_item_rejects_bad_inv_update_count_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    auto_equip_item_for_test(&mut session, AutoEquipItem {
            inv_update: InvUpdate { items: Vec::new() },
            pack_slot: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on invalid InvUpdate count before missing-item handling"
    );
}

#[tokio::test]
async fn auto_store_bag_item_rejects_non_empty_inv_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    auto_store_bag_item_for_test(&mut session, AutoStoreBagItem {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            container_slot_a: wow_entities::INVENTORY_SLOT_BAG_START,
            container_slot_b: INVENTORY_SLOT_BAG_0,
            slot_a: 0,
        })
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "C++ returns on non-empty InvUpdate before source-container validation"
    );
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_source_position_mismatch_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 62);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 107,
            db_guid: 62,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    auto_equip_item_slot_for_test(&mut session, AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)],
            },
            item: item_guid,
            item_dst_slot: EQUIPMENT_SLOT_MAINHAND,
        })
        .await;

    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .is_none()
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
}

#[tokio::test]
async fn auto_equip_item_slot_rejects_non_equipment_destination_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let item_guid = ObjectGuid::create_item(1, 63);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: item_guid,
            entry_id: 108,
            db_guid: 63,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );

    auto_equip_item_slot_for_test(&mut session, AutoEquipItemSlot {
            inv_update: InvUpdate {
                items: vec![(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)],
            },
            item: item_guid,
            item_dst_slot: INVENTORY_SLOT_ITEM_START + 1,
        })
        .await;

    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1)
            .is_none()
    );
}
