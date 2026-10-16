//! Loading represented item and inventory state from persistence.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    pub(in crate::session) fn can_use_inventory_item_represented_with_loading_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
        not_loading: bool,
    ) -> InventoryResult {
        let player_conditions = self.player_condition_projection_cx_like_cpp();
        wow_world_player_conditions::can_use_inventory_item_represented_with_loading_like_cpp(
            &player_conditions,
            item,
            runtime_item,
            not_loading,
        )
    }
    /// C++ `Player::_LoadInventory` collection side effects for a loaded item.
    pub(crate) fn apply_loaded_inventory_item_collection_hooks_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) {
        let _ = crate::session::cx_lifecycle(self)
            .check_account_heirloom_upgrades_like_cpp(item.object().entry());
        let _ = wow_world_inventory::add_item_appearance_for_runtime_item_like_cpp(self, item);
    }
    pub(crate) fn register_loaded_inventory_item_duration_refs_like_cpp(
        &mut self,
        loaded_item_guids: &[ObjectGuid],
        loaded_equipped_item_guids: &[ObjectGuid],
    ) -> (Vec<PlayerItemTimeUpdate>, Vec<PlayerEnchantTimeUpdate>) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.register_loaded_inventory_item_duration_refs_like_cpp(
            &mut hub,
            loaded_item_guids,
            loaded_equipped_item_guids,
        )
    }
    pub(crate) fn player_equipment_inventory_authority_complete_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_equipment_inventory_authority_complete_like_cpp(hub)
    }
}
