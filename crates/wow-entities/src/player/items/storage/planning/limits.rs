use super::ItemStorageView;
use crate::Item;
use crate::player::{
    CanTakeMoreSimilarItemsArgs, CanTakeMoreSimilarItemsOutcome, ITEM_LIMIT_CATEGORY_MODE_HAVE,
    ItemStorageRef, can_take_more_similar_ok, is_bank_pos, is_equipment_pos, is_inventory_pos,
};
use wow_constants::InventoryResult;

impl ItemStorageView<'_> {
    pub(in crate::player::items::storage) fn can_take_more_similar_items(
        &self,
        args: CanTakeMoreSimilarItemsArgs<'_>,
    ) -> CanTakeMoreSimilarItemsOutcome {
        let Some(proto) = args.proto else {
            return CanTakeMoreSimilarItemsOutcome {
                result: InventoryResult::ItemMaxCount,
                no_space_count: Some(args.count),
                offending_item_id: None,
            };
        };

        if args.source_item.is_some_and(Item::loot_generated) {
            return CanTakeMoreSimilarItemsOutcome {
                result: InventoryResult::LootGone,
                no_space_count: None,
                offending_item_id: None,
            };
        }

        if (proto.max_count <= 0 && proto.item_limit_category == 0) || proto.max_count == i32::MAX {
            return can_take_more_similar_ok();
        }

        if proto.max_count > 0 {
            let max_count = proto.max_count as u32;
            if args.current_item_count.saturating_add(args.count) > max_count {
                return CanTakeMoreSimilarItemsOutcome {
                    result: InventoryResult::ItemMaxCount,
                    no_space_count: Some(
                        args.current_item_count
                            .saturating_add(args.count)
                            .saturating_sub(max_count),
                    ),
                    offending_item_id: None,
                };
            }
        }

        if proto.item_limit_category != 0 {
            let Some(limit_category) = args.limit_category else {
                return CanTakeMoreSimilarItemsOutcome {
                    result: InventoryResult::NotEquippable,
                    no_space_count: Some(args.count),
                    offending_item_id: None,
                };
            };

            if limit_category.flags == ITEM_LIMIT_CATEGORY_MODE_HAVE {
                let limit_quantity = u32::from(limit_category.quantity);
                if args.current_limit_category_count.saturating_add(args.count) > limit_quantity {
                    return CanTakeMoreSimilarItemsOutcome {
                        result: InventoryResult::ItemMaxLimitCategoryCountExceededIs,
                        no_space_count: Some(
                            args.current_limit_category_count
                                .saturating_add(args.count)
                                .saturating_sub(limit_quantity),
                        ),
                        offending_item_id: Some(proto.entry),
                    };
                }
            }
        }

        can_take_more_similar_ok()
    }

    pub(in crate::player::items::storage) fn item_count_by_entry(
        &self,
        entry: u32,
        in_bank_also: bool,
        skip_item: Option<&Item>,
        stored_items: &[ItemStorageRef<'_>],
    ) -> u32 {
        stored_items
            .iter()
            .filter(|stored| {
                is_equipment_pos(stored.bag, stored.slot)
                    || is_inventory_pos(stored.bag, stored.slot)
                    || (in_bank_also && is_bank_pos(stored.bag, stored.slot))
            })
            .filter(|stored| {
                skip_item.is_none_or(|skip| stored.item.object().guid() != skip.object().guid())
            })
            .filter(|stored| stored.item.object().entry() == entry)
            .map(|stored| stored.item.count())
            .sum()
    }

    pub(in crate::player::items::storage) fn item_count_with_limit_category(
        &self,
        limit_category: u32,
        skip_item: Option<&Item>,
        stored_items: &[ItemStorageRef<'_>],
    ) -> u32 {
        stored_items
            .iter()
            .filter(|stored| {
                skip_item.is_none_or(|skip| stored.item.object().guid() != skip.object().guid())
            })
            .filter(|stored| {
                stored
                    .template
                    .is_some_and(|template| template.item_limit_category == limit_category)
            })
            .map(|stored| stored.item.count())
            .sum()
    }
}
