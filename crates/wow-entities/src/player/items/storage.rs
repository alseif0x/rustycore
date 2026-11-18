// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory and bank storage: bags, slots, stacking and destroy.

mod buyback;
mod item_tracking;
mod placement;
mod planning;
mod projection;
mod publication;
mod removal;
mod swap;
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
            existing_item.map(Item::read_view),
            source_item.map(Item::read_view),
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
            source_item.map(Item::read_view),
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
            source_item.map(Item::read_view),
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
}
