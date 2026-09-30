//! buyback for the existing items owner.

use super::*;

impl WorldSession {
    pub(crate) fn insert_buyback_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.store_buyback_item_in_slot_like_cpp(slot, item)
        })
        .flatten()
    }
    pub(crate) fn remove_buyback_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.remove_buyback_item_from_slot_like_cpp(slot)
        })
        .flatten()
    }
    pub(crate) fn resolved_buyback_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.buyback_items().clone())
    }
    #[cfg(test)]
    pub(crate) fn buyback_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.player_item_test_fixture_like_cpp.buyback_items
    }
}
