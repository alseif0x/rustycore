//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn remove_arena_enchantments_scans_inventory_and_bags_like_cpp() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(2);
    let allowed = item_with_guid_entry(1260, 7600);
    let blocked = item_with_guid_entry(1261, 7601);
    let missing_ref = item_with_guid_entry(1262, 7602);
    let bag = ObjectGuid::create_item(1, 1263);
    let bag_blocked = item_with_guid_entry(1264, 7604);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, allowed.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, blocked.object().guid())
        .unwrap();
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, bag)
        .unwrap();
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag, 3)
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 0, bag_blocked.object().guid())
        .unwrap();
    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 1, missing_ref.object().guid())
        .unwrap();

    let actions = player.remove_arena_enchantments(
        EnchantmentSlot::EnhancementTemporary,
        &[
            ArenaEnchantmentItemRef::new(
                allowed.object().guid(),
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START,
                100,
                true,
            ),
            ArenaEnchantmentItemRef::new(
                blocked.object().guid(),
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START + 1,
                200,
                false,
            ),
            ArenaEnchantmentItemRef::new(
                bag_blocked.object().guid(),
                INVENTORY_SLOT_BAG_START,
                0,
                300,
                false,
            ),
        ],
    );

    assert_eq!(
        actions,
        vec![
            RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
                item_guid: blocked.object().guid(),
                bag: INVENTORY_SLOT_BAG_0,
                slot: INVENTORY_SLOT_ITEM_START + 1,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
                item_guid: bag_blocked.object().guid(),
                bag: INVENTORY_SLOT_BAG_START,
                slot: 0,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
            RemoveArenaEnchantmentAction::MissingInventoryItemRef {
                item_guid: missing_ref.object().guid(),
                bag: INVENTORY_SLOT_BAG_START,
                slot: 1,
                enchantment_slot: EnchantmentSlot::EnhancementTemporary,
            },
        ]
    );
}
#[test]
fn titan_grip_and_equipped_weapon_helpers_match_cpp_representable_rules() {
    let mut player = Player::new(None, false);
    let two_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon2Hand,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2000, 1)
    };
    let one_hand = ItemStorageTemplate {
        inventory_type: InventoryType::Weapon,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2001, 1)
    };
    let ranged = ItemStorageTemplate {
        inventory_type: InventoryType::Ranged,
        class_id: ItemClass::Weapon,
        ..ItemStorageTemplate::regular_item(2002, 1)
    };
    let ranged_right_non_wand = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Bow as u32,
        ..ItemStorageTemplate::regular_item(2003, 1)
    };
    let wand = ItemStorageTemplate {
        inventory_type: InventoryType::RangedRight,
        class_id: ItemClass::Weapon,
        subclass_id: ItemSubClassWeapon::Wand as u32,
        ..ItemStorageTemplate::regular_item(2004, 1)
    };

    assert!(Player::is_use_equipped_weapon(false, false, true));
    assert!(!Player::is_use_equipped_weapon(true, false, true));
    assert!(!Player::is_use_equipped_weapon(false, true, false));

    assert!(!player.can_titan_grip());
    assert_eq!(player.titan_grip_penalty_spell_id(), 0);
    assert!(player.is_two_hand_used_template(Some(&two_hand)));
    assert!(player.is_two_hand_used_template(Some(&ranged)));
    assert!(player.is_two_hand_used_template(Some(&ranged_right_non_wand)));
    assert!(!player.is_two_hand_used_template(Some(&wand)));
    assert!(!player.is_two_hand_used_template(None));

    player.set_can_titan_grip(true, 49152);
    player.set_can_titan_grip(true, 99999);
    assert!(player.can_titan_grip());
    assert_eq!(player.titan_grip_penalty_spell_id(), 49152);
    assert!(!player.is_two_hand_used_template(Some(&two_hand)));

    assert!(Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&one_hand),
        Some(&two_hand),
    ));
    assert!(Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&two_hand),
        Some(&one_hand),
    ));
    assert!(!Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&two_hand),
        None,
    ));
    assert!(!Player::is_using_two_handed_weapon_in_one_hand_template(
        Some(&one_hand),
        Some(&one_hand),
    ));

    assert_eq!(
        player.check_titan_grip_penalty_action(true, false),
        TitanGripPenaltyAction::Cast(49152)
    );
    assert_eq!(
        player.check_titan_grip_penalty_action(true, true),
        TitanGripPenaltyAction::None
    );
    assert_eq!(
        player.check_titan_grip_penalty_action(false, true),
        TitanGripPenaltyAction::Remove(49152)
    );

    player.set_can_titan_grip(false, 0);
    assert_eq!(
        player.check_titan_grip_penalty_action(true, false),
        TitanGripPenaltyAction::None
    );
}
#[test]
fn swap_item_preflight_matches_cpp_no_source_child_and_dead_order() {
    let player = Player::new(None, false);
    let src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let parent = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD);

    assert_eq!(
        player.swap_item_preflight_plan(src, dst, true, None, None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::NoSource,
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut child_source = SwapItemPreflightItem::regular();
    child_source.is_child = true;
    child_source.parent_pos = Some(parent);
    assert_eq!(
        player.swap_item_preflight_plan(src, dst, false, Some(child_source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::ChildRedirect {
                first_src: dst,
                first_dst: src,
                second_src: parent,
                second_dst: dst,
            },
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut child_dst = SwapItemPreflightItem::regular();
    child_dst.is_child = true;
    child_dst.parent_pos = Some(parent);
    assert_eq!(
        player.swap_item_preflight_plan(
            dst,
            src,
            true,
            Some(SwapItemPreflightItem::regular()),
            Some(child_dst)
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::ChildRedirect {
                first_src: dst,
                first_dst: src,
                second_src: parent,
                second_dst: dst,
            },
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut blocked_source = SwapItemPreflightItem::regular();
    blocked_source.can_unequip_result = InventoryResult::CantEquipEver;
    assert_eq!(
        player.swap_item_preflight_plan(src, dst, false, Some(blocked_source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );
}
#[test]
fn swap_item_preflight_matches_cpp_unequip_and_bag_self_guards() {
    let player = Player::new(None, false);
    let equipped_src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let source = SwapItemPreflightItem::regular();

    assert_eq!(
        player.swap_item_preflight_plan(equipped_src, inventory_dst, true, Some(source), None),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Continue,
            src_unequip_swap: Some(true),
            dst_unequip_swap: None,
        }
    );

    let mut blocked_source = SwapItemPreflightItem::regular();
    blocked_source.can_unequip_result = InventoryResult::ClientLockedOut;
    assert_eq!(
        player.swap_item_preflight_plan(
            equipped_src,
            inventory_dst,
            true,
            Some(blocked_source),
            None
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::ClientLockedOut),
            src_unequip_swap: Some(true),
            dst_unequip_swap: None,
        }
    );

    let bag_slot = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let inside_same_bag = make_item_pos(INVENTORY_SLOT_BAG_START, 0);
    assert_eq!(
        player.swap_item_preflight_plan(
            bag_slot,
            inside_same_bag,
            true,
            Some(SwapItemPreflightItem::bag(false)),
            None,
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::BagInBag),
            src_unequip_swap: Some(false),
            dst_unequip_swap: None,
        }
    );
    assert_eq!(
        player.swap_item_preflight_plan(
            inside_same_bag,
            bag_slot,
            true,
            Some(SwapItemPreflightItem::regular()),
            Some(SwapItemPreflightItem::bag(false)),
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::CantSwap),
            src_unequip_swap: None,
            dst_unequip_swap: None,
        }
    );

    let mut blocked_dst = SwapItemPreflightItem::bag(true);
    blocked_dst.can_unequip_result = InventoryResult::CantEquipEver;
    let other_bag_slot = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1);
    assert_eq!(
        player.swap_item_preflight_plan(
            inventory_dst,
            other_bag_slot,
            true,
            Some(SwapItemPreflightItem::bag(true)),
            Some(blocked_dst),
        ),
        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Error(InventoryResult::CantEquipEver),
            src_unequip_swap: None,
            dst_unequip_swap: Some(true),
        }
    );
}
#[test]
fn swap_item_empty_destination_plan_matches_cpp_move_case() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1);
    let bank_src = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START + 1);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            inventory_dst,
            true,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::OccupiedDestination,
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            bank_src,
            inventory_dst,
            false,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::MoveToInventory {
                quest_added_from_bank: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            inventory_dst,
            false,
            InventoryResult::InvFull,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::Error(InventoryResult::InvFull),
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            bank_dst,
            false,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::MoveToBank {
                quest_removed: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            equip_dst,
            false,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::Equip {
                dest: equip_dest,
                auto_unequip_offhand: true,
            },
        }
    );

    assert_eq!(
        player.swap_item_empty_destination_plan(
            inventory_src,
            make_item_pos(BUYBACK_SLOT_START, 0),
            false,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
        ),
        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::InvalidDestinationNoop,
        }
    );
}
#[test]
fn swap_item_merge_fill_plan_matches_cpp_occupied_non_bag_case() {
    let player = Player::new(None, false);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            true,
            false,
            3,
            4,
            20,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::ContinueToRealSwap,
            send_refund_info: false,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::ContinueToRealSwap,
            send_refund_info: false,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::MoveMergedStackToInventory,
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            bank_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::MoveMergedStackToBank,
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            equip_dst,
            false,
            false,
            3,
            4,
            20,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::EquipMergedStack {
                dest: equip_dest,
                auto_unequip_offhand: true,
            },
            send_refund_info: true,
        }
    );

    assert_eq!(
        player.swap_item_merge_fill_plan(
            inventory_dst,
            false,
            false,
            15,
            12,
            20,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            true,
        ),
        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::PartialFill {
                source_remaining_count: 7,
                destination_count: 20,
                send_updates: true,
            },
            send_refund_info: true,
        }
    );
}
#[test]
fn swap_item_real_swap_validation_plan_matches_cpp_bidirectional_checks() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    let equip_src = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_LEGS);
    let equip_dest = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_LEGS);
    let equip_dest2 = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target: SwapItemRealSwapTarget::Bank,
                destination_target: SwapItemRealSwapTarget::Inventory,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::InvFull,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::InvFull,
                subject: SwapItemRealSwapValidationSubject::Source,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            inventory_src,
            bank_dst,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::Ok,
            InventoryResult::ClientLockedOut,
            InventoryResult::Ok,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::ClientLockedOut,
                subject: SwapItemRealSwapValidationSubject::Destination,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            equip_src,
            equip_dst,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest,
            InventoryResult::DestroyNonemptyBag,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::Ok,
            equip_dest2,
            InventoryResult::Ok,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Error {
                result: InventoryResult::DestroyNonemptyBag,
                subject: SwapItemRealSwapValidationSubject::Source,
            },
        }
    );

    assert_eq!(
        player.swap_item_real_swap_validation_plan(
            make_item_pos(BUYBACK_SLOT_START, 0),
            make_item_pos(BUYBACK_SLOT_START + 1, 0),
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            InventoryResult::CantSwap,
            equip_dest2,
            InventoryResult::CantSwap,
        ),
        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target: SwapItemRealSwapTarget::None,
                destination_target: SwapItemRealSwapTarget::None,
            },
        }
    );
}
#[test]
fn swap_item_bag_exchange_plan_matches_cpp_empty_bag_exchange() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let inventory_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1);
    let bag_slot_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let full_items = [
        SwapBagItemRef::new(0, true),
        SwapBagItemRef::new(2, true),
        SwapBagItemRef::new(4, true),
    ];
    let full_bag = SwapBagRef::new(false, 5, &full_items);
    let empty_bag = SwapBagRef::new(true, 4, &[]);

    assert_eq!(
        player.swap_item_bag_exchange_plan(inventory_src, inventory_dst, None, Some(full_bag)),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Continue,
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source: true,
                moves: vec![
                    SwapBagItemMove {
                        from_slot: 0,
                        to_slot: 0,
                    },
                    SwapBagItemMove {
                        from_slot: 2,
                        to_slot: 1,
                    },
                    SwapBagItemMove {
                        from_slot: 4,
                        to_slot: 2,
                    },
                ],
            },
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(full_bag),
            Some(empty_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source: false,
                moves: vec![
                    SwapBagItemMove {
                        from_slot: 0,
                        to_slot: 0,
                    },
                    SwapBagItemMove {
                        from_slot: 2,
                        to_slot: 1,
                    },
                    SwapBagItemMove {
                        from_slot: 4,
                        to_slot: 2,
                    },
                ],
            },
        }
    );

    assert_eq!(
        player.swap_item_bag_exchange_plan(
            bag_slot_src,
            inventory_dst,
            Some(empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Continue,
        }
    );

    let blocked_items = [SwapBagItemRef::new(0, true), SwapBagItemRef::new(1, false)];
    let blocked_bag = SwapBagRef::new(false, 2, &blocked_items);
    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(empty_bag),
            Some(blocked_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
        }
    );

    let small_empty_bag = SwapBagRef::new(true, 2, &[]);
    assert_eq!(
        player.swap_item_bag_exchange_plan(
            inventory_src,
            inventory_dst,
            Some(small_empty_bag),
            Some(full_bag),
        ),
        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Error(InventoryResult::CantSwap),
        }
    );
}
#[test]
fn swap_item_real_swap_execution_plan_matches_cpp_final_actions() {
    let player = Player::new(None, false);
    let inventory_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let equip_dst = make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_CHEST);

    assert_eq!(
        player.swap_item_real_swap_execution_plan(
            inventory_src,
            equip_dst,
            SwapItemRealSwapTarget::Equip { dest: equip_dst },
            SwapItemRealSwapTarget::Inventory,
            false,
            false,
            false,
        ),
        SwapItemRealSwapExecutionPlan {
            remove_destination_update: false,
            remove_source_update: false,
            source_target: SwapItemRealSwapTarget::Equip { dest: equip_dst },
            destination_target: SwapItemRealSwapTarget::Inventory,
            apply_item_dependent_auras: true,
            release_loot: false,
            auto_unequip_offhand: true,
        }
    );

    let bag_src = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START);
    let bank_dst = make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START);
    assert!(
        player
            .swap_item_real_swap_execution_plan(
                bag_src,
                bank_dst,
                SwapItemRealSwapTarget::Bank,
                SwapItemRealSwapTarget::Inventory,
                true,
                true,
                false,
            )
            .release_loot
    );

    let bag_dst = make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START + 1);
    assert!(
        player
            .swap_item_real_swap_execution_plan(
                bank_dst,
                bag_dst,
                SwapItemRealSwapTarget::Inventory,
                SwapItemRealSwapTarget::Bank,
                true,
                false,
                true,
            )
            .release_loot
    );
    assert!(
        !player
            .swap_item_real_swap_execution_plan(
                bank_dst,
                bag_dst,
                SwapItemRealSwapTarget::Inventory,
                SwapItemRealSwapTarget::Bank,
                false,
                false,
                true,
            )
            .release_loot
    );
}
