// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Inventory item counts, slot allocation and store admission.

use super::super::super::*;

impl Player {
    pub fn can_store_item_in_specific_slot(
        &self,
        bag: u8,
        slot: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        swap: bool,
        existing_item: Option<&Item>,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        bag_proto: Option<&ItemStorageTemplate>,
    ) -> InventoryResult {
        let existing_item = existing_item.filter(|existing| {
            source_item.is_none_or(|source| existing.object().guid() != source.object().guid())
        });

        if let Some(source) = source_item {
            if source_is_not_empty_bag && !is_bag_pos(make_item_pos(bag, slot)) {
                return InventoryResult::DestroyNonemptyBag;
            }

            let source_is_child = source.has_item_flag(ItemFieldFlags::CHILD);
            if source_is_child && !is_equipment_pos(bag, slot) && !is_child_equipment_pos(bag, slot)
            {
                return InventoryResult::WrongBagType3;
            }
            if !source_is_child && is_child_equipment_pos(bag, slot) {
                return InventoryResult::WrongBagType3;
            }
        }

        let need_space = if existing_item.is_none() || swap {
            if slot == REAGENT_BAG_SLOT_START {
                return InventoryResult::WrongBagType;
            }

            if bag == INVENTORY_SLOT_BAG_0 {
                if cpp_keyring_family_gate_applies(slot)
                    && !proto.bag_family.contains(BagFamilyMask::KEYS)
                {
                    return InventoryResult::WrongBagType;
                }

                if (BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot)
                    || slot as usize >= PLAYER_SLOT_END
                {
                    return InventoryResult::WrongBagType;
                }
            } else {
                if self.get_bag_by_pos(bag).is_none() {
                    return InventoryResult::WrongBagType;
                }

                let Some(bag_proto) = bag_proto else {
                    return InventoryResult::WrongBagType;
                };

                if slot >= bag_proto.container_slots {
                    return InventoryResult::WrongBagType;
                }

                if !item_can_go_into_bag(proto, bag_proto) {
                    return InventoryResult::WrongBagType;
                }
            }

            proto.max_stack_size
        } else {
            let existing_item = existing_item.expect("checked Some above");
            let result = existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size);
            if result != InventoryResult::Ok {
                return result;
            }

            proto.max_stack_size - existing_item.count()
        };

        let need_space = need_space.min(*count);
        let new_position = ItemPosCount::new(make_item_pos(bag, slot), need_space);
        if !new_position.is_contained_in(dest) {
            dest.push(new_position);
            *count -= need_space;
        }

        InventoryResult::Ok
    }

    pub fn can_store_item_in_inventory_slots(
        &self,
        slot_begin: u8,
        slot_end: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        merge: bool,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        skip_bag: u8,
        skip_slot: u8,
        slot_items: &[ItemSlotRef<'_>],
    ) -> InventoryResult {
        if source_item.is_some() && source_is_not_empty_bag {
            return InventoryResult::DestroyNonemptyBag;
        }

        for slot in slot_begin..slot_end {
            if skip_bag == INVENTORY_SLOT_BAG_0 && slot == skip_slot {
                continue;
            }

            if slot == REAGENT_BAG_SLOT_START {
                continue;
            }

            let existing_item =
                item_ref_by_pos(slot_items, INVENTORY_SLOT_BAG_0, slot).filter(|existing| {
                    source_item
                        .is_none_or(|source| existing.object().guid() != source.object().guid())
                });

            if existing_item.is_some() != merge {
                continue;
            }

            let mut need_space = proto.max_stack_size;
            if let Some(existing_item) = existing_item {
                if existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size)
                    != InventoryResult::Ok
                {
                    continue;
                }

                need_space -= existing_item.count();
            }

            need_space = need_space.min(*count);
            let new_position =
                ItemPosCount::new(make_item_pos(INVENTORY_SLOT_BAG_0, slot), need_space);
            if !new_position.is_contained_in(dest) {
                dest.push(new_position);
                *count -= need_space;

                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }

    pub fn can_store_item_in_bag(
        &self,
        bag: u8,
        dest: &mut Vec<ItemPosCount>,
        proto: &ItemStorageTemplate,
        count: &mut u32,
        merge: bool,
        non_specialized: bool,
        source_item: Option<&Item>,
        source_is_not_empty_bag: bool,
        skip_bag: u8,
        skip_slot: u8,
        bag_proto: Option<&ItemStorageTemplate>,
        slot_items: &[ItemSlotRef<'_>],
    ) -> InventoryResult {
        if bag == skip_bag {
            return InventoryResult::WrongBagType;
        }

        let Some(bag_storage) = self
            .inventory
            .bags
            .get(bag as usize)
            .and_then(Option::as_ref)
        else {
            return InventoryResult::WrongBagType;
        };

        if source_item.is_some_and(|source| source.object().guid() == bag_storage.bag_guid) {
            return InventoryResult::WrongBagType;
        }

        if let Some(source) = source_item {
            if source_is_not_empty_bag {
                return InventoryResult::DestroyNonemptyBag;
            }

            if source.has_item_flag(ItemFieldFlags::CHILD) {
                return InventoryResult::WrongBagType3;
            }
        }

        let Some(bag_proto) = bag_proto else {
            return InventoryResult::WrongBagType;
        };

        let bag_is_regular_container = bag_proto.class_id == ItemClass::Container
            && bag_proto.subclass_id == ItemSubClassContainer::Container as u32;
        if non_specialized != bag_is_regular_container {
            return InventoryResult::WrongBagType;
        }

        if !item_can_go_into_bag(proto, bag_proto) {
            return InventoryResult::WrongBagType;
        }

        for slot in 0..bag_storage.bag_size {
            if slot == skip_slot {
                continue;
            }

            let existing_item = item_ref_by_pos(slot_items, bag, slot).filter(|existing| {
                source_item.is_none_or(|source| existing.object().guid() != source.object().guid())
            });

            if existing_item.is_some() != merge {
                continue;
            }

            let mut need_space = proto.max_stack_size;
            if let Some(existing_item) = existing_item {
                if existing_item.can_be_merged_partly_with(proto.entry, proto.max_stack_size)
                    != InventoryResult::Ok
                {
                    continue;
                }

                need_space -= existing_item.count();
            }

            need_space = need_space.min(*count);
            let new_position = ItemPosCount::new(make_item_pos(bag, slot), need_space);
            if !new_position.is_contained_in(dest) {
                dest.push(new_position);
                *count -= need_space;

                if *count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        InventoryResult::Ok
    }

    pub fn can_take_more_similar_items(
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

    pub fn item_count_by_entry(
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

    pub fn item_count_with_limit_category(
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

    pub fn item_by_entry<'a>(
        &self,
        entry: u32,
        location: ItemSearchLocation,
        stored_items: &'a [ItemStorageRef<'a>],
    ) -> Option<ItemStorageRef<'a>> {
        let mut result = None;
        self.for_each_item_storage_ref(location, stored_items, |stored| {
            if stored.item.object().entry() == entry {
                result = Some(stored);
                ItemSearchCallbackResult::Stop
            } else {
                ItemSearchCallbackResult::Continue
            }
        });
        result
    }

    pub fn item_list_by_entry<'a>(
        &self,
        entry: u32,
        in_bank_also: bool,
        stored_items: &'a [ItemStorageRef<'a>],
    ) -> Vec<ItemStorageRef<'a>> {
        let mut location = ItemSearchLocation::EQUIPMENT
            | ItemSearchLocation::INVENTORY
            | ItemSearchLocation::REAGENT_BANK;
        if in_bank_also {
            location |= ItemSearchLocation::BANK;
        }

        let mut item_list = Vec::new();
        self.for_each_item_storage_ref(location, stored_items, |stored| {
            if stored.item.object().entry() == entry {
                item_list.push(stored);
            }
            ItemSearchCallbackResult::Continue
        });
        item_list
    }

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
                self.guid(),
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
            .saturating_add(self.active_data.num_backpack_slots)
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
