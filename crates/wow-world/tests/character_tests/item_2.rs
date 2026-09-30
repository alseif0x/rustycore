//! Item scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[cfg(test)]
fn relocate_bag_exchange_child_like_cpp(
    item: &mut wow_entities::Item,
    destination_bag_guid: ObjectGuid,
    destination_slot: u8,
) {
    item.set_container_guid(destination_bag_guid);
    item.set_contained_in(destination_bag_guid);
    item.set_slot(destination_slot);
}

#[tokio::test]
async fn use_equipment_set_ignored_guid_preserves_slot_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let item_guid = ObjectGuid::create_item(1, 57);
    insert_inventory_item_for_test(
        &mut session,
        2,
        InventoryItem {
            guid: item_guid,
            entry_id: 102,
            db_guid: 57,
            inventory_type: Some(InventoryType::Shoulders as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[2] = ObjectGuid::new(0x0C00_0400_0000_0000_i64, -1_i64);

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0102, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0102, 0)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, 2)
            .unwrap()
            .guid,
        item_guid
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}

#[tokio::test]
async fn use_equipment_set_skips_non_weapon_slots_in_combat_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    set_in_combat_for_test_like_cpp(&mut session, true);
    let head_guid = ObjectGuid::create_item(1, 58);
    let mainhand_guid = ObjectGuid::create_item(1, 59);
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: head_guid,
            entry_id: 103,
            db_guid: 58,
            inventory_type: Some(InventoryType::Head as u8),
        },
    );
    insert_inventory_item_for_test(
        &mut session,
        INVENTORY_SLOT_ITEM_START + 1,
        InventoryItem {
            guid: mainhand_guid,
            entry_id: 104,
            db_guid: 59,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut items = [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
    items[0] = head_guid;
    items[EQUIPMENT_SLOT_MAINHAND as usize] = mainhand_guid;

    session
        .handle_use_equipment_set(use_equipment_set_packet(0x0103, items))
        .await;

    assert_eq!(
        read_use_equipment_set_result(send_rx.try_recv().unwrap()),
        (0x0103, 0)
    );
    assert!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, 0)
            .is_none()
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .unwrap()
            .guid,
        head_guid
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .unwrap()
            .guid,
        mainhand_guid
    );
}

#[test]
fn bag_exchange_child_updates_runtime_container_and_wire_field_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let old_bag_guid = ObjectGuid::create_item(1, 80);
    let new_bag_guid = ObjectGuid::create_item(1, 81);
    let mut child = wow_entities::Item::new(900);
    child.set_container_guid_and_slot(old_bag_guid, INVENTORY_SLOT_BAG_START);
    child.set_contained_in(old_bag_guid);
    child.set_slot(7);

    relocate_bag_exchange_child_like_cpp(&mut child, new_bag_guid, 2);

    assert_eq!(child.container_guid(), new_bag_guid);
    assert_eq!(child.data().contained_in, new_bag_guid);
    assert_eq!(child.slot(), 2);
    assert_ne!(child.data().contained_in, player_guid);
}
