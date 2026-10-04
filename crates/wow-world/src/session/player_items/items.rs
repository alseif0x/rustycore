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
    #[cfg(test)]
    pub(crate) fn vendor_buy_item_test_override_like_cpp(
        &self,
    ) -> Option<VendorBuyItemTestOverrideLikeCpp> {
        self.interaction.vendor_buy_item_test_override_like_cpp()
    }
    /// Bounded C++ `CollectionMgr::OnItemAdded`.
    pub(crate) fn on_item_added_to_collection_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Vec<wow_entities::PlayerValuesUpdate> {
        let item_id = item.object().entry();
        let mut updates = Vec::new();

        if self
            .catalogs
            .heirloom_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
            && self.add_account_heirloom_like_cpp(item_id, 0)
            && let Some(update) = crate::session::hub_mut(self)
                .add_player_heirloom_dynamic_fields_like_cpp(item_id, 0)
        {
            updates.push(update);
        }

        if let Some(update) = self.add_item_appearance_for_runtime_item_like_cpp(item) {
            updates.push(update);
        }

        updates
    }
    pub fn item_display_id(&self, item_id: u32, appearance_mod_id: u32) -> Option<u32> {
        self.catalogs.item_display_id(item_id, appearance_mod_id)
    }
    pub(crate) fn record_represented_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> bool {
        !self
            .record_represented_items_set_item_events_like_cpp(item_guid, apply)
            .is_empty()
    }
    pub(in crate::session) fn record_represented_items_set_item_events_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let (state, hub) = crate::session::split_inventory_mut(self);
        let inventory_access = hub.core.owned_inventory_access_like_cpp();
        let item_modifiers = hub.core.owned_item_modifiers_access_like_cpp();
        let shared = hub.shared();
        let item_sets = shared.owned_item_set_access_like_cpp();
        state.record_represented_items_set_item_like_cpp(
            &inventory_access,
            &item_modifiers,
            &item_sets,
            item_guid,
            apply,
            cfg!(test),
        )
    }
    pub(crate) fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.item_drop_rate_like_cpp(hub, item_id)
    }
    pub fn is_item_bound_account_wide(&self, item_id: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.is_item_bound_account_wide(hub, item_id)
    }
    pub(crate) fn insert_buyback_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.insert_buyback_item_like_cpp(&mut hub, slot, item)
    }
    pub(crate) fn remove_buyback_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.remove_buyback_item_like_cpp(&mut hub, slot)
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
    pub(crate) fn direct_item_contains_items(&self, item_guid: ObjectGuid) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.direct_item_contains_items(hub, item_guid)
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
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_remove_item_like_cpp(
        &mut self,
        remove: RepresentedAuctionRemoveItemLikeCpp,
    ) {
        self.inventory
            .record_represented_auction_remove_item_like_cpp(remove)
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_sell_item_like_cpp(
        &mut self,
        sell: RepresentedAuctionSellItemLikeCpp,
    ) {
        self.inventory
            .record_represented_auction_sell_item_like_cpp(sell)
    }
    pub(in crate::session) fn record_represented_offhand_item_mod_remove_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> bool {
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_OFFHAND, false) != 0
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
    pub(in crate::session) fn uncage_cast_item_still_matches_like_cpp(
        &self,
        cast_item_entry: u32,
        modifiers: SpellCastBattlePetItemModifiersLikeCpp,
    ) -> Option<(u8, u8, InventoryItem)> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.uncage_cast_item_still_matches_like_cpp(hub, cast_item_entry, modifiers)
    }
    pub(crate) async fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp {
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle persistence port is unavailable".to_owned(),
            };
        };
        port.load_uncage_item_state_like_cpp(wow_persistence::PlayerUncageItemStateRequestLikeCpp {
            player_guid: player_db_guid,
            item_guid: item_db_guid,
        })
        .await
    }
    pub(crate) fn use_represented_gameobject_item_forge_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::ItemForgeUseSource,
    ) -> bool {
        self.world_entities.record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::ItemForgeUsed {
                gameobject_guid,
                player_guid,
                condition_id: source.condition_id,
                forge_type: source.forge_type,
            },
        );

        true
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/player_items/items/f3_shims.rs"]
mod f3_shims;
