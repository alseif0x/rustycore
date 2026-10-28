//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn cast_all_obtain_scan_freezes_backpack_end_and_resolves_live_bags_and_slots() {
    let player_guid = ObjectGuid::create_player(1, 410);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);

    let backpack = [
        ObjectGuid::create_item(1, 411),
        ObjectGuid::create_item(1, 412),
    ];
    let bag_guid = ObjectGuid::create_item(1, 413);
    let bag_item = ObjectGuid::create_item(1, 414);
    let keyring = ObjectGuid::create_item(1, 415);
    let bank = ObjectGuid::create_item(1, 416);

    for (slot, guid) in [
        (INVENTORY_SLOT_ITEM_START, backpack[0]),
        (INVENTORY_SLOT_ITEM_START + 1, backpack[1]),
        (INVENTORY_SLOT_BAG_START, bag_guid),
        (KEYRING_SLOT_START, keyring),
        (BANK_SLOT_ITEM_START, bank),
    ] {
        player.store_top_level_item(slot, guid).unwrap();
    }
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 2)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 1, bag_item)
        .unwrap();

    for guid in [backpack[0], backpack[1], bag_guid, bag_item, keyring, bank] {
        let mut item = Item::default();
        item.object_mut().create(guid);
        item.set_owner_guid(player_guid);
        player
            .inventory_runtime_mut_like_cpp()
            .store_item_object_like_cpp(item);
    }

    let backpack_end = player
        .cast_all_obtain_backpack_end_like_cpp()
        .expect("complete backpack topology");
    assert_eq!(backpack_end, INVENTORY_SLOT_ITEM_START + 2);
    assert_eq!(
        (INVENTORY_SLOT_ITEM_START..backpack_end)
            .map(|slot| player
                .cast_all_obtain_item_at_like_cpp(INVENTORY_SLOT_BAG_0, slot, None)
                .expect("live backpack slot authority"))
            .flatten()
            .collect::<Vec<_>>(),
        backpack
    );
    let (resolved_bag_guid, bag_size) = player
        .cast_all_obtain_bag_like_cpp(INVENTORY_SLOT_BAG_START)
        .expect("live bag authority")
        .expect("equipped inventory bag");
    assert_eq!(resolved_bag_guid, bag_guid);
    assert_eq!(bag_size, 2);
    assert_eq!(
        (0..bag_size)
            .map(|slot| player
                .cast_all_obtain_item_at_like_cpp(
                    INVENTORY_SLOT_BAG_START,
                    slot,
                    Some(resolved_bag_guid),
                )
                .expect("live contained slot authority"))
            .flatten()
            .collect::<Vec<_>>(),
        [bag_item]
    );
    assert_eq!(
        player.cast_all_obtain_item_at_like_cpp(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, None),
        None,
        "the obtain scan cannot read keyring positions"
    );
    assert_eq!(
        player.cast_all_obtain_item_at_like_cpp(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, None),
        None,
        "the obtain scan cannot read bank positions"
    );

    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(false);
    assert_eq!(player.cast_all_obtain_backpack_end_like_cpp(), None);
    assert_eq!(
        player.cast_all_obtain_bag_like_cpp(INVENTORY_SLOT_BAG_START),
        None
    );
    assert_eq!(
        player.cast_all_obtain_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            None
        ),
        None
    );
}

#[test]
fn destroy_item_count_by_entry_plan_matches_cpp_scan_order_and_partial_stop() {
    let player = Player::new(None, false);
    let mut inventory = Item::default();
    let mut bag_item = Item::default();
    let mut bank = Item::default();

    inventory.object_mut().set_entry(900);
    inventory.set_count(2);
    bag_item.object_mut().set_entry(900);
    bag_item.set_count(3);
    bank.object_mut().set_entry(900);
    bank.set_count(5);

    let items = [
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank),
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_START, 4, &bag_item),
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, &inventory),
    ];

    let plan = player.destroy_item_count_by_entry_plan(900, 4, false, 16, &items);

    assert_eq!(plan.removed_count, 4);
    assert_eq!(
        plan.actions,
        vec![
            DestroyItemCountAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
                removed_count: 2,
                remaining_count: 0,
                destroy_stack: true,
            },
            DestroyItemCountAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 4,
                removed_count: 2,
                remaining_count: 1,
                destroy_stack: false,
            },
        ]
    );
}
#[test]
fn destroy_item_count_by_entry_plan_matches_cpp_unequip_check_for_full_equipment_stack() {
    let player = Player::new(None, false);
    let mut equipped = Item::default();
    let mut bank = Item::default();

    equipped.object_mut().set_entry(901);
    equipped.set_count(1);
    bank.object_mut().set_entry(901);
    bank.set_count(1);

    let mut blocked_equipped =
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND, &equipped);
    blocked_equipped.can_unequip_result = InventoryResult::CantEquipEver;
    let items = [
        blocked_equipped,
        DestroyItemCountItemRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank),
    ];

    let plan = player.destroy_item_count_by_entry_plan(901, 1, true, 16, &items);

    assert_eq!(plan.removed_count, 1);
    assert_eq!(
        plan.actions,
        vec![DestroyItemCountAction {
            bag: INVENTORY_SLOT_BAG_0,
            slot: BANK_SLOT_ITEM_START,
            removed_count: 1,
            remaining_count: 0,
            destroy_stack: true,
        }]
    );
}
#[test]
fn destroy_zone_limited_item_plan_matches_cpp_scan_order() {
    let player = Player::new(None, false);
    let items = [
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_START, 2, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, false),
    ];

    assert_eq!(
        player.destroy_zone_limited_item_plan(16, &items),
        vec![
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: KEYRING_SLOT_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 2,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: EQUIPMENT_SLOT_CHEST,
            },
        ]
    );
}

#[test]
fn normal_spell_costs_debit_canonical_inventory_and_currency_owners() {
    let player_guid = ObjectGuid::create_player(1, 420);
    let item_guid = ObjectGuid::create_item(1, 421);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(700);
    item.set_owner_guid(player_guid);
    item.set_count(2);
    item.set_state(ItemUpdateState::Unchanged);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    player.gameplay_state_mut().currencies_loaded_like_cpp = true;
    player.gameplay_state_mut().currencies.insert(
        395,
        crate::PlayerCurrency {
            state: crate::PlayerCurrencyState::Unchanged,
            quantity: 9,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );

    let plan = player
        .plan_spell_cast_resource_debits_like_cpp(&[(700, 1)], &[(395, 5)])
        .expect("complete native owners plan both costs");
    assert_eq!(plan.item_actions().len(), 1);
    assert_eq!(plan.item_actions()[0].item_guid(), item_guid);
    assert_eq!(plan.item_actions()[0].remaining_count(), 1);
    assert_eq!(plan.currency_actions()[0].remaining_quantity(), 4);

    let outcome = player
        .apply_spell_cast_resource_debits_like_cpp(&plan)
        .expect("the unchanged snapshot still matches at synchronous apply");
    assert_eq!(outcome.item_actions.as_slice(), plan.item_actions());
    assert_eq!(outcome.currency_actions.as_slice(), plan.currency_actions());
    let item = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
    assert_eq!(item.count(), 1);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert_eq!(player.gameplay_state().currencies[&395].quantity, 4);
    assert_eq!(
        player.gameplay_state().currencies[&395].state,
        crate::PlayerCurrencyState::Changed
    );
}

#[test]
fn normal_currency_only_cost_does_not_require_inventory_topology() {
    let player_guid = ObjectGuid::create_player(1, 424);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.gameplay_state_mut().currencies_loaded_like_cpp = true;
    player.gameplay_state_mut().currencies.insert(
        395,
        crate::PlayerCurrency {
            state: crate::PlayerCurrencyState::Unchanged,
            quantity: 9,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        },
    );

    let plan = player
        .plan_spell_cast_resource_debits_like_cpp(&[], &[(395, 5)])
        .expect("currency-only costs do not depend on unrelated inventory authority");
    assert!(plan.item_actions().is_empty());
    assert_eq!(plan.currency_actions()[0].remaining_quantity(), 4);
}

#[test]
fn normal_spell_reagent_step_rejects_stale_state_without_partial_mutation() {
    let player_guid = ObjectGuid::create_player(1, 425);
    let item_guid = ObjectGuid::create_item(1, 426);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();
    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(702);
    item.set_owner_guid(player_guid);
    item.set_count(2);
    item.force_state(ItemUpdateState::Unchanged);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    let action = player
        .plan_spell_cast_reagent_step_like_cpp(702, 2)
        .expect("one physical step is available");
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&item_guid)
        .unwrap()
        .set_count(1);
    assert!(
        player
            .apply_spell_cast_reagent_step_like_cpp(action)
            .is_none()
    );
    assert_eq!(
        player.inventory_runtime_like_cpp().item_objects()[&item_guid].count(),
        1,
        "stale action is rejected before any further debit"
    );
}

#[test]
fn normal_spell_cast_item_use_updates_charges_and_consumes_one_physical_item() {
    let player_guid = ObjectGuid::create_player(1, 422);
    let item_guid = ObjectGuid::create_item(1, 423);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(701);
    item.set_owner_guid(player_guid);
    item.set_count(2);
    item.set_spell_charges(0, 1);
    item.set_state(ItemUpdateState::Unchanged);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    let plan = PlayerSpellCastItemUsePlanLikeCpp {
        item_guid,
        entry_id: 701,
        expected_charges: [1, 0, 0, 0, 0],
        updated_charges: [0, 0, 0, 0, 0],
        writes_charges: true,
        marks_item_changed: true,
        consumes_one_item: true,
    };
    let outcome = player
        .apply_spell_cast_item_use_like_cpp(plan)
        .expect("canonical physical item still matches its charge snapshot");
    assert!(outcome.removed_one);
    assert!(!outcome.removed_stack);
    assert_eq!(outcome.remaining_count, 1);
    let item = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
    assert_eq!(item.count(), 1);
    assert_eq!(item.data().spell_charges[0], 0);
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
}

#[test]
fn normal_resource_plan_tombstones_a_consumed_new_item() {
    let player_guid = ObjectGuid::create_player(1, 427);
    let item_guid = ObjectGuid::create_item(1, 428);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(703);
    item.set_owner_guid(player_guid);
    item.set_count(1);
    item.force_state(ItemUpdateState::New);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    let plan = player
        .plan_spell_cast_resource_debits_like_cpp(&[(703, 1)], &[])
        .expect("new physical stack is available");
    player
        .apply_spell_cast_resource_debits_like_cpp(&plan)
        .expect("the complete resource plan still matches");
    let removed = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
    assert_eq!(removed.update_state(), ItemUpdateState::Removed);
    assert_eq!(removed.slot(), NULL_SLOT);
    assert!(
        player
            .get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}

#[test]
fn normal_cast_item_use_tombstones_a_consumed_new_item() {
    let player_guid = ObjectGuid::create_player(1, 429);
    let item_guid = ObjectGuid::create_item(1, 430);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.set_inventory_slot_count(2);
    player
        .inventory_runtime_mut_like_cpp()
        .set_equipment_inventory_authority_complete_like_cpp(true);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, item_guid)
        .unwrap();

    let mut item = Item::default();
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(704);
    item.set_owner_guid(player_guid);
    item.set_count(1);
    item.force_state(ItemUpdateState::New);
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_object_like_cpp(item);

    let outcome = player
        .apply_spell_cast_item_use_like_cpp(PlayerSpellCastItemUsePlanLikeCpp {
            item_guid,
            entry_id: 704,
            expected_charges: [0; crate::MAX_ITEM_SPELLS],
            updated_charges: [0; crate::MAX_ITEM_SPELLS],
            writes_charges: false,
            marks_item_changed: false,
            consumes_one_item: true,
        })
        .expect("physical cast item remains at its canonical position");
    assert!(outcome.removed_stack);
    let removed = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
    assert_eq!(removed.update_state(), ItemUpdateState::Removed);
    assert_eq!(removed.slot(), NULL_SLOT);
    assert!(
        player
            .get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START)
            .is_none()
    );
}

#[test]
fn destroy_conjured_items_plan_matches_cpp_scan_order_without_keyring() {
    let player = Player::new(None, false);
    let items = [
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, KEYRING_SLOT_START, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_START, 1, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, true),
        DestroyFilteredItemRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true),
    ];

    assert_eq!(
        player.destroy_conjured_items_plan(16, &items),
        vec![
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_START,
                slot: 1,
            },
            DestroyFilteredItemAction {
                bag: INVENTORY_SLOT_BAG_0,
                slot: EQUIPMENT_SLOT_CHEST,
            },
        ]
    );
}
#[test]
fn store_item_object_mutates_empty_top_level_slot_like_cpp_storeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 600);
    let mut player = Player::new(None, false);
    let mut item = Item::default();

    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.clear_active_player_data_changes();
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::OnAcquire);
    item.force_state(ItemUpdateState::Unchanged);
    item.clear_item_data_changes();

    player
        .store_item_object(INVENTORY_SLOT_ITEM_START, &mut item, 4)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(item_guid)
    );
    assert_eq!(
        player.active_data().inv_slots[INVENTORY_SLOT_ITEM_START as usize],
        item_guid
    );
    assert_eq!(item.count(), 4);
    assert_eq!(item.data().contained_in, player_guid);
    assert_eq!(item.owner_guid(), player_guid);
    assert_eq!(item.slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(item.container_guid(), ObjectGuid::EMPTY);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_0);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert!(
        player
            .active_player_data_changes_mask()
            .is_set(ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT + INVENTORY_SLOT_ITEM_START as usize)
    );
}
#[test]
fn store_item_object_binds_on_equip_only_for_bag_positions_like_cpp_storeitem() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);

    let mut inventory_item = Item::default();
    inventory_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 601));
    inventory_item.set_bonding(ItemBondingType::OnEquip);
    player
        .store_item_object(INVENTORY_SLOT_ITEM_START, &mut inventory_item, 1)
        .unwrap();
    assert!(!inventory_item.is_soul_bound());

    let mut bag_item = Item::default();
    bag_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 602));
    bag_item.set_bonding(ItemBondingType::OnEquip);
    player
        .store_item_object(INVENTORY_SLOT_BAG_START, &mut bag_item, 1)
        .unwrap();
    assert!(bag_item.is_soul_bound());
}
#[test]
fn store_item_object_rejects_occupied_slot_until_stack_merge_registry_exists() {
    let existing = ObjectGuid::create_item(1, 700);
    let incoming = ObjectGuid::create_item(1, 701);
    let mut player = Player::new(None, false);
    let mut item = Item::default();
    item.object_mut().create(incoming);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, existing)
        .unwrap();
    let result = player.store_item_object(INVENTORY_SLOT_ITEM_START, &mut item, 3);

    assert_eq!(
        result,
        Err(PlayerStorageError::OccupiedPlayerSlot(
            INVENTORY_SLOT_ITEM_START
        ))
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(existing)
    );
    assert_eq!(item.count(), 0);
    assert_eq!(item.update_state(), ItemUpdateState::Unchanged);
}
#[test]
fn store_cloned_item_object_keeps_source_and_stores_clone_like_cpp_storeitem_clone() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 760);
    let clone_guid = ObjectGuid::create_item(1, 761);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_bonding(ItemBondingType::OnAcquire);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);

    let cloned = player
        .store_cloned_item_object(INVENTORY_SLOT_ITEM_START, &source, clone_guid, 3)
        .unwrap();

    assert_eq!(source.object().guid(), source_guid);
    assert_eq!(source.count(), 8);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(clone_guid)
    );
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.object().entry(), 6948);
    assert_eq!(cloned.count(), 3);
    assert_eq!(cloned.owner_guid(), owner);
    assert!(cloned.is_soul_bound());
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.slot(), INVENTORY_SLOT_ITEM_START);
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}
#[test]
fn split_item_to_empty_top_level_object_matches_cpp_split_allocation() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 762);
    let clone_guid = ObjectGuid::create_item(1, 763);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, source_guid)
        .unwrap();

    let cloned = player
        .split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START + 1,
            &mut source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.count(), 5);
    assert_eq!(source.update_state(), ItemUpdateState::Changed);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
        Some(source_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        Some(clone_guid)
    );
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.count(), 3);
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}
#[test]
fn split_item_to_empty_top_level_object_rolls_back_source_like_cpp_on_failure() {
    let owner = ObjectGuid::create_player(1, 42);
    let source_guid = ObjectGuid::create_item(1, 764);
    let occupied_guid = ObjectGuid::create_item(1, 765);
    let clone_guid = ObjectGuid::create_item(1, 766);
    let mut player = Player::new(None, false);
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.force_state(ItemUpdateState::Unchanged);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, source_guid)
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, occupied_guid)
        .unwrap();

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START + 1,
            &mut source,
            clone_guid,
            3,
        ),
        Err(PlayerStorageError::OccupiedPlayerSlot(
            INVENTORY_SLOT_ITEM_START + 1
        ))
    );

    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        Some(occupied_guid)
    );
}
#[test]
fn store_bag_item_object_mutates_bag_branch_like_cpp_storeitem() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 800);
    let item_guid = ObjectGuid::create_item(1, 801);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    bag.item_mut().force_state(ItemUpdateState::Unchanged);
    bag.clear_container_data_changes();
    item.object_mut().create(item_guid);
    item.set_bonding(ItemBondingType::Quest);
    item.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item_object(INVENTORY_SLOT_BAG_START, &mut bag, 2, &mut item, 3)
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(item_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(item_guid));
    assert_eq!(item.count(), 3);
    assert_eq!(item.data().contained_in, bag_guid);
    assert_eq!(item.owner_guid(), owner);
    assert_eq!(item.container_guid(), bag_guid);
    assert_eq!(item.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(item.slot(), 2);
    assert!(item.is_soul_bound());
    assert_eq!(item.update_state(), ItemUpdateState::Changed);
    assert_eq!(bag.item().update_state(), ItemUpdateState::Changed);
    assert!(
        bag.container_data_changes_mask()
            .is_set(crate::CONTAINER_DATA_SLOTS_FIRST_BIT + 2)
    );
}
#[test]
fn store_bag_item_object_rejects_mismatched_or_occupied_bag_slot() {
    let owner = ObjectGuid::create_player(1, 42);
    let registered_bag = ObjectGuid::create_item(1, 810);
    let actual_bag = ObjectGuid::create_item(1, 811);
    let existing = ObjectGuid::create_item(1, 812);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut item = Item::default();

    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: actual_bag,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    item.object_mut().create(ObjectGuid::create_item(1, 813));
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, registered_bag, 4)
        .unwrap();

    assert_eq!(
        player.store_bag_item_object(INVENTORY_SLOT_BAG_START, &mut bag, 2, &mut item, 1),
        Err(PlayerStorageError::MismatchedBagGuid {
            bag: INVENTORY_SLOT_BAG_START,
            expected: registered_bag,
            actual: actual_bag,
        })
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START + 1, actual_bag, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START + 1, 2, existing)
        .unwrap();
    assert_eq!(
        player.store_bag_item_object(INVENTORY_SLOT_BAG_START + 1, &mut bag, 2, &mut item, 1),
        Err(PlayerStorageError::OccupiedBagItemSlot {
            bag: INVENTORY_SLOT_BAG_START + 1,
            slot: 2,
        })
    );
    assert_eq!(item.count(), 0);
    assert_eq!(bag.item_by_pos(2), None);
}
#[test]
fn store_cloned_bag_item_object_keeps_source_and_stores_clone_like_cpp_storeitem_clone() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 860);
    let source_guid = ObjectGuid::create_item(1, 861);
    let clone_guid = ObjectGuid::create_item(1, 862);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_bonding(ItemBondingType::OnEquip);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    source.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    let cloned = player
        .store_cloned_bag_item_object(
            INVENTORY_SLOT_BAG_START,
            &mut bag,
            2,
            &source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.object().guid(), source_guid);
    assert_eq!(source.count(), 8);
    assert!(source.is_refundable());
    assert!(source.is_bop_tradeable());
    assert_eq!(source.update_state(), ItemUpdateState::Unchanged);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(clone_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(clone_guid));
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.object().entry(), 6948);
    assert_eq!(cloned.count(), 3);
    assert_eq!(cloned.owner_guid(), owner);
    assert!(!cloned.is_soul_bound());
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.container_guid(), bag_guid);
    assert_eq!(cloned.bag_slot(), INVENTORY_SLOT_BAG_START);
    assert_eq!(cloned.slot(), 2);
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}
#[test]
fn split_item_to_empty_bag_item_object_matches_cpp_split_allocation() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 870);
    let source_guid = ObjectGuid::create_item(1, 871);
    let clone_guid = ObjectGuid::create_item(1, 872);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut source = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    source.object_mut().create(source_guid);
    source.object_mut().set_entry(6948);
    source.set_count(8);
    source.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    bag.store_item(1, &mut source);
    source.force_state(ItemUpdateState::Unchanged);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 1, source_guid)
        .unwrap();
    let cloned = player
        .split_item_to_empty_bag_item_object(
            INVENTORY_SLOT_BAG_START,
            &mut bag,
            2,
            &mut source,
            clone_guid,
            3,
        )
        .unwrap();

    assert_eq!(source.count(), 5);
    assert_eq!(source.update_state(), ItemUpdateState::Changed);
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 1),
        Some(source_guid)
    );
    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(clone_guid)
    );
    assert_eq!(bag.item_by_pos(1), Some(source_guid));
    assert_eq!(bag.item_by_pos(2), Some(clone_guid));
    assert_eq!(cloned.object().guid(), clone_guid);
    assert_eq!(cloned.count(), 3);
    assert!(!cloned.is_refundable());
    assert!(!cloned.is_bop_tradeable());
    assert_eq!(cloned.update_state(), ItemUpdateState::New);
}
#[test]
fn split_item_rejects_zero_all_or_too_many_like_cpp_guards() {
    let mut player = Player::new(None, false);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 880));
    source.set_count(8);

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 881),
            0,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 0,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 882),
            8,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 8,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 883),
            9,
        ),
        Err(PlayerStorageError::TooFewItemsToSplit {
            available: 8,
            requested: 9,
        })
    );
    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::New);
}
#[test]
fn split_item_rejects_loot_and_trade_states_in_cpp_order() {
    let mut player = Player::new(None, false);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 884));
    source.set_count(8);
    source.set_loot_generated(true);
    source.set_in_trade(true);

    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 885),
            8,
        ),
        Err(PlayerStorageError::SplitItemLootGenerated)
    );

    source.set_loot_generated(false);
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 886),
            8,
        ),
        Err(PlayerStorageError::InvalidSplitCount {
            available: 8,
            requested: 8,
        })
    );
    assert_eq!(
        player.split_item_to_empty_top_level_object(
            INVENTORY_SLOT_ITEM_START,
            &mut source,
            ObjectGuid::create_item(1, 887),
            3,
        ),
        Err(PlayerStorageError::SplitItemInTrade)
    );
    assert_eq!(source.count(), 8);
    assert_eq!(source.update_state(), ItemUpdateState::New);
}
#[test]
fn merge_top_level_item_stack_object_matches_cpp_existing_stack_branch() {
    let owner = ObjectGuid::create_player(1, 42);
    let existing_guid = ObjectGuid::create_item(1, 820);
    let incoming_guid = ObjectGuid::create_item(1, 821);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    existing.object_mut().create(existing_guid);
    existing.set_bonding(ItemBondingType::OnEquip);
    existing.set_count(5);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.set_refund_recipient(ObjectGuid::create_player(1, 99));
    incoming.set_paid_money(10);
    incoming.set_paid_extended_cost(20);
    incoming.force_state(ItemUpdateState::Unchanged);

    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, existing_guid)
        .unwrap();
    player
        .merge_top_level_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &mut existing,
            &mut incoming,
            3,
        )
        .unwrap();

    assert_eq!(existing.count(), 8);
    assert!(existing.is_soul_bound());
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(incoming.owner_guid(), owner);
    assert!(!incoming.is_refundable());
    assert!(!incoming.is_bop_tradeable());
    assert_eq!(incoming.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(incoming.paid_money(), 0);
    assert_eq!(incoming.paid_extended_cost(), 0);
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}
#[test]
fn merge_top_level_item_stack_object_rejects_empty_or_mismatched_slot() {
    let expected = ObjectGuid::create_item(1, 830);
    let actual = ObjectGuid::create_item(1, 831);
    let mut player = Player::new(None, false);
    let mut existing = Item::default();
    let mut incoming = Item::default();
    existing.object_mut().create(actual);

    assert_eq!(
        player.merge_top_level_item_stack_object(
            INVENTORY_SLOT_ITEM_START,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::EmptyPlayerSlot(
            INVENTORY_SLOT_ITEM_START
        ))
    );

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, expected)
        .unwrap();
    assert_eq!(
        player.merge_top_level_item_stack_object(
            INVENTORY_SLOT_ITEM_START,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedItemGuid {
            slot: INVENTORY_SLOT_ITEM_START,
            expected,
            actual,
        })
    );
}
#[test]
fn merge_bag_item_stack_object_matches_cpp_existing_stack_branch() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 840);
    let existing_guid = ObjectGuid::create_item(1, 841);
    let incoming_guid = ObjectGuid::create_item(1, 842);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut existing = Item::default();
    let mut incoming = Item::default();

    player.unit_mut().world_mut().object_mut().create(owner);
    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    bag.item_mut().set_slot(INVENTORY_SLOT_BAG_START);
    bag.item_mut().force_state(ItemUpdateState::Unchanged);
    existing.object_mut().create(existing_guid);
    existing.set_bonding(ItemBondingType::OnEquip);
    existing.set_count(5);
    existing.force_state(ItemUpdateState::Unchanged);
    incoming.object_mut().create(incoming_guid);
    incoming.set_item_flag(ItemFieldFlags::REFUNDABLE | ItemFieldFlags::BOP_TRADEABLE);
    incoming.set_refund_recipient(ObjectGuid::create_player(1, 99));
    incoming.set_paid_money(10);
    incoming.set_paid_extended_cost(20);
    incoming.force_state(ItemUpdateState::Unchanged);
    bag.store_item(2, &mut existing);

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, existing_guid)
        .unwrap();
    player
        .merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            3,
        )
        .unwrap();

    assert_eq!(
        player.get_item_by_pos(INVENTORY_SLOT_BAG_START, 2),
        Some(existing_guid)
    );
    assert_eq!(bag.item_by_pos(2), Some(existing_guid));
    assert_eq!(existing.count(), 8);
    assert!(!existing.is_soul_bound());
    assert_eq!(existing.update_state(), ItemUpdateState::Changed);
    assert_eq!(bag.item().update_state(), ItemUpdateState::Unchanged);
    assert_eq!(incoming.owner_guid(), owner);
    assert!(!incoming.is_refundable());
    assert!(!incoming.is_bop_tradeable());
    assert_eq!(incoming.refund_recipient(), ObjectGuid::EMPTY);
    assert_eq!(incoming.paid_money(), 0);
    assert_eq!(incoming.paid_extended_cost(), 0);
    assert_eq!(incoming.update_state(), ItemUpdateState::Removed);
}
