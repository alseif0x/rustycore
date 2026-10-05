// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Buyback adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{BUYBACK_SLOT_COUNT, WorldSession};

impl WorldSession {
    pub(crate) fn set_buyback_slot_metadata_like_cpp(
        &mut self,
        slot: u8,
        price: u32,
        timestamp: i64,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_buyback_slot_metadata_like_cpp(&mut hub, slot, price, timestamp)
    }

    pub(crate) fn resolved_buyback_price_like_cpp(&self) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_buyback_price_like_cpp(hub)
    }

    pub(crate) fn resolved_current_buyback_slot_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_current_buyback_slot_like_cpp(hub)
    }

    #[cfg(test)]
    pub(crate) fn buyback_price_like_cpp(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        self.inventory.buyback_price_for_test_like_cpp()
    }

    #[cfg(test)]
    pub(crate) fn buyback_timestamp_like_cpp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        self.inventory.buyback_timestamp_for_test_like_cpp()
    }

    #[cfg(test)]
    pub(crate) fn current_buyback_slot_like_cpp(&self) -> u8 {
        self.inventory.current_buyback_slot_for_test_like_cpp()
    }
}
