//! economy and pet adapters for the existing items owner.

use super::*;

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_remove_item_like_cpp(
        &mut self,
        remove: RepresentedAuctionRemoveItemLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_remove_items_like_cpp.push(remove);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_remove_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionRemoveItemLikeCpp] {
        &self.represented_auction_remove_items_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_sell_item_like_cpp(
        &mut self,
        sell: RepresentedAuctionSellItemLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_sell_items_like_cpp.push(sell);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_sell_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionSellItemLikeCpp] {
        &self.represented_auction_sell_items_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_item_mod_reapply_events_like_cpp(
        &self,
    ) -> &[RepresentedItemModsReapplyEventLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_item_mod_reapply_events_like_cpp
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
        let (bag, slot, inventory_item) =
            self.get_inventory_item_by_guid_like_cpp(modifiers.source_item_guid)?;
        if inventory_item.entry_id != cast_item_entry {
            return None;
        }
        let item = self.resolved_inventory_item_object_like_cpp(modifiers.source_item_guid)?;
        (item.object().entry() == cast_item_entry
            && item.get_modifier(ItemModifier::BattlePetSpeciesId) == modifiers.species_id
            && item.get_modifier(ItemModifier::BattlePetBreedData) == modifiers.breed_data
            && item.get_modifier(ItemModifier::BattlePetLevel) == u32::from(modifiers.level)
            && item.get_modifier(ItemModifier::BattlePetDisplayId) == modifiers.display_id)
            .then_some((bag, slot, inventory_item))
    }
    pub(crate) async fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
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
        self.represented_gameobject_use_effects.push(
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
