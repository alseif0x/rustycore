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
    /// Remove a fully-looted runtime item after its DB rows were deleted.
    pub(crate) fn remove_fully_looted_runtime_item(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        if bag == INVENTORY_SLOT_BAG_0
            && self
                .resolved_inventory_item_like_cpp(slot)
                .is_some_and(|item| item.guid == item_guid)
        {
            self.remove_inventory_item_like_cpp(slot);
        }
        self.remove_inventory_item_object(item_guid);
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_has_item_count_like_cpp(hub, item_entry, count)
    }
    pub(crate) fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        self.loot.has_active_non_item_loot_views_like_cpp()
    }
    pub(in crate::session) fn represented_has_item_fit_to_spell_requirements_like_cpp(
        &self,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_has_item_fit_to_spell_requirements_like_cpp(hub, equipped)
    }
    pub(crate) fn resolved_buyback_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_buyback_items_like_cpp(hub)
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
