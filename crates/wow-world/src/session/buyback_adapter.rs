// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Buyback adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{BUYBACK_SLOT_COUNT, BUYBACK_SLOT_END, BUYBACK_SLOT_START, WorldSession};

impl WorldSession {
    pub(crate) fn clear_buyback_runtime_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.clear_buyback_like_cpp();
        });
    }

    pub(crate) fn set_current_buyback_slot_like_cpp(&mut self, slot: u8) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.set_current_buyback_slot(slot);
        });
    }

    pub(crate) fn set_buyback_slot_metadata_like_cpp(
        &mut self,
        slot: u8,
        price: u32,
        timestamp: i64,
    ) {
        if !(BUYBACK_SLOT_START..BUYBACK_SLOT_END).contains(&slot) {
            return;
        }
        let index = (slot - BUYBACK_SLOT_START) as usize;
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.set_buyback_price_and_timestamp_like_cpp(index, price, timestamp);
        });
    }

    pub(crate) fn clear_buyback_slot_metadata_like_cpp(&mut self, slot: u8) {
        self.set_buyback_slot_metadata_like_cpp(slot, 0, 0);
    }

    pub(crate) fn select_buyback_slot_cpp(&self) -> Option<u8> {
        let buyback_items = self.resolved_buyback_items_like_cpp()?;
        let buyback_timestamp = self.resolved_buyback_timestamp_like_cpp()?;
        let mut slot = self.resolved_current_buyback_slot_like_cpp()?;
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

    pub(crate) fn advance_buyback_slot_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            if inventory.current_buyback_slot() < BUYBACK_SLOT_END - 1 {
                inventory.set_current_buyback_slot(inventory.current_buyback_slot() + 1);
            }
        });
    }

    pub(crate) fn resolved_buyback_price_like_cpp(&self) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| *inventory.buyback_price())
    }

    pub(crate) fn resolved_buyback_timestamp_like_cpp(&self) -> Option<[i64; BUYBACK_SLOT_COUNT]> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| *inventory.buyback_timestamp())
    }

    pub(crate) fn resolved_current_buyback_slot_like_cpp(&self) -> Option<u8> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.current_buyback_slot())
    }

    #[cfg(test)]
    pub(crate) fn buyback_price_like_cpp(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        &self.player_item_test_fixture_like_cpp.buyback_price
    }

    #[cfg(test)]
    pub(crate) fn buyback_timestamp_like_cpp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        &self.player_item_test_fixture_like_cpp.buyback_timestamp
    }

    #[cfg(test)]
    pub(crate) fn current_buyback_slot_like_cpp(&self) -> u8 {
        self.player_item_test_fixture_like_cpp.current_buyback_slot
    }
}
