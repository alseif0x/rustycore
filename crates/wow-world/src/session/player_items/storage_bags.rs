//! Represented bag slots and their container capacity.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn send_bag_slot_values_update_like_cpp(&self, bag_slot: u8, changed_slot: u8) {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.send_bag_slot_values_update_like_cpp(hub, bag_slot, changed_slot)
    }
}
