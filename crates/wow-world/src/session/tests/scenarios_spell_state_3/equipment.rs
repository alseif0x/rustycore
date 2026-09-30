//! Existing equipment/publication failures remain application integration cases.

use super::*;

#[test]
fn remove_known_spell_auto_unequip_delinks_offhand_when_store_fails_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let offhand_item_id = 30_005_u32;
    let filler_item_id = 30_006_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_005);
    let player_guid = ObjectGuid::create_player(1, 159);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RemoveOffhandFallback".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                offhand_item_id,
                InventoryType::WeaponOffhand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe as u8,
            ),
            (
                filler_item_id,
                InventoryType::NonEquip,
                0,
                ItemClass::Consumable,
                0,
            ),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(false);
        let _ = player.visualize_item(
            EQUIPMENT_SLOT_OFFHAND,
            offhand_guid,
            VisibleItemValues {
                item_id: offhand_item_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            },
        );
    });
    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 40_000 + i64::from(offset));
        equip_represented_test_item_like_cpp(
            &mut session,
            slot,
            guid,
            filler_item_id,
            InventoryType::NonEquip,
        );
    }

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield,
            stored_destination: None,
            needs_mail_fallback: true,
        }],
        "C++ AutoUnequipOffhandIfNeed falls back to MoveItemFromInventory + mail when CanStoreItem fails"
    );
    assert!(
        !session
            .inventory_items_like_cpp()
            .contains_key(&EQUIPMENT_SLOT_OFFHAND),
        "C++ MoveItemFromInventory removes the item from the offhand slot"
    );
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&offhand_guid)
        .expect("mail fallback keeps the standalone item object represented");
    assert_eq!(runtime_item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(runtime_item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(runtime_item.slot(), NULL_SLOT);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            (
                player.active_data().inv_slots[EQUIPMENT_SLOT_OFFHAND as usize],
                player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
            )
        }),
        Some((ObjectGuid::EMPTY, VisibleItemValues::default())),
        "C++ MoveItemFromInventory clears offhand InvSlot/VisibleItem before mail fallback"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::UpdateObject, ServerOpcodes::UpdateObject],
        "C++ MoveItemFromInventory(update=true) sends player and item values updates before the represented mail fallback"
    );
}

#[test]
fn remove_known_spell_auto_unequip_stores_offhand_in_represented_bag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let offhand_item_id = 30_007_u32;
    let filler_item_id = 30_008_u32;
    let bag_item_id = 30_009_u32;
    let offhand_guid = ObjectGuid::create_item(1, 30_007);
    let bag_guid = ObjectGuid::create_item(1, 30_009);
    let player_guid = ObjectGuid::create_player(1, 160);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "RemoveOffhandBagStore".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[
            (
                offhand_item_id,
                InventoryType::WeaponOffhand,
                0,
                ItemClass::Weapon,
                ItemSubClassWeapon::Axe as u8,
            ),
            (
                filler_item_id,
                InventoryType::NonEquip,
                0,
                ItemClass::Consumable,
                0,
            ),
            (bag_item_id, InventoryType::Bag, 0, ItemClass::Container, 0),
        ],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_OFFHAND,
        offhand_guid,
        offhand_item_id,
        InventoryType::WeaponOffhand,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_BAG_START,
        bag_guid,
        bag_item_id,
        InventoryType::Bag,
    );
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_can_dual_wield_like_cpp(false);
        let _ = player.visualize_item(
            EQUIPMENT_SLOT_OFFHAND,
            offhand_guid,
            VisibleItemValues {
                item_id: offhand_item_id as i32,
                item_appearance_mod_id: 0,
                item_visual: 0,
            },
        );
        let _ = player.store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid);
        let _ = player.register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4);
    });
    for offset in 0..INVENTORY_DEFAULT_SIZE {
        let slot = INVENTORY_SLOT_ITEM_START + offset;
        let guid = ObjectGuid::create_item(1, 41_000 + i64::from(offset));
        equip_represented_test_item_like_cpp(
            &mut session,
            slot,
            guid,
            filler_item_id,
            InventoryType::NonEquip,
        );
    }

    assert!(session.represented_auto_unequip_offhand_if_need_like_cpp(false));

    assert_eq!(
        session.represented_auto_unequip_offhand_requests_like_cpp(),
        &[RepresentedAutoUnequipOffhandLikeCpp {
            item_guid: offhand_guid,
            item_entry: offhand_item_id,
            reason: RepresentedAutoUnequipOffhandReasonLikeCpp::LostDualWield,
            stored_destination: Some((INVENTORY_SLOT_BAG_START, 0)),
            needs_mail_fallback: false,
        }],
        "C++ StoreItem stores the offhand item inside an equipped bag when backpack slots are full"
    );
    assert!(
        !session
            .inventory_items_like_cpp()
            .contains_key(&EQUIPMENT_SLOT_OFFHAND),
        "C++ RemoveItem removes the direct offhand slot before Bag::StoreItem"
    );
    assert_eq!(
        session
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_START, 0)
            .map(|item| item.guid),
        Some(offhand_guid),
        "represented Bag::StoreItem lookup resolves the moved offhand item"
    );
    let runtime_item = session
        .inventory_item_objects_like_cpp()
        .get(&offhand_guid)
        .expect("stored bag item remains a runtime item object");
    assert_eq!(runtime_item.data().contained_in, bag_guid);
    assert_eq!(runtime_item.container_guid(), bag_guid);
    assert_eq!(runtime_item.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(runtime_item.slot(), 0);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| {
            (
                player.active_data().inv_slots[EQUIPMENT_SLOT_OFFHAND as usize],
                player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
                player
                    .inventory()
                    .bags
                    .get(INVENTORY_SLOT_BAG_START as usize)
                    .and_then(Option::as_ref)
                    .and_then(|bag| bag.item_by_pos(0)),
            )
        }),
        Some((
            ObjectGuid::EMPTY,
            VisibleItemValues::default(),
            Some(offhand_guid)
        )),
        "C++ RemoveItem clears offhand InvSlot/VisibleItem and Bag::StoreItem stores the child item in the equipped bag"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject
        ],
        "C++ RemoveItem(update=true) + Bag::StoreItem(update=true) emit player, item, and bag slot values updates"
    );
}

