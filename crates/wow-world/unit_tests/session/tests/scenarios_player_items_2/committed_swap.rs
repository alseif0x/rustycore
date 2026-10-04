// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[test]
fn committed_swap_exchanges_top_level_items_and_object_containers_without_packets() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_190);
    session.set_player_guid(Some(owner));
    let source = ObjectGuid::create_item(1, 30_190);
    let destination = ObjectGuid::create_item(1, 30_191);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_190, InventoryType::NonEquip);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START + 1, destination, 30_191, InventoryType::NonEquip);
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.apply_committed_inventory_item_swap_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1));
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, destination);
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1).unwrap().guid, source);
    let items = session.resolved_inventory_item_objects_like_cpp().unwrap();
    assert_eq!(items[&source].slot(), INVENTORY_SLOT_ITEM_START + 1);
    assert_eq!(items[&destination].slot(), INVENTORY_SLOT_ITEM_START);
    assert!(items[&source].container_guid().is_empty());
    assert_eq!(items[&source].data().contained_in, owner);
    assert_eq!(items[&destination].data().contained_in, owner);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_swap_moves_bag_children_and_registers_native_destination_bags() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_192);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(&mut session, &registry, owner, Position::ZERO, 571);
    session.set_player_guid(Some(owner));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    install_remove_spell_offhand_templates_like_cpp(&mut session, &[
        (30_192, InventoryType::Bag, 0, ItemClass::Container, 0),
        (30_193, InventoryType::Bag, 0, ItemClass::Container, 0),
    ]);
    let source = ObjectGuid::create_item(1, 30_192);
    let destination = ObjectGuid::create_item(1, 30_193);
    let source_child = ObjectGuid::create_item(1, 30_194);
    let destination_child = ObjectGuid::create_item(1, 30_195);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_BAG_START, source, 30_192, InventoryType::Bag);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_BAG_START + 1, destination, 30_193, InventoryType::Bag);
    for (guid, bag_guid, bag_slot, child_slot) in [
        (source_child, source, INVENTORY_SLOT_BAG_START, 1),
        (destination_child, destination, INVENTORY_SLOT_BAG_START + 1, 2),
    ] {
        let mut item = session.make_inventory_item_object(guid, 30_194, owner, 1, 0, ItemContext::None, child_slot);
        item.set_container_guid_and_slot(bag_guid, bag_slot);
        session.insert_inventory_item_object(item);
    }
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.register_bag_storage(INVENTORY_SLOT_BAG_START, source, 4).unwrap();
        player.register_bag_storage(INVENTORY_SLOT_BAG_START + 1, destination, 4).unwrap();
        player.store_bag_item(INVENTORY_SLOT_BAG_START, 1, source_child).unwrap();
        player.store_bag_item(INVENTORY_SLOT_BAG_START + 1, 2, destination_child).unwrap();
    });
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.apply_committed_inventory_item_swap_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1));
    assert_eq!(session.canonical_player_snapshot_like_cpp(|player| {
        let bags = &player.inventory().bags;
        let source_bag = bags[INVENTORY_SLOT_BAG_START as usize + 1].as_ref().unwrap();
        let destination_bag = bags[INVENTORY_SLOT_BAG_START as usize].as_ref().unwrap();
        (source_bag.bag_guid, source_bag.bag_size, source_bag.slots[1], destination_bag.bag_guid, destination_bag.slots[2])
    }), Some((source, 4, Some(source_child), destination, Some(destination_child))));
    let items = session.resolved_inventory_item_objects_like_cpp().unwrap();
    assert_eq!(items[&source_child].container_guid(), source);
    assert_eq!(items[&destination_child].container_guid(), destination);
    assert_eq!(items[&source_child].bag_slot(), INVENTORY_SLOT_BAG_START + 1);
    assert_eq!(items[&destination_child].bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(items[&source_child].slot(), 1);
    assert_eq!(items[&destination_child].slot(), 2);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_swap_missing_destination_rejects_without_mutating_source() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_196)));
    let source = ObjectGuid::create_item(1, 30_196);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_196, InventoryType::NonEquip);
    assert!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).is_some());
    assert!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START).is_none());
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_committed_inventory_item_swap_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_START, 0));
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert_eq!(session.resolved_inventory_item_object_like_cpp(source).unwrap().slot(), INVENTORY_SLOT_ITEM_START);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_swap_stale_same_guid_owner_does_not_use_fixture_or_replacement() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_197);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(owner, "StaleCommittedSwap".to_string(), Position::ZERO, 571, 1, 1, 20, 0));
    session.ensure_canonical_world_map_for_current_player_like_cpp().unwrap();
    let source = ObjectGuid::create_item(1, 30_197);
    let destination = ObjectGuid::create_item(1, 30_198);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_197, InventoryType::NonEquip);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START + 1, destination, 30_198, InventoryType::NonEquip);
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(owner);
    let replacement_handle = canonical.lock().unwrap().install_detached_player_like_cpp(replacement).unwrap();
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_committed_inventory_item_swap_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1));
    assert_eq!(session.inventory_item_objects_like_cpp()[&source].slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(session.inventory_item_objects_like_cpp()[&destination].slot(), INVENTORY_SLOT_ITEM_START + 1);
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(replacement_handle, |player| player.inventory_runtime_like_cpp().item_objects().is_empty()), Some(true));
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
