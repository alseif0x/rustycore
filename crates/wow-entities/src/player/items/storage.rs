// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Inventory and bank storage: bags, slots, stacking and destroy.

use super::super::*;

mod bag_operations;
mod bank;
mod buyback;
mod destruction_plans;
mod durations;
mod lookup_iteration;
mod mutations;
mod store_admission;
mod swap_plans;
mod usability;

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

    /// C++ `Player::GetInventorySlotCount` (`Player.h:1332`).
    pub const fn inventory_slot_count(&self) -> u8 {
        self.active_data.num_backpack_slots
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
}
