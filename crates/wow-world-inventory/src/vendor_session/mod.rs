// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor and bank character handlers: the vendor list, buy, sell, buyback,
//! purchase refund and repair paths, the banker activation and the bank slot
//! purchase (#1363 slice 8b; moved from the World
//! `handlers/character/{vendor,vendor/*,vendor_admission,bank,items,mod}.rs`).
//!
//! C++ anchors stay on each body: `WorldSession::HandleListInventoryOpcode`,
//! `HandleBuyItemOpcode`/`Player::BuyItemFromVendorSlot`,
//! `HandleSellItemOpcode`, `HandleBuybackItem`, `HandleItemRefund`/
//! `Player::RefundItem`, `HandleRepairItemOpcode`, `HandleBankerActivateOpcode`
//! and `HandleBuyBankSlotOpcode`.
//!
//! The bodies run on the Player's session through [`VendorSessionHostLikeCpp`].
//! Each host method is the same-named `WorldSession` operation, called at the
//! exact point the body called it before the move. The money boundary, the
//! quest-objective drain, the money staging and the VALUES publication are the
//! ones [`crate::BattlePetPurchaseHostLikeCpp`] already lends; their futures are
//! returned unchanged. A World wrapper that was one `split_inventory_{ref,mut}`
//! plus one `InventoryState` call is that same split and call here.

use std::collections::HashMap;
use std::future::Future;

use wow_constants::{InventoryResult, InventoryType, ItemContext};
use wow_core::ObjectGuid;
use wow_data::PlayerConditionContextLikeCpp;
use wow_entities::{
    Item, ItemPosCount, MAX_MONEY_AMOUNT, PlayerCurrency, PlayerInventoryItem as InventoryItem,
    PlayerValuesUpdate, SendNewItemPlan, WorldObject,
};
use wow_world_core::session::{HubMut, PlayerCurrencyDelta};
use wow_world_interaction::InteractionState;
use wow_world_lifecycle::ExclusivePlayerMoneyPersistenceLikeCpp;

use crate::battle_pet_purchase::BattlePetPurchaseHostLikeCpp;
use crate::{ExtendedCostItemTurninChange, InventoryState};

mod bank;
mod buy;
mod buyback;
mod catalog;
mod list_inventory;
mod refund;
mod repair;
mod sell;

pub use bank::{handle_banker_activate, handle_buy_bank_slot_with_prices_and_generator_like_cpp};
pub use buy::handle_buy_item_with_generator_like_cpp;
pub use buyback::handle_buy_back_item_with_generator_like_cpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use catalog::update_vendor_item_current_count;
pub use list_inventory::handle_list_inventory;
pub use refund::handle_item_purchase_refund_with_generator_like_cpp;
pub use repair::handle_repair_item_with_generator_like_cpp;
pub use sell::handle_sell_item_with_generator_like_cpp;

/// The represented player-condition context the World projects once per
/// vendor operation, viewed against its host at each evaluation point.
pub trait PlayerConditionContextSourceLikeCpp<H: ?Sized> {
    fn as_context<'a>(&'a self, host: &'a H) -> Option<PlayerConditionContextLikeCpp<'a>>;
}

pub trait VendorSessionHostLikeCpp: BattlePetPurchaseHostLikeCpp {
    type PlayerConditionContext: PlayerConditionContextSourceLikeCpp<Self>;
    fn interaction(&self) -> &InteractionState;
    fn interaction_mut(&mut self) -> &mut InteractionState;
    fn split_inventory_mut(&mut self) -> (&mut InventoryState, HubMut<'_>);
    fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem>;
    fn is_active_loot_guid(&self, guid: ObjectGuid) -> bool;
    fn enqueue_represented_quest_objective_money_changed_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    );
    fn represented_player_condition_context_like_cpp(&self)
    -> Option<Self::PlayerConditionContext>;
    fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, wow_conditions::ConditionUnitSnapshot)>;
    fn has_currency(&self, currency_id: u32, amount: u32) -> bool;
    fn player_currency_quantity(&self, currency_id: u32) -> Option<u32>;
    fn add_currency_item_refund(
        &mut self,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()>;
    fn plan_add_currency_vendor_like_cpp(
        &self,
        currencies: &mut HashMap<u32, PlayerCurrency>,
        currency_id: u32,
        amount: u32,
    ) -> Result<Option<PlayerCurrencyDelta>, ()>;
    fn plan_store_new_direct_inventory_item(
        &self,
        entry_id: u32,
        count: u32,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)>;
    fn plan_store_new_direct_inventory_item_at(
        &self,
        entry_id: u32,
        count: u32,
        bag: u8,
        slot: u8,
    ) -> Option<(InventoryResult, Vec<ItemPosCount>, Option<u32>)>;
    #[allow(clippy::too_many_arguments)]
    fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item;
    fn on_item_added_to_collection_like_cpp(&mut self, item: &Item) -> Vec<PlayerValuesUpdate>;
    fn apply_item_turnin_changes(
        &mut self,
        player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    );
    fn send_new_item_plan(&self, plan: &SendNewItemPlan);
    fn send_stat_update(&mut self) -> bool;
    fn sync_player_registry_state_like_cpp(&self);
    fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool;
    fn send_show_bank_like_cpp(&mut self, banker_guid: ObjectGuid);
    fn quest_source_item_quest_log_item_id_like_cpp(
        &mut self,
        entry_id: u32,
    ) -> impl Future<Output = u32> + Send;
    fn await_exclusive_player_money_transaction_outcome_like_cpp<F>(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        outcome_future: F,
        money_before: u64,
        money_after: u64,
        operation: &'static str,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send
    where
        F: Future<Output = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp> + Send;
    /// The Player's known spells (`Player::HasSpell` source), read by the
    /// appearance gate's required-ability and learning-effect checks.
    fn item_appearance_known_spells_like_cpp(&self) -> Vec<i32>;
}

fn player_money_gain_like_cpp(current_money: u64, amount: u64) -> Option<u64> {
    if amount == 0 {
        return Some(current_money);
    }

    let max_gain = MAX_MONEY_AMOUNT.checked_sub(amount)?;
    if current_money <= max_gain {
        Some(current_money + amount)
    } else {
        None
    }
}

fn item_is_currently_looted_like_cpp(item: &wow_entities::Item) -> bool {
    item.loot_generated()
}

fn item_is_not_empty_bag_like_cpp(
    inventory_type: Option<InventoryType>,
    contains_items: bool,
) -> bool {
    matches!(inventory_type, Some(InventoryType::Bag)) && contains_items
}

#[cfg(test)]
#[path = "../../unit_tests/vendor_session_helpers.rs"]
mod tests;
