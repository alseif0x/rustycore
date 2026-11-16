// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory and bank storage: bags, slots, stacking and destroy.

mod planning;
mod projection;
mod topology;
pub use planning::ItemStorageView;
pub use projection::InventoryPlayerSnapshotLikeCpp;

use super::super::*;

impl Player {
    fn item_storage_view(&self) -> ItemStorageView<'_> {
        ItemStorageView::new(
            self.guid(),
            &self.inventory,
            self.active_data.num_backpack_slots,
        )
    }

    pub fn is_valid_pos(&self, bag: u8, slot: u8, explicit_pos: bool) -> bool {
        self.item_storage_view()
            .is_valid_pos(bag, slot, explicit_pos)
    }

    pub fn is_valid_packed_pos(&self, pos: u16, explicit_pos: bool) -> bool {
        self.item_storage_view()
            .is_valid_packed_pos(pos, explicit_pos)
    }

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
        let update_requested = self.inventory_values_mut().set_bank_bag_slot_count(count);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) {
        let update_requested = self
            .inventory_values_mut()
            .mark_bank_bag_slot_count_changed_like_cpp();
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn set_bank_bag_slot_flag_value_like_cpp(&mut self, index: usize, value: u32) -> bool {
        let update_request = self
            .inventory_values_mut()
            .set_bank_bag_slot_flag_value_like_cpp(index, value);
        self.apply_inventory_value_update_request(update_request.unwrap_or(false));
        update_request.is_some()
    }

    pub fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) {
        let update_requested = self
            .inventory_values_mut()
            .mark_bank_bag_slot_flag_changed_like_cpp(index);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn set_visible_item_slot(&mut self, slot: u8, item: Option<VisibleItemValues>) {
        let update_requested = self
            .inventory_values_mut()
            .set_visible_item_slot(slot, item);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn mark_visible_item_slot_changed(&mut self, slot: u8) {
        let update_requested = self
            .inventory_values_mut()
            .mark_visible_item_slot_changed(slot);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn set_inventory_slot_count(&mut self, count: u8) {
        let update_requested = self.inventory_values_mut().set_inventory_slot_count(count);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn set_inv_slot(&mut self, slot: usize, guid: ObjectGuid) {
        let update_requested = self.inventory_values_mut().set_inv_slot(slot, guid);
        self.apply_inventory_value_update_request(update_requested);
    }

    pub fn mark_inv_slot_changed(&mut self, slot: usize) {
        let update_requested = self.inventory_values_mut().mark_inv_slot_changed(slot);
        self.apply_inventory_value_update_request(update_requested);
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
        self.item_storage_view().can_store_item_in_specific_slot(
            bag,
            slot,
            dest,
            proto,
            count,
            swap,
            existing_item,
            source_item,
            source_is_not_empty_bag,
            bag_proto,
        )
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
        self.item_storage_view().can_store_item_in_inventory_slots(
            slot_begin,
            slot_end,
            dest,
            proto,
            count,
            merge,
            source_item,
            source_is_not_empty_bag,
            skip_bag,
            skip_slot,
            slot_items,
        )
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
        self.item_storage_view().can_store_item_in_bag(
            bag,
            dest,
            proto,
            count,
            merge,
            non_specialized,
            source_item,
            source_is_not_empty_bag,
            skip_bag,
            skip_slot,
            bag_proto,
            slot_items,
        )
    }

    pub fn can_take_more_similar_items(
        &self,
        args: CanTakeMoreSimilarItemsArgs<'_>,
    ) -> CanTakeMoreSimilarItemsOutcome {
        self.item_storage_view().can_take_more_similar_items(args)
    }

    pub fn item_count_by_entry(
        &self,
        entry: u32,
        in_bank_also: bool,
        skip_item: Option<&Item>,
        stored_items: &[ItemStorageRef<'_>],
    ) -> u32 {
        self.item_storage_view().item_count_by_entry(
            entry,
            in_bank_also,
            skip_item.map(Item::read_view),
            stored_items,
        )
    }

    pub fn item_count_with_limit_category(
        &self,
        limit_category: u32,
        skip_item: Option<&Item>,
        stored_items: &[ItemStorageRef<'_>],
    ) -> u32 {
        self.item_storage_view().item_count_with_limit_category(
            limit_category,
            skip_item.map(Item::read_view),
            stored_items,
        )
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
        self.item_storage_view().can_store_item(dest, args)
    }

    pub fn can_use_item_template(&self, args: CanUseItemTemplateArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_use_item_template(args)
    }

    pub fn can_use_item(&self, args: CanUseItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_use_item(args)
    }

    pub fn can_bank_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanBankItemArgs<'_>,
    ) -> InventoryResult {
        self.item_storage_view()
            .can_bank_item(dest, args, self.data.num_bank_slots)
    }

    pub fn find_equip_slot(&self, args: FindEquipSlotArgs<'_>) -> u8 {
        self.item_storage_view().find_equip_slot(args)
    }

    pub fn can_equip_item(&self, args: CanEquipItemArgs<'_>) -> CanEquipItemOutcome {
        self.item_storage_view().can_equip_item(args)
    }

    pub fn can_unequip_item(&self, args: CanUnequipItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_unequip_item(args)
    }

    pub fn can_equip_unique_item_template(
        &self,
        args: CanEquipUniqueItemTemplateArgs<'_>,
    ) -> InventoryResult {
        self.item_storage_view()
            .can_equip_unique_item_template(args)
    }

    pub fn can_equip_unique_item(&self, args: CanEquipUniqueItemArgs<'_>) -> InventoryResult {
        self.item_storage_view().can_equip_unique_item(args)
    }

    pub fn is_two_hand_used_template(&self, main_template: Option<&ItemStorageTemplate>) -> bool {
        self.item_storage_view()
            .is_two_hand_used_template(main_template, self.can_titan_grip)
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
        self.inventory
            .register_bag_storage(bag_slot, bag_guid, bag_size)
    }

    pub fn store_top_level_item(
        &mut self,
        slot: u8,
        guid: ObjectGuid,
    ) -> Result<(), PlayerStorageError> {
        self.inventory
            .store_top_level_item_native_like_cpp(slot, guid)?;
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
        let removed = self.inventory.take_top_level_item_native_like_cpp(slot)?;
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
        self.item_storage_view()
            .swap_item_preflight_plan(src, dst, is_alive, src_item, dst_item)
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
        self.inventory.store_bag_item(bag, slot, guid)
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
        self.inventory.remove_bag_item(bag, slot)
    }

    pub fn get_bag_by_pos(&self, bag: u8) -> Option<ObjectGuid> {
        if is_bag_storage_slot(bag) {
            self.inventory.bag_at(bag).map(|b| b.bag_guid)
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
        callback: impl FnMut(ObjectGuid) -> ItemSearchCallbackResult,
    ) -> bool {
        self.item_storage_view()
            .for_each_item_guid(location, callback)
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
}
