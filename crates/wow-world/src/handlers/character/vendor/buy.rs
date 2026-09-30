// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vendor item purchase handler and transaction/publication flow.

use super::rules::{
    VendorBuyTemplateBlock, VendorExtendedCostBlock, vendor_buy_coinage_update_like_cpp,
    vendor_buy_currency_packet_quantity_to_cpp_count, vendor_buy_currency_quantity_block_result,
    vendor_buy_direct_inventory_destination, vendor_buy_direct_store_block_result,
    vendor_buy_extended_cost_block_result, vendor_buy_extended_cost_currency_costs,
    vendor_buy_extended_cost_item_costs, vendor_buy_muid_to_cpp_slot,
    vendor_buy_packet_quantity_to_cpp_count, vendor_buy_player_condition_block_result_like_cpp,
    vendor_buy_quantity_and_price, vendor_buy_required_reputation_block_result,
    vendor_buy_template_block_result, vendor_conditions_block_result, vendor_list_item_refundable,
    vendor_stored_new_item_flags_like_cpp,
};
use super::*;

mod currency;
mod item_purchase;
mod publication;

struct VendorItemPurchasePublication {
    new_stacks: Vec<(u8, u64, ObjectGuid, u32, u32)>,
    existing_updates: Vec<(u8, ObjectGuid, u64, u32)>,
    changed_slots: Vec<(u8, ObjectGuid)>,
    collection_updates: Vec<wow_entities::PlayerValuesUpdate>,
    purchased_item_plan: SendNewItemPlan,
    extended_cost_currency_costs: Vec<(u32, u32)>,
    store_dest: Vec<wow_entities::ItemPosCount>,
    quantity: u32,
    buy_price: u64,
    new_gold: u64,
    max_durability: u32,
    new_quantity: i32,
}

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_buy_item(&mut self, buy: BuyItem) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.handle_buy_item_with_generator_like_cpp(generator.as_ref(), buy)
            .await;
    }

    /// Handle CMSG_BUY_ITEM — player buys an item from a vendor.
    ///
    /// C++ refs: `HandleBuyItemOpcode` (`Handlers/ItemHandler.cpp:530-564`)
    /// delegates to `Player::BuyItemFromVendorSlot` (`Player.cpp:22338+`).
    pub async fn handle_buy_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        buy: BuyItem,
    ) {
        debug!(
            "BuyItem: item={} qty={} muid={} from {:?}",
            buy.item_id, buy.quantity, buy.muid, buy.vendor_guid
        );

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();
        let vendor_slot = match vendor_buy_muid_to_cpp_slot(buy.muid) {
            Some(slot) => slot,
            None => return,
        };

        // ── Get vendor NPC entry from creature GUID ──
        let vendor_entry = match self.mutate_world_creature(buy.vendor_guid, |c| c.entry()) {
            Some(entry) => entry,
            None => {
                warn!("BuyItem: vendor {:?} not in creatures", buy.vendor_guid);
                self.send_buy_error(
                    BuyResult::DistanceTooFar,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };

        let vendor_catalog = self.vendor_catalog_persistence_port_like_cpp();

        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return;
        };
        if let Some(store) = condition_store.as_ref() {
            let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            };
            let player_condition_object = self.build_condition_player_object_like_cpp();
            let vendor_condition_object =
                self.build_condition_creature_object_like_cpp(buy.vendor_guid);
            let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                .as_ref()
                .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                .unwrap_or((None, None));
            if !wow_conditions::is_vendor_item_conditions_with_snapshots_like_cpp(
                store.as_ref(),
                vendor_entry,
                buy.item_id as u32,
                player_condition_object.as_ref(),
                vendor_object,
                player_unit_snapshot,
                self.condition_player_snapshot_like_cpp(),
                vendor_unit_snapshot,
                player_condition_store.as_deref(),
                player_condition_context.as_context(self),
            ) {
                warn!(
                    "BuyItem: conditions not met for creature entry {} item {}",
                    vendor_entry, buy.item_id
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            }
        }

        if buy.item_type == ItemVendorType::Currency as i32 {
            self.purchase_vendor_currency(
                &buy,
                player_guid,
                map_id,
                vendor_entry,
                vendor_slot,
                &vendor_catalog,
                &player_condition_store,
                &player_condition_context,
            )
            .await;
            return;
        }

        let Some(purchase) = self
            .purchase_vendor_item(
                item_guid_generator,
                &buy,
                player_guid,
                map_id,
                vendor_entry,
                vendor_slot,
                &vendor_catalog,
                &condition_store,
                &player_condition_store,
                &player_condition_context,
            )
            .await
        else {
            return;
        };
        self.publish_vendor_item_purchase(
            item_guid_generator,
            &buy,
            player_guid,
            map_id,
            purchase,
        )
        .await;
    }
}
