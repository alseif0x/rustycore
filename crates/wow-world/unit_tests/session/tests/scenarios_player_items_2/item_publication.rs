// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[test]
fn inventory_item_publication_preserves_bytes_masks_order_and_current_map() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_200);
    let guid = ObjectGuid::create_item(1, 30_200);
    session.set_player_guid(Some(owner));
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        guid,
        30_200,
        InventoryType::NonEquip,
    );
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_contained_in(owner);
        item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        item.clear_enchantment(EnchantmentSlot::EnhancementTemporary);
    });
    session.set_player_map_position_like_cpp(489, Position::ZERO);
    let _ = drain_server_packet_bytes(&send_rx);
    let relocation_values = session
        .resolved_inventory_item_object_like_cpp(guid)
        .unwrap()
        .data()
        .clone();
    session.send_item_relocation_values_update_like_cpp(
        guid,
        true,
        &[EnchantmentSlot::EnhancementTemporary],
    );
    session.update_inventory_item_object_like_cpp(guid, |item| {
        item.set_item_flag(ItemFieldFlags::SOULBOUND)
    });
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.send_item_dynamic_flags_values_update_like_cpp(guid);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(u16::from_le_bytes([packets[0][6], packets[0][7]]), 489);
    assert_eq!(u16::from_le_bytes([packets[1][6], packets[1][7]]), 571);
    let mut relocation_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    for bit in [
        wow_entities::ITEM_DATA_PARENT_BIT,
        wow_entities::ITEM_DATA_CONTAINED_IN_BIT,
        wow_entities::ITEM_DATA_DYNAMIC_FLAGS2_BIT,
        wow_entities::ITEM_DATA_ENCHANTMENT_PARENT_BIT,
        wow_entities::ITEM_DATA_ENCHANTMENT_FIRST_BIT
            + EnchantmentSlot::EnhancementTemporary as usize,
    ] {
        relocation_mask.set(bit);
    }
    let relocation = wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: relocation_mask,
            values: relocation_values,
        }),
    };
    let mut dynamic_mask = wow_entities::UpdateMask::new(wow_entities::ITEM_DATA_BITS);
    dynamic_mask.set(wow_entities::ITEM_DATA_PARENT_BIT);
    dynamic_mask.set(wow_entities::ITEM_DATA_DYNAMIC_FLAGS_BIT);
    let dynamic = wow_entities::ItemValuesUpdate {
        changed_object_type_mask: 1 << wow_entities::TYPEID_ITEM,
        object_data: None,
        item_data: Some(wow_entities::ItemDataUpdate {
            mask: dynamic_mask,
            values: session
                .resolved_inventory_item_object_like_cpp(guid)
                .unwrap()
                .data()
                .clone(),
        }),
    };
    assert_eq!(
        packets[0],
        wow_world_core::entity_update_bridge::item_values_update_to_update_object(
            guid,
            489,
            &relocation
        )
        .unwrap()
        .to_bytes()
    );
    assert_eq!(
        packets[1],
        wow_world_core::entity_update_bridge::item_values_update_to_update_object(
            guid, 571, &dynamic
        )
        .unwrap()
        .to_bytes()
    );
}

#[test]
fn inventory_container_publication_clears_old_slot_and_uses_fresh_children_and_map() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_201);
    let bag = ObjectGuid::create_item(1, 30_201);
    let child = ObjectGuid::create_item(1, 30_202);
    session.set_player_guid(Some(owner));
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[(30_201, InventoryType::Bag, 0, ItemClass::Container, 0)],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        bag,
        30_201,
        InventoryType::Bag,
    );
    let mut item =
        session.make_inventory_item_object(child, 30_202, owner, 1, 0, ItemContext::None, 1);
    item.set_container_guid_and_slot(bag, INVENTORY_SLOT_BAG_START);
    session.insert_inventory_item_object(item);
    session.set_player_map_position_like_cpp(489, Position::ZERO);
    let _ = drain_server_packet_bytes(&send_rx);
    let publish = |session: &WorldSession, slot| {
        session
            .inventory
            .send_bag_object_slot_values_update_with_access_like_cpp(
                &session.core.owned_inventory_access_like_cpp(),
                &session.core.packet_publication_access_like_cpp(),
                session.catalogs.items.store.as_ref(),
                session.catalogs.items.stats_store.as_ref(),
                bag,
                slot,
            );
    };
    publish(&session, 1);
    session.update_inventory_item_object_like_cpp(child, |item| item.set_slot(2));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    publish(&session, 1);
    publish(&session, 2);
    publish(&session, wow_entities::MAX_BAG_SIZE as u8);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 3);
    for (index, changed_slot, value, map) in [
        (0, 1, child, 489),
        (1, 1, ObjectGuid::EMPTY, 571),
        (2, 2, child, 571),
    ] {
        let mut mask = wow_entities::UpdateMask::new(wow_entities::CONTAINER_DATA_BITS);
        mask.set(wow_entities::CONTAINER_DATA_SLOTS_PARENT_BIT);
        mask.set(wow_entities::CONTAINER_DATA_SLOTS_FIRST_BIT + changed_slot);
        let mut slots = [ObjectGuid::EMPTY; wow_entities::MAX_BAG_SIZE];
        slots[changed_slot] = value;
        let update = wow_entities::BagValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_CONTAINER,
            object_data: None,
            item_data: None,
            container_data: Some(wow_entities::ContainerDataUpdate {
                mask,
                values: wow_entities::ContainerDataValues {
                    num_slots: 4,
                    slots,
                },
            }),
        };
        assert_eq!(
            packets[index],
            wow_world_core::entity_update_bridge::bag_values_update_to_update_object(
                bag, map, &update
            )
            .unwrap()
            .to_bytes()
        );
    }
}

#[test]
fn inventory_item_and_container_publication_reject_stale_owner_without_fixture_packets() {
    let (mut session, _, send_rx) = make_session();
    let owner = ObjectGuid::create_player(1, 30_203);
    let guid = ObjectGuid::create_item(1, 30_203);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        owner,
        "StaleItemPublication".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        20,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .unwrap();
    install_remove_spell_offhand_templates_like_cpp(
        &mut session,
        &[(30_203, InventoryType::Bag, 0, ItemClass::Container, 0)],
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        INVENTORY_SLOT_ITEM_START,
        guid,
        30_203,
        InventoryType::Bag,
    );
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(guid)
            .is_some()
    );
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    let _ = drain_server_packet_bytes(&send_rx);
    session.send_item_relocation_values_update_like_cpp(guid, true, &[]);
    session.send_item_dynamic_flags_values_update_like_cpp(guid);
    session
        .inventory
        .send_bag_object_slot_values_update_with_access_like_cpp(
            &session.core.owned_inventory_access_like_cpp(),
            &session.core.packet_publication_access_like_cpp(),
            session.catalogs.items.store.as_ref(),
            session.catalogs.items.stats_store.as_ref(),
            guid,
            0,
        );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| player
                .inventory_runtime_like_cpp()
                .item_objects()
                .is_empty()),
        Some(true)
    );
}
