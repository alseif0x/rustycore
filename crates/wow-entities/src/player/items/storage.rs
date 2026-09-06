// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory and bank storage: bags, slots, stacking and destroy.

use super::super::*;

impl Player {
    pub fn inventory(&self) -> &PlayerInventoryStorage {
        &self.inventory
    }

    pub fn inventory_runtime_like_cpp(&self) -> &PlayerInventoryRuntime {
        &self.inventory_runtime
    }

    pub fn inventory_runtime_mut_like_cpp(&mut self) -> &mut PlayerInventoryRuntime {
        &mut self.inventory_runtime
    }

    /// C++ `Player::GetItemCount`: summarize the canonical carried/bank item
    /// topology without publishing a second mutable count cache.
    pub fn inventory_item_counts_like_cpp(&self) -> HashMap<u32, u32> {
        let inventory_items = self.inventory_runtime.inventory_items();
        let item_objects = self.inventory_runtime.item_objects();
        inventory_items
            .values()
            .filter_map(|inventory_item| item_objects.get(&inventory_item.guid))
            .chain(item_objects.values().filter(|item| {
                !item.container_guid().is_empty()
                    && item_objects.contains_key(&item.container_guid())
            }))
            .filter(|item| !item.is_in_trade())
            .fold(HashMap::new(), |mut counts, item| {
                let entry_id = item.object().entry();
                counts
                    .entry(entry_id)
                    .and_modify(|count| *count = count.saturating_add(item.count()))
                    .or_insert(item.count());
                counts
            })
    }

    /// C++ `Player::GetBankBagSlotCount` (`Player.h:1334`).
    pub const fn bank_bag_slot_count(&self) -> u8 {
        self.data.num_bank_slots
    }

    /// C++ `Player::GetInventorySlotCount` (`Player.h:1332`).
    pub const fn inventory_slot_count(&self) -> u8 {
        self.active_data.num_backpack_slots
    }

    pub fn bank_bag_slot_flag_value_like_cpp(&self, index: usize) -> Option<u32> {
        self.active_data.bank_bag_slot_flags.get(index).copied()
    }

    pub fn soulbound_tradeable_items(&self) -> &HashSet<ObjectGuid> {
        &self.soulbound_tradeable_items
    }

    pub fn item_durations(&self) -> &[ObjectGuid] {
        &self.item_durations
    }

    pub fn set_bank_bag_slot_count(&mut self, count: u8) {
        self.set_player_u8(PLAYER_DATA_NUM_BANK_SLOTS_BIT, count, |data| {
            &mut data.num_bank_slots
        });
    }

    pub fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) {
        self.mark_player_data(PLAYER_DATA_NUM_BANK_SLOTS_BIT);
    }

    pub fn set_bank_bag_slot_flag_value_like_cpp(&mut self, index: usize, value: u32) -> bool {
        if index >= self.active_data.bank_bag_slot_flags.len() {
            return false;
        }

        if self.active_data.bank_bag_slot_flags[index] != value {
            self.active_data.bank_bag_slot_flags[index] = value;
            self.mark_bank_bag_slot_flag_changed_like_cpp(index);
        }
        true
    }

    pub fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) {
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT,
            index,
        );
    }

    pub fn set_visible_item_slot(&mut self, slot: u8, item: Option<VisibleItemValues>) {
        if slot >= EQUIPMENT_SLOT_END {
            return;
        }

        let value = item.unwrap_or_default();
        let target = &mut self.data.visible_items[slot as usize];
        if *target != value {
            *target = value;
            self.mark_player_data_array(
                PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT,
                PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT,
                slot as usize,
            );
        }
    }

    pub fn mark_visible_item_slot_changed(&mut self, slot: u8) {
        if slot >= EQUIPMENT_SLOT_END {
            return;
        }

        self.mark_player_data_array(
            PLAYER_DATA_VISIBLE_ITEMS_PARENT_BIT,
            PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT,
            slot as usize,
        );
    }

    pub fn set_inventory_slot_count(&mut self, count: u8) {
        self.set_active_u8(ACTIVE_PLAYER_DATA_NUM_BACKPACK_SLOTS_BIT, count, |data| {
            &mut data.num_backpack_slots
        });
    }

    pub fn set_inv_slot(&mut self, slot: usize, guid: ObjectGuid) {
        if slot >= PLAYER_SLOT_END || self.active_data.inv_slots[slot] == guid {
            return;
        }

        self.active_data.inv_slots[slot] = guid;
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
            slot,
        );
    }

    pub fn mark_inv_slot_changed(&mut self, slot: usize) {
        if slot >= PLAYER_SLOT_END {
            return;
        }

        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_INV_SLOTS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT,
            slot,
        );
    }

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

    pub fn can_use_item_template(&self, args: CanUseItemTemplateArgs<'_>) -> InventoryResult {
        if args.proto.is_none() {
            return InventoryResult::ItemNotFound;
        }

        if args.internal_item {
            return InventoryResult::CantEquipEver;
        }

        if args.faction_horde && args.team != TEAM_HORDE_ID {
            return InventoryResult::CantEquipEver;
        }

        if args.faction_alliance && args.team != TEAM_ALLIANCE_ID {
            return InventoryResult::CantEquipEver;
        }

        if !args.allowable_class_matches || !args.allowable_race_matches {
            return InventoryResult::CantEquipEver;
        }

        if args.required_skill != 0 {
            if args.required_skill_value == 0 {
                return InventoryResult::ProficiencyNeeded;
            }

            if args.required_skill_value < args.required_skill_rank {
                return InventoryResult::CantEquipSkill;
            }
        }

        if args.required_spell != 0 && !args.has_required_spell {
            return InventoryResult::ProficiencyNeeded;
        }

        if !args.skip_required_level_check && args.player_level < args.base_required_level {
            return InventoryResult::CantEquipLevelI;
        }

        if args.holiday_id != 0 && !args.holiday_active {
            return InventoryResult::ClientLockedOut;
        }

        if args.required_reputation_faction != 0
            && args.player_reputation_rank < args.required_reputation_rank
        {
            return InventoryResult::CantEquipReputation;
        }

        if matches!(args.effect0_spell_id, Some(483 | 55_884))
            && args.effect1_spell_id.is_some()
            && args.has_effect1_spell
        {
            return InventoryResult::InternalBagError;
        }

        if args
            .artifact_specialization
            .is_some_and(|spec| spec != args.primary_specialization)
        {
            return InventoryResult::CantUseItem;
        }

        InventoryResult::Ok
    }

    pub fn can_use_item(&self, mut args: CanUseItemArgs<'_>) -> InventoryResult {
        let Some(source) = args.source_item else {
            return InventoryResult::ItemNotFound;
        };

        if !args.is_alive && args.not_loading {
            return InventoryResult::PlayerDead;
        }

        let Some(proto) = args.proto else {
            return InventoryResult::ItemNotFound;
        };

        if source.is_binded_not_with(self.guid(), proto, args.source_bop_trade_allowed_for_player) {
            return InventoryResult::NotOwner;
        }

        if args.player_level < args.item_required_level {
            return InventoryResult::CantEquipLevelI;
        }

        args.template_args.proto = args.proto;
        args.template_args.skip_required_level_check = true;
        let template_result = self.can_use_item_template(args.template_args);
        if template_result != InventoryResult::Ok {
            return template_result;
        }

        if args.item_skill != 0 {
            let allow_equip = args.proto_is_heirloom
                && proto.class_id == ItemClass::Armor
                && !args.has_item_skill
                && match args.player_class {
                    CLASS_HUNTER | CLASS_SHAMAN => args.item_skill == SKILL_MAIL,
                    CLASS_PALADIN | CLASS_WARRIOR => args.item_skill == SKILL_PLATE_MAIL,
                    _ => false,
                };

            if !allow_equip && args.item_skill_value == 0 {
                return InventoryResult::ProficiencyNeeded;
            }
        }

        InventoryResult::Ok
    }

    pub fn can_bank_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanBankItemArgs<'_>,
    ) -> InventoryResult {
        let Some(source) = args.source_item else {
            return if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            };
        };

        let Some(proto) = args.proto else {
            return if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            };
        };

        if source.loot_generated() {
            return InventoryResult::LootGone;
        }

        if source.is_binded_not_with(self.guid(), proto, args.source_bop_trade_allowed_for_player) {
            return InventoryResult::NotOwner;
        }

        if args.source_is_currency_token {
            return InventoryResult::CantSwap;
        }

        let similar_result = self.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: args.proto,
            count: source.count(),
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
        if similar_result.result != InventoryResult::Ok {
            return similar_result.result;
        }

        let mut count = source.count();

        if args.bag != NULL_BAG && args.slot != NULL_SLOT {
            if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&args.slot) {
                if !args.source_is_bag {
                    return InventoryResult::WrongSlot;
                }

                if args.slot - BANK_SLOT_BAG_START >= self.data.num_bank_slots {
                    return InventoryResult::NoBankSlot;
                }

                if args.can_use_result != InventoryResult::Ok {
                    return args.can_use_result;
                }
            }

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
                return result;
            }

            if count == 0 {
                return InventoryResult::Ok;
            }
        }

        if args.bag != NULL_BAG {
            if args.source_is_not_empty_bag {
                return InventoryResult::BagInBag;
            }

            if proto.max_stack_size != 1 {
                if args.bag == INVENTORY_SLOT_BAG_0 {
                    let result = self.can_store_item_in_inventory_slots(
                        BANK_SLOT_ITEM_START,
                        BANK_SLOT_ITEM_END,
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
                        return result;
                    }
                    if count == 0 {
                        return InventoryResult::Ok;
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
                        return result;
                    }
                    if count == 0 {
                        return InventoryResult::Ok;
                    }
                }
            }

            if args.bag == INVENTORY_SLOT_BAG_0 {
                let result = self.can_store_item_in_inventory_slots(
                    BANK_SLOT_ITEM_START,
                    BANK_SLOT_ITEM_END,
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
                    return result;
                }
                if count == 0 {
                    return InventoryResult::Ok;
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
                    return result;
                }
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        if proto.max_stack_size != 1 {
            let result = self.can_store_item_in_inventory_slots(
                BANK_SLOT_ITEM_START,
                BANK_SLOT_ITEM_END,
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
                return result;
            }
            if count == 0 {
                return InventoryResult::Ok;
            }

            if !proto.bag_family.is_empty() {
                for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
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
                    if count == 0 {
                        return InventoryResult::Ok;
                    }
                }
            }

            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
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
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        if !proto.bag_family.is_empty() {
            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
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
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        let result = self.can_store_item_in_inventory_slots(
            BANK_SLOT_ITEM_START,
            BANK_SLOT_ITEM_END,
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
            return result;
        }
        if count == 0 {
            return InventoryResult::Ok;
        }

        for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
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
            if count == 0 {
                return InventoryResult::Ok;
            }
        }

        InventoryResult::BankFull
    }

    pub fn top_level_item_guid(&self, slot: u8) -> Option<ObjectGuid> {
        self.inventory.items.get(slot as usize).copied().flatten()
    }

    pub fn register_bag_storage(
        &mut self,
        bag_slot: u8,
        bag_guid: ObjectGuid,
        bag_size: u8,
    ) -> Result<(), PlayerStorageError> {
        if !is_bag_storage_slot(bag_slot) {
            return Err(PlayerStorageError::InvalidBagSlot(bag_slot));
        }
        if bag_size as usize > MAX_BAG_SIZE {
            return Err(PlayerStorageError::InvalidBagItemSlot(bag_size));
        }

        self.inventory.bags[bag_slot as usize] = Some(PlayerBagStorage::new(bag_guid, bag_size));
        Ok(())
    }

    pub fn store_top_level_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        self.inventory.items[slot as usize] = Some(guid);
        self.set_inv_slot(slot as usize, guid);
        Ok(())
    }

    pub fn visualize_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
        visible: VisibleItemValues,
    ) -> Result<(), PlayerStorageError> {
        self.store_top_level_item(slot, guid)?;
        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, Some(visible));
        }
        Ok(())
    }

    pub fn visualize_item_object(
        &mut self,
        slot: u8,
        item: &mut Item,
        visible: VisibleItemValues,
    ) -> Result<(), PlayerStorageError> {
        let item_guid = item.object().guid();
        self.store_top_level_item(slot, item_guid)?;

        let owner_guid = self.guid();
        item.bind_if_visualized();
        item.set_contained_in(owner_guid);
        item.set_owner_guid(owner_guid);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);

        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, Some(visible));
        }

        item.set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_item_object(
        &mut self,
        slot: u8,
        item: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        if self.inventory.items[slot as usize].is_some() {
            return Err(PlayerStorageError::OccupiedPlayerSlot(slot));
        }

        let item_guid = item.object().guid();
        self.store_top_level_item(slot, item_guid)?;

        let owner_guid = self.guid();
        item.set_count(count);
        item.bind_if_stored(is_bag_storage_slot(slot));
        item.set_contained_in(owner_guid);
        item.set_owner_guid(owner_guid);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);
        item.set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_cloned_item_object(
        &mut self,
        slot: u8,
        source: &Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        let mut cloned = source.clone_item_for_store(new_guid, Some(self.guid()), count);
        self.store_item_object(slot, &mut cloned, count)?;
        Ok(cloned)
    }

    pub fn split_item_to_empty_top_level_object(
        &mut self,
        slot: u8,
        source: &mut Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        validate_split_source(source, count)?;

        let cloned = self.store_cloned_item_object(slot, source, new_guid, count)?;
        source.set_count(source.count() - count);
        source.set_state(ItemUpdateState::Changed);
        Ok(cloned)
    }

    pub fn merge_top_level_item_stack_object(
        &mut self,
        slot: u8,
        existing: &mut Item,
        incoming: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        let Some(expected_guid) = self.top_level_item_guid(slot) else {
            return Err(PlayerStorageError::EmptyPlayerSlot(slot));
        };

        let actual_guid = existing.object().guid();
        if expected_guid != actual_guid {
            return Err(PlayerStorageError::MismatchedItemGuid {
                slot,
                expected: expected_guid,
                actual: actual_guid,
            });
        }

        existing.bind_if_stored(is_bag_storage_slot(slot));
        existing.set_count(existing.count() + count);
        existing.set_state(ItemUpdateState::Changed);

        let owner_guid = self.guid();
        incoming.set_owner_guid(owner_guid);
        incoming.set_not_refundable();
        incoming.clear_soulbound_tradeable();
        incoming.set_state(ItemUpdateState::Removed);
        Ok(())
    }

    pub fn remove_top_level_item(
        &mut self,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        let removed = self.inventory.items[slot as usize].take();
        self.set_inv_slot(slot as usize, ObjectGuid::EMPTY);
        if slot < EQUIPMENT_SLOT_END {
            self.set_visible_item_slot(slot, None);
        }
        if is_bag_storage_slot(slot) {
            self.inventory.bags[slot as usize] = None;
        }
        Ok(removed)
    }

    pub fn remove_item_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let item_guid = item.object().guid();
        let removed = if bag == INVENTORY_SLOT_BAG_0 {
            let Some(expected_guid) = self.top_level_item_guid(slot) else {
                return Err(PlayerStorageError::EmptyPlayerSlot(slot));
            };
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            if slot < INVENTORY_SLOT_BAG_END {
                item.remove_item_flag2(ItemFieldFlags2::EQUIPPED);
            }

            self.remove_top_level_item(slot)?
        } else {
            let Some(bag_object) = bag_object else {
                return Err(PlayerStorageError::UnknownBag(bag));
            };
            let expected_bag_guid = self
                .get_bag_by_pos(bag)
                .ok_or(PlayerStorageError::UnknownBag(bag))?;
            let actual_bag_guid = bag_object.item().object().guid();
            if expected_bag_guid != actual_bag_guid {
                return Err(PlayerStorageError::MismatchedBagGuid {
                    bag,
                    expected: expected_bag_guid,
                    actual: actual_bag_guid,
                });
            }

            let expected_guid = self
                .inventory
                .bags
                .get(bag as usize)
                .and_then(Option::as_ref)
                .and_then(|bag_storage| bag_storage.item_by_pos(slot))
                .ok_or(PlayerStorageError::EmptyBagItemSlot { bag, slot })?;
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedBagItemGuid {
                    bag,
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            bag_object.remove_item(slot);
            self.remove_bag_item(bag, slot)?
        };

        item.set_contained_in(ObjectGuid::EMPTY);
        item.set_slot(NULL_SLOT);
        item.set_container_guid(ObjectGuid::EMPTY);
        Ok(removed)
    }

    pub fn move_item_from_inventory_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let removed = self.remove_item_object(bag, slot, Some(&mut *item), bag_object)?;
        if removed.is_some() {
            item.set_not_refundable();
        }
        Ok(removed)
    }

    pub fn finalize_move_item_to_inventory_object(
        &self,
        original_item_guid: ObjectGuid,
        last_item: &mut Item,
        in_character_inventory_db: bool,
    ) -> bool {
        if original_item_guid != last_item.object().guid() {
            return false;
        }

        if last_item.owner_guid() != self.guid() {
            last_item.set_owner_guid(self.guid());
        }

        last_item.set_state(if in_character_inventory_db {
            ItemUpdateState::Changed
        } else {
            ItemUpdateState::New
        });
        true
    }

    pub fn destroy_item_object(
        &mut self,
        bag: u8,
        slot: u8,
        item: Option<&mut Item>,
        bag_object: Option<&mut Bag>,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let Some(item) = item else {
            return Ok(None);
        };

        let item_guid = item.object().guid();
        let removed = if bag == INVENTORY_SLOT_BAG_0 {
            let Some(expected_guid) = self.top_level_item_guid(slot) else {
                return Err(PlayerStorageError::EmptyPlayerSlot(slot));
            };
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            self.remove_top_level_item(slot)?
        } else {
            let Some(bag_object) = bag_object else {
                return Err(PlayerStorageError::UnknownBag(bag));
            };
            let expected_bag_guid = self
                .get_bag_by_pos(bag)
                .ok_or(PlayerStorageError::UnknownBag(bag))?;
            let actual_bag_guid = bag_object.item().object().guid();
            if expected_bag_guid != actual_bag_guid {
                return Err(PlayerStorageError::MismatchedBagGuid {
                    bag,
                    expected: expected_bag_guid,
                    actual: actual_bag_guid,
                });
            }

            let expected_guid = self
                .inventory
                .bags
                .get(bag as usize)
                .and_then(Option::as_ref)
                .and_then(|bag_storage| bag_storage.item_by_pos(slot))
                .ok_or(PlayerStorageError::EmptyBagItemSlot { bag, slot })?;
            if expected_guid != item_guid {
                return Err(PlayerStorageError::MismatchedBagItemGuid {
                    bag,
                    slot,
                    expected: expected_guid,
                    actual: item_guid,
                });
            }

            bag_object.remove_item(slot);
            self.remove_bag_item(bag, slot)?
        };

        item.set_not_refundable();
        item.clear_soulbound_tradeable();
        item.set_contained_in(ObjectGuid::EMPTY);
        item.set_slot(NULL_SLOT);
        item.set_container_guid(ObjectGuid::EMPTY);
        item.set_state(ItemUpdateState::Removed);
        Ok(removed)
    }

    pub fn destroy_item_count_for_item_object(
        &mut self,
        item: Option<&mut Item>,
        count: &mut u32,
        bag_object: Option<&mut Bag>,
    ) -> Result<(), PlayerStorageError> {
        let Some(item) = item else {
            return Ok(());
        };

        if item.count() <= *count {
            *count -= item.count();
            let bag = item.bag_slot();
            let slot = item.slot();
            self.destroy_item_object(bag, slot, Some(item), bag_object)?;
        } else {
            item.set_count(item.count() - *count);
            *count = 0;
            item.set_state(ItemUpdateState::Changed);
        }

        Ok(())
    }

    pub fn destroy_item_count_by_entry_plan(
        &self,
        item_entry: u32,
        count: u32,
        unequip_check: bool,
        inventory_slot_count: u8,
        items: &[DestroyItemCountItemRef<'_>],
    ) -> DestroyItemCountPlan {
        let mut plan = DestroyItemCountPlan {
            removed_count: 0,
            actions: Vec::new(),
        };
        if count == 0 {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_ITEM_START,
            BANK_SLOT_ITEM_END,
            false,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_bag_ranges(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            BANK_SLOT_BAG_START,
            BANK_SLOT_BAG_END,
            true,
            unequip_check,
        );
        if plan.removed_count >= count {
            return plan;
        }

        destroy_item_count_scan_top_level_range(
            &mut plan,
            items,
            item_entry,
            count,
            CHILD_EQUIPMENT_SLOT_START,
            CHILD_EQUIPMENT_SLOT_END,
            false,
            unequip_check,
        );

        plan
    }

    pub fn destroy_zone_limited_item_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            KEYRING_SLOT_START,
            KEYRING_SLOT_END,
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }

    pub fn destroy_conjured_items_plan(
        &self,
        inventory_slot_count: u8,
        items: &[DestroyFilteredItemRef],
    ) -> Vec<DestroyFilteredItemAction> {
        let mut actions = Vec::new();
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            INVENTORY_SLOT_ITEM_START,
            INVENTORY_SLOT_ITEM_START.saturating_add(inventory_slot_count),
        );
        destroy_filtered_scan_bag_ranges(
            &mut actions,
            items,
            INVENTORY_SLOT_BAG_START,
            INVENTORY_SLOT_BAG_END,
        );
        destroy_filtered_scan_top_level_range(
            &mut actions,
            items,
            EQUIPMENT_SLOT_HEAD,
            INVENTORY_SLOT_BAG_END,
        );
        actions
    }

    pub fn swap_item_preflight_plan(
        &self,
        src: u16,
        dst: u16,
        is_alive: bool,
        src_item: Option<SwapItemPreflightItem>,
        dst_item: Option<SwapItemPreflightItem>,
    ) -> SwapItemPreflightPlan {
        let Some(src_item) = src_item else {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::NoSource,
                src_unequip_swap: None,
                dst_unequip_swap: None,
            };
        };

        if src_item.is_child {
            if let Some(parent_pos) = src_item.parent_pos {
                if is_equipment_packed_pos(src) {
                    return SwapItemPreflightPlan {
                        result: SwapItemPreflightResult::ChildRedirect {
                            first_src: dst,
                            first_dst: src,
                            second_src: parent_pos,
                            second_dst: dst,
                        },
                        src_unequip_swap: None,
                        dst_unequip_swap: None,
                    };
                }
            }
        } else if let Some(dst_item) = dst_item {
            if dst_item.is_child {
                if let Some(parent_pos) = dst_item.parent_pos {
                    if is_equipment_packed_pos(dst) {
                        return SwapItemPreflightPlan {
                            result: SwapItemPreflightResult::ChildRedirect {
                                first_src: src,
                                first_dst: dst,
                                second_src: parent_pos,
                                second_dst: src,
                            },
                            src_unequip_swap: None,
                            dst_unequip_swap: None,
                        };
                    }
                }
            }
        }

        if !is_alive {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
                src_unequip_swap: None,
                dst_unequip_swap: None,
            };
        }

        let mut src_unequip_swap = None;
        if is_equipment_packed_pos(src) || is_bag_pos(src) {
            let swap = !is_bag_pos(src)
                || is_bag_pos(dst)
                || dst_item.is_some_and(|item| item.is_bag && item.is_empty_bag);
            src_unequip_swap = Some(swap);
            if src_item.can_unequip_result != InventoryResult::Ok {
                return SwapItemPreflightPlan {
                    result: SwapItemPreflightResult::Error(src_item.can_unequip_result),
                    src_unequip_swap,
                    dst_unequip_swap: None,
                };
            }
        }

        let [_src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, _dst_slot] = dst.to_be_bytes();
        if is_bag_pos(src) && src_slot == dst_bag {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::BagInBag),
                src_unequip_swap,
                dst_unequip_swap: None,
            };
        }

        let [src_bag, _src_slot] = src.to_be_bytes();
        let [_dst_bag, dst_slot] = dst.to_be_bytes();
        if is_bag_pos(dst) && src_bag == dst_slot {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::CantSwap),
                src_unequip_swap,
                dst_unequip_swap: None,
            };
        }

        let mut dst_unequip_swap = None;
        if let Some(dst_item) = dst_item {
            if is_equipment_packed_pos(dst) || is_bag_pos(dst) {
                let swap = !is_bag_pos(dst)
                    || is_bag_pos(src)
                    || (src_item.is_bag && src_item.is_empty_bag);
                dst_unequip_swap = Some(swap);
                if dst_item.can_unequip_result != InventoryResult::Ok {
                    return SwapItemPreflightPlan {
                        result: SwapItemPreflightResult::Error(dst_item.can_unequip_result),
                        src_unequip_swap,
                        dst_unequip_swap,
                    };
                }
            }
        }

        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Continue,
            src_unequip_swap,
            dst_unequip_swap,
        }
    }

    pub fn swap_item_empty_destination_plan(
        &self,
        src: u16,
        dst: u16,
        dst_item_present: bool,
        can_store_result: InventoryResult,
        can_bank_result: InventoryResult,
        can_equip_result: InventoryResult,
        equip_dest: u16,
    ) -> SwapItemEmptyDestinationPlan {
        if dst_item_present {
            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::OccupiedDestination,
            };
        }

        if is_inventory_packed_pos(dst) {
            if can_store_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_store_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::MoveToInventory {
                    quest_added_from_bank: is_bank_packed_pos(src),
                },
            };
        }

        if is_bank_packed_pos(dst) {
            if can_bank_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_bank_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::MoveToBank {
                    quest_removed: true,
                },
            };
        }

        if is_equipment_packed_pos(dst) {
            if can_equip_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_equip_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::Equip {
                    dest: equip_dest,
                    auto_unequip_offhand: true,
                },
            };
        }

        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::InvalidDestinationNoop,
        }
    }

    pub fn swap_item_merge_fill_plan(
        &self,
        dst: u16,
        source_is_bag: bool,
        destination_is_bag: bool,
        source_count: u32,
        destination_count: u32,
        source_max_stack_size: u32,
        can_store_result: InventoryResult,
        can_bank_result: InventoryResult,
        can_equip_result: InventoryResult,
        equip_dest: u16,
        is_in_world: bool,
    ) -> SwapItemMergeFillPlan {
        if source_is_bag || destination_is_bag {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::ContinueToRealSwap,
                send_refund_info: false,
            };
        }

        let destination_kind = if is_inventory_packed_pos(dst) {
            Some((
                can_store_result,
                SwapItemMergeFillResult::MoveMergedStackToInventory,
            ))
        } else if is_bank_packed_pos(dst) {
            Some((
                can_bank_result,
                SwapItemMergeFillResult::MoveMergedStackToBank,
            ))
        } else if is_equipment_packed_pos(dst) {
            Some((
                can_equip_result,
                SwapItemMergeFillResult::EquipMergedStack {
                    dest: equip_dest,
                    auto_unequip_offhand: true,
                },
            ))
        } else {
            None
        };

        let Some((validation_result, move_result)) = destination_kind else {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::InvalidDestinationNoop,
                send_refund_info: false,
            };
        };

        if validation_result != InventoryResult::Ok {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::ContinueToRealSwap,
                send_refund_info: false,
            };
        }

        if source_count.saturating_add(destination_count) <= source_max_stack_size {
            return SwapItemMergeFillPlan {
                result: move_result,
                send_refund_info: true,
            };
        }

        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::PartialFill {
                source_remaining_count: source_count
                    .saturating_add(destination_count)
                    .saturating_sub(source_max_stack_size),
                destination_count: source_max_stack_size,
                send_updates: is_in_world,
            },
            send_refund_info: true,
        }
    }

    pub fn swap_item_real_swap_validation_plan(
        &self,
        src: u16,
        dst: u16,
        source_can_store_result: InventoryResult,
        source_can_bank_result: InventoryResult,
        source_can_equip_result: InventoryResult,
        source_equip_dest: u16,
        source_equip_dest_can_unequip_result: InventoryResult,
        destination_can_store_result: InventoryResult,
        destination_can_bank_result: InventoryResult,
        destination_can_equip_result: InventoryResult,
        destination_equip_dest: u16,
        destination_equip_dest_can_unequip_result: InventoryResult,
    ) -> SwapItemRealSwapValidationPlan {
        let (source_result, source_target) = swap_item_real_swap_target_for_destination(
            dst,
            source_can_store_result,
            source_can_bank_result,
            source_can_equip_result,
            source_equip_dest,
            source_equip_dest_can_unequip_result,
        );
        if source_result != InventoryResult::Ok {
            return SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: source_result,
                    subject: SwapItemRealSwapValidationSubject::Source,
                },
            };
        }

        let (destination_result, destination_target) = swap_item_real_swap_target_for_destination(
            src,
            destination_can_store_result,
            destination_can_bank_result,
            destination_can_equip_result,
            destination_equip_dest,
            destination_equip_dest_can_unequip_result,
        );
        if destination_result != InventoryResult::Ok {
            return SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: destination_result,
                    subject: SwapItemRealSwapValidationSubject::Destination,
                },
            };
        }

        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target,
                destination_target,
            },
        }
    }

    pub fn swap_item_bag_exchange_plan(
        &self,
        src: u16,
        dst: u16,
        source_bag: Option<SwapBagRef<'_>>,
        destination_bag: Option<SwapBagRef<'_>>,
    ) -> SwapItemBagExchangePlan {
        let (Some(source_bag), Some(destination_bag)) = (source_bag, destination_bag) else {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            };
        };

        let Some((empty_bag_is_source, empty_bag, full_bag)) =
            (if source_bag.is_empty && !is_bag_pos(src) {
                Some((true, source_bag, destination_bag))
            } else if destination_bag.is_empty && !is_bag_pos(dst) {
                Some((false, destination_bag, source_bag))
            } else {
                None
            })
        else {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            };
        };

        let mut count = 0u8;
        for slot in 0..full_bag.bag_size {
            if let Some(item_ref) = full_bag.items.iter().find(|item| item.slot == slot) {
                if !item_ref.can_go_into_empty_bag {
                    return SwapItemBagExchangePlan {
                        result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
                    };
                }
                count = count.saturating_add(1);
            }
        }

        if count > empty_bag.bag_size {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Error(InventoryResult::CantSwap),
            };
        }

        let mut moves = Vec::new();
        let mut to_slot = 0u8;
        for slot in 0..full_bag.bag_size {
            if full_bag.items.iter().any(|item| item.slot == slot) {
                moves.push(SwapBagItemMove {
                    from_slot: slot,
                    to_slot,
                });
                to_slot = to_slot.saturating_add(1);
            }
        }

        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source,
                moves,
            },
        }
    }

    pub fn swap_item_real_swap_execution_plan(
        &self,
        src: u16,
        dst: u16,
        source_target: SwapItemRealSwapTarget,
        destination_target: SwapItemRealSwapTarget,
        ae_loot_view_not_empty: bool,
        source_bag_has_looted_item: bool,
        destination_bag_has_looted_item: bool,
    ) -> SwapItemRealSwapExecutionPlan {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let apply_item_dependent_auras = (src_bag == INVENTORY_SLOT_BAG_0
            && src_slot < INVENTORY_SLOT_BAG_END)
            || (dst_bag == INVENTORY_SLOT_BAG_0 && dst_slot < INVENTORY_SLOT_BAG_END);
        let release_loot = ae_loot_view_not_empty
            && ((is_bag_pos(src) && source_bag_has_looted_item)
                || (is_bag_pos(dst) && destination_bag_has_looted_item));

        SwapItemRealSwapExecutionPlan {
            remove_destination_update: false,
            remove_source_update: false,
            source_target,
            destination_target,
            apply_item_dependent_auras,
            release_loot,
            auto_unequip_offhand: true,
        }
    }

    pub fn swap_item_orchestration_plan(
        &self,
        preflight: SwapItemPreflightPlan,
        empty_destination: Option<SwapItemEmptyDestinationPlan>,
        merge_fill: Option<SwapItemMergeFillPlan>,
        real_swap_validation: Option<SwapItemRealSwapValidationPlan>,
        bag_exchange: Option<SwapItemBagExchangePlan>,
        real_swap_execution: Option<SwapItemRealSwapExecutionPlan>,
    ) -> SwapItemOrchestrationPlan {
        match preflight.result {
            SwapItemPreflightResult::NoSource => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::NoSource,
                };
            }
            SwapItemPreflightResult::ChildRedirect {
                first_src,
                first_dst,
                second_src,
                second_dst,
            } => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::ChildRedirect {
                        first_src,
                        first_dst,
                        second_src,
                        second_dst,
                    },
                };
            }
            SwapItemPreflightResult::Error(result) => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error {
                        result,
                        item_order: SwapItemErrorItemOrder::SourceDestination,
                    },
                };
            }
            SwapItemPreflightResult::Continue => {}
        }

        let Some(empty_destination) = empty_destination else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::EmptyDestination,
                ),
            };
        };
        match empty_destination.result {
            SwapItemEmptyDestinationResult::OccupiedDestination => {}
            SwapItemEmptyDestinationResult::Error(result) => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error {
                        result,
                        item_order: SwapItemErrorItemOrder::SourceOnly,
                    },
                };
            }
            _ => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::EmptyDestination(empty_destination),
                };
            }
        }

        let Some(merge_fill) = merge_fill else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(SwapItemMissingPhase::MergeFill),
            };
        };
        if merge_fill.result != SwapItemMergeFillResult::ContinueToRealSwap {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MergeFill(merge_fill),
            };
        }

        let Some(real_swap_validation) = real_swap_validation else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::RealSwapValidation,
                ),
            };
        };
        let (source_target, destination_target) = match real_swap_validation.result {
            SwapItemRealSwapValidationResult::Error { result, subject } => {
                let item_order = match subject {
                    SwapItemRealSwapValidationSubject::Source => {
                        SwapItemErrorItemOrder::SourceDestination
                    }
                    SwapItemRealSwapValidationSubject::Destination => {
                        SwapItemErrorItemOrder::DestinationSource
                    }
                };

                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error { result, item_order },
                };
            }
            SwapItemRealSwapValidationResult::Continue {
                source_target,
                destination_target,
            } => (source_target, destination_target),
        };

        let Some(bag_exchange) = bag_exchange else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::BagExchange,
                ),
            };
        };
        if let SwapItemBagExchangeResult::Error(result) = &bag_exchange.result {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::Error {
                    result: *result,
                    item_order: SwapItemErrorItemOrder::SourceDestination,
                },
            };
        }

        let Some(real_swap_execution) = real_swap_execution else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::RealSwapExecution,
                ),
            };
        };
        if real_swap_execution.source_target != source_target
            || real_swap_execution.destination_target != destination_target
        {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::InconsistentRealSwapTargets {
                    validation_source_target: source_target,
                    validation_destination_target: destination_target,
                    execution_source_target: real_swap_execution.source_target,
                    execution_destination_target: real_swap_execution.destination_target,
                },
            };
        }

        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::RealSwap {
                bag_exchange,
                execution: real_swap_execution,
            },
        }
    }

    pub fn store_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        let bag_storage = self
            .inventory
            .bags
            .get_mut(bag as usize)
            .and_then(Option::as_mut)
            .ok_or(PlayerStorageError::UnknownBag(bag))?;
        if slot as usize >= MAX_BAG_SIZE || slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(slot));
        }

        bag_storage.set_item(slot, Some(guid));
        Ok(())
    }

    pub fn store_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        item: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        let bag_guid = bag.item().object().guid();
        let bag_storage = self
            .inventory
            .bags
            .get(bag_slot as usize)
            .and_then(Option::as_ref)
            .ok_or(PlayerStorageError::UnknownBag(bag_slot))?;

        if bag_storage.bag_guid != bag_guid {
            return Err(PlayerStorageError::MismatchedBagGuid {
                bag: bag_slot,
                expected: bag_storage.bag_guid,
                actual: bag_guid,
            });
        }

        if item_slot as usize >= MAX_BAG_SIZE || item_slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(item_slot));
        }

        if bag_storage.item_by_pos(item_slot).is_some() {
            return Err(PlayerStorageError::OccupiedBagItemSlot {
                bag: bag_slot,
                slot: item_slot,
            });
        }

        item.set_count(count);
        item.bind_if_stored(false);
        bag.store_item(item_slot, item);
        self.store_bag_item(bag_slot, item_slot, item.object().guid())?;
        item.set_state(ItemUpdateState::Changed);
        bag.item_mut().set_state(ItemUpdateState::Changed);
        Ok(())
    }

    pub fn store_cloned_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        source: &Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        let mut cloned = source.clone_item_for_store(new_guid, Some(self.guid()), count);
        self.store_bag_item_object(bag_slot, bag, item_slot, &mut cloned, count)?;
        Ok(cloned)
    }

    pub fn split_item_to_empty_bag_item_object(
        &mut self,
        bag_slot: u8,
        bag: &mut Bag,
        item_slot: u8,
        source: &mut Item,
        new_guid: ObjectGuid,
        count: u32,
    ) -> Result<Item, PlayerStorageError> {
        validate_split_source(source, count)?;

        let cloned =
            self.store_cloned_bag_item_object(bag_slot, bag, item_slot, source, new_guid, count)?;
        source.set_count(source.count() - count);
        source.set_state(ItemUpdateState::Changed);
        Ok(cloned)
    }

    pub fn merge_bag_item_stack_object(
        &mut self,
        bag_slot: u8,
        bag: &Bag,
        item_slot: u8,
        existing: &mut Item,
        incoming: &mut Item,
        count: u32,
    ) -> Result<(), PlayerStorageError> {
        let bag_guid = bag.item().object().guid();
        let bag_storage = self
            .inventory
            .bags
            .get(bag_slot as usize)
            .and_then(Option::as_ref)
            .ok_or(PlayerStorageError::UnknownBag(bag_slot))?;

        if bag_storage.bag_guid != bag_guid {
            return Err(PlayerStorageError::MismatchedBagGuid {
                bag: bag_slot,
                expected: bag_storage.bag_guid,
                actual: bag_guid,
            });
        }

        if item_slot as usize >= MAX_BAG_SIZE || item_slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(item_slot));
        }

        let Some(expected_guid) = bag_storage.item_by_pos(item_slot) else {
            return Err(PlayerStorageError::EmptyBagItemSlot {
                bag: bag_slot,
                slot: item_slot,
            });
        };

        let bag_slot_guid = bag.item_by_pos(item_slot).unwrap_or(ObjectGuid::EMPTY);
        if bag_slot_guid != expected_guid {
            return Err(PlayerStorageError::MismatchedBagItemGuid {
                bag: bag_slot,
                slot: item_slot,
                expected: expected_guid,
                actual: bag_slot_guid,
            });
        }

        let actual_guid = existing.object().guid();
        if expected_guid != actual_guid {
            return Err(PlayerStorageError::MismatchedBagItemGuid {
                bag: bag_slot,
                slot: item_slot,
                expected: expected_guid,
                actual: actual_guid,
            });
        }

        existing.bind_if_stored(false);
        existing.set_count(existing.count() + count);
        existing.set_state(ItemUpdateState::Changed);

        let owner_guid = self.guid();
        incoming.set_owner_guid(owner_guid);
        incoming.set_not_refundable();
        incoming.clear_soulbound_tradeable();
        incoming.set_state(ItemUpdateState::Removed);
        Ok(())
    }

    pub fn remove_bag_item(
        &mut self,
        bag: u8,
        slot: u8,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        let bag_storage = self
            .inventory
            .bags
            .get_mut(bag as usize)
            .and_then(Option::as_mut)
            .ok_or(PlayerStorageError::UnknownBag(bag))?;
        if slot as usize >= MAX_BAG_SIZE || slot >= bag_storage.bag_size {
            return Err(PlayerStorageError::InvalidBagItemSlot(slot));
        }

        let removed = bag_storage.item_by_pos(slot);
        bag_storage.set_item(slot, None);
        Ok(removed)
    }

    pub fn get_bag_by_pos(&self, bag: u8) -> Option<ObjectGuid> {
        if is_bag_storage_slot(bag) {
            self.inventory.bags[bag as usize].map(|bag| bag.bag_guid)
        } else {
            None
        }
    }

    pub fn get_item_by_pos(&self, bag: u8, slot: u8) -> Option<ObjectGuid> {
        if bag == INVENTORY_SLOT_BAG_0
            && (slot as usize) < PLAYER_SLOT_END
            && !is_buyback_slot(slot)
        {
            return self.inventory.items[slot as usize];
        }

        self.inventory
            .bags
            .get(bag as usize)
            .and_then(|bag| bag.as_ref())
            .and_then(|bag| bag.item_by_pos(slot))
    }

    pub fn get_item_by_packed_pos(&self, pos: u16) -> Option<ObjectGuid> {
        self.get_item_by_pos((pos >> 8) as u8, (pos & 0xFF) as u8)
    }

    pub fn get_item_by_guid(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let mut found = false;
        self.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |item_guid| {
            if item_guid == guid {
                found = true;
                ItemSearchCallbackResult::Stop
            } else {
                ItemSearchCallbackResult::Continue
            }
        });

        found.then_some(guid)
    }

    pub fn for_each_item_guid(
        &self,
        location: ItemSearchLocation,
        mut callback: impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        if location.contains(ItemSearchLocation::EQUIPMENT) {
            for slot in 0..EQUIPMENT_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in PROFESSION_SLOT_START..PROFESSION_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::INVENTORY) {
            let inventory_end = INVENTORY_SLOT_ITEM_START
                .saturating_add(self.active_data.num_backpack_slots)
                .min(INVENTORY_SLOT_ITEM_END);
            for slot in INVENTORY_SLOT_BAG_START..inventory_end {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in KEYRING_SLOT_START..KEYRING_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for slot in CHILD_EQUIPMENT_SLOT_START..CHILD_EQUIPMENT_SLOT_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for bag_slot in INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::BANK) {
            for slot in BANK_SLOT_ITEM_START..BANK_SLOT_BAG_END {
                if self.visit_top_slot(slot, &mut callback) {
                    return false;
                }
            }
            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        if location.contains(ItemSearchLocation::REAGENT_BANK) {
            for bag_slot in REAGENT_BAG_SLOT_START..REAGENT_BAG_SLOT_END {
                if self.visit_bag_items(bag_slot, &mut callback) {
                    return false;
                }
            }
        }

        true
    }

    pub fn for_each_item_storage_ref<'a>(
        &self,
        location: ItemSearchLocation,
        stored_items: &'a [ItemStorageRef<'a>],
        mut callback: impl FnMut(ItemStorageRef<'a>) -> ItemSearchCallbackResult,
    ) -> bool {
        self.for_each_item_guid(location, |guid| {
            if let Some(stored) = item_storage_ref_by_guid(stored_items, guid) {
                callback(stored)
            } else {
                ItemSearchCallbackResult::Continue
            }
        })
    }

    pub fn get_item_from_buyback_slot(&self, slot: u8) -> Option<ObjectGuid> {
        if is_buyback_slot(slot) {
            self.inventory.items[slot as usize]
        } else {
            None
        }
    }

    pub fn remove_item_from_buyback_slot(&mut self, slot: u8) -> Option<ObjectGuid> {
        if !is_buyback_slot(slot) {
            return None;
        }

        let removed = self.inventory.items[slot as usize].take();
        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        self.set_inv_slot(slot as usize, ObjectGuid::EMPTY);
        self.set_buyback_price(buyback_index, 0);
        self.set_buyback_timestamp(buyback_index, 0);
        if self.inventory.items[self.inventory.current_buyback_slot as usize].is_some() {
            self.inventory.current_buyback_slot = slot;
        }
        removed
    }

    pub fn remove_item_from_buyback_slot_object(
        &mut self,
        slot: u8,
        item: Option<&mut Item>,
        delete_item: bool,
    ) -> Result<Option<ObjectGuid>, PlayerStorageError> {
        if !is_buyback_slot(slot) {
            return Ok(None);
        }

        let stored_guid = self.inventory.items[slot as usize];
        let mut item = item;
        if let (Some(expected), Some(actual_item)) = (stored_guid, item.as_deref()) {
            let actual = actual_item.object().guid();
            if expected != actual {
                return Err(PlayerStorageError::MismatchedItemGuid {
                    slot,
                    expected,
                    actual,
                });
            }
        }

        if stored_guid.is_some() {
            if let Some(item) = item.as_deref_mut() {
                item.object_mut().remove_from_world();
                if delete_item {
                    item.set_state(ItemUpdateState::Removed);
                }
            }
        }

        Ok(self.remove_item_from_buyback_slot(slot))
    }

    pub fn add_item_to_buyback_slot(&mut self, guid: ObjectGuid, price: u32, timestamp: i64) -> u8 {
        let mut slot = self.inventory.current_buyback_slot;
        if self.inventory.items[slot as usize].is_some() {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = self.active_data.buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if self.inventory.items[candidate as usize].is_none() {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = self.active_data.buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }
            slot = oldest_slot;
        }

        self.remove_item_from_buyback_slot(slot);
        self.inventory.items[slot as usize] = Some(guid);
        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        self.set_inv_slot(slot as usize, guid);
        self.set_buyback_price(buyback_index, price);
        self.set_buyback_timestamp(buyback_index, timestamp);

        if self.inventory.current_buyback_slot < BUYBACK_SLOT_END - 1 {
            self.inventory.current_buyback_slot += 1;
        }

        slot
    }

    pub fn add_item_to_buyback_slot_object(
        &mut self,
        item: &Item,
        item_template: Option<&ItemStorageTemplate>,
        game_time: i64,
        login_time: i64,
        overwritten_item: Option<&mut Item>,
    ) -> Result<u8, PlayerStorageError> {
        let mut slot = self.inventory.current_buyback_slot;
        if self.inventory.items[slot as usize].is_some() {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = self.active_data.buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if self.inventory.items[candidate as usize].is_none() {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = self.active_data.buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }
            slot = oldest_slot;
        }

        self.remove_item_from_buyback_slot_object(slot, overwritten_item, true)?;

        let buyback_index = (slot - BUYBACK_SLOT_START) as usize;
        let price = item_template
            .map(|proto| proto.sell_price.wrapping_mul(item.count()))
            .unwrap_or(0);
        let timestamp = (game_time - login_time + (30 * 3600)) as u32 as i64;

        self.inventory.items[slot as usize] = Some(item.object().guid());
        self.set_inv_slot(slot as usize, item.object().guid());
        self.set_buyback_price(buyback_index, price);
        self.set_buyback_timestamp(buyback_index, timestamp);

        if self.inventory.current_buyback_slot < BUYBACK_SLOT_END - 1 {
            self.inventory.current_buyback_slot += 1;
        }

        Ok(slot)
    }

    pub fn add_tradeable_item(&mut self, item: &Item) {
        self.soulbound_tradeable_items.insert(item.object().guid());
    }

    pub fn remove_tradeable_item(&mut self, item: &Item) {
        self.soulbound_tradeable_items.remove(&item.object().guid());
    }

    pub fn update_soulbound_trade_items(
        &mut self,
        items: &[SoulboundTradeableItemRef],
    ) -> Vec<ObjectGuid> {
        let player_guid = self.guid();
        let mut removed = Vec::new();
        self.soulbound_tradeable_items.retain(|guid| {
            let keep = items.iter().any(|item| {
                item.guid == *guid && item.owner_guid == player_guid && !item.trade_expired
            });
            if !keep {
                removed.push(*guid);
            }
            keep
        });
        removed
    }

    pub fn add_item_durations(&mut self, item: &Item) -> Option<PlayerItemTimeUpdate> {
        let expiration = item.data().expiration;
        if expiration == 0 {
            return None;
        }

        let item_guid = item.object().guid();
        self.item_durations.push(item_guid);
        Some(PlayerItemTimeUpdate {
            item_guid,
            expiration,
        })
    }

    pub fn remove_item_durations(&mut self, item: &Item) -> bool {
        let item_guid = item.object().guid();
        if let Some(index) = self
            .item_durations
            .iter()
            .position(|stored_guid| *stored_guid == item_guid)
        {
            self.item_durations.remove(index);
            true
        } else {
            false
        }
    }

    pub fn update_item_duration_plan(
        &self,
        items: &[ItemDurationRef],
        time: u32,
        realtime_only: bool,
    ) -> Vec<UpdateItemDurationAction> {
        let mut actions = Vec::new();
        for item_guid in &self.item_durations {
            if let Some(item) = items.iter().find(|item| item.guid == *item_guid) {
                if realtime_only && !item.real_duration {
                    continue;
                }
                if item.expiration == 0 {
                    continue;
                }
                if item.expiration <= time {
                    actions.push(UpdateItemDurationAction::Expire {
                        item_guid: *item_guid,
                    });
                } else {
                    actions.push(UpdateItemDurationAction::UpdateExpiration {
                        item_guid: *item_guid,
                        expiration: item.expiration - time,
                    });
                }
            } else {
                actions.push(UpdateItemDurationAction::MissingItem {
                    item_guid: *item_guid,
                });
            }
        }
        actions
    }

    pub fn send_item_durations_plan(&self, items: &[ItemDurationRef]) -> Vec<PlayerItemTimeUpdate> {
        self.item_durations
            .iter()
            .filter_map(|item_guid| {
                items
                    .iter()
                    .find(|item| item.guid == *item_guid)
                    .map(|item| PlayerItemTimeUpdate {
                        item_guid: *item_guid,
                        expiration: item.expiration,
                    })
            })
            .collect()
    }

    pub fn send_new_item_plan(
        &self,
        item: Option<&Item>,
        template: SendNewItemTemplateRef,
        args: SendNewItemArgs,
    ) -> Option<SendNewItemPlan> {
        let item = item?;
        let battle_pet_breed_data = item.get_modifier(ItemModifier::BattlePetBreedData);
        let is_encounter_loot = args.dungeon_encounter_id != 0;
        let delivery =
            if args.broadcast && args.player_in_group && !template.dont_report_loot_log_to_party {
                SendNewItemDelivery::GroupBroadcast
            } else {
                SendNewItemDelivery::Direct
            };
        let modifications = item
            .data()
            .modifiers
            .iter()
            .enumerate()
            .filter_map(|(modifier_type, &value)| {
                (value != 0).then_some(SendNewItemModifier {
                    value: value as i32,
                    modifier_type: modifier_type as u8,
                })
            })
            .collect();

        Some(SendNewItemPlan {
            player_guid: self.guid(),
            item_guid: item.object().guid(),
            item_entry: item.object().entry(),
            item_instance: SendNewItemInstancePlan {
                item_id: item.object().entry(),
                random_properties_seed: item.data().property_seed,
                random_properties_id: item.data().random_properties_id,
                modifications,
            },
            slot: item.bag_slot(),
            slot_in_bag: if item.count() == args.quantity {
                i16::from(item.slot())
            } else {
                -1
            },
            quest_log_item_id: template.quest_log_item_id,
            quantity: args.quantity,
            quantity_in_inventory: args.quantity_in_inventory,
            battle_pet_species_id: item.get_modifier(ItemModifier::BattlePetSpeciesId),
            battle_pet_breed_id: battle_pet_breed_data & 0x00FF_FFFF,
            battle_pet_breed_quality: ((battle_pet_breed_data >> 24) & 0xFF) as u8,
            battle_pet_level: item.get_modifier(ItemModifier::BattlePetLevel),
            pushed: args.pushed,
            created: args.created,
            display_text: if is_encounter_loot {
                SendNewItemDisplayText::EncounterLoot
            } else {
                SendNewItemDisplayText::Normal
            },
            dungeon_encounter_id: args.dungeon_encounter_id,
            is_encounter_loot,
            delivery,
        })
    }

    fn visit_top_slot(
        &self,
        slot: u8,
        callback: &mut impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        self.inventory.items[slot as usize]
            .map(|guid| matches!(callback(guid), ItemSearchCallbackResult::Stop))
            .unwrap_or(false)
    }

    fn visit_bag_items(
        &self,
        bag_slot: u8,
        callback: &mut impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        let Some(bag) = self.inventory.bags[bag_slot as usize] else {
            return false;
        };

        bag.slots
            .iter()
            .take(bag.bag_size as usize)
            .filter_map(|guid| *guid)
            .any(|guid| matches!(callback(guid), ItemSearchCallbackResult::Stop))
    }
}
