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
        use wow_packet::packets::update::{ItemCreateData, UpdateObject};

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

        if buy.item_type != ItemVendorType::Item as i32 {
            warn!("BuyItem: unsupported item type {}", buy.item_type);
            return;
        }

        // ── Validate: player alive ──
        let quantity = vendor_buy_packet_quantity_to_cpp_count(buy.quantity);
        let (store_bag, store_slot) =
            match vendor_buy_direct_inventory_destination(player_guid, &buy) {
                Some(destination) => destination,
                None => {
                    warn!(
                        "BuyItem: rejected slot {} above C++ MAX_BAG_SIZE {}",
                        buy.slot, MAX_BAG_SIZE
                    );
                    return;
                }
            };

        let vendor_trade_port = match self.vendor_trade_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let vendor_item = match self
            .resolve_vendor_buy_item_by_cpp_slot(
                vendor_catalog.as_deref(),
                vendor_entry,
                vendor_slot,
                buy.item_id as u32,
            )
            .await
        {
            Some(item) if item.item_type == ItemVendorType::Item as i32 => item,
            _ => {
                warn!(
                    "BuyItem: vendor slot {} item {} not found for vendor {}",
                    vendor_slot, buy.item_id, vendor_entry
                );
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        let sparse_template = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(buy.item_id as u32));
        let allowable_class = sparse_template.map(|template| template.allowable_class);
        let bonding = sparse_template.map(|template| template.bonding);
        let flags2 = sparse_template.map(|template| template.flags[1]);
        let required_reputation_faction =
            sparse_template.map(|template| template.required_reputation_faction);
        let required_reputation_rank =
            sparse_template.map(|template| template.required_reputation_rank);
        if let Some(block) = vendor_buy_template_block_result(
            allowable_class,
            bonding,
            flags2,
            self.player_class_like_cpp(),
            self.player_race_like_cpp(),
            self.security > 0,
        ) {
            match block {
                VendorBuyTemplateBlock::BuyError(result) => {
                    self.send_buy_error(result, None, buy.item_id as u32);
                }
                VendorBuyTemplateBlock::Silent => {}
            }
            return;
        }
        if condition_store.is_none()
            && let Some(result) = vendor_conditions_block_result(vendor_item.has_vendor_conditions)
        {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
            vendor_item.player_condition_id,
            player_condition_store.as_deref(),
            player_condition_context.as_context(self),
        ) {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }
        let vendor_current_count = self.vendor_item_current_count(
            buy.vendor_guid,
            vendor_item.item_id,
            vendor_item.max_count,
            vendor_item.incr_time,
            vendor_item.buy_count,
        );
        if vendor_item.max_count != 0 && vendor_current_count < quantity {
            self.send_buy_error(
                BuyResult::ItemAlreadySold,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }
        if let Some(result) = vendor_buy_required_reputation_block_result(
            required_reputation_faction,
            required_reputation_rank,
            -1,
        ) {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return;
        }
        if let Some(result) = vendor_buy_extended_cost_block_result(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            self.currency_types_store().map(|store| store.as_ref()),
            |item_id, amount| self.has_item_count_direct_inventory(item_id, amount),
            |currency_id, amount| self.has_currency(currency_id, amount),
            true,
            vendor_item.extended_cost,
            vendor_item.buy_count,
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
            return;
        }
        let extended_cost_item_costs = vendor_buy_extended_cost_item_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        let extended_cost_currency_costs = vendor_buy_extended_cost_currency_costs(
            self.item_extended_cost_store().map(|store| store.as_ref()),
            vendor_item.extended_cost,
            vendor_item.buy_count,
            quantity,
        );
        if let Some(result) = vendor_buy_direct_store_block_result(store_bag, store_slot, quantity)
        {
            self.send_equip_error(result, None, None, 0, 0);
            return;
        }

        let (quantity, buy_price): (u32, u64) =
            vendor_buy_quantity_and_price(vendor_item.buy_price, vendor_item.buy_count, quantity);
        let max_durability = vendor_item.max_durability;
        let refund_template = self.item_storage_template(buy.item_id as u32);
        let creates_refund_metadata = vendor_list_item_refundable(
            refund_template.as_ref().map(|template| template.flags),
            refund_template
                .as_ref()
                .map(|template| template.max_stack_size),
            vendor_item.extended_cost as i32,
        );

        // ── Check gold ──
        let Some(admission_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        if admission_money < buy_price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        }

        let (store_result, store_dest, _) = match self.plan_store_new_direct_inventory_item_at(
            buy.item_id as u32,
            quantity,
            store_bag,
            store_slot,
        ) {
            Some(plan) => plan,
            None => {
                self.send_buy_error(
                    BuyResult::CantFindItem,
                    Some(buy.vendor_guid),
                    buy.muid as u32,
                );
                return;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return;
        }
        let quest_log_item_id = self
            .quest_source_item_quest_log_item_id_like_cpp(buy.item_id as u32)
            .await;

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return;
        };
        let new_item_count = store_dest
            .iter()
            .filter(|dest| {
                let slot = (dest.pos & 0x00FF) as u8;
                !inventory_items.contains_key(&slot)
            })
            .count();
        let Some(allocated_new_item_guids) = self
            .allocate_item_instance_guids_with_generator_like_cpp(
                item_guid_generator,
                new_item_count,
            )
        else {
            warn!(
                count = new_item_count,
                "BuyItem: process-wide item GUID allocator is unavailable"
            );
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let Some(old_gold) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let new_gold = old_gold.saturating_sub(buy_price);

        let mut existing_updates = Vec::new();
        let mut new_stacks = Vec::new();
        let mut persistence_new_stacks = Vec::new();
        for dest in &store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                warn!(
                    "BuyItem: direct inventory plan produced unsupported bag {}",
                    bag
                );
                self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                return;
            }

            if let Some(inv_item) = self.resolved_inventory_item_like_cpp(slot) {
                let Some(existing_item) =
                    self.resolved_inventory_item_object_like_cpp(inv_item.guid)
                else {
                    warn!("BuyItem: missing runtime item object for slot {}", slot);
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.muid as u32,
                    );
                    return;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                existing_updates.push((slot, inv_item.guid, inv_item.db_guid, new_count));
            } else {
                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!("BuyItem: preallocated item GUID count did not match store plan");
                    self.send_buy_error(
                        BuyResult::CantFindItem,
                        Some(buy.vendor_guid),
                        buy.muid as u32,
                    );
                    return;
                };

                let item_flags =
                    vendor_stored_new_item_flags_like_cpp(refund_template.as_ref(), bag, slot);
                persistence_new_stacks.push(
                    wow_persistence::VendorPurchasedStackPersistenceLikeCpp {
                        item_guid: db_guid,
                        item_entry: buy.item_id as u32,
                        owner_guid: player_guid.counter() as u64,
                        count: dest.count,
                        durability: max_durability,
                        flags: item_flags,
                        random_properties_id: 0,
                        property_seed: 0,
                        context: ItemContext::Vendor as u8,
                        inventory_slot: slot,
                    },
                );

                new_stacks.push((slot, db_guid, item_guid, dest.count, item_flags));
            }
        }
        let refund_item_db_guid = creates_refund_metadata
            .then(|| {
                new_stacks.last_mut().map(|stack| {
                    stack.4 |= ItemFieldFlags::REFUNDABLE.bits();
                    (stack.1, stack.4)
                })
            })
            .flatten();
        let mut item_turnin_changes = Vec::new();
        for &(item_id, amount) in &extended_cost_item_costs {
            let Some(mut changes) = self.plan_destroy_item_count_direct_inventory(item_id, amount)
            else {
                self.send_equip_error(InventoryResult::VendorMissingTurnins, None, None, 0, 0);
                return;
            };
            item_turnin_changes.append(&mut changes);
        }
        let Some(mut planned_currencies) = self.player_currencies_like_cpp() else {
            return;
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
        let persistence_request =
            wow_persistence::VendorTradePersistenceRequestLikeCpp::ItemPurchase(
                wow_persistence::VendorItemPurchasePersistenceLikeCpp {
                    player_guid: player_guid.counter() as u64,
                    money_before: old_gold,
                    money_after: new_gold,
                    existing_stacks: existing_updates
                        .iter()
                        .map(|&(_, _, item_guid, new_count)| {
                            wow_persistence::VendorExistingStackPersistenceLikeCpp {
                                item_guid,
                                new_count,
                            }
                        })
                        .collect(),
                    new_stacks: persistence_new_stacks,
                    refund_metadata: refund_item_db_guid.map(|(item_guid, flags_after)| {
                        wow_persistence::VendorRefundMetadataPersistenceLikeCpp {
                            item_guid,
                            player_guid: player_guid.counter() as u64,
                            paid_money: buy_price,
                            paid_extended_cost: vendor_item.extended_cost as u16,
                            flags_after,
                        }
                    }),
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
                old_gold,
                new_gold,
                "vendor item purchase",
            )
            .await
        else {
            warn!("BuyItem: store transaction did not commit");
            self.send_buy_error(
                BuyResult::CantFindItem,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return;
        };

        // The SQL transaction also owns the turn-ins, returned inventory
        // stacks, currencies and refund metadata. Publish all of that runtime
        // state synchronously before reopening payout admission; cancelling
        // the handler after COMMIT must not leave runtime at the pre-buy state.
        if !self.stage_player_money_change_like_cpp(old_gold, new_gold) {
            self.kick(
                "canonical Player money owner became unavailable after vendor purchase COMMIT",
            );
            return;
        }
        self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
        if !self.set_player_currencies_like_cpp(planned_currencies) {
            self.kick("canonical Player currency owner became unavailable after vendor COMMIT");
            return;
        }
        for &(_, item_guid, _, new_count) in &existing_updates {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
            );
        }

        let inv_type = self.item_template_inventory_type(buy.item_id as u32);
        let mut collection_updates = Vec::new();
        for &(slot, db_guid, item_guid, stack_count, item_flags) in &new_stacks {
            self.insert_inventory_item_like_cpp(
                slot,
                crate::session::InventoryItem {
                    guid: item_guid,
                    entry_id: buy.item_id as u32,
                    db_guid,
                    inventory_type: inv_type,
                },
            );
            let mut item_object = self.make_inventory_item_object(
                item_guid,
                buy.item_id as u32,
                player_guid,
                stack_count,
                max_durability,
                ItemContext::Vendor,
                slot,
            );
            item_object.replace_all_item_flags(ItemFieldFlags::from_bits_retain(item_flags));
            if refund_item_db_guid.is_some_and(|(refund_db_guid, _)| refund_db_guid == db_guid) {
                item_object.set_refund_recipient(player_guid);
                item_object.set_paid_money(buy_price);
                item_object.set_paid_extended_cost(vendor_item.extended_cost as u32);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }

        let changed_slots: Vec<_> = new_stacks
            .iter()
            .map(|&(slot, _, item_guid, _, _)| (slot, item_guid))
            .collect();
        let Some(quantity_in_inventory) =
            self.represented_non_bank_item_count_like_cpp(buy.item_id as u32)
        else {
            return;
        };
        let purchased_item_plan = store_dest.last().and_then(|dest| {
            let slot = (dest.pos & 0x00FF) as u8;
            let item_guid = self.resolved_inventory_item_like_cpp(slot)?.guid;
            let item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
            let battle_pet_breed_data = item.get_modifier(ItemModifier::BattlePetBreedData);
            let modifications = item
                .data()
                .modifiers
                .iter()
                .enumerate()
                .filter_map(|(modifier_type, &value)| {
                    (value != 0).then_some(SendNewItemModifier {
                        value: value as i32,
                        modifier_type: modifier_type as u8,
                    })
                })
                .collect();
            Some(SendNewItemPlan {
                player_guid,
                item_guid,
                item_entry: item.object().entry(),
                item_instance: SendNewItemInstancePlan {
                    item_id: item.object().entry(),
                    random_properties_seed: item.data().property_seed,
                    random_properties_id: item.data().random_properties_id,
                    modifications,
                },
                slot: item.bag_slot(),
                slot_in_bag: if item.count() == quantity {
                    i16::from(item.slot())
                } else {
                    -1
                },
                quest_log_item_id,
                quantity,
                quantity_in_inventory,
                battle_pet_species_id: item.get_modifier(ItemModifier::BattlePetSpeciesId),
                battle_pet_breed_id: battle_pet_breed_data & 0x00FF_FFFF,
                battle_pet_breed_quality: ((battle_pet_breed_data >> 24) & 0xFF) as u8,
                battle_pet_level: item.get_modifier(ItemModifier::BattlePetLevel),
                pushed: true,
                created: false,
                display_text: SendNewItemDisplayText::Normal,
                dungeon_encounter_id: 0,
                is_encounter_loot: false,
                delivery: SendNewItemDelivery::Direct,
            })
        });
        let Some(purchased_item_plan) = purchased_item_plan else {
            // The durable purchase is already committed. Fail closed at the
            // packet boundary rather than fabricating an ItemPush GUID or
            // rolling runtime back out of sync with the database.
            warn!(
                item = buy.item_id,
                "BuyItem: committed item is missing from the published runtime inventory"
            );
            return;
        };
        let new_quantity = if vendor_item.max_count == 0 {
            -1
        } else {
            self.update_vendor_item_current_count(
                buy.vendor_guid,
                vendor_item.item_id,
                vendor_item.max_count,
                vendor_item.incr_time,
                vendor_item.buy_count,
                quantity,
            ) as i32
        };
        drop(money_persistence);

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
