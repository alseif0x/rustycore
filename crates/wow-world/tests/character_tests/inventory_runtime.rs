//! Inventory packet and committed-runtime contracts from the Character suite.

use std::sync::Arc;
use wow_constants::{EnchantmentSlot, InventoryResult, InventoryType, ItemContext, ServerOpcodes};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_entities::{INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START};
use wow_packet::packets::item::CancelTempEnchantment;
use wow_packet::WorldPacket;
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    apply_inventory_swap_for_test, destroy_inventory_stack_for_test,
    get_inventory_item_by_pos_for_test,
    insert_inventory_item_for_test, insert_inventory_item_object_for_test,
    inventory_descendants_for_test, inventory_item_objects_for_test,
    make_inventory_item_object_for_test, set_equipment_set_guid_generator_for_test,
    set_failing_player_inventory_persistence_port_for_test,
};

pub(super) fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(16);
    (
        WorldSession::new(
            1,
            "TestAccount".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        ),
        send_rx,
    )
}

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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    set_equipment_set_guid_generator_for_test(&mut session, Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn make_storage_move_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
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
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        wow_core::guid::HighGuid::Item,
        1,
    )));
    set_equipment_set_guid_generator_for_test(&mut session, Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

fn insert_cancel_temp_enchant_test_item(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    slot: u8,
    enchantment_id: i32,
) -> ObjectGuid {
    let item_guid = ObjectGuid::create_item(1, 70_000 + i64::from(slot));
    let _ = insert_inventory_item_for_test(
        session,
        slot,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = make_inventory_item_object_for_test(
        session,
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    item.set_enchantment(
        EnchantmentSlot::EnhancementTemporary,
        enchantment_id,
        12_000,
        3,
    );
    let _ = insert_inventory_item_object_for_test(session, item);
    item_guid
}

#[tokio::test]
async fn cancel_temp_enchantment_clears_equipped_temporary_enchant_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 901);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = inventory_item_objects_for_test(&session)
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
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 36, 902);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 36 })
        .await;

    let item = inventory_item_objects_for_test(&session)
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id,
        902
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn cancel_temp_enchantment_ignores_missing_enchant_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    let item_guid = insert_cancel_temp_enchant_test_item(&mut session, player_guid, 15, 0);

    session
        .handle_cancel_temp_enchantment(CancelTempEnchantment { slot: 15 })
        .await;

    let item = inventory_item_objects_for_test(&session)
        .get(&item_guid)
        .unwrap();
    assert_eq!(
        item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].duration,
        12_000
    );
    assert!(send_rx.try_recv().is_err());
}

#[test]
fn recursive_destroy_descendants_are_deepest_first_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let parent_guid = ObjectGuid::create_item(1, 90);
    let child_bag_guid = ObjectGuid::create_item(1, 91);
    let leaf_guid = ObjectGuid::create_item(1, 92);
    session.set_player_guid(Some(player_guid));

    let parent = make_inventory_item_object_for_test(
        &session,
        parent_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_ITEM_START,
    );
    insert_inventory_item_object_for_test(&mut session, parent);
    let mut child_bag = make_inventory_item_object_for_test(
        &session,
        child_bag_guid,
        601,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child_bag.set_container_guid_and_slot(parent_guid, wow_entities::INVENTORY_SLOT_ITEM_START);
    insert_inventory_item_object_for_test(&mut session, child_bag);
    let mut leaf = make_inventory_item_object_for_test(
        &session,
        leaf_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    leaf.set_container_guid_and_slot(child_bag_guid, 0);
    insert_inventory_item_object_for_test(&mut session, leaf);

    let descendants = inventory_descendants_for_test(&session, parent_guid)
        .expect("fixture canonical inventory owner");
    assert_eq!(
        descendants
            .iter()
            .map(|(_, _, item)| item.guid)
            .collect::<Vec<_>>(),
        vec![leaf_guid, child_bag_guid]
    );
}

#[test]
fn committed_swap_updates_top_level_and_nested_container_positions_like_cpp() {
    let (mut session, _send_rx) = make_storage_move_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 80);
    let nested_guid = ObjectGuid::create_item(1, 81);
    let backpack_guid = ObjectGuid::create_item(1, 82);
    session.set_player_guid(Some(player_guid));
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 80,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: backpack_guid,
            entry_id: 701,
            db_guid: 82,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let bag = make_inventory_item_object_for_test(
        &session,
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    insert_inventory_item_object_for_test(&mut session, bag);
    let mut nested = make_inventory_item_object_for_test(
        &session,
        nested_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    nested.set_container_guid_and_slot(bag_guid, INVENTORY_SLOT_BAG_START);
    insert_inventory_item_object_for_test(&mut session, nested);
    let backpack = make_inventory_item_object_for_test(
        &session,
        backpack_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    insert_inventory_item_object_for_test(&mut session, backpack);

    assert!(apply_inventory_swap_for_test(&mut session, 
        INVENTORY_SLOT_BAG_START,
        0,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
    ));
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .map(|item| item.guid),
        Some(nested_guid)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_START, 0)
            .map(|item| item.guid),
        Some(backpack_guid)
    );
}

fn inventory_failure_result(packet: &[u8]) -> i32 {
    assert_eq!(
        u16::from_le_bytes([packet[0], packet[1]]),
        ServerOpcodes::InventoryChangeFailure as u16
    );
    i32::from_le_bytes(packet[2..6].try_into().expect("inventory result bytes"))
}

#[tokio::test]
async fn recursive_destroy_commit_failure_keeps_bag_and_children_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 93);
    let child_guid = ObjectGuid::create_item(1, 94);
    session.set_player_guid(Some(player_guid));
    insert_inventory_item_for_test(
        &mut session,
        wow_entities::INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: bag_guid,
            entry_id: 600,
            db_guid: 93,
            inventory_type: Some(InventoryType::Bag as u8),
        },
    );
    let bag = make_inventory_item_object_for_test(
        &session,
        bag_guid,
        600,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::INVENTORY_SLOT_ITEM_START,
    );
    insert_inventory_item_object_for_test(&mut session, bag.clone());
    let mut child = make_inventory_item_object_for_test(
        &session,
        child_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        0,
    );
    child.set_container_guid_and_slot(bag_guid, wow_entities::INVENTORY_SLOT_ITEM_START);
    insert_inventory_item_object_for_test(&mut session, child);
    set_failing_player_inventory_persistence_port_for_test(&mut session);
    let item = get_inventory_item_by_pos_for_test(
        &session,
        wow_entities::INVENTORY_SLOT_BAG_0,
        wow_entities::INVENTORY_SLOT_ITEM_START,
    )
    .expect("bag metadata");

    assert!(
        !destroy_inventory_stack_for_test(&mut session, 
                wow_entities::INVENTORY_SLOT_BAG_0,
                wow_entities::INVENTORY_SLOT_ITEM_START,
                item,
                Some(bag),
                "recursive destroy test",
            )
            .await
    );
    assert!(
        inventory_item_objects_for_test(&session)
            .contains_key(&bag_guid)
    );
    assert!(
        inventory_item_objects_for_test(&session)
            .contains_key(&child_guid)
    );
    assert_eq!(
        inventory_failure_result(&send_rx.try_recv().expect("transaction failure packet")),
        InventoryResult::InternalBagError as i32
    );
}
