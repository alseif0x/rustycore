//! Ordered instance and realm publication of a committed vendor item purchase.

use super::*;

impl WorldSession {
    pub(super) async fn publish_vendor_item_purchase(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        buy: &BuyItem,
        player_guid: ObjectGuid,
        map_id: u16,
        purchase: VendorItemPurchasePublication,
    ) {
        use wow_packet::packets::update::{ItemCreateData, UpdateObject};

        let VendorItemPurchasePublication {
            new_stacks,
            existing_updates,
            changed_slots,
            collection_updates,
            purchased_item_plan,
            extended_cost_currency_costs,
            store_dest,
            quantity,
            buy_price,
            new_gold,
            max_durability,
            new_quantity,
        } = purchase;

        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        for &(currency_id, amount) in &extended_cost_currency_costs {
            let Some(quantity) = self
                .player_currency_quantity(currency_id)
                .and_then(|quantity| i32::try_from(quantity).ok())
            else {
                continue;
            };
            let Some(amount) = i32::try_from(amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::vendor_loss(
                currency_id as i32,
                quantity,
                amount,
            ));
        }

        info!(
            "BuyItem: player {:?} bought item {} across {} destination(s) for {} copper (remaining: {})",
            player_guid,
            buy.item_id,
            store_dest.len(),
            buy_price,
            new_gold
        );

        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(
                    |&(_, _, item_guid, stack_count, item_flags)| ItemCreateData {
                        item_guid,
                        entry_id: buy.item_id,
                        owner_guid: player_guid,
                        contained_in: player_guid,
                        stack_count,
                        dynamic_flags: item_flags,
                        durability: max_durability,
                        max_durability,
                        random_properties_seed: 0,
                        random_properties_id: 0,
                        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                        gems: Vec::new(),
                        context: ItemContext::Vendor as u8,
                        container_slots: 0,
                        container_item_guids: [ObjectGuid::EMPTY; 36],
                    },
                )
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }

        for &(_, item_guid, _, new_count) in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        // C++ `StoreNewItem` publishes item object changes on the instance
        // socket before `_StoreOrEquipNewItem` emits its two realm-routed
        // result packets. Preserve that physical cross-socket order.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("vendor socket ordering fence failed after durable item purchase");
            return;
        }
        self.send_packet_realm(&BuySucceeded {
            vendor_guid: buy.vendor_guid,
            muid: buy.muid,
            new_quantity,
            quantity_bought: quantity as i32,
        });
        self.send_new_item_plan(&purchased_item_plan);
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("vendor socket ordering fence failed after durable item purchase");
            return;
        }

        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            vendor_buy_coinage_update_like_cpp(buy_price, new_gold),
        );
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
    }
}
