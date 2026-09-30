//! Vendor currency purchases, including admission, durable outcome and publication.

use super::*;

impl WorldSession {
    pub(super) async fn purchase_vendor_currency(
        &mut self,
        buy: &BuyItem,
        player_guid: ObjectGuid,
        map_id: u16,
        vendor_entry: u32,
        vendor_slot: u32,
        vendor_catalog: &Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>>,
        player_condition_store: &Option<Arc<PlayerConditionStore>>,
        player_condition_context: &crate::session::RepresentedPlayerConditionContextLikeCpp,
    ) {
        if !vendor_currency_type_is_known(
            self.currency_types_store().map(|store| store.as_ref()),
            buy.item_id as u32,
        ) {
            self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
            return;
        }

        let quantity = vendor_buy_currency_packet_quantity_to_cpp_count(buy.quantity);
        let vendor_item = match self
            .resolve_vendor_buy_item_by_cpp_slot(
                vendor_catalog.as_deref(),
                vendor_entry,
                vendor_slot,
                buy.item_id as u32,
            )
            .await
        {
            Some(item) if item.item_type == ItemVendorType::Currency as i32 => item,
            _ => {
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.item_id as u32,
                );
                return;
            }
        };

        if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
            vendor_item.player_condition_id,
            player_condition_store.as_deref(),
            player_condition_context.as_context(self),
        ) {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }

        if let Some(result) =
            vendor_buy_currency_quantity_block_result(vendor_item.max_count, quantity)
        {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }

        if vendor_item.extended_cost == 0 {
            self.send_buy_error(BuyResult::CantFindItem, None, buy.item_id as u32);
            return;
        }

        if let Some(result) = vendor_buy_extended_cost_block_result(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            self.currency_types_store().map(|store| store.as_ref()),
            |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
            |currency_id, amount| self.has_currency(currency_id, amount),
            true,
            vendor_item.extended_cost,
            vendor_item.max_count,
            quantity,
        ) {
            match result {
                VendorExtendedCostBlock::Equip(result) => {
                    self.send_equip_error(result, None, None, 0, 0);
                }
                VendorExtendedCostBlock::Buy(result) => {
                    self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
                }
                VendorExtendedCostBlock::Silent => {}
            }
            // C++ BuyItemFromVendorSlot returns for every failed extended
            // cost preflight before it derives or commits any costs.
            return;
        }

        let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.max_count,
            quantity,
        );
        let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.max_count,
            quantity,
        );
        let vendor_trade_port = match self.vendor_trade_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let mut item_turnin_changes = Vec::new();
        for &(item_id, amount) in &extended_cost_item_costs {
            let Some(mut changes) =
                self.plan_destroy_item_count_direct_inventory(item_id, amount)
            else {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            };
            item_turnin_changes.append(&mut changes);
        }
        let Some(mut planned_currencies) = self.player_currencies_like_cpp() else {
            return;
        };
        let currency_gain = match self.plan_add_currency_vendor_like_cpp(
            &mut planned_currencies,
            buy.item_id as u32,
            quantity,
        ) {
            Ok(delta) => delta,
            Err(()) => {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            }
        };
        for &(currency_id, amount) in &extended_cost_currency_costs {
            if i32::try_from(amount).is_err()
                || !wow_entities::plan_remove_currency_like_cpp(
                    &mut planned_currencies,
                    currency_id,
                    amount,
                )
            {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            }
        }

        let currency_save = self.plan_player_currency_save_like_cpp(
            player_guid.counter() as u64,
            &mut planned_currencies,
        );

        // C++ mutates currency plus extended-cost turn-ins in one
        // serialized Player turn. Rust crosses SQL here, so retain the
        // same cancellation/unknown-COMMIT quarantine used by purchases
        // that also change money. Equal money sentinels deliberately make
        // an ambiguous result indeterminate: the money row cannot prove
        // whether these currency/item statements committed.
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let Some(money_marker) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let persistence_request =
            wow_persistence::VendorTradePersistenceRequestLikeCpp::CurrencyPurchase(
                wow_persistence::VendorCurrencyPurchasePersistenceLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_before: money_marker,
                    money_after: money_marker,
                    item_turnins: super::items::item_turnin_persistence_rows_like_cpp(
                        player_guid,
                        &item_turnin_changes,
                    ),
                    currency_save,
                },
            );
        let Some(money_persistence) = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                vendor_trade_port.persist_vendor_trade_like_cpp(persistence_request),
                money_marker,
                money_marker,
                "vendor currency purchase",
            )
            .await
        else {
            warn!("BuyItem: currency vendor transaction did not commit");
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.item_id as u32,
            );
            return;
        };

        // Publish the entire committed state before reopening payout/save
        // admission. No await may split durable success from runtime.
        if !self.set_player_currencies_like_cpp(planned_currencies) {
            self.kick("canonical Player currency owner became unavailable after vendor COMMIT");
            return;
        }
        self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
        drop(money_persistence);

        if let Some(delta) = currency_gain {
            let (Some(quantity), Some(amount)) = (
                i32::try_from(delta.quantity).ok(),
                i32::try_from(delta.amount).ok(),
            ) else {
                return;
            };
            let mut packet =
                SetCurrency::vendor_gain(delta.currency_id as i32, quantity, amount);
            packet.weekly_quantity = delta
                .weekly_quantity
                .and_then(|value| i32::try_from(value).ok());
            packet.max_quantity = delta
                .max_quantity
                .and_then(|value| i32::try_from(value).ok());
            packet.total_earned = delta
                .total_earned
                .and_then(|value| i32::try_from(value).ok());
            packet.suppress_chat_log = delta.suppress_chat_log;
            self.send_packet(&packet);
        }
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
        return;
    }
}
