use super::ItemStorageView;
use crate::player::{
    CHILD_EQUIPMENT_SLOT_END, CHILD_EQUIPMENT_SLOT_START, CanStoreItemArgs, CanStoreItemOutcome,
    CanTakeMoreSimilarItemsArgs, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END,
    INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, ItemPosCount,
    KEYRING_SLOT_END, KEYRING_SLOT_START, NULL_BAG, NULL_SLOT, bag_template_by_pos,
    can_store_item_count_zero, can_store_item_error, item_ref_by_pos,
};
use wow_constants::{
    BagFamilyMask, InventoryResult, ItemBondingType, ItemClass, ItemFieldFlags,
    ItemSubClassContainer,
};

impl ItemStorageView<'_> {
    /// Plans an item placement against the borrowed Player storage state.
    pub fn can_store_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanStoreItemArgs<'_>,
    ) -> CanStoreItemOutcome {
        let Some(proto) = args.proto else {
            return can_store_item_error(
                if args.swap {
                    InventoryResult::CantSwap
                } else {
                    InventoryResult::ItemNotFound
                },
                args.count,
                0,
            );
        };

        if let Some(source) = args.source_item {
            if source.loot_generated() {
                return can_store_item_error(InventoryResult::LootGone, args.count, 0);
            }

            if source.is_binded_not_with(
                self.owner_guid,
                proto,
                args.source_bop_trade_allowed_for_player,
            ) {
                return can_store_item_error(InventoryResult::NotOwner, args.count, 0);
            }
        }

        let mut count = args.count;
        let similar_result = self.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: args.proto,
            count,
            source_item: args.source_item,
            current_item_count: self.item_count_by_entry(
                proto.entry,
                true,
                args.source_item,
                args.stored_items,
            ),
            limit_category: args.limit_category,
            current_limit_category_count: self.item_count_with_limit_category(
                proto.item_limit_category,
                args.source_item,
                args.stored_items,
            ),
        });
        let no_similar_count = if similar_result.result == InventoryResult::Ok {
            0
        } else {
            let no_similar_count = similar_result.no_space_count.unwrap_or(0);
            if count == no_similar_count {
                return can_store_item_error(similar_result.result, no_similar_count, 0);
            }
            count -= no_similar_count;
            no_similar_count
        };

        if args.bag != NULL_BAG && args.slot != NULL_SLOT {
            let result = self.can_store_item_in_specific_slot(
                args.bag,
                args.slot,
                dest,
                proto,
                &mut count,
                args.swap,
                item_ref_by_pos(args.slot_items, args.bag, args.slot),
                args.source_item,
                args.source_is_not_empty_bag,
                bag_template_by_pos(args.bag_templates, args.bag),
            );
            if result != InventoryResult::Ok {
                return can_store_item_error(result, count, no_similar_count);
            }

            if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                return outcome;
            }
        }

        let inventory_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.backpack_slots)
            .min(INVENTORY_SLOT_ITEM_END);

        if args.bag != NULL_BAG {
            if proto.max_stack_size != 1 {
                if args.bag == INVENTORY_SLOT_BAG_0 {
                    let result = self.can_store_item_in_inventory_slots(
                        CHILD_EQUIPMENT_SLOT_START,
                        CHILD_EQUIPMENT_SLOT_END,
                        dest,
                        proto,
                        &mut count,
                        true,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        return can_store_item_error(result, count, no_similar_count);
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }

                    let result = self.can_store_item_in_inventory_slots(
                        INVENTORY_SLOT_ITEM_START,
                        inventory_end,
                        dest,
                        proto,
                        &mut count,
                        true,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        return can_store_item_error(result, count, no_similar_count);
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }
                } else {
                    let mut result = self.can_store_item_in_bag(
                        args.bag,
                        dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        NULL_BAG,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, args.bag),
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        result = self.can_store_item_in_bag(
                            args.bag,
                            dest,
                            proto,
                            &mut count,
                            true,
                            true,
                            args.source_item,
                            args.source_is_not_empty_bag,
                            NULL_BAG,
                            args.slot,
                            bag_template_by_pos(args.bag_templates, args.bag),
                            args.slot_items,
                        );
                    }
                    if result != InventoryResult::Ok {
                        return can_store_item_error(result, count, no_similar_count);
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }
                }
            }

            if args.bag == INVENTORY_SLOT_BAG_0 {
                if proto.bag_family.contains(BagFamilyMask::KEYS) {
                    let result = self.can_store_item_in_inventory_slots(
                        KEYRING_SLOT_START,
                        KEYRING_SLOT_END,
                        dest,
                        proto,
                        &mut count,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        return can_store_item_error(result, count, no_similar_count);
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }
                }

                if args
                    .source_item
                    .is_some_and(|source| source.has_item_flag(ItemFieldFlags::CHILD))
                {
                    let result = self.can_store_item_in_inventory_slots(
                        CHILD_EQUIPMENT_SLOT_START,
                        CHILD_EQUIPMENT_SLOT_END,
                        dest,
                        proto,
                        &mut count,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        return can_store_item_error(result, count, no_similar_count);
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }
                }

                let result = self.can_store_item_in_inventory_slots(
                    INVENTORY_SLOT_ITEM_START,
                    inventory_end,
                    dest,
                    proto,
                    &mut count,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    return can_store_item_error(result, count, no_similar_count);
                }
                if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                    return outcome;
                }
            } else {
                let mut result = self.can_store_item_in_bag(
                    args.bag,
                    dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    NULL_BAG,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, args.bag),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    result = self.can_store_item_in_bag(
                        args.bag,
                        dest,
                        proto,
                        &mut count,
                        false,
                        true,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        NULL_BAG,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, args.bag),
                        args.slot_items,
                    );
                }
                if result != InventoryResult::Ok {
                    return can_store_item_error(result, count, no_similar_count);
                }
                if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                    return outcome;
                }
            }
        }

        if proto.max_stack_size != 1 {
            let result = self.can_store_item_in_inventory_slots(
                CHILD_EQUIPMENT_SLOT_START,
                CHILD_EQUIPMENT_SLOT_END,
                dest,
                proto,
                &mut count,
                true,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                return can_store_item_error(result, count, no_similar_count);
            }
            if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                return outcome;
            }

            let result = self.can_store_item_in_inventory_slots(
                INVENTORY_SLOT_ITEM_START,
                inventory_end,
                dest,
                proto,
                &mut count,
                true,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                return can_store_item_error(result, count, no_similar_count);
            }
            if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                return outcome;
            }

            if !proto.bag_family.is_empty() {
                for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                    let result = self.can_store_item_in_bag(
                        bag_slot,
                        dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, bag_slot),
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        continue;
                    }
                    if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                        return outcome;
                    }
                }
            }

            for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                let result = self.can_store_item_in_bag(
                    bag_slot,
                    dest,
                    proto,
                    &mut count,
                    true,
                    true,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, bag_slot),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    continue;
                }
                if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                    return outcome;
                }
            }
        }

        if !proto.bag_family.is_empty() {
            if proto.bag_family.contains(BagFamilyMask::KEYS) {
                let result = self.can_store_item_in_inventory_slots(
                    KEYRING_SLOT_START,
                    KEYRING_SLOT_END,
                    dest,
                    proto,
                    &mut count,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    return can_store_item_error(result, count, no_similar_count);
                }
                if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                    return outcome;
                }
            }

            for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                let result = self.can_store_item_in_bag(
                    bag_slot,
                    dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, bag_slot),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    continue;
                }
                if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                    return outcome;
                }
            }
        }

        if args.source_is_not_empty_bag {
            return CanStoreItemOutcome {
                result: InventoryResult::BagInBag,
                no_space_count: None,
            };
        }

        if args
            .source_item
            .is_some_and(|source| source.has_item_flag(ItemFieldFlags::CHILD))
        {
            let result = self.can_store_item_in_inventory_slots(
                CHILD_EQUIPMENT_SLOT_START,
                CHILD_EQUIPMENT_SLOT_END,
                dest,
                proto,
                &mut count,
                false,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                return can_store_item_error(result, count, no_similar_count);
            }
            if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                return outcome;
            }
        }

        let mut search_slot_start = INVENTORY_SLOT_ITEM_START;
        if args.source_item.is_none()
            && proto.class_id == ItemClass::Container
            && proto.subclass_id == ItemSubClassContainer::Container as u32
            && matches!(
                proto.bonding,
                ItemBondingType::None | ItemBondingType::OnAcquire
            )
        {
            search_slot_start = INVENTORY_SLOT_BAG_START;
        }

        let result = self.can_store_item_in_inventory_slots(
            search_slot_start,
            inventory_end,
            dest,
            proto,
            &mut count,
            false,
            args.source_item,
            args.source_is_not_empty_bag,
            args.bag,
            args.slot,
            args.slot_items,
        );
        if result != InventoryResult::Ok {
            return can_store_item_error(result, count, no_similar_count);
        }
        if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
            return outcome;
        }

        for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
            let result = self.can_store_item_in_bag(
                bag_slot,
                dest,
                proto,
                &mut count,
                false,
                true,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                bag_template_by_pos(args.bag_templates, bag_slot),
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                continue;
            }
            if let Some(outcome) = can_store_item_count_zero(count, no_similar_count) {
                return outcome;
            }
        }

        can_store_item_error(InventoryResult::InvFull, count, no_similar_count)
    }
}
