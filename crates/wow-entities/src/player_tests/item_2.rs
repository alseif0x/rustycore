//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn can_equip_unique_item_object_matches_cpp_template_then_gem_order() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(704, 1);
    let source = Item::default();
    let gem_proto = ItemStorageTemplate::regular_item(705, 1);
    let socketed_gems = [
        SocketedGemUniqueRef::new(None, true, None, 1),
        SocketedGemUniqueRef::new(Some(&gem_proto), true, None, 1),
    ];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 705, 0)];
    let base_equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 704, 0)];

    assert_eq!(
        player.can_equip_unique_item(can_equip_unique_args(None, Some(&proto))),
        InventoryResult::ItemNotFound
    );

    let mut template_first = can_equip_unique_args(Some(&source), Some(&proto));
    template_first.unique_equippable = true;
    template_first.equipped_gems = &base_equipped_gems;
    template_first.socketed_gems = &socketed_gems;
    assert_eq!(
        player.can_equip_unique_item(template_first),
        InventoryResult::ItemUniqueEquippable
    );

    let mut gem_args = can_equip_unique_args(Some(&source), Some(&proto));
    gem_args.socketed_gems = &socketed_gems;
    gem_args.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item(gem_args),
        InventoryResult::ItemUniqueEquippable
    );
}
#[test]
fn can_equip_unique_item_object_matches_cpp_socketed_gem_limit_count() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(706, 1);
    let gem_proto = ItemStorageTemplate {
        item_limit_category: 20,
        ..ItemStorageTemplate::regular_item(707, 1)
    };
    let limit = ItemLimitCategoryTemplate {
        id: 20,
        quantity: 2,
        flags: ITEM_LIMIT_CATEGORY_MODE_EQUIP,
    };
    let socketed_gems = [SocketedGemUniqueRef::new(
        Some(&gem_proto),
        false,
        Some(&limit),
        2,
    )];
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_CHEST, 708, 20)];

    let mut source = Item::default();
    source.set_slot(INVENTORY_SLOT_ITEM_START);
    let mut unequipped = can_equip_unique_args(Some(&source), Some(&proto));
    unequipped.socketed_gems = &socketed_gems;
    unequipped.equipped_gems = &equipped_gems;
    assert_eq!(
        player.can_equip_unique_item(unequipped),
        InventoryResult::ItemMaxCountEquippedSocketed
    );

    let mut equipped_source = Item::default();
    equipped_source.set_slot(EQUIPMENT_SLOT_FINGER1);
    let mut equipped = can_equip_unique_args(Some(&equipped_source), Some(&proto));
    equipped.socketed_gems = &socketed_gems;
    equipped.equipped_gems = &equipped_gems;
    assert_eq!(player.can_equip_unique_item(equipped), InventoryResult::Ok);
}
#[test]
fn item_pos_count_containment_matches_cpp_pos_only_check() {
    let target = ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, 10), 1);
    let positions = [ItemPosCount::new(
        make_item_pos(INVENTORY_SLOT_BAG_0, 10),
        99,
    )];

    assert!(target.is_contained_in(&positions));
    assert!(
        !ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, 11), 1).is_contained_in(&positions)
    );
}
#[test]
fn can_store_item_in_specific_slot_allocates_empty_top_level_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut dest = Vec::new();
    let mut count = 7;

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            7,
        )]
    );
    assert_eq!(count, 0);

    let mut duplicate_count = 3;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut duplicate_count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(dest.len(), 1);
    assert_eq!(duplicate_count, 3);
}
#[test]
fn can_store_item_in_specific_slot_merges_existing_stack_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut existing = Item::default();
    existing
        .object_mut()
        .create(ObjectGuid::create_item(1, 100));
    existing.object_mut().set_entry(6948);
    existing.set_count(12);
    let mut dest = Vec::new();
    let mut count = 10;

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut dest,
            &proto,
            &mut count,
            false,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            8,
        )]
    );
    assert_eq!(count, 2);

    existing.object_mut().set_entry(6949);
    let mut swap_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut swap_count,
            true,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );

    let mut blocked_count = 2;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut blocked_count,
            false,
            Some(&existing),
            None,
            false,
            None,
        ),
        InventoryResult::CantStack
    );
    assert_eq!(blocked_count, 2);
}
#[test]
fn can_store_item_in_specific_slot_applies_source_move_guards_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 101));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            true,
            None,
        ),
        InventoryResult::DestroyNonemptyBag
    );

    let mut bag_slot_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut bag_slot_count,
            false,
            None,
            Some(&source),
            true,
            None,
        ),
        InventoryResult::Ok
    );

    let mut same_source_count = 1;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut same_source_count,
            false,
            Some(&source),
            Some(&source),
            false,
            None,
        ),
        InventoryResult::Ok
    );

    source.set_item_flag(ItemFieldFlags::CHILD);
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            false,
            None,
        ),
        InventoryResult::WrongBagType3
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            CHILD_EQUIPMENT_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            Some(&source),
            false,
            None,
        ),
        InventoryResult::Ok
    );
}
#[test]
fn can_store_item_in_specific_slot_applies_empty_slot_fit_guards_like_cpp() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let herb_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::HerbContainer as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(101, 1)
    };
    let herb = ItemStorageTemplate {
        bag_family: BagFamilyMask::HERBS,
        ..ItemStorageTemplate::regular_item(2447, 20)
    };

    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            REAGENT_BAG_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            BUYBACK_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::WrongBagType
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 300), 2)
        .unwrap();
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            2,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            Some(&regular_bag_proto),
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            None,
            None,
            false,
            Some(&herb_bag_proto),
        ),
        InventoryResult::WrongBagType
    );

    let mut dest = Vec::new();
    let mut count = 3;
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_START,
            0,
            &mut dest,
            &herb,
            &mut count,
            false,
            None,
            None,
            false,
            Some(&herb_bag_proto),
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            3
        )]
    );
}
#[test]
fn can_store_item_in_specific_slot_preserves_cpp_keyring_gate_condition() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut count = 1;

    assert!(!cpp_keyring_family_gate_applies(KEYRING_SLOT_START));
    assert_eq!(
        player.can_store_item_in_specific_slot(
            INVENTORY_SLOT_BAG_0,
            KEYRING_SLOT_START,
            &mut Vec::new(),
            &proto,
            &mut count,
            false,
            None,
            None,
            false,
            None,
        ),
        InventoryResult::Ok
    );
}
#[test]
fn can_store_item_in_inventory_slots_merges_matching_stacks_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut matching = Item::default();
    matching
        .object_mut()
        .create(ObjectGuid::create_item(1, 200));
    matching.object_mut().set_entry(6948);
    matching.set_count(16);
    let mut wrong_entry = Item::default();
    wrong_entry
        .object_mut()
        .create(ObjectGuid::create_item(1, 201));
    wrong_entry.object_mut().set_entry(6949);
    wrong_entry.set_count(1);
    let slot_items = [
        ItemSlotRef::new(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, &matching),
        ItemSlotRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &wrong_entry,
        ),
    ];
    let mut dest = Vec::new();
    let mut count = 6;

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 3,
            &mut dest,
            &proto,
            &mut count,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            4,
        )]
    );
    assert_eq!(count, 2);
}
#[test]
fn can_store_item_in_inventory_slots_allocates_empty_slots_like_cpp() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut occupied = Item::default();
    occupied
        .object_mut()
        .create(ObjectGuid::create_item(1, 202));
    occupied.object_mut().set_entry(6948);
    occupied.set_count(1);
    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &occupied,
    )];
    let mut dest = vec![ItemPosCount::new(
        make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
        1,
    )];
    let mut count = 7;

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 3,
            &mut dest,
            &proto,
            &mut count,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                1,
            ),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 2),
                7,
            ),
        ]
    );
    assert_eq!(count, 0);
}
#[test]
fn can_store_item_in_inventory_slots_applies_cpp_source_and_skip_rules() {
    let player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 203));
    source.object_mut().set_entry(6948);
    source.set_count(1);

    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 1,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            Some(&source),
            true,
            NULL_BAG,
            NULL_SLOT,
            &[],
        ),
        InventoryResult::DestroyNonemptyBag
    );

    let slot_items = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        &source,
    )];
    let mut dest = Vec::new();
    let mut count = 1;
    assert_eq!(
        player.can_store_item_in_inventory_slots(
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START + 2,
            &mut dest,
            &proto,
            &mut count,
            false,
            Some(&source),
            false,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            &slot_items,
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
            1,
        )]
    );
    assert_eq!(count, 0);
}
#[test]
fn can_store_item_in_bag_applies_cpp_bag_and_source_guards() {
    let mut player = Player::new(None, false);
    let proto = ItemStorageTemplate::regular_item(6948, 20);
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 4,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let bag_guid = ObjectGuid::create_item(1, 300);

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            None,
            false,
            INVENTORY_SLOT_BAG_START,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut source_bag = Item::default();
    source_bag.object_mut().create(bag_guid);
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source_bag),
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(1, 301));
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source),
            true,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::DestroyNonemptyBag
    );

    source.set_item_flag(ItemFieldFlags::CHILD);
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &proto,
            &mut 1,
            false,
            true,
            Some(&source),
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType3
    );
}
#[test]
fn can_store_item_in_bag_applies_cpp_specialized_mode_and_family_rules() {
    let mut player = Player::new(None, false);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 310), 2)
        .unwrap();
    let misc = ItemStorageTemplate::regular_item(6948, 20);
    let herb = ItemStorageTemplate {
        bag_family: BagFamilyMask::HERBS,
        ..ItemStorageTemplate::regular_item(2447, 20)
    };
    let regular_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(100, 1)
    };
    let herb_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::HerbContainer as u32,
        container_slots: 2,
        ..ItemStorageTemplate::regular_item(101, 1)
    };

    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &misc,
            &mut 1,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&regular_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut Vec::new(),
            &misc,
            &mut 1,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&herb_bag_proto),
            &[],
        ),
        InventoryResult::WrongBagType
    );

    let mut dest = Vec::new();
    let mut count = 1;
    assert_eq!(
        player.can_store_item_in_bag(
            INVENTORY_SLOT_BAG_START,
            &mut dest,
            &herb,
            &mut count,
            false,
            false,
            None,
            false,
            NULL_BAG,
            NULL_SLOT,
            Some(&herb_bag_proto),
            &[],
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_START, 0),
            1,
        )]
    );
}
