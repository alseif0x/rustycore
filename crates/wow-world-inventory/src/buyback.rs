// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::{BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START};
use wow_world_core::session::{HubMut, HubRef, OwnedInventoryAccessLikeCpp};

impl crate::InventoryState {
    pub fn clear_buyback_runtime_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.clear_buyback_like_cpp();
        });
    }

    pub fn set_current_buyback_slot_like_cpp(&mut self, hub: &mut HubMut<'_>, slot: u8) {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.set_current_buyback_slot(slot);
        });
    }

    pub fn set_buyback_slot_metadata_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        slot: u8,
        price: u32,
        timestamp: i64,
    ) {
        if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
            return;
        }
        let index = (slot - BUYBACK_SLOT_START) as usize;
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            inventory.set_buyback_price_and_timestamp_like_cpp(index, price, timestamp);
        });
    }

    pub fn clear_buyback_slot_metadata_like_cpp(&mut self, hub: &mut HubMut<'_>, slot: u8) {
        self.set_buyback_slot_metadata_like_cpp(hub, slot, 0, 0);
    }

    pub fn select_buyback_slot_cpp(&self, hub: HubRef<'_>) -> Option<u8> {
        let buyback_items = self.resolved_buyback_items_like_cpp(hub)?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp(hub)?;
        let mut slot = self.resolved_current_buyback_slot_like_cpp(hub)?;
        if buyback_items.contains_key(&slot) {
            let mut oldest_slot = BUYBACK_SLOT_START;
            let mut oldest_time = buyback_timestamp[0];

            for candidate in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
                let candidate_index = (candidate - BUYBACK_SLOT_START) as usize;
                if !buyback_items.contains_key(&candidate) {
                    oldest_slot = candidate;
                    break;
                }
                let candidate_time = buyback_timestamp[candidate_index];
                if oldest_time > candidate_time {
                    oldest_time = candidate_time;
                    oldest_slot = candidate;
                }
            }

            slot = oldest_slot;
        }

        Some(slot)
    }

    pub fn advance_buyback_slot_cpp(&mut self, hub: &mut HubMut<'_>) {
        self.mutate_player_inventory_runtime_like_cpp(hub, |inventory| {
            if inventory.current_buyback_slot() < BUYBACK_SLOT_END - 1 {
                inventory.set_current_buyback_slot(inventory.current_buyback_slot() + 1);
            }
        });
    }

    pub fn resolved_buyback_price_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_buyback_price_with_access_like_cpp(&access)
    }

    pub(crate) fn resolved_buyback_price_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            .map(|inventory| *inventory.buyback_price())
    }

    pub fn resolved_buyback_timestamp_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<[i64; BUYBACK_SLOT_COUNT]> {
        let access = hub.core.owned_inventory_access_like_cpp();
        self.resolved_buyback_timestamp_with_access_like_cpp(&access)
    }

    pub(crate) fn resolved_buyback_timestamp_with_access_like_cpp(
        &self,
        access: &OwnedInventoryAccessLikeCpp<'_>,
    ) -> Option<[i64; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_with_access_like_cpp(access)
            .map(|inventory| *inventory.buyback_timestamp())
    }

    pub fn resolved_current_buyback_slot_like_cpp(&self, hub: HubRef<'_>) -> Option<u8> {
        self.resolved_player_inventory_runtime_like_cpp(hub)
            .map(|inventory| inventory.current_buyback_slot())
    }
}
