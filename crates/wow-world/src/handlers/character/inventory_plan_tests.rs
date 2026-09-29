use super::*;

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
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, false),
        "ordinary and child inventory relocations must not re-credit quest items"
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::BANK_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Inventory,
        ),
        (false, true)
    );
    assert_eq!(
        inventory_storage_move_quest_directions_like_cpp(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            InventoryStorageTargetLikeCpp::Bank,
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
