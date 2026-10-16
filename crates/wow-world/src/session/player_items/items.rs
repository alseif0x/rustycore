//! Remaining represented item operations owned by the inventory responsibility.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn set_vendor_buy_item_test_override_like_cpp(
        &mut self,
        item: VendorBuyItemTestOverrideLikeCpp,
    ) {
        self.interaction
            .set_vendor_buy_item_test_override_like_cpp(item);
    }
    pub(crate) fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_drop_rate_like_cpp(hub, item_id)
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_spell_cast_item_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .and_then(|state| state.spell_cast_item_guid)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/items/f3_shims.rs"]
mod f3_shims;
