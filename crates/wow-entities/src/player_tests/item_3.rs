//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn can_store_item_in_bag_scans_slots_like_cpp_merge_and_empty_modes() {
    let mut player = Player::new(None, false);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 320), 3)
        .unwrap();
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 3,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut matching = Item::default();
    matching
        .object_mut()
        .create(ObjectGuid::create_item(1, 321));
    matching.object_mut().set_entry(6948);
    matching.set_count(16);
    let mut wrong_entry = Item::default();
    wrong_entry
        .object_mut()
        .create(ObjectGuid::create_item(1, 322));
    wrong_entry.object_mut().set_entry(6949);
    wrong_entry.set_count(1);
    let slot_items = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_START, 0, &matching),
        ItemSlotRef::new(INVENTORY_SLOT_BAG_START, 1, &wrong_entry),
    ];
    let mut merge_dest = Vec::new();
    let mut merge_count = 6;

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut merge_dest,
            &proto,
            &mut merge_count,
            true,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        merge_dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            4,
        )]
    );
    assert_eq!(merge_count, 2);

    let mut empty_dest = Vec::new();
    let mut empty_count = 7;
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut empty_dest,
            &proto,
            &mut empty_count,
            false,
            true,
            None,
            false,
            NULL_BAG,
            2,
            Some(&regular_bag_proto),
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert!(empty_dest.is_empty());
    assert_eq!(empty_count, 7);
}
#[test]
fn can_take_more_similar_items_matches_cpp_max_count_guards() {
    let player = Player::new(None, false);
    let unlimited = ItemStorageTemplate::regular_item(6948, 20);

    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: None,
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(3),
            offending_item_id: None,
        }
    );
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&unlimited),
            count: 3,
            source_item: None,
            current_item_count: 999,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        can_take_more_similar_ok()
    );

    let mut source = Item::default();
    source.set_loot_generated(true);
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&unlimited),
            count: 3,
            source_item: Some(&source),
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::LootGone,
            no_space_count: None,
            offending_item_id: None,
        }
    );

    let limited = ItemStorageTemplate {
        max_count: 10,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited),
            count: 4,
            source_item: None,
            current_item_count: 8,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(2),
            offending_item_id: None,
        }
    );

    let max_int = ItemStorageTemplate {
        max_count: i32::MAX,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&max_int),
            count: 4,
            source_item: None,
            current_item_count: u32::MAX - 4,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        can_take_more_similar_ok()
    );
}
#[test]
fn can_take_more_similar_items_matches_cpp_limit_category_guards() {
    let player = Player::new(None, false);
    let limited_category = ItemStorageTemplate {
        item_limit_category: 77,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };

    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: None,
            current_limit_category_count: 0,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::NotEquippable,
            no_space_count: Some(3),
            offending_item_id: None,
        }
    );

    let have_limit = ItemLimitCategoryTemplate {
        id: 77,
        quantity: 5,
        flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 3,
            source_item: None,
            current_item_count: 0,
            limit_category: Some(&have_limit),
            current_limit_category_count: 4,
        }),
        CanTakeMoreSimilarItemsOutcome {
            result: InventoryResult::ItemMaxLimitCategoryCountExceededIs,
            no_space_count: Some(2),
            offending_item_id: Some(6948),
        }
    );

    let equip_limit = ItemLimitCategoryTemplate {
        id: 77,
        quantity: 1,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    assert_eq!(
        player.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: Some(&limited_category),
            count: 99,
            source_item: None,
            current_item_count: 0,
            limit_category: Some(&equip_limit),
            current_limit_category_count: 99,
        }),
        can_take_more_similar_ok()
    );
}
#[test]
fn item_count_by_entry_matches_cpp_locations_and_skip_item() {
    let player = Player::new(None, false);
    let mut inventory_item = Item::default();
    inventory_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 610));
    inventory_item.object_mut().set_entry(6948);
    inventory_item.set_count(2);
    let mut bank_item = Item::default();
    bank_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 611));
    bank_item.object_mut().set_entry(6948);
    bank_item.set_count(3);
    let mut other_item = Item::default();
    other_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 612));
    other_item.object_mut().set_entry(6949);
    other_item.set_count(7);
    let stored = [
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            None,
        ),
    ];

    assert_eq!(player.item_count_by_entry(6948, false, None, &stored), 2);
    assert_eq!(player.item_count_by_entry(6948, true, None, &stored), 5);
    assert_eq!(
        player.item_count_by_entry(6948, true, Some(&inventory_item), &stored),
        3
    );
}
#[test]
fn item_count_with_limit_category_matches_cpp_everywhere_and_skip_item() {
    let player = Player::new(None, false);
    let limited_template = ItemStorageTemplate {
        item_limit_category: 77,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let other_template = ItemStorageTemplate {
        item_limit_category: 78,
        ..ItemStorageTemplate::regular_item(6949, 20)
    };
    let mut limited_item = Item::default();
    limited_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 620));
    limited_item.object_mut().set_entry(6948);
    limited_item.set_count(2);
    let mut bank_limited_item = Item::default();
    bank_limited_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 621));
    bank_limited_item.object_mut().set_entry(6948);
    bank_limited_item.set_count(3);
    let mut other_item = Item::default();
    other_item
        .object_mut()
        .create(ObjectGuid::create_item(1, 622));
    other_item.object_mut().set_entry(6949);
    other_item.set_count(7);
    let stored = [
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &limited_item,
            Some(&limited_template),
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            BANK_SLOT_ITEM_START,
            &bank_limited_item,
            Some(&limited_template),
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            Some(&other_template),
        ),
    ];

    assert_eq!(player.item_count_with_limit_category(77, None, &stored), 5);
    assert_eq!(
        player.item_count_with_limit_category(77, Some(&limited_item), &stored),
        3
    );
}
#[test]
fn item_by_entry_matches_cpp_for_each_item_order_and_stop() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let equipped = item_with_guid_entry(640, 900);
    let inventory_bag = item_with_guid_entry(641, 900);
    let inventory_item = item_with_guid_entry(642, 900);
    let bag_item = item_with_guid_entry(643, 900);
    let bank_item = item_with_guid_entry(644, 900);

    player
        .store_top_level_item(EQUIPMENT_SLOT_CHEST, equipped.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, inventory_bag.object().guid())
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, inventory_bag.object().guid(), 4)
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item.object().guid())
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 0, bag_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item.object().guid())
        .unwrap();

    let stored = [
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_START, 0, &bag_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_BAG_START,
            &inventory_bag,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST, &equipped, None),
    ];

    let default_found = player
        .item_by_entry(900, ItemSearchLocation::DEFAULT, &stored)
        .unwrap();
    assert_eq!(default_found.item.object().guid(), equipped.object().guid());

    let inventory_found = player
        .item_by_entry(900, ItemSearchLocation::INVENTORY, &stored)
        .unwrap();
    assert_eq!(
        inventory_found.item.object().guid(),
        inventory_bag.object().guid()
    );

    assert!(
        player
            .item_by_entry(901, ItemSearchLocation::EVERYWHERE, &stored)
            .is_none()
    );
}
#[test]
fn item_list_by_entry_matches_cpp_locations_bank_and_reagent_order() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let equipped = item_with_guid_entry(650, 901);
    let inventory_item = item_with_guid_entry(651, 901);
    let bank_item = item_with_guid_entry(652, 901);
    let reagent_bag = item_with_guid_entry(653, 1);
    let reagent_item = item_with_guid_entry(654, 901);
    let other_item = item_with_guid_entry(655, 902);

    player
        .store_top_level_item(EQUIPMENT_SLOT_HEAD, equipped.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(REAGENT_BAG_SLOT_START, reagent_bag.object().guid())
        .unwrap();
    player
        .register_bag_storage(REAGENT_BAG_SLOT_START, reagent_bag.object().guid(), 3)
        .unwrap();
    player
        .store_bag_item(REAGENT_BAG_SLOT_START, 1, reagent_item.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, other_item.object().guid())
        .unwrap();

    let stored = [
        ItemStorageRef::new(REAGENT_BAG_SLOT_START, 1, &reagent_item, None),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START, &bank_item, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &inventory_item,
            None,
        ),
        ItemStorageRef::new(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD, &equipped, None),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &other_item,
            None,
        ),
    ];

    let without_bank = player.item_list_by_entry(901, false, &stored);
    assert_eq!(
        without_bank
            .iter()
            .map(|stored| stored.item.object().guid())
            .collect::<Vec<_>>(),
        vec![
            equipped.object().guid(),
            inventory_item.object().guid(),
            reagent_item.object().guid(),
        ]
    );

    let with_bank = player.item_list_by_entry(901, true, &stored);
    assert_eq!(
        with_bank
            .iter()
            .map(|stored| stored.item.object().guid())
            .collect::<Vec<_>>(),
        vec![
            equipped.object().guid(),
            inventory_item.object().guid(),
            bank_item.object().guid(),
            reagent_item.object().guid(),
        ]
    );
}
#[test]
fn can_store_item_preflight_matches_cpp_template_source_and_similar_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);

    assert_eq!(
        player.can_store_item(
            &mut Vec::new(),
            can_store_args(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, None, 3),
        ),
        CanStoreItemOutcome {
            result: InventoryResult::ItemNotFound,
            no_space_count: Some(3),
        }
    );

    let mut swap_missing = can_store_args(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, None, 3);
    swap_missing.swap = true;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), swap_missing),
        CanStoreItemOutcome {
            result: InventoryResult::CantSwap,
            no_space_count: Some(3),
        }
    );

    let mut source = Item::default();
    source.set_loot_generated(true);
    let mut loot_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        3,
    );
    loot_args.source_item = Some(&source);
    assert_eq!(
        player.can_store_item(&mut Vec::new(), loot_args),
        CanStoreItemOutcome {
            result: InventoryResult::LootGone,
            no_space_count: Some(3),
        }
    );

    source.set_loot_generated(false);
    source.set_owner_guid(ObjectGuid::create_player(1, 42));
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    let mut bound_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        3,
    );
    bound_args.source_item = Some(&source);
    assert_eq!(
        player.can_store_item(&mut Vec::new(), bound_args),
        CanStoreItemOutcome {
            result: InventoryResult::NotOwner,
            no_space_count: Some(3),
        }
    );

    let limited_proto = ItemStorageTemplate {
        max_count: 3,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let mut similar_args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&limited_proto),
        3,
    );
    let mut existing_limited = Item::default();
    existing_limited
        .object_mut()
        .create(ObjectGuid::create_item(1, 501));
    existing_limited.object_mut().set_entry(6948);
    existing_limited.set_count(3);
    let stored_limited = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START + 1,
        &existing_limited,
        Some(&limited_proto),
    )];
    similar_args.stored_items = &stored_limited;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), similar_args),
        CanStoreItemOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(3),
        }
    );
}
#[test]
fn can_store_item_reports_item_max_count_after_partial_similar_limit_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let proto = ItemStorageTemplate {
        max_count: 10,
        ..ItemStorageTemplate::regular_item(6948, 20)
    };
    let mut args = can_store_args(NULL_BAG, NULL_SLOT, Some(&proto), 5);
    let mut existing_limited = Item::default();
    existing_limited
        .object_mut()
        .create(ObjectGuid::create_item(1, 502));
    existing_limited.object_mut().set_entry(6948);
    existing_limited.set_count(7);
    let stored_limited = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START + 1,
        &existing_limited,
        Some(&proto),
    )];
    args.stored_items = &stored_limited;
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(&mut dest, args),
        CanStoreItemOutcome {
            result: InventoryResult::ItemMaxCount,
            no_space_count: Some(2),
        }
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            3,
        )]
    );
}
#[test]
fn can_store_item_fills_specific_slot_then_continues_search_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 401));
    existing.object_mut().set_entry(6948);
    existing.set_count(15);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &existing,
    )];
    let mut args = can_store_args(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        Some(&proto),
        10,
    );
    args.slot_items = &slot_items;
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(&mut dest, args),
        CanStoreItemOutcome {
            result: InventoryResult::Ok,
            no_space_count: None,
        }
    );
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                5,
            ),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                5,
            ),
        ]
    );
}
#[test]
fn can_store_item_general_search_handles_new_bag_direct_equip_and_bag_in_bag() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(16);
    let bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        bonding: ItemBondingType::None,
        max_stack_size: 1,
        container_slots: 16,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let mut dest = Vec::new();

    assert_eq!(
        player.can_store_item(
            &mut dest,
            can_store_args(NULL_BAG, NULL_SLOT, Some(&bag_proto), 1)
        ),
        CanStoreItemOutcome {
            result: InventoryResult::Ok,
            no_space_count: None,
        }
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START),
            1,
        )]
    );

    let source = Item::default();
    let mut bag_in_bag_args = can_store_args(NULL_BAG, NULL_SLOT, Some(&bag_proto), 1);
    bag_in_bag_args.source_item = Some(&source);
    bag_in_bag_args.source_is_not_empty_bag = true;
    assert_eq!(
        player.can_store_item(&mut Vec::new(), bag_in_bag_args),
        CanStoreItemOutcome {
            result: InventoryResult::BagInBag,
            no_space_count: None,
        }
    );
}
#[test]
fn can_bank_item_preflight_matches_cpp_item_template_and_source_guards() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 700));
    source.object_mut().set_entry(6948);
    source.set_count(3);

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                None
            ),
        ),
        InventoryResult::ItemNotFound
    );

    let mut missing_swap = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        None,
    );
    missing_swap.swap = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), missing_swap),
        InventoryResult::CantSwap
    );

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                None,
                Some(&source)
            ),
        ),
        InventoryResult::ItemNotFound
    );

    source.set_loot_generated(true);
    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::LootGone
    );

    source.set_loot_generated(false);
    source.set_owner_guid(ObjectGuid::create_player(1, 42));
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_ITEM_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::NotOwner
    );

    source.remove_item_flag(ItemFieldFlags::SOULBOUND);
    let mut currency_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&proto),
        Some(&source),
    );
    currency_args.source_is_currency_token = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), currency_args),
        InventoryResult::CantSwap
    );

    let limited_proto = ItemStorageTemplate {
        max_count: 3,
        ..proto
    };
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 701));
    existing.object_mut().set_entry(6948);
    existing.set_count(3);
    let stored = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &existing,
        Some(&limited_proto),
    )];
    let mut limit_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_ITEM_START,
        Some(&limited_proto),
        Some(&source),
    );
    limit_args.stored_items = &stored;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), limit_args),
        InventoryResult::ItemMaxCount
    );
}
#[test]
fn can_bank_item_specific_bank_bag_slot_matches_cpp_guards() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 1);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 710));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_bank_item(
            &mut Vec::new(),
            can_bank_args(
                INVENTORY_SLOT_BAG_0,
                BANK_SLOT_BAG_START,
                Some(&proto),
                Some(&source),
            ),
        ),
        InventoryResult::WrongSlot
    );

    let mut bag_args = can_bank_args(
        INVENTORY_SLOT_BAG_0,
        BANK_SLOT_BAG_START,
        Some(&proto),
        Some(&source),
    );
    bag_args.source_is_bag = true;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), bag_args),
        InventoryResult::NoBankSlot
    );

    player.set_bank_bag_slot_count(1);
    bag_args.can_use_result = InventoryResult::CantUseItem;
    assert_eq!(
        player.can_bank_item(&mut Vec::new(), bag_args),
        InventoryResult::CantUseItem
    );
}
