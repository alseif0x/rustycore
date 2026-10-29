// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Cast-all-obtain access to canonical Player item topology.

use super::super::*;

impl Player {
    /// C++ `Player::GetItemCount(entry, false)`: enumerate only the selected
    /// equipment, carried inventory and reagent-bank locations. A selected
    /// GUID without its canonical Item object makes the count unavailable.
    pub fn item_counts_for_location_like_cpp(
        &self,
        location: ItemSearchLocation,
    ) -> Option<HashMap<u32, u32>> {
        let item_objects = self.inventory_runtime.item_objects();
        let mut counts = HashMap::new();
        let mut complete = true;
        let traversed = self.for_each_item_guid(location, |guid| {
            let Some(item) = item_objects.get(&guid) else {
                complete = false;
                return ItemSearchCallbackResult::Stop;
            };
            if !item.is_in_trade() {
                let entry_id = item.object().entry();
                counts
                    .entry(entry_id)
                    .and_modify(|count: &mut u32| *count = count.saturating_add(item.count()))
                    .or_insert(item.count());
            }
            ItemSearchCallbackResult::Continue
        });
        (traversed && complete).then_some(counts)
    }

    /// C++ `Player::CastAllObtainSpells` freezes `inventoryEnd` once before
    /// scanning the backpack. Gear, keyring, bank and reagent-bank slots are
    /// intentionally outside this operation.
    pub fn cast_all_obtain_backpack_end_like_cpp(&self) -> Option<u8> {
        if !self
            .inventory_runtime
            .equipment_inventory_authority_complete_like_cpp()
            && !self.inventory_runtime.carried_inventory_loaded_like_cpp()
        {
            return None;
        }
        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(self.active_data.num_backpack_slots)
            .min(INVENTORY_SLOT_ITEM_END);
        Some(backpack_end)
    }

    /// C++ resolves each equipped bag and its size only when its outer-loop
    /// slot is reached, after earlier backpack and bag casts have returned.
    /// The returned GUID identifies that bag for its inner slot scan.
    pub fn cast_all_obtain_bag_like_cpp(&self, bag_slot: u8) -> Option<Option<(ObjectGuid, u8)>> {
        if !self
            .inventory_runtime
            .equipment_inventory_authority_complete_like_cpp()
            && !self.inventory_runtime.carried_inventory_loaded_like_cpp()
            || !(INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&bag_slot)
        {
            return None;
        }

        let bag_guid = self
            .inventory
            .items
            .get(usize::from(bag_slot))
            .copied()
            .flatten();
        let bag_storage = self.inventory.bag_at(bag_slot);
        match (bag_guid, bag_storage) {
            (None, None) => Some(None),
            (Some(guid), Some(storage)) if storage.bag_guid == guid => {
                let bag_item = self.inventory_runtime.item_objects().get(&guid)?;
                if bag_item.object().guid() != guid
                    || bag_item.owner_guid() != self.guid()
                    || bag_item.update_state() == ItemUpdateState::Removed
                {
                    return None;
                }
                Some(Some((guid, storage.bag_size)))
            }
            _ => None,
        }
    }

    /// Resolve a live obtain slot. `expected_bag` is the bag object captured
    /// when C++ entered that bag's inner loop; a replaced or retired bag makes
    /// the owner unavailable instead of silently scanning another container.
    /// Empty live slots are represented by `Some(None)`.
    pub fn cast_all_obtain_item_at_like_cpp(
        &self,
        bag_slot: u8,
        slot: u8,
        expected_bag: Option<ObjectGuid>,
    ) -> Option<Option<ObjectGuid>> {
        if !self
            .inventory_runtime
            .equipment_inventory_authority_complete_like_cpp()
            && !self.inventory_runtime.carried_inventory_loaded_like_cpp()
        {
            return None;
        }
        if bag_slot == INVENTORY_SLOT_BAG_0 {
            if expected_bag.is_some()
                || !(INVENTORY_SLOT_ITEM_START..INVENTORY_SLOT_ITEM_END).contains(&slot)
            {
                return None;
            }
        } else {
            let expected_bag = expected_bag?;
            if !(INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&bag_slot)
                || self.get_bag_by_pos(bag_slot) != Some(expected_bag)
            {
                return None;
            }
        }

        let Some(item_guid) = self.get_item_by_pos(bag_slot, slot) else {
            return Some(None);
        };
        let item = self.inventory_runtime.item_objects().get(&item_guid)?;
        if item.object().guid() != item_guid
            || item.owner_guid() != self.guid()
            || item.update_state() == ItemUpdateState::Removed
        {
            return None;
        }
        Some(Some(item_guid))
    }
}
