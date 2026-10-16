//! Represented inventory storage: the complete store, remove and swap operations over the canonical Player item slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;
use wow_entities::ItemObjectUpdateLikeCpp;

impl WorldSession {
    pub(crate) fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_object(&mut hub, item)
    }
    pub(crate) fn apply_inventory_item_object_updates_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        updates: &[ItemObjectUpdateLikeCpp],
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_inventory_item_object_updates_like_cpp(&mut hub, item_guid, updates)
    }
    pub(crate) fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_inventory_item_like_cpp(&mut hub, slot, item)
    }
    pub(crate) fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_inventory_item_counts_like_cpp(hub)
    }
    pub(crate) fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        self.can_use_inventory_item_represented_with_loading_like_cpp(item, runtime_item, true)
    }
    pub(crate) fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_items_like_cpp(hub)
    }
    pub(crate) fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_like_cpp(hub, slot)
    }
    pub(crate) fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_inventory_item_object_like_cpp(hub, guid)
    }
}

impl crate::session::InventoryCxRef<'_> {
    pub(crate) fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        let max_durability = self
            .hub
            .catalogs
            .item_template_max_durability(entry_id)
            .max(durability);
        wow_world_inventory::make_inventory_item_object_like_cpp(
            item_guid,
            entry_id,
            owner_guid,
            count,
            durability,
            max_durability,
            context,
            slot,
            self.lifecycle.total_played_time_like_cpp(),
        )
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage/f3_shims.rs"]
mod f3_shims;
