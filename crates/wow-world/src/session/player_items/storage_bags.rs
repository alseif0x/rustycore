//! Represented bag slots and their container capacity.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn represented_bag_contains_active_item_loot_like_cpp(
        &self,
        bag_guid: ObjectGuid,
    ) -> bool {
        if self.loot.active_loot_view_owners_is_empty_like_cpp() {
            return false;
        }

        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items.values().any(|item| {
                    item.container_guid() == bag_guid
                        && self
                            .loot
                            .has_active_loot_view_owner_like_cpp(item.object().guid())
                        && self
                            .loot
                            .cached_loot_contains_owner_like_cpp(item.object().guid())
                })
            })
    }
    pub(crate) fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_bag_slot_values_update_like_cpp(hub, bag_slot, changed_slot)
    }
}
