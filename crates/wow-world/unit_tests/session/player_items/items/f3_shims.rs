// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_item_guid_generator_like_cpp(&mut self, generator: Arc<ObjectGuidGenerator>) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_item_guid_generator_like_cpp(&mut hub, generator)
    }
    #[cfg(test)]
    pub(crate) fn item_guid_generator_like_cpp_for_bridge(
        &self,
    ) -> Option<Arc<ObjectGuidGenerator>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_guid_generator_like_cpp_for_bridge(hub)
    }
    #[cfg(test)]
    pub(crate) fn allocate_item_instance_guids_like_cpp(
        &self,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.allocate_item_instance_guids_like_cpp(hub, count)
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_remove_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionRemoveItemLikeCpp] {
        self.inventory.represented_auction_remove_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_sell_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionSellItemLikeCpp] {
        self.inventory.represented_auction_sell_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn buyback_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        self.inventory.buyback_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_item_mod_reapply_events_like_cpp(
        &self,
    ) -> &[RepresentedItemModsReapplyEventLikeCpp] {
        self.inventory
            .represented_item_mod_reapply_events_like_cpp()
    }
}
