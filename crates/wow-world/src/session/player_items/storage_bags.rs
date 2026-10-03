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
        if self.loot.active_loot_view_owners.is_empty() {
            return false;
        }

        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items.values().any(|item| {
                    item.container_guid() == bag_guid
                        && self
                            .loot
                            .active_loot_view_owners
                            .contains(&item.object().guid())
                        && self.loot.loot_table.contains_key(&item.object().guid())
                })
            })
    }
    pub(crate) fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_bag_slot_values_update_like_cpp(hub, bag_slot, changed_slot)
    }
}
