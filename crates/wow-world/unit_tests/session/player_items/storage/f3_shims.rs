// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn inventory_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        self.inventory.inventory_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn inventory_item_objects_like_cpp(&self) -> &HashMap<ObjectGuid, Item> {
        self.inventory.inventory_item_objects_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn update_inventory_item_object_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        update: impl FnOnce(&mut Item),
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.update_inventory_item_object_like_cpp(&mut hub, item_guid, update)
    }
    pub(crate) fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mutate_player_inventory_runtime_like_cpp(&mut hub, update)
    }
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
        crate::session::cx_inventory_ref(self).make_inventory_item_object(
            item_guid, entry_id, owner_guid, count, durability, context, slot,
        )
    }
    pub(crate) fn clear_all_inventory_runtime_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_all_inventory_runtime_like_cpp(&mut hub)
    }
}
