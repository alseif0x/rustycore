// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Destruction planning and split-source validation for item stacks.

use super::super::{
    DestroyFilteredItemAction, DestroyFilteredItemRef, DestroyItemCountAction,
    DestroyItemCountItemRef, DestroyItemCountPlan, INVENTORY_SLOT_BAG_0, InventoryResult, Item,
    MAX_BAG_SIZE, PlayerStorageError,
};

fn destroy_item_count_item_by_pos<'a>(
    items: &[DestroyItemCountItemRef<'a>],
    bag: u8,
    slot: u8,
) -> Option<DestroyItemCountItemRef<'a>> {
    items
        .iter()
        .find(|item_ref| item_ref.bag == bag && item_ref.slot == slot)
        .copied()
}

fn destroy_item_count_consider_item(
    plan: &mut DestroyItemCountPlan,
    item_ref: DestroyItemCountItemRef<'_>,
    item_entry: u32,
    requested_count: u32,
    require_unequip_for_full_stack: bool,
    unequip_check: bool,
) {
    if plan.removed_count >= requested_count
        || item_ref.item.object().entry() != item_entry
        || item_ref.item.is_in_trade()
    {
        return;
    }

    let needed = requested_count - plan.removed_count;
    let item_count = item_ref.item.count();
    if item_count <= needed {
        if require_unequip_for_full_stack
            && unequip_check
            && item_ref.can_unequip_result != InventoryResult::Ok
        {
            return;
        }

        plan.actions.push(DestroyItemCountAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
            removed_count: item_count,
            remaining_count: 0,
            destroy_stack: true,
        });
        plan.removed_count += item_count;
    } else {
        plan.actions.push(DestroyItemCountAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
            removed_count: needed,
            remaining_count: item_count - needed,
            destroy_stack: false,
        });
        plan.removed_count = requested_count;
    }
}

pub(in crate::player) fn destroy_item_count_scan_top_level_range(
    plan: &mut DestroyItemCountPlan,
    items: &[DestroyItemCountItemRef<'_>],
    item_entry: u32,
    requested_count: u32,
    start: u8,
    end: u8,
    require_unequip_for_full_stack: bool,
    unequip_check: bool,
) {
    for slot in start..end {
        if let Some(item_ref) = destroy_item_count_item_by_pos(items, INVENTORY_SLOT_BAG_0, slot) {
            destroy_item_count_consider_item(
                plan,
                item_ref,
                item_entry,
                requested_count,
                require_unequip_for_full_stack,
                unequip_check,
            );
            if plan.removed_count >= requested_count {
                return;
            }
        }
    }
}

pub(in crate::player) fn destroy_item_count_scan_bag_ranges(
    plan: &mut DestroyItemCountPlan,
    items: &[DestroyItemCountItemRef<'_>],
    item_entry: u32,
    requested_count: u32,
    start_bag: u8,
    end_bag: u8,
) {
    for bag in start_bag..end_bag {
        for slot in 0..MAX_BAG_SIZE as u8 {
            if let Some(item_ref) = destroy_item_count_item_by_pos(items, bag, slot) {
                destroy_item_count_consider_item(
                    plan,
                    item_ref,
                    item_entry,
                    requested_count,
                    false,
                    false,
                );
                if plan.removed_count >= requested_count {
                    return;
                }
            }
        }
    }
}

fn destroy_filtered_item_by_pos(
    items: &[DestroyFilteredItemRef],
    bag: u8,
    slot: u8,
) -> Option<DestroyFilteredItemRef> {
    items
        .iter()
        .find(|item_ref| item_ref.bag == bag && item_ref.slot == slot)
        .copied()
}

fn destroy_filtered_consider_item(
    actions: &mut Vec<DestroyFilteredItemAction>,
    item_ref: DestroyFilteredItemRef,
) {
    if item_ref.should_destroy {
        actions.push(DestroyFilteredItemAction {
            bag: item_ref.bag,
            slot: item_ref.slot,
        });
    }
}

pub(in crate::player) fn destroy_filtered_scan_top_level_range(
    actions: &mut Vec<DestroyFilteredItemAction>,
    items: &[DestroyFilteredItemRef],
    start: u8,
    end: u8,
) {
    for slot in start..end {
        if let Some(item_ref) = destroy_filtered_item_by_pos(items, INVENTORY_SLOT_BAG_0, slot) {
            destroy_filtered_consider_item(actions, item_ref);
        }
    }
}

pub(in crate::player) fn destroy_filtered_scan_bag_ranges(
    actions: &mut Vec<DestroyFilteredItemAction>,
    items: &[DestroyFilteredItemRef],
    start_bag: u8,
    end_bag: u8,
) {
    for bag in start_bag..end_bag {
        for slot in 0..MAX_BAG_SIZE as u8 {
            if let Some(item_ref) = destroy_filtered_item_by_pos(items, bag, slot) {
                destroy_filtered_consider_item(actions, item_ref);
            }
        }
    }
}

pub(in crate::player) fn validate_split_source(
    source: &Item,
    count: u32,
) -> Result<(), PlayerStorageError> {
    if source.loot_generated() {
        return Err(PlayerStorageError::SplitItemLootGenerated);
    }

    let available = source.count();
    if count == 0 || available == count {
        return Err(PlayerStorageError::InvalidSplitCount {
            available,
            requested: count,
        });
    }

    if available < count {
        return Err(PlayerStorageError::TooFewItemsToSplit {
            available,
            requested: count,
        });
    }

    if source.is_in_trade() {
        return Err(PlayerStorageError::SplitItemInTrade);
    }

    Ok(())
}
