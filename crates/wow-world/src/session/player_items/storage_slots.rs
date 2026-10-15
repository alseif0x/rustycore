//! Slot resolution for represented inventory storage: free space, position search and fit checks.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn represented_empty_inventory_positions_like_cpp(&self) -> Option<Vec<(u8, u8)>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_empty_inventory_positions_like_cpp(hub)
    }
    pub(crate) fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.get_inventory_item_by_pos(hub, bag, slot)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/storage_slots/f3_shims.rs"]
mod f3_shims;
