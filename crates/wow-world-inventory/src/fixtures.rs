// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handle-less Player item state and effect evidence used by Session tests.

use crate::{
    RepresentedAuctionPlaceBidLikeCpp, RepresentedAuctionReplicateRequestLikeCpp,
    RepresentedCombatStatRecalculationLikeCpp, RepresentedGuildBankInventoryMoveLikeCpp,
    RepresentedGuildBankListRequestLikeCpp, RepresentedGuildBankMoneyMoveLikeCpp,
    RepresentedGuildBankTabActionLikeCpp, RepresentedGuildRepairBankWithdrawLikeCpp,
    RepresentedItemBonusActionLikeCpp, RepresentedItemModsReapplyEventLikeCpp,
    RepresentedItemSetAuraRefreshEventLikeCpp, RepresentedItemSetSpellEventLikeCpp,
    RepresentedTransmogCriteriaEvent,
};
use std::collections::HashMap;
use wow_core::ObjectGuid;
use wow_entities::{
    BUYBACK_SLOT_COUNT, BUYBACK_SLOT_START, INVENTORY_DEFAULT_SIZE, Item, PlayerCurrency,
    PlayerInventoryItem as InventoryItem, PlayerItemModifierRuntimeStateLikeCpp,
    TitanGripPenaltyAction,
};

pub struct PlayerItemTestFixtureLikeCpp {
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(crate) player_bank_bag_slot_count_like_cpp: u8,
    /// Test-only bootstrap for fixtures without a canonical `Player` owner.
    pub(crate) player_inventory_slot_count_like_cpp: u8,
    /// In-memory inventory: slot → (item ObjectGuid, entry_id, db_guid).
    pub(crate) inventory_items: HashMap<u8, InventoryItem>,
    /// In-memory buyback slots, kept separate from normal inventory like C++ `GetItemByGuid`.
    pub(crate) buyback_items: HashMap<u8, InventoryItem>,
    pub(crate) buyback_price: [u32; BUYBACK_SLOT_COUNT],
    pub(crate) buyback_timestamp: [i64; BUYBACK_SLOT_COUNT],
    pub(crate) current_buyback_slot: u8,
    pub(crate) represented_item_mod_reapply_events_like_cpp:
        Vec<RepresentedItemModsReapplyEventLikeCpp>,
    pub(crate) represented_item_bonus_actions_like_cpp: Vec<RepresentedItemBonusActionLikeCpp>,
    pub(crate) represented_item_modifier_runtime_like_cpp: PlayerItemModifierRuntimeStateLikeCpp,
    pub(crate) represented_item_set_spell_events_like_cpp:
        Vec<RepresentedItemSetSpellEventLikeCpp>,
    pub(crate) represented_item_set_aura_refresh_events_like_cpp:
        Vec<RepresentedItemSetAuraRefreshEventLikeCpp>,
    pub(crate) represented_combat_stat_recalculations_like_cpp:
        Vec<RepresentedCombatStatRecalculationLikeCpp>,
    pub(crate) represented_titan_grip_penalty_actions_like_cpp: Vec<TitanGripPenaltyAction>,
    pub(crate) represented_avg_equipped_item_level_updates_like_cpp: Vec<f32>,
}

impl Default for PlayerItemTestFixtureLikeCpp {
    fn default() -> Self {
        Self {
            player_bank_bag_slot_count_like_cpp: 0,
            player_inventory_slot_count_like_cpp: INVENTORY_DEFAULT_SIZE,
            inventory_items: HashMap::new(),
            buyback_items: HashMap::new(),
            buyback_price: [0; BUYBACK_SLOT_COUNT],
            buyback_timestamp: [0; BUYBACK_SLOT_COUNT],
            current_buyback_slot: BUYBACK_SLOT_START,
            represented_item_mod_reapply_events_like_cpp: Vec::new(),
            represented_item_bonus_actions_like_cpp: Vec::new(),
            represented_item_modifier_runtime_like_cpp:
                wow_entities::PlayerItemModifierRuntimeStateLikeCpp::default(),
            represented_item_set_spell_events_like_cpp: Vec::new(),
            represented_item_set_aura_refresh_events_like_cpp: Vec::new(),
            represented_combat_stat_recalculations_like_cpp: Vec::new(),
            represented_titan_grip_penalty_actions_like_cpp: Vec::new(),
            represented_avg_equipped_item_level_updates_like_cpp: Vec::new(),
        }
    }
}

impl crate::InventoryState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn buyback_price_for_test_like_cpp(&self) -> &[u32; BUYBACK_SLOT_COUNT] {
        &self.player_item_test_fixture_like_cpp.buyback_price
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn buyback_timestamp_for_test_like_cpp(&self) -> &[i64; BUYBACK_SLOT_COUNT] {
        &self.player_item_test_fixture_like_cpp.buyback_timestamp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn current_buyback_slot_for_test_like_cpp(&self) -> u8 {
        self.player_item_test_fixture_like_cpp.current_buyback_slot
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_gold_for_test_like_cpp(&self) -> u64 {
        self.player_gold
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_gold_for_test_like_cpp(&mut self, gold: u64) {
        self.player_gold = gold;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_player_currency_for_test_like_cpp(
        &mut self,
        currency_id: u32,
        currency: PlayerCurrency,
    ) -> Option<PlayerCurrency> {
        self.player_currencies.insert(currency_id, currency)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_currency_for_test_like_cpp(
        &self,
        currency_id: &u32,
    ) -> Option<&PlayerCurrency> {
        self.player_currencies.get(currency_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_inventory_item_object_for_test_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        item: Item,
    ) -> Option<Item> {
        self.inventory_item_objects.insert(item_guid, item)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_object_for_test_like_cpp(
        &self,
        item_guid: &ObjectGuid,
    ) -> Option<&Item> {
        self.inventory_item_objects.get(item_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_object_mut_for_test_like_cpp(
        &mut self,
        item_guid: &ObjectGuid,
    ) -> Option<&mut Item> {
        self.inventory_item_objects.get_mut(item_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_object_contains_for_test_like_cpp(
        &self,
        item_guid: &ObjectGuid,
    ) -> bool {
        self.inventory_item_objects.contains_key(item_guid)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_objects_empty_for_test_like_cpp(&self) -> bool {
        self.inventory_item_objects.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_inventory_item_for_test_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.player_item_test_fixture_like_cpp
            .inventory_items
            .insert(slot, item)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_for_test_like_cpp(&self, slot: &u8) -> Option<&InventoryItem> {
        self.player_item_test_fixture_like_cpp
            .inventory_items
            .get(slot)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_at_slot_for_test_like_cpp(&self, slot: &u8) -> &InventoryItem {
        &self.player_item_test_fixture_like_cpp.inventory_items[slot]
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_item_slot_contains_for_test_like_cpp(&self, slot: &u8) -> bool {
        self.player_item_test_fixture_like_cpp
            .inventory_items
            .contains_key(slot)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn inventory_items_empty_for_test_like_cpp(&self) -> bool {
        self.player_item_test_fixture_like_cpp
            .inventory_items
            .is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_buyback_item_for_test_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.player_item_test_fixture_like_cpp
            .buyback_items
            .insert(slot, item)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_transmog_criteria_events_for_test_like_cpp(
        &self,
    ) -> &[RepresentedTransmogCriteriaEvent] {
        &self.represented_transmog_criteria_events
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn clear_represented_transmog_criteria_events_for_test_like_cpp(&mut self) {
        self.represented_transmog_criteria_events.clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_auction_place_bid_like_cpp(
        &mut self,
        bid: RepresentedAuctionPlaceBidLikeCpp,
    ) {
        self.represented_auction_place_bids_like_cpp.push(bid);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_auction_replicate_request_like_cpp(
        &mut self,
        request: RepresentedAuctionReplicateRequestLikeCpp,
    ) {
        self.represented_auction_replicate_requests_like_cpp
            .push(request);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auction_place_bids_like_cpp(
        &self,
    ) -> &[RepresentedAuctionPlaceBidLikeCpp] {
        &self.represented_auction_place_bids_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_auction_replicate_requests_like_cpp(
        &self,
    ) -> &[RepresentedAuctionReplicateRequestLikeCpp] {
        &self.represented_auction_replicate_requests_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_avg_equipped_item_level_update_for_test_like_cpp(
        &mut self,
        item_level: f32,
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_avg_equipped_item_level_updates_like_cpp
            .push(item_level);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_titan_grip_penalty_action_for_test_like_cpp(
        &mut self,
        action: TitanGripPenaltyAction,
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_titan_grip_penalty_actions_like_cpp
            .push(action);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_titan_grip_penalty_actions_for_test_like_cpp(
        &self,
    ) -> &[TitanGripPenaltyAction] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_titan_grip_penalty_actions_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_item_set_spell_events_for_test_like_cpp(
        &mut self,
        events: &[RepresentedItemSetSpellEventLikeCpp],
    ) {
        self.player_item_test_fixture_like_cpp
            .represented_item_set_spell_events_like_cpp
            .extend(events.iter().copied());
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_bank_inventory_move_like_cpp(
        &mut self,
        movement: RepresentedGuildBankInventoryMoveLikeCpp,
    ) {
        self.represented_guild_bank_inventory_moves_like_cpp
            .push(movement);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_bank_list_request_like_cpp(
        &mut self,
        request: RepresentedGuildBankListRequestLikeCpp,
    ) {
        self.represented_guild_bank_list_requests_like_cpp
            .push(request);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_bank_money_move_like_cpp(
        &mut self,
        movement: RepresentedGuildBankMoneyMoveLikeCpp,
    ) {
        self.represented_guild_bank_money_moves_like_cpp
            .push(movement);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_bank_tab_action_like_cpp(
        &mut self,
        action: RepresentedGuildBankTabActionLikeCpp,
    ) {
        self.represented_guild_bank_tab_actions_like_cpp
            .push(action);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_guild_repair_bank_withdraw_like_cpp(
        &mut self,
        withdraw: RepresentedGuildRepairBankWithdrawLikeCpp,
    ) {
        self.represented_guild_repair_bank_withdraws_like_cpp
            .push(withdraw);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_inventory_slot_count_for_test_like_cpp(&self) -> u8 {
        self.player_item_test_fixture_like_cpp
            .player_inventory_slot_count_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_bank_bag_slot_count_for_test_like_cpp(&self) -> u8 {
        self.player_item_test_fixture_like_cpp
            .player_bank_bag_slot_count_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_bank_bag_slot_flags_for_test_like_cpp(&self) -> &[u32; 7] {
        &self.represented_bank_bag_slot_flags_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_equipment_sets_for_test_like_cpp(
        &self,
    ) -> &wow_entities::PlayerEquipmentSetsLikeCpp {
        &self.represented_equipment_sets_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_void_storage_items_for_test_like_cpp(
        &self,
    ) -> &[Option<wow_entities::PlayerVoidStorageItemLikeCpp>] {
        &self.represented_void_storage_items_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_void_storage_loaded_for_test_like_cpp(&self) -> bool {
        self.represented_void_storage_loaded_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_combat_stat_recalculations_for_test_like_cpp(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        &self
            .player_item_test_fixture_like_cpp
            .represented_combat_stat_recalculations_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_loaded_void_storage_item_for_test_like_cpp(
        &mut self,
        slot_index: usize,
        item: &wow_entities::PlayerVoidStorageItemLikeCpp,
    ) -> bool {
        let items = &mut self.represented_void_storage_items_like_cpp;
        if items[slot_index].is_none()
            && !items
                .iter()
                .flatten()
                .any(|loaded| loaded.item_id == item.item_id)
        {
            items[slot_index] = Some(item.clone());
            true
        } else {
            false
        }
    }
}
