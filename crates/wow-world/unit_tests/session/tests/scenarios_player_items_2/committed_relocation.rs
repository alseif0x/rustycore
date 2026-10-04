// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[test]
fn committed_relocation_same_position_and_occupied_destination_preserve_both_items() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_215);
    session.set_player_guid(Some(owner));
    let source = ObjectGuid::create_item(1, 30_215);
    let destination = ObjectGuid::create_item(1, 30_216);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_215, InventoryType::NonEquip);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START + 1, destination, 30_216, InventoryType::NonEquip);
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 7));
    assert!(!session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, 7));
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1).unwrap().guid, destination);
    let objects = session.resolved_inventory_item_objects_like_cpp().unwrap();
    assert_eq!(objects[&source].slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(objects[&source].count(), 1);
    assert_eq!(objects[&destination].slot(), INVENTORY_SLOT_ITEM_START + 1);
    assert_eq!(objects[&destination].count(), 1);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_relocation_stale_same_guid_owner_rejects_available_fixture_source() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_214);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(owner, "StaleRelocation".to_string(), Position::ZERO, 571, 1, 1, 20, 0));
    session.ensure_canonical_world_map_for_current_player_like_cpp().unwrap();
    let source = ObjectGuid::create_item(1, 30_214);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_214, InventoryType::NonEquip);
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement.unit_mut().world_mut().object_mut().create(owner);
    let replacement_handle = canonical.lock().unwrap().install_detached_player_like_cpp(replacement).unwrap();
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, 1));
    assert_eq!(session.inventory_item_objects_like_cpp()[&source].slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(canonical.lock().unwrap().with_player_like_cpp(replacement_handle, |player| player.inventory_runtime_like_cpp().item_objects().is_empty()), Some(true));
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_relocation_moves_count_and_can_reverse_temporary_projection_without_packets() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_210);
    let source = ObjectGuid::create_item(1, 30_210);
    session.set_player_guid(Some(owner));
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_210, InventoryType::NonEquip);
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, 3));
    assert!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).is_none());
    let item = session.resolved_inventory_item_object_like_cpp(source).unwrap();
    assert_eq!(item.count(), 3);
    assert_eq!(item.slot(), INVENTORY_SLOT_ITEM_START + 1);
    assert_eq!(item.data().contained_in, owner);
    assert!(session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, 1));
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert_eq!(session.resolved_inventory_item_object_like_cpp(source).unwrap().count(), 1);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_relocation_moves_native_bag_and_child_cached_bag_slot() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_211);
    let registry = Arc::new(PlayerRegistry::default());
    bind_canonical_test_player_to_registry_like_cpp(&mut session, &registry, owner, Position::ZERO, 571);
    session.set_player_guid(Some(owner));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    install_remove_spell_offhand_templates_like_cpp(&mut session, &[(30_211, InventoryType::Bag, 0, ItemClass::Container, 0)]);
    let bag = ObjectGuid::create_item(1, 30_211);
    let child = ObjectGuid::create_item(1, 30_212);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_BAG_START, bag, 30_211, InventoryType::Bag);
    let mut item = session.make_inventory_item_object(child, 30_212, owner, 1, 0, ItemContext::None, 2);
    item.set_container_guid_and_slot(bag, INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(item);
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.register_bag_storage(INVENTORY_SLOT_BAG_START, bag, 4).unwrap();
        player.store_bag_item(INVENTORY_SLOT_BAG_START, 2, child).unwrap();
    });
    let _ = drain_server_opcodes(&send_rx);
    assert!(session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1, 1));
    assert_eq!(session.canonical_player_snapshot_like_cpp(|player| {
        let bag = player.inventory().bags[INVENTORY_SLOT_BAG_START as usize + 1].as_ref().unwrap();
        (bag.bag_guid, bag.bag_size, bag.slots[2])
    }), Some((bag, 4, Some(child))));
    let item = session.resolved_inventory_item_object_like_cpp(child).unwrap();
    assert_eq!(item.container_guid(), bag);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_START + 1);
    assert_eq!(item.slot(), 2);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn committed_relocation_missing_source_or_container_preserves_fixture_source() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 30_213)));
    assert!(!session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, 1));
    let source = ObjectGuid::create_item(1, 30_213);
    equip_represented_test_item_like_cpp(&mut session, INVENTORY_SLOT_ITEM_START, source, 30_213, InventoryType::NonEquip);
    let _ = drain_server_opcodes(&send_rx);
    assert!(!session.apply_committed_inventory_item_relocation_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, INVENTORY_SLOT_BAG_START, 0, 1));
    assert_eq!(session.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START).unwrap().guid, source);
    assert_eq!(session.resolved_inventory_item_object_like_cpp(source).unwrap().slot(), INVENTORY_SLOT_ITEM_START);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
