// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World adapter for the vendor and bank bodies moved to
//! `wow_world_inventory::vendor_session` (#1363 slice 8b); the trainer gossip
//! menu moved to `wow_world_character::gossip_session` (#1363 slice 8e). Every
//! adapter method is a field access or a pure forward to the same-named
//! inherent `WorldSession` method or moved free function (#1363 slice 12b1b).

use super::*;

use std::future::Future;

use wow_entities::{ItemPosCount, PlayerCurrency};
use wow_world_core::session::PlayerCurrencyDelta;
use wow_world_interaction::InteractionState;
use wow_world_inventory::{
    InventoryState, PlayerConditionContextSourceLikeCpp, VendorSessionHostLikeCpp,
};

use crate::session::{
    ExclusivePlayerMoneyPersistenceLikeCpp, HubMut, RepresentedPlayerConditionContextLikeCpp,
};

impl PlayerConditionContextSourceLikeCpp<WorldSession>
    for RepresentedPlayerConditionContextLikeCpp
{
    fn as_context<'a>(
        &'a self,
        host: &'a WorldSession,
    ) -> Option<wow_data::PlayerConditionContextLikeCpp<'a>> {
        RepresentedPlayerConditionContextLikeCpp::as_context(self, host)
    }
}

impl VendorSessionHostLikeCpp for WorldSession {
    type PlayerConditionContext = RepresentedPlayerConditionContextLikeCpp;
    fn interaction(&self) -> &InteractionState {
        &self.interaction
    }
    fn interaction_mut(&mut self) -> &mut InteractionState {
        &mut self.interaction
    }
    fn split_inventory_mut(&mut self) -> (&mut InventoryState, HubMut<'_>) {
        crate::session::split_inventory_mut(self)
    }
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        WorldSession::insert_inventory_item_like_cpp(self, slot, item)
    }
    fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool {
        self.loot.is_active_loot_guid(guid)
    }
    fn enqueue_represented_quest_objective_money_changed_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) {
        self.quest_state
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
    }
    fn represented_player_condition_context_like_cpp(
        &self,
    ) -> Option<RepresentedPlayerConditionContextLikeCpp> {
        WorldSession::represented_player_condition_context_like_cpp(self)
    }
    fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, wow_conditions::ConditionUnitSnapshot)> {
        WorldSession::build_condition_creature_object_like_cpp(self, npc_guid)
    }
    fn has_currency(&self, currency_id: u32, amount: u32) -> bool {
        WorldSession::has_currency(self, currency_id, amount)
    }
    fn player_currency_quantity(&self, currency_id: u32) -> Option<u32> {
        WorldSession::player_currency_quantity(self, currency_id)
    }
    fn add_currency_item_refund(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        WorldSession::add_currency_item_refund(self, currency_id, amount)
    }
    fn plan_add_currency_vendor_like_cpp(
        &self,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        WorldSession::plan_add_currency_vendor_like_cpp(self, currencies, currency_id, amount)
    }
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        application::plan_store_new_direct_inventory_item(self, entry_id, count)
    }
    fn plan_store_new_direct_inventory_item_at(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)> {
        application::plan_store_new_direct_inventory_item_at(self, entry_id, count, bag, slot)
    }
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> wow_entities::Item {
        crate::session::cx_inventory_ref(self).make_inventory_item_object(
            item_guid, entry_id, owner_guid, count, durability, context, slot,
        )
    }
    fn on_item_added_to_collection_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Vec<wow_entities::PlayerValuesUpdate> {
        wow_world_inventory::on_item_added_to_collection_like_cpp(self, item)
    }
    fn apply_item_turnin_changes(
        &mut self,
        player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        WorldSession::apply_item_turnin_changes(self, player_guid, map_id, changes)
    }
    fn send_new_item_plan(&self, plan: &SendNewItemPlan) {
        WorldSession::send_new_item_plan(self, plan)
    }
    fn send_stat_update(&mut self) -> bool {
        WorldSession::send_stat_update(self)
    }
    fn sync_player_registry_state_like_cpp(&self) {
        WorldSession::sync_player_registry_state_like_cpp(self)
    }
    fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        WorldSession::remove_represented_feign_death_if_needed_like_cpp(self)
    }
    fn send_show_bank_like_cpp(&mut self, banker_guid: ObjectGuid) {
        WorldSession::send_show_bank_like_cpp(self, banker_guid)
    }
    fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> impl Future<Output = u32> + Send {
        wow_world_quest::quest_source_item_quest_log_item_id_like_cpp(self, entry_id)
    }
    fn await_exclusive_player_money_transaction_outcome_like_cpp<F>(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        outcome_future: F,
        money_before: u64,
        money_after: u64,
        operation: &'static str,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send
    where
        F: Future<Output = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> + Send,
    {
        WorldSession::await_exclusive_player_money_transaction_outcome_like_cpp(
            self,
            money_persistence,
            outcome_future,
            money_before,
            money_after,
            operation,
        )
    }
    fn item_appearance_known_spells_like_cpp(&self) -> Vec<i32> {
        wow_world_spell::known_spells_like_cpp(self)
    }
}
