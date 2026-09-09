//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn can_bank_item_fills_specific_slot_then_continues_bank_search_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 720));
    source.object_mut().set_entry(6948);
    source.set_count(10);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 721));
    existing.object_mut().set_entry(6948);
    existing.set_count(15);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        &existing,
    )];
    let mut args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        Some(&source),
    );
    args.slot_items = &slot_items;
    let mut dest = Vec::new();

    assert_eq!(player.can_bank_item(&mut dest, args), InventoryResult::Ok);
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START), 5),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1),
                5
            ),
        ]
    );
}
#[test]
fn can_bank_item_general_search_and_full_bank_match_cpp() {
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 730));
    source.object_mut().set_entry(6948);
    source.set_count(3);
    let player = Player::new(None, false);
    let mut dest = Vec::new();

    assert_eq!(
        player.can_bank_item(
            &mut dest,
            can_bank_args(NULL_BAG, NULL_SLOT, Some(&proto), Some(&source)),
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START),
            3,
        )]
    );

    let mut occupied_items = Vec::new();
    for idx in 0..(BANK_SLOT_ITEM_END - BANK_SLOT_ITEM_START) {
        let mut occupied = Item::default();
        occupied
            .object_mut()
            .create(ObjectGuid::create_item(1, 800 + idx as i64));
        occupied.object_mut().set_entry(9999);
        occupied.set_count(1);
        occupied_items.push(occupied);
    }
    let slot_items = occupied_items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            ItemSlotRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + idx as u8, item)
        })
        .collect::<Vec<_>>();
    let mut full_args = can_bank_args(NULL_BAG, NULL_SLOT, Some(&proto), Some(&source));
    full_args.slot_items = &slot_items;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), full_args),
        InventoryResult::BankFull
    );
}
#[test]
fn active_player_fields_and_inventory_slots_mark_cpp_bits() {
    let mut player = Player::new(None, false);

    player.set_xp(123);
    player.set_next_level_xp(456);
    player.set_honor_like_cpp(789);
    player.set_honor_next_level_like_cpp(8_800);
    player.set_free_primary_professions(2);
    player.set_watched_faction_index_like_cpp(42);
    player.set_inventory_slot_count(16);
    player.set_inv_slot(3, ObjectGuid::new(4, 5));

    assert_eq!(player.active_data().xp, 123);
    assert_eq!(player.active_data().next_level_xp, 456);
    assert_eq!(player.active_data().honor, 789);
    assert_eq!(player.active_data().honor_next_level, 8_800);
    assert_eq!(player.active_data().character_points, 2);
    assert_eq!(player.active_data().watched_faction_index, 42);
    assert_eq!(player.active_data().num_backpack_slots, 16);
    assert_eq!(player.active_data().inv_slots[3], ObjectGuid::new(4, 5));
    assert_eq!(player.active_data().buyback_price, [0; BUYBACK_SLOT_COUNT]);
    assert_eq!(
        player.active_data().buyback_timestamp,
        [0; BUYBACK_SLOT_COUNT]
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_XP_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_NEXT_LEVEL_XP_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_CHARACTER_POINTS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT + 3)
    );
    assert_eq!(player.active_data().transmog, Vec::<u32>::new());
    assert_eq!(player.active_data().transmog_update_mask, None);

    let transmog_slot = player.add_transmog_block_like_cpp(0);
    assert_eq!(transmog_slot, 0);
    assert_eq!(player.active_data().transmog, vec![0]);
    assert_eq!(player.active_data().transmog_update_mask, Some(vec![1]));
    assert!(player.add_transmog_flag_like_cpp(transmog_slot, 1 << 7));
    assert_eq!(player.active_data().transmog, vec![1 << 7]);
    assert!(!player.add_transmog_flag_like_cpp(10, 1));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_TRANSMOG_BIT)
    );

    player.clear_data_changes();
    assert_eq!(player.active_data().conditional_transmog, Vec::<i32>::new());
    assert_eq!(player.active_data().conditional_transmog_update_mask, None);
    assert_eq!(player.add_conditional_transmog_like_cpp(65), 0);
    assert_eq!(player.add_conditional_transmog_like_cpp(96), 1);
    assert_eq!(player.active_data().conditional_transmog, vec![65, 96]);
    assert_eq!(
        player.active_data().conditional_transmog_update_mask,
        Some(vec![0b11])
    );
    assert!(player.remove_conditional_transmog_like_cpp(65));
    assert_eq!(player.active_data().conditional_transmog, vec![96]);
    assert_eq!(
        player.active_data().conditional_transmog_update_mask,
        Some(vec![0b11])
    );
    assert!(!player.remove_conditional_transmog_like_cpp(65));
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT)
    );
}
#[test]
fn player_inventory_storage_matches_cpp_get_item_by_pos_rules() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);
    player.clear_active_player_data_changes();

    let equipped = ObjectGuid::create_item(1, 100);
    let bag_guid = ObjectGuid::create_item(1, 200);
    let bag_item = ObjectGuid::create_item(1, 201);
    let buyback = ObjectGuid::create_item(1, 300);

    player.store_top_level_item(0, equipped).unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid)
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, bag_item)
        .unwrap();
    player
        .store_top_level_item(BUYBACK_SLOT_START, buyback)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0),
        Some(equipped)
    );
    assert_eq!(
        player.get_item_by_packed_pos((u16::from(INVENTORY_SLOT_BAG_0) << 8) | 0),
        Some(equipped)
    );
    assert_eq!(
        player.get_bag_by_pos(INVENTORY_SLOT_BAG_START),
        Some(bag_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(bag_item)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, BUYBACK_SLOT_START),
        None
    );
    assert_eq!(
        player.get_item_from_buyback_slot(BUYBACK_SLOT_START),
        Some(buyback)
    );
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
}
#[test]
fn visible_item_slot_marks_cpp_playerdata_array_bits() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    let visible = VisibleItemValues {
        item_id: 19019,
        item_appearance_mod_id: 7,
        item_visual: 3,
    };
    player.set_visible_item_slot(15, Some(visible));

    assert_eq!(player.data().visible_items[15], visible);
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT + 15)
    );

    player.clear_player_data_changes();
    player.set_visible_item_slot(15, None);
    assert_eq!(
        player.data().visible_items[15],
        VisibleItemValues::default()
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT + 15)
    );
}
#[test]
fn visualize_item_updates_equipment_storage_and_visible_item_like_cpp() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();
    player.clear_active_player_data_changes();

    let guid = ObjectGuid::create_item(1, 500);
    let visible = VisibleItemValues {
        item_id: 500,
        item_appearance_mod_id: 1,
        item_visual: 2,
    };

    player.visualize_item(0, guid, visible).unwrap();

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0), Some(guid));
    assert_eq!(player.active_data().inv_slots[0], guid);
    assert_eq!(player.data().visible_items[0], visible);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT)
    );
    assert!(
        player
            .player_data_changes_mask()
            .is_set(PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT)
    );

    player.remove_top_level_item(0).unwrap();
    assert_eq!(player.data().visible_items[0], VisibleItemValues::default());
    assert_eq!(player.active_data().inv_slots[0], ObjectGuid::EMPTY);
}
#[test]
fn visualize_item_object_mutates_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 500);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 500,
        item_appearance_mod_id: 1,
        item_visual: 2,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.clear_data_changes();
    player.clear_active_player_data_changes();
    item.object_mut().create(item_guid);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 700), 4);
    item.set_bonding(ItemBondingType::OnEquip);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    player.visualize_item_object(0, &mut item, visible).unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, 0),
        Some(item_guid)
    );
    assert_eq!(player.active_data().inv_slots[0], item_guid);
    assert_eq!(player.data().visible_items[0], visible);
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), 0);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}
#[test]
fn equip_item_object_empty_slot_visualizes_and_flags_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 510);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 510,
        item_appearance_mod_id: 4,
        item_visual: 9,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::OnEquip);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    assert_eq!(
        player
            .equip_item_object(
                make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
                &mut item,
                None,
                visible,
            )
            .unwrap(),
        EquipItemObjectOutcome::Equipped
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        Some(item_guid)
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        visible
    );
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), EQUIPMENT_SLOT_MAINHAND);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert!(item.is_soul_bound());
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}
#[test]
fn equip_item_object_merges_existing_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 511);
    let incoming_guid = ObjectGuid::create_item(1, 512);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    existing.object_mut().create(existing_guid);
    existing.set_count(2);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_count(3);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(EQUIPMENT_SLOT_FINGER1, existing_guid)
        .unwrap();

    assert_eq!(
        player
            .equip_item_object(
                make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_FINGER1),
                &mut incoming,
                Some(&mut existing),
                VisibleItemValues::default(),
            )
            .unwrap(),
        EquipItemObjectOutcome::Merged
    );

    assert_eq!(existing.count(), 5);
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(incoming.owner_guid(), player_guid);
    assert!(!incoming.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!incoming.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}
#[test]
fn quick_equip_item_object_visualizes_and_flags_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 513);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let visible = VisibleItemValues {
        item_id: 513,
        item_appearance_mod_id: 8,
        item_visual: 1,
    };

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .quick_equip_item_object(
            make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_OFFHAND),
            &mut item,
            visible,
        )
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_OFFHAND),
        Some(item_guid)
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_OFFHAND as usize],
        visible
    );
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), EQUIPMENT_SLOT_OFFHAND);
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}
#[test]
fn remove_item_object_unlinks_equipment_without_clearing_owner_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 514);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
    player
        .visualize_item(
            EQUIPMENT_SLOT_MAINHAND,
            item_guid,
            VisibleItemValues {
                item_id: 514,
                item_appearance_mod_id: 3,
                item_visual: 2,
            },
        )
        .unwrap();

    assert_eq!(
        player
            .remove_item_object(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        None
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        VisibleItemValues::default()
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert!(!item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
}
#[test]
fn remove_item_object_unlinks_bag_item_like_cpp_bag_removeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let item_guid = ObjectGuid::create_item(1, 515);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    bag.item_mut().object_mut().create(bag_guid);
    bag.item_mut().set_owner_guid(player_guid);
    item.object_mut().create(item_guid);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 10)
        .unwrap();
    bag.store_item(2, &mut item);
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, item_guid)
        .unwrap();

    assert_eq!(
        player
            .remove_item_object(INVENTORY_SLOT_BAG_START, 2, Some(&mut item), Some(&mut bag))
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2), None);
    assert_eq!(bag.data().slots[2], ObjectGuid::EMPTY);
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.slot(), NULL_SLOT);
}
#[test]
fn move_item_from_inventory_object_unlinks_and_clears_refund_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 516);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(INVENTORY_SLOT_ITEM_START);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE);
    item.set_refund_recipient(player_guid);
    item.set_paid_money(10);
    item.set_paid_extended_cost(20);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    assert_eq!(
        player
            .move_item_from_inventory_object(
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        None
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert_eq!(item.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(item.paid_money(), 0);
    assert_eq!(item.paid_extended_cost(), 0);
}
#[test]
fn finalize_move_item_to_inventory_object_marks_original_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 517);
    let other_owner = ObjectGuid::create_player(1, 77);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(other_owner);
    item.force_state(ItemUpdateState::Unchanged);

    assert!(player.finalize_move_item_to_inventory_object(item_guid, &mut item, false));
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.update_state(), ItemUpdateState::New);

    item.force_state(ItemUpdateState::Unchanged);
    assert!(player.finalize_move_item_to_inventory_object(item_guid, &mut item, true));
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}
#[test]
fn finalize_move_item_to_inventory_object_skips_merged_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let original_guid = ObjectGuid::create_item(1, 518);
    let merged_guid = ObjectGuid::create_item(1, 519);
    let mut player = Player::new(None, false);
    let mut merged = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    merged.object_mut().create(merged_guid);
    merged.force_state(ItemUpdateState::Unchanged);

    assert!(!player.finalize_move_item_to_inventory_object(original_guid, &mut merged, false));
    assert_eq!(merged.owner_guid(), ObjectGuid::EMPTY);
    assert_eq!(merged.update_state(), ItemUpdateState::Unchanged);
}
#[test]
fn destroy_item_object_removes_top_level_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 520);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(EQUIPMENT_SLOT_MAINHAND);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .visualize_item(
            EQUIPMENT_SLOT_MAINHAND,
            item_guid,
            VisibleItemValues {
                item_id: 520,
                item_appearance_mod_id: 6,
                item_visual: 7,
            },
        )
        .unwrap();

    assert_eq!(
        player
            .destroy_item_object(
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                Some(&mut item),
                None,
            )
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND),
        None
    );
    assert_eq!(
        player.data().visible_items[EQUIPMENT_SLOT_MAINHAND as usize],
        VisibleItemValues::default()
    );
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!item.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert!(item.has_item_flag2(ItemFieldFlags2::EQUIPPED));
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}
#[test]
fn destroy_item_object_removes_bag_item_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 801);
    let item_guid = ObjectGuid::create_item(1, 521);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    bag.item_mut().object_mut().create(bag_guid);
    bag.item_mut().set_owner_guid(player_guid);
    item.object_mut().create(item_guid);
    item.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 10)
        .unwrap();
    bag.store_item(3, &mut item);
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 3, item_guid)
        .unwrap();

    assert_eq!(
        player
            .destroy_item_object(INVENTORY_SLOT_BAG_START, 3, Some(&mut item), Some(&mut bag))
            .unwrap(),
        Some(item_guid)
    );

    assert_eq!(player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 3), None);
    assert_eq!(bag.data().slots[3], ObjectGuid::EMPTY);
    assert_eq!(item.data().contained_in, ObjectGuid::EMPTY);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.slot(), NULL_SLOT);
    assert!(!item.has_item_flag(ItemFieldFlags::REFUNDABLE));
    assert!(!item.has_item_flag(ItemFieldFlags::BOP_TRADEABLE));
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}
#[test]
fn destroy_item_count_for_item_object_decrements_partial_stack_like_cpp() {
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let mut count = 3;

    item.set_count(8);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .destroy_item_count_for_item_object(Some(&mut item), &mut count, None)
        .unwrap();

    assert_eq!(item.count(), 5);
    assert_eq!(count, 0);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}
#[test]
fn destroy_item_count_for_item_object_destroys_full_stack_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 522);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    let mut count = 7;

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    item.object_mut().create(item_guid);
    item.set_owner_guid(player_guid);
    item.set_contained_in(player_guid);
    item.set_slot(INVENTORY_SLOT_ITEM_START);
    item.set_count(5);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    player
        .destroy_item_count_for_item_object(Some(&mut item), &mut count, None)
        .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        None
    );
    assert_eq!(item.slot(), NULL_SLOT);
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
}
