//! Slot resolution for represented inventory storage: free space, position search and fit checks.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_direct_inventory_slot_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<(u8, InventoryItem)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_direct_inventory_slot_by_guid_like_cpp(hub, guid)
    }
    pub(crate) fn move_represented_direct_inventory_item_to_pos_like_cpp(
        &mut self,
        src: u8,
        dst_bag: u8,
        dst_slot: u8,
    ) -> bool {
        if dst_bag == INVENTORY_SLOT_BAG_0 {
            return self.move_represented_direct_inventory_item_like_cpp(src, dst_slot);
        }
        if !is_represented_bag_slot(dst_bag)
            || self.get_inventory_item_by_pos(dst_bag, dst_slot).is_some()
        {
            return false;
        }

        let Some(src_item) = self.resolved_inventory_item_like_cpp(src) else {
            return false;
        };
        let Some(bag_item) = self.resolved_inventory_item_like_cpp(dst_bag) else {
            return false;
        };
        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return false;
        };
        if !item_objects.contains_key(&src_item.guid) || !item_objects.contains_key(&bag_item.guid)
        {
            return false;
        }

        self.remove_inventory_item_like_cpp(src);
        self.apply_inventory_item_object_updates_like_cpp(
            src_item.guid,
            &[
                wow_entities::ItemObjectUpdateLikeCpp::SetContainedIn(bag_item.guid),
                wow_entities::ItemObjectUpdateLikeCpp::SetSlot(dst_slot),
                wow_entities::ItemObjectUpdateLikeCpp::SetContainerGuidAndSlot(
                    bag_item.guid,
                    dst_bag,
                ),
            ],
        )
    }
    pub(crate) fn set_inventory_item_object_slot(&mut self, item_guid: ObjectGuid, slot: u8) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_inventory_item_object_slot(&mut hub, item_guid, slot)
    }
    pub(crate) fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_empty_inventory_positions_like_cpp(hub)
    }
    pub(crate) fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_pos(hub, bag, slot)
    }
    pub(crate) fn is_valid_inventory_pos_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        explicit_pos: bool,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.is_valid_inventory_pos_like_cpp(hub, bag, slot, explicit_pos)
    }
    pub(crate) fn represented_inventory_descendants_postorder_like_cpp(
        &self,
        container_guid: ObjectGuid,
    ) -> Option<Vec<(u8, u8, InventoryItem)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_inventory_descendants_postorder_like_cpp(hub, container_guid)
    }
    pub(crate) fn set_player_inventory_slot_count_like_cpp(&mut self, count: u8) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_inventory_slot_count_like_cpp(&mut hub, count)
    }
    #[cfg(test)]
    pub(crate) fn player_inventory_slot_count_like_cpp(&self) -> u8 {
        self.resolved_player_inventory_slot_count_like_cpp()
            .expect("test Player inventory-slot owner must resolve")
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage_slots/f3_shims.rs"]
mod f3_shims;
