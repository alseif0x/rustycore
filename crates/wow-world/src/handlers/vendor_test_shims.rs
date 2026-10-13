// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the vendor and bank bodies moved to
//! `wow_world_inventory::vendor_session` (#1363 slice 8b). Each forwards to
//! the moved body with this session as its host, so the scenarios keep their
//! method syntax and the character hotspot's test lines do not grow. The
//! buy-item, repair-item and buy-bank-slot entry points moved here verbatim
//! from the `cfg(test)` impls of `handlers/character/{vendor/buy,vendor,bank}.rs`.
//!
//! The seven one-statement wrappers after them came verbatim from
//! `handlers/character/items.rs` and
//! `session/{buyback_adapter,player_items/bank,player_items/storage_slots,publication/operations}.rs`:
//! their only production callers were the moved bodies, which now make the
//! same split or core call through the host.

#![cfg(test)]

use std::sync::Arc;

use wow_constants::SellResult;
use wow_core::ObjectGuid;
use wow_packet::packets::gossip::Hello;
use wow_packet::packets::misc::{BuyBankSlot, BuyItem, RepairItem};

use wow_entities::BUYBACK_SLOT_COUNT;
use wow_world_inventory::ExtendedCostItemTurninChange;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_buy_item(&mut self, buy: BuyItem) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        wow_world_inventory::handle_buy_item_with_generator_like_cpp(self, generator.as_ref(), buy)
            .await;
    }

    pub(crate) async fn handle_repair_item(&mut self, repair: RepairItem) {
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::handle_repair_item_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            repair,
        )
        .await;
    }

    pub(crate) async fn handle_buy_bank_slot(&mut self, buy: BuyBankSlot) {
        let prices = self
            .bank_bag_slot_prices_store_for_test_like_cpp()
            .cloned()
            .unwrap_or_else(|| Arc::new(wow_data::BankBagSlotPricesStore::from_entries([])));
        let generators = self.id_generators_for_test_like_cpp();
        wow_world_inventory::handle_buy_bank_slot_with_prices_and_generator_like_cpp(
            self,
            prices.as_ref(),
            generators.item.as_ref(),
            buy,
        )
        .await;
    }

    pub(crate) async fn handle_banker_activate(&mut self, hello: Hello) {
        wow_world_inventory::handle_banker_activate(self, hello).await;
    }

    pub(crate) fn update_vendor_item_current_count(
        &mut self,
        vendor_guid: ObjectGuid,
        item_id: u32,
        max_count: u32,
        incr_time: u32,
        buy_count: u32,
        used_count: u32,
    ) -> u32 {
        wow_world_inventory::update_vendor_item_current_count(
            self,
            vendor_guid,
            item_id,
            max_count,
            incr_time,
            buy_count,
            used_count,
        )
    }

    pub(crate) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.has_item_count_direct_inventory(hub, item_entry, count)
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.plan_destroy_item_count_direct_inventory(hub, item_entry, count)
    }

    pub(crate) fn set_buyback_slot_metadata_like_cpp(
        &mut self,
        slot: u8,
        price: u32,
        timestamp: i64,
    ) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_buyback_slot_metadata_like_cpp(&mut hub, slot, price, timestamp)
    }

    pub(crate) fn resolved_buyback_price_like_cpp(&self) -> Option<[u32; BUYBACK_SLOT_COUNT]> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_buyback_price_like_cpp(hub)
    }

    pub(crate) fn resolved_player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.resolved_player_bank_bag_slot_count_like_cpp(hub)
    }

    pub(crate) fn set_inventory_item_object_slot(&mut self, item_guid: ObjectGuid, slot: u8) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_inventory_item_object_slot(&mut hub, item_guid, slot)
    }

    pub(crate) fn send_sell_error(
        &self,
        result: SellResult,
        creature_guid: Option<ObjectGuid>,
        item_guid: ObjectGuid,
    ) {
        self.core.send_sell_error(result, creature_guid, item_guid)
    }
}
