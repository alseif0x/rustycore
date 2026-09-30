//! Player storage and equipment planning helpers.

use super::{
    is_bank_packed_pos, is_equipment_packed_pos, is_inventory_packed_pos, BANK_SLOT_BAG_END,
    BANK_SLOT_BAG_START, BUYBACK_SLOT_END, BUYBACK_SLOT_START, CanEquipItemOutcome,
    CanStoreItemOutcome, CanTakeMoreSimilarItemsOutcome, DestroyFilteredItemAction,
    DestroyFilteredItemRef, DestroyItemCountAction, DestroyItemCountItemRef, DestroyItemCountPlan,
    EQUIPMENT_SLOT_BACK, EQUIPMENT_SLOT_BODY, EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_FEET,
    EQUIPMENT_SLOT_FINGER1, EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HANDS, EQUIPMENT_SLOT_HEAD,
    EQUIPMENT_SLOT_LEGS, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_NECK, EQUIPMENT_SLOT_OFFHAND,
    EQUIPMENT_SLOT_SHOULDERS, EQUIPMENT_SLOT_TABARD, EQUIPMENT_SLOT_TRINKET1,
    EQUIPMENT_SLOT_TRINKET2, EQUIPMENT_SLOT_WAIST, EQUIPMENT_SLOT_WRISTS, EquippedGemRef,
    FindEquipSlotArgs, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END, INVENTORY_SLOT_BAG_START,
    InventoryResult, InventoryType, Item, ItemClass, ItemStorageRef, ItemSubclassProfession,
    MAX_BAG_SIZE, NULL_SLOT, PlayerStorageError,
    PROFESSION_SLOT_COOKING_GEAR1, PROFESSION_SLOT_COOKING_TOOL, PROFESSION_SLOT_FISHING_TOOL,
    PROFESSION_SLOT_MAX_COUNT, PROFESSION_SLOT_PROFESSION1_GEAR1,
    PROFESSION_SLOT_PROFESSION1_GEAR2, PROFESSION_SLOT_PROFESSION1_TOOL,
    PROFESSION_SLOT_PROFESSION2_GEAR1, PROFESSION_SLOT_PROFESSION2_GEAR2,
    REAGENT_BAG_SLOT_END, REAGENT_BAG_SLOT_START, SwapItemRealSwapTarget,
};

pub(super) fn equip_slot_candidates(args: FindEquipSlotArgs<'_>) -> [u8; 4] {
    let mut slots = [NULL_SLOT; 4];
    match args.proto.inventory_type {
        InventoryType::Head => slots[0] = EQUIPMENT_SLOT_HEAD,
        InventoryType::Neck => slots[0] = EQUIPMENT_SLOT_NECK,
        InventoryType::Shoulders => slots[0] = EQUIPMENT_SLOT_SHOULDERS,
        InventoryType::Body => slots[0] = EQUIPMENT_SLOT_BODY,
        InventoryType::Chest | InventoryType::Robe => slots[0] = EQUIPMENT_SLOT_CHEST,
        InventoryType::Waist => slots[0] = EQUIPMENT_SLOT_WAIST,
        InventoryType::Legs => slots[0] = EQUIPMENT_SLOT_LEGS,
        InventoryType::Feet => slots[0] = EQUIPMENT_SLOT_FEET,
        InventoryType::Wrists => slots[0] = EQUIPMENT_SLOT_WRISTS,
        InventoryType::Hands => slots[0] = EQUIPMENT_SLOT_HANDS,
        InventoryType::Finger => {
            slots[0] = EQUIPMENT_SLOT_FINGER1;
            slots[1] = EQUIPMENT_SLOT_FINGER2;
        }
        InventoryType::Trinket => {
            slots[0] = EQUIPMENT_SLOT_TRINKET1;
            slots[1] = EQUIPMENT_SLOT_TRINKET2;
        }
        InventoryType::Cloak => slots[0] = EQUIPMENT_SLOT_BACK,
        InventoryType::Weapon => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
            if args.can_dual_wield {
                slots[1] = EQUIPMENT_SLOT_OFFHAND;
            }
        }
        InventoryType::Shield | InventoryType::WeaponOffhand | InventoryType::Holdable => {
            slots[0] = EQUIPMENT_SLOT_OFFHAND;
        }
        InventoryType::Ranged | InventoryType::WeaponMainhand | InventoryType::RangedRight => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
        }
        InventoryType::Weapon2Hand => {
            slots[0] = EQUIPMENT_SLOT_MAINHAND;
            if args.can_dual_wield && args.can_titan_grip {
                slots[1] = EQUIPMENT_SLOT_OFFHAND;
            }
        }
        InventoryType::Tabard => slots[0] = EQUIPMENT_SLOT_TABARD,
        InventoryType::Bag => {
            slots[0] = INVENTORY_SLOT_BAG_START;
            slots[1] = INVENTORY_SLOT_BAG_START + 1;
            slots[2] = INVENTORY_SLOT_BAG_START + 2;
            slots[3] = INVENTORY_SLOT_BAG_START + 3;
        }
        InventoryType::ProfessionTool | InventoryType::ProfessionGear => {
            if args.proto.class_id != ItemClass::Profession || !args.has_required_profession_skill {
                return slots;
            }

            let is_tool = args.proto.inventory_type == InventoryType::ProfessionTool;
            match args.proto.subclass_id {
                value if value == ItemSubclassProfession::Cooking as u32 => {
                    slots[0] = if is_tool {
                        PROFESSION_SLOT_COOKING_TOOL
                    } else {
                        PROFESSION_SLOT_COOKING_GEAR1
                    };
                }
                value if value == ItemSubclassProfession::Fishing as u32 => {
                    if !is_tool {
                        return [NULL_SLOT; 4];
                    }
                    slots[0] = PROFESSION_SLOT_FISHING_TOOL;
                }
                value
                    if value == ItemSubclassProfession::Blacksmithing as u32
                        || value == ItemSubclassProfession::Leatherworking as u32
                        || value == ItemSubclassProfession::Alchemy as u32
                        || value == ItemSubclassProfession::Herbalism as u32
                        || value == ItemSubclassProfession::Mining as u32
                        || value == ItemSubclassProfession::Tailoring as u32
                        || value == ItemSubclassProfession::Engineering as u32
                        || value == ItemSubclassProfession::Enchanting as u32
                        || value == ItemSubclassProfession::Skinning as u32
                        || value == ItemSubclassProfession::Jewelcrafting as u32
                        || value == ItemSubclassProfession::Inscription as u32 =>
                {
                    let Some(profession_slot) = args.profession_slot else {
                        return [NULL_SLOT; 4];
                    };

                    if is_tool {
                        slots[0] = PROFESSION_SLOT_PROFESSION1_TOOL
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                    } else {
                        // C++ writes slots[0] twice here, so primary profession gear1 is unreachable.
                        slots[0] = PROFESSION_SLOT_PROFESSION1_GEAR1
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                        slots[0] = PROFESSION_SLOT_PROFESSION1_GEAR2
                            + profession_slot * PROFESSION_SLOT_MAX_COUNT;
                    }
                }
                _ => return [NULL_SLOT; 4],
            }
        }
        _ => return slots,
    }
    slots
}

pub(super) fn paired_unique_ignore_slot(slot: u8) -> Option<u8> {
    match slot {
        EQUIPMENT_SLOT_MAINHAND => Some(EQUIPMENT_SLOT_OFFHAND),
        EQUIPMENT_SLOT_OFFHAND => Some(EQUIPMENT_SLOT_MAINHAND),
        EQUIPMENT_SLOT_FINGER1 => Some(EQUIPMENT_SLOT_FINGER2),
        EQUIPMENT_SLOT_FINGER2 => Some(EQUIPMENT_SLOT_FINGER1),
        EQUIPMENT_SLOT_TRINKET1 => Some(EQUIPMENT_SLOT_TRINKET2),
        EQUIPMENT_SLOT_TRINKET2 => Some(EQUIPMENT_SLOT_TRINKET1),
        PROFESSION_SLOT_PROFESSION1_GEAR1 => Some(PROFESSION_SLOT_PROFESSION1_GEAR2),
        PROFESSION_SLOT_PROFESSION1_GEAR2 => Some(PROFESSION_SLOT_PROFESSION1_GEAR1),
        PROFESSION_SLOT_PROFESSION2_GEAR1 => Some(PROFESSION_SLOT_PROFESSION2_GEAR2),
        PROFESSION_SLOT_PROFESSION2_GEAR2 => Some(PROFESSION_SLOT_PROFESSION2_GEAR1),
        _ => None,
    }
}

pub(super) fn has_equipped_item_entry(
    equipped_items: &[ItemStorageRef<'_>],
    entry: u32,
    except_slot: u8,
) -> bool {
    equipped_items.iter().any(|stored| {
        stored.bag == INVENTORY_SLOT_BAG_0
            && stored.slot != except_slot
            && stored.item.object().entry() == entry
    })
}

pub(super) fn has_equipped_gem_entry(equipped_gems: &[EquippedGemRef], entry: u32, except_slot: u8) -> bool {
    equipped_gems
        .iter()
        .any(|gem| gem.slot != except_slot && gem.entry == entry)
}

pub(super) fn equipped_item_limit_category_count(
    equipped_items: &[ItemStorageRef<'_>],
    limit_category: u32,
    except_slot: u8,
) -> u32 {
    equipped_items
        .iter()
        .filter(|stored| {
            stored.bag == INVENTORY_SLOT_BAG_0
                && stored.slot != except_slot
                && stored
                    .template
                    .is_some_and(|template| template.item_limit_category == limit_category)
        })
        .map(|stored| stored.item.count())
        .sum()
}

pub(super) fn equipped_gem_limit_category_count(
    equipped_gems: &[EquippedGemRef],
    limit_category: u32,
    except_slot: u8,
) -> u32 {
    equipped_gems
        .iter()
        .filter(|gem| gem.slot != except_slot && gem.limit_category == limit_category)
        .count() as u32
}

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

pub(super) fn destroy_item_count_scan_top_level_range(
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

pub(super) fn destroy_item_count_scan_bag_ranges(
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

pub(super) fn destroy_filtered_scan_top_level_range(
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

pub(super) fn destroy_filtered_scan_bag_ranges(
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

pub(super) fn swap_item_real_swap_target_for_destination(
    destination: u16,
    can_store_result: InventoryResult,
    can_bank_result: InventoryResult,
    can_equip_result: InventoryResult,
    equip_dest: u16,
    equip_dest_can_unequip_result: InventoryResult,
) -> (InventoryResult, SwapItemRealSwapTarget) {
    if is_inventory_packed_pos(destination) {
        return (can_store_result, SwapItemRealSwapTarget::Inventory);
    }

    if is_bank_packed_pos(destination) {
        return (can_bank_result, SwapItemRealSwapTarget::Bank);
    }

    if is_equipment_packed_pos(destination) {
        if can_equip_result == InventoryResult::Ok {
            return (
                equip_dest_can_unequip_result,
                SwapItemRealSwapTarget::Equip { dest: equip_dest },
            );
        }

        return (
            can_equip_result,
            SwapItemRealSwapTarget::Equip { dest: equip_dest },
        );
    }

    (InventoryResult::Ok, SwapItemRealSwapTarget::None)
}

pub(super) fn is_bag_storage_slot(slot: u8) -> bool {
    (INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&slot)
        || (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&slot)
        || (REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END).contains(&slot)
}

pub fn is_buyback_slot(slot: u8) -> bool {
    (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
}

pub(super) fn validate_split_source(source: &Item, count: u32) -> Result<(), PlayerStorageError> {
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

pub(super) fn can_store_item_error(
    result: InventoryResult,
    count: u32,
    no_similar_count: u32,
) -> CanStoreItemOutcome {
    CanStoreItemOutcome {
        result,
        no_space_count: Some(count + no_similar_count),
    }
}

pub(super) fn can_store_item_count_zero(count: u32, no_similar_count: u32) -> Option<CanStoreItemOutcome> {
    (count == 0).then(|| {
        if no_similar_count == 0 {
            CanStoreItemOutcome {
                result: InventoryResult::Ok,
                no_space_count: None,
            }
        } else {
            can_store_item_error(InventoryResult::ItemMaxCount, count, no_similar_count)
        }
    })
}

pub(super) fn can_equip_item_outcome(result: InventoryResult) -> CanEquipItemOutcome {
    CanEquipItemOutcome {
        result,
        dest: 0,
        unique_ignore_slot: None,
    }
}

pub(super) fn can_take_more_similar_ok() -> CanTakeMoreSimilarItemsOutcome {
    CanTakeMoreSimilarItemsOutcome {
        result: InventoryResult::Ok,
        no_space_count: None,
        offending_item_id: None,
    }
}
