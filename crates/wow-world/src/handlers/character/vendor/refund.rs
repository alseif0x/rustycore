//! refund for the existing vendor owner.

use super::*;

impl WorldSession {

    /// Handle CMSG_ITEM_PURCHASE_REFUND.
    ///
    /// C++ ref: `ItemHandler.HandleItemRefund` -> `Player::RefundItem`.
    pub async fn handle_item_purchase_refund_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        refund: ItemPurchaseRefund,
    ) {
        const REFUND_RESULT_OK: u8 = 0;
        const REFUND_RESULT_ERR_GENERIC: u8 = 10;

        #[derive(Debug, Clone)]
        struct PlannedNewStack {
            slot: u8,
            entry_id: u32,
            count: u32,
            max_durability: u32,
        }

        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let map_id = self.player_map_id_like_cpp();

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return;
        };
        let Some((refund_slot, refund_inv_item)) = inventory_items
            .iter()
            .find(|(_, item)| item.guid == refund.item_guid)
            .map(|(&slot, item)| (slot, item.clone()))
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} not in inventory",
                refund.item_guid
            );
            return;
        };

        let Some(refund_item) = self.resolved_inventory_item_object_like_cpp(refund.item_guid)
        else {
            warn!(
                "ItemPurchaseRefund: item {:?} missing runtime object",
                refund.item_guid
            );
            return;
        };

        if self.is_active_loot_guid(refund.item_guid)
            || item_is_currently_looted_like_cpp(&refund_item)
        {
            return;
        }
        if !refund_item.is_refundable() {
            return;
        }

        let vendor_trade_port = match self.vendor_trade_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs() as i64)
            .unwrap_or(0);

        if refund_item.is_refund_expired_at(now_secs)
            || refund_item.refund_recipient() != player_guid
        {
            let new_flags = refund_item.item_flags_bits() & !ItemFieldFlags::REFUNDABLE.bits();
            let outcome = vendor_trade_port
                .clear_refund_metadata_like_cpp(
                    wow_persistence::VendorRefundCleanupPersistenceLikeCpp {
                        item_guid: refund_inv_item.db_guid,
                        flags_after: new_flags,
                    },
                )
                .await;
            if !matches!(
                outcome,
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. }
            ) {
                warn!(
                    ?outcome,
                    "ItemPurchaseRefund: refund cleanup transaction failed"
                );
                return;
            }

            let _ = self.apply_inventory_item_object_updates_like_cpp(
                refund.item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetNotRefundable],
            );
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: refund.item_guid,
            });

            if refund_item.is_refund_expired_at(now_secs) {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: None,
                });
            }
            return;
        }

        let Some(extended_cost) = self
            .item_extended_cost_store()
            .and_then(|store| store.get(refund_item.paid_extended_cost()))
            .copied()
        else {
            return;
        };

        let contents = crate::handlers::entities::item_purchase_contents_from_extended_cost(
            &extended_cost,
            refund_item.paid_money(),
        );

        let mut item_costs = Vec::new();
        for i in 0..5 {
            let item_id = extended_cost.item_id[i] as u32;
            let count = extended_cost.item_count[i] as u32;
            if item_id != 0 && count != 0 {
                item_costs.push((item_id, count));
            }
        }

        let mut currency_costs = Vec::new();
        for i in 0..5 {
            let season_earned = match i {
                0 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_1),
                1 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_2),
                2 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3),
                3 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_4),
                4 => extended_cost
                    .flags
                    .contains(ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_5),
                _ => false,
            };
            if season_earned {
                continue;
            }
            let currency_id = extended_cost.currency_id[i] as u32;
            let count = extended_cost.currency_count[i] as u32;
            if currency_id != 0 && count != 0 {
                currency_costs.push((currency_id, count));
            }
        }

        let mut planned_existing_counts =
            std::collections::HashMap::<u8, (ObjectGuid, u64, u32)>::new();
        let mut planned_new_stacks = Vec::<PlannedNewStack>::new();
        for &(entry_id, count) in &item_costs {
            let (store_result, store_dest, _) =
                match self.plan_store_new_direct_inventory_item(entry_id, count) {
                    Some(plan) => plan,
                    None => {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                };
            if store_result != InventoryResult::Ok {
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: Some(contents),
                });
                return;
            }

            for dest in store_dest {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }

                let max_stack = self
                    .item_storage_template(entry_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
                    .max(1);

                if let Some(existing) = self.resolved_inventory_item_like_cpp(slot) {
                    let Some(existing_object) =
                        self.resolved_inventory_item_object_like_cpp(existing.guid)
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let base_count = planned_existing_counts
                        .get(&slot)
                        .map(|(_, _, count)| *count)
                        .unwrap_or_else(|| existing_object.count());
                    let new_count = base_count.saturating_add(dest.count);
                    if existing.entry_id != entry_id || new_count > max_stack {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_existing_counts
                        .insert(slot, (existing.guid, existing.db_guid, new_count));
                    continue;
                }

                if let Some(new_stack) = planned_new_stacks
                    .iter_mut()
                    .find(|stack| stack.slot == slot)
                {
                    if new_stack.entry_id == entry_id
                        && new_stack.count.saturating_add(dest.count) <= max_stack
                    {
                        new_stack.count = new_stack.count.saturating_add(dest.count);
                        continue;
                    }

                    let backpack_end =
                        INVENTORY_SLOT_ITEM_START.saturating_add(INVENTORY_DEFAULT_SIZE);
                    let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
                        return;
                    };
                    let Some(alt_slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                        !inventory_items.contains_key(slot)
                            && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                    }) else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    let Some((InventoryResult::Ok, alt_dest, _)) = self
                        .plan_store_new_direct_inventory_item_at(
                            entry_id,
                            dest.count,
                            u8::from(INVENTORY_SLOT_BAG_0),
                            alt_slot,
                        )
                    else {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    };
                    if alt_dest.len() != 1 || (alt_dest[0].pos & 0x00FF) as u8 != alt_slot {
                        self.send_packet(&ItemPurchaseRefundResult {
                            item_guid: refund.item_guid,
                            result: REFUND_RESULT_ERR_GENERIC,
                            contents: Some(contents),
                        });
                        return;
                    }
                    planned_new_stacks.push(PlannedNewStack {
                        slot: alt_slot,
                        entry_id,
                        count: dest.count,
                        max_durability: self.item_template_max_durability(entry_id),
                    });
                    continue;
                }

                planned_new_stacks.push(PlannedNewStack {
                    slot,
                    entry_id,
                    count: dest.count,
                    max_durability: self.item_template_max_durability(entry_id),
                });
            }
        }

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let money_gain = player_money_gain_like_cpp(old_money, refund_item.paid_money());
        let money_overflow = money_gain.is_none();
        let new_gold = money_gain.unwrap_or(old_money);
        let mut created_new_stacks = Vec::new();
        let mut persistence_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) = self.allocate_item_instance_guids_with_generator_like_cpp(
                item_guid_generator,
                planned_new_stacks.len(),
            ) else {
                warn!(
                    count = planned_new_stacks.len(),
                    "ItemPurchaseRefund: process-wide item GUID allocator is unavailable"
                );
                self.send_packet(&ItemPurchaseRefundResult {
                    item_guid: refund.item_guid,
                    result: REFUND_RESULT_ERR_GENERIC,
                    contents: Some(contents),
                });
                return;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                persistence_new_stacks.push(
                    wow_persistence::VendorRefundReturnedStackPersistenceLikeCpp {
                        item_guid: db_guid,
                        item_entry: stack.entry_id,
                        owner_guid: player_guid.counter() as u64,
                        count: stack.count,
                        durability: stack.max_durability,
                        inventory_slot: stack.slot,
                    },
                );

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return;
        };
        let mut currency_deltas = Vec::new();
        for &(currency_id, amount) in &currency_costs {
            match self.add_currency_item_refund(currency_id, amount) {
                Ok(Some(delta)) => currency_deltas.push(delta),
                Ok(None) => {}
                Err(()) => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    self.send_packet(&ItemPurchaseRefundResult {
                        item_guid: refund.item_guid,
                        result: REFUND_RESULT_ERR_GENERIC,
                        contents: Some(contents),
                    });
                    return;
                }
            }
        }
        let Some(mut persisted_currencies) = self.player_currencies_like_cpp() else {
            self.set_player_currencies_like_cpp(currency_snapshot);
            return;
        };
        let currency_save = self.plan_player_currency_save_like_cpp(
            player_guid.counter() as u64,
            &mut persisted_currencies,
        );
        if !self.set_player_currencies_like_cpp(persisted_currencies) {
            self.set_player_currencies_like_cpp(currency_snapshot);
            return;
        }
        let persistence_request = wow_persistence::VendorTradePersistenceRequestLikeCpp::Refund(
            wow_persistence::VendorRefundPersistenceLikeCpp {
                player_guid: player_guid.counter() as u64,
                refunded_item_guid: refund_inv_item.db_guid,
                money_before: old_money,
                money_after: new_gold,
                existing_stacks: planned_existing_counts
                    .values()
                    .map(|&(_, item_guid, new_count)| {
                        wow_persistence::VendorExistingStackPersistenceLikeCpp {
                            item_guid,
                            new_count,
                        }
                    })
                    .collect(),
                new_stacks: persistence_new_stacks,
                currency_save,
            },
        );
        let Some(money_persistence) = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                vendor_trade_port.persist_vendor_trade_like_cpp(persistence_request),
                old_money,
                new_gold,
                "vendor item purchase refund",
            )
            .await
        else {
            self.set_player_currencies_like_cpp(currency_snapshot);
            warn!("ItemPurchaseRefund: refund transaction did not commit");
            self.send_packet(&ItemPurchaseRefundResult {
                item_guid: refund.item_guid,
                result: REFUND_RESULT_ERR_GENERIC,
                contents: Some(contents),
            });
            return;
        };

        // Refund COMMIT covers money, currencies, destruction of the refunded
        // item, and every restored stack. Publish the corresponding runtime
        // inventory before the guard opens or an await permits cancellation.
        if !self.stage_player_money_change_like_cpp(old_money, new_gold) {
            self.kick("canonical Player money owner became unavailable after item-refund COMMIT");
            return;
        }
        self.remove_inventory_item_like_cpp(refund_slot);
        self.remove_inventory_item_object(refund.item_guid);

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
            );
        }

        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                ItemContext::None,
                stack.slot,
            );
            self.insert_inventory_item_object(item_object);
        }
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        if money_overflow {
            self.send_equip_error(InventoryResult::TooMuchGold, None, None, 0, 0);
        }

        self.send_packet(&ItemPurchaseRefundResult {
            item_guid: refund.item_guid,
            result: REFUND_RESULT_OK,
            contents: Some(contents),
        });
        self.send_packet(&ItemExpirePurchaseRefund {
            item_guid: refund.item_guid,
        });

        for delta in currency_deltas {
            let Some(type_id) = i32::try_from(delta.currency_id).ok() else {
                continue;
            };
            let Some(quantity) = i32::try_from(delta.quantity).ok() else {
                continue;
            };
            let Some(amount) = i32::try_from(delta.amount).ok() else {
                continue;
            };
            self.send_packet(&SetCurrency::item_refund_gain(
                type_id,
                quantity,
                amount,
                delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                delta.suppress_chat_log,
            ));
        }

        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: 0,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: 0,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for &(item_guid, _, new_count) in planned_existing_counts.values() {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
        }

        let mut changed_slots = Vec::new();
        changed_slots.push((refund_slot, ObjectGuid::EMPTY));
        changed_slots.extend(
            created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid)),
        );
        self.send_player_values_update_from_entity_bridge(
            &changed_slots,
            &[],
            &[],
            &[],
            Some(new_gold),
        );

        if refund_slot < 19 {
            self.send_stat_update();
        }
    }
}
