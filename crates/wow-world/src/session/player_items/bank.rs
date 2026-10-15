//! Represented bank storage slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(crate) fn represented_non_bank_item_count_like_cpp(&self, entry_id: u32) -> Option<u32> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_non_bank_item_count_like_cpp(hub, entry_id)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/bank/f3_shims.rs"]
mod f3_shims;
