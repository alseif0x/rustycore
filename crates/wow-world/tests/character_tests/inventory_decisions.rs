use super::*;
use wow_world::test_fixtures::{
    direct_inventory_swap_persistence_for_test as plan_direct_inventory_swap_persistence_like_cpp,
    autostore_bank_target_for_test as autostore_bank_target_like_cpp,
    autostore_bank_quest_checks_for_test as autostore_bank_quest_checks_like_cpp,
    loaded_inventory_slot_count_for_test as loaded_inventory_slot_count_with_legacy_rust_compat,
    player_money_gain_for_test as player_money_gain_like_cpp,
    item_is_not_empty_bag_for_test as item_is_not_empty_bag_like_cpp,
    inventory_move_quest_directions_for_test as inventory_storage_move_quest_directions_like_cpp,
    bind_inventory_destination_for_test as bind_inventory_item_for_destination_like_cpp,
    item_dynamic_flags_changed_for_test as item_dynamic_flags_changed_like_cpp,
    parse_equipment_cache_for_test as parse_equipment_cache,
    item_is_currently_looted_for_test as item_is_currently_looted_like_cpp,
    bank_store_destination_applies_obtain_spells_for_test as bank_store_destination_applies_obtain_spells_like_cpp,
    destroy_item_count_action_for_test as destroy_item_count_action,
};

#[test]
fn direct_inventory_swap_persistence_plan_replaces_both_occupied_positions_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 58),
        entry_id: 103,
        db_guid: 58,
        inventory_type: None,
    };
    let dst = InventoryItem {
        guid: ObjectGuid::create_item(1, 59),
        entry_id: 104,
        db_guid: 59,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), Some(&dst)),
        inventory_position_updates_for_test(&[(36, 58), (35, 59)]),
        "C++ _SaveInventory replaces each changed item's final position in one transaction"
    );
}

#[test]
fn direct_inventory_swap_persistence_plan_moves_into_empty_position_like_cpp() {
    let src = InventoryItem {
        guid: ObjectGuid::create_item(1, 60),
        entry_id: 105,
        db_guid: 60,
        inventory_type: None,
    };

    assert_eq!(
        plan_direct_inventory_swap_persistence_like_cpp(35, 36, Some(&src), None),
        inventory_position_updates_for_test(&[(36, 60)])
    );
}

#[test]
fn autostore_bank_target_depends_on_source_domain_like_cpp() {
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, wow_entities::BANK_SLOT_ITEM_START,),
        InventoryStorageTargetForTest::inventory()
    );
    assert_eq!(
        autostore_bank_target_like_cpp(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START,),
        InventoryStorageTargetForTest::bank()
    );
}

#[test]
fn bank_move_quest_removal_follows_opcode_not_direction_like_cpp() {
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetForTest::bank()),
        InventoryQuestChecksForTest::none(),
        "C++ AutoStore inventory-to-bank must select no quest check even though its target is Bank"
    );
    assert_eq!(
        autostore_bank_quest_checks_like_cpp(InventoryStorageTargetForTest::inventory()),
        InventoryQuestChecksForTest::autostore_added()
    );
}

#[test]
fn legacy_zero_inventory_slots_loads_base_backpack_capacity() {
    assert_eq!(
        loaded_inventory_slot_count_with_legacy_rust_compat(0),
        INVENTORY_DEFAULT_SIZE
    );
    assert_eq!(loaded_inventory_slot_count_with_legacy_rust_compat(24), 24);
}

#[test]
fn player_money_gain_like_cpp_enforces_max_money_amount() {
    assert_eq!(player_money_gain_like_cpp(0, 0), Some(0));
    assert_eq!(
        player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 1, 1),
        Some(MAX_MONEY_AMOUNT)
    );
    assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT, 1), None);
    assert_eq!(player_money_gain_like_cpp(MAX_MONEY_AMOUNT - 10, 11), None);
    assert_eq!(player_money_gain_like_cpp(0, MAX_MONEY_AMOUNT + 1), None);
}

#[test]
fn sell_non_empty_bag_guard_matches_cpp_is_not_empty_bag() {
    assert!(item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Bag),
        false
    ));
    assert!(!item_is_not_empty_bag_like_cpp(
        Some(InventoryType::Chest),
        true
    ));
    assert!(!item_is_not_empty_bag_like_cpp(None, true));
}

#[test]
fn inventory_move_quest_checks_only_cross_bank_boundary_like_cpp() {
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetForTest::inventory(),
        ),
        (false, false),
        "ordinary and child inventory relocations must not re-credit quest items"
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            InventoryStorageTargetForTest::inventory(),
        ),
        (false, true)
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetForTest::bank(),
        ),
        (true, false)
    );
}

#[test]
fn equip_destination_uses_visualize_binding_rule_like_cpp() {
    let mut equipped = wow_entities::Item::default();
    equipped.set_bonding(ItemBondingType::OnEquip);
    let unequipped = equipped.clone();
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped,
        wow_entities::make_item_pos(
            INVENTORY_SLOT_BAG_0,
            wow_entities::EQUIPMENT_SLOT_MAINHAND,
        ),
    );
    assert!(
        equipped.is_soul_bound(),
        "C++ Player::VisualizeItem binds BIND_ON_EQUIP before EquipItem persistence"
    );
    assert!(
        item_dynamic_flags_changed_like_cpp(&unequipped, &equipped),
        "the equip path must publish ITEM_DATA_DYNAMIC_FLAGS after applying binding"
    );

    let mut backpack = wow_entities::Item::default();
    backpack.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut backpack,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
    );
    assert!(
        !backpack.is_soul_bound(),
        "ordinary C++ _StoreItem destinations keep BIND_ON_EQUIP unbound"
    );

    let mut equipped_bag = wow_entities::Item::default();
    equipped_bag.set_bonding(ItemBondingType::OnEquip);
    bind_inventory_item_for_destination_like_cpp(
        &mut equipped_bag,
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
    );
    assert!(equipped_bag.is_soul_bound());
}

#[test]
fn parse_equipment_cache_empty() {
    let eq = parse_equipment_cache("");
    for slot in &eq {
        assert_eq!(slot.display_id, 0);
        assert_eq!(slot.inv_type, 0);
    }
}

#[test]
fn item_currently_looted_guard_uses_runtime_loot_generated_state() {
    let mut item = wow_entities::Item::default();
    assert!(!item_is_currently_looted_like_cpp(&item));

    item.set_loot_generated(true);
    assert!(item_is_currently_looted_like_cpp(&item));
}

#[test]
fn parse_equipment_cache_real_data() {
    // Real data from DB: first slot has inv_type=0, next few slots have gear
    let cache = "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 4 2470 0 0 0 20 33257 0 1 0";
    let eq = parse_equipment_cache(cache);
    // Slot 0: all zeros
    assert_eq!(eq[0].display_id, 0);
    // Slot 3: inv_type=4, display_id=2470
    assert_eq!(eq[3].inv_type, 4);
    assert_eq!(eq[3].display_id, 2470);
    // Slot 4: inv_type=20, display_id=33257, subclass=1
    assert_eq!(eq[4].inv_type, 20);
    assert_eq!(eq[4].display_id, 33257);
    assert_eq!(eq[4].subclass, 1);
}

#[test]
fn top_level_bank_destination_applies_obtain_spells_like_cpp_store_item() {
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        INVENTORY_SLOT_BAG_0
    ));
    assert!(bank_store_destination_applies_obtain_spells_like_cpp(
        wow_entities::INVENTORY_SLOT_BAG_START
    ));
    assert!(
        !bank_store_destination_applies_obtain_spells_like_cpp(wow_entities::BANK_SLOT_BAG_START),
        "C++ _StoreItem excludes bank-bag containers but not bag-0 bank slots"
    );
}


#[test]
fn destroy_item_count_action_matches_cpp_direct_item_branch() {
    assert_eq!(
        destroy_item_count_action(5, 0),
        DestroyItemCountActionForTest::full_stack()
    );
    assert_eq!(
        destroy_item_count_action(5, 5),
        DestroyItemCountActionForTest::full_stack()
    );
    assert_eq!(
        destroy_item_count_action(5, 7),
        DestroyItemCountActionForTest::full_stack()
    );
    assert_eq!(
        destroy_item_count_action(5, 2),
        DestroyItemCountActionForTest::partial_stack(3)
    );
}
