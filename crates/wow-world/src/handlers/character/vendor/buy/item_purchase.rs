//! Vendor item admission and committed runtime publication under the money guard.

use super::*;

impl WorldSession {
    pub(super) async fn purchase_vendor_item(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        buy: &BuyItem,
        player_guid: ObjectGuid,
        map_id: u16,
        vendor_entry: u32,
        vendor_slot: u32,
        vendor_catalog: &Option<Arc<dyn wow_persistence::VendorCatalogPersistencePortLikeCpp>>,
        condition_store: &Option<Arc<ConditionEntriesByTypeStore>>,
        player_condition_store: &Option<Arc<PlayerConditionStore>>,
        player_condition_context: &crate::session::RepresentedPlayerConditionContextLikeCpp,
    ) -> Option<VendorItemPurchasePublication> {
        if buy.item_type != ItemVendorType::Item as i32 {
            warn!("BuyItem: unsupported item type {}", buy.item_type);
            return None;
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
                    return None;
                }
            };

        let vendor_trade_port = match self.vendor_trade_persistence_port_like_cpp() {
            Some(port) => port,
            None => return None,
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
                return None;
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
            return None;
        }
        if condition_store.is_none()
            && let Some(result) = vendor_conditions_block_result(vendor_item.has_vendor_conditions)
        {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return None;
        }
        if let Some(result) = vendor_buy_player_condition_block_result_like_cpp(
            vendor_item.player_condition_id,
            player_condition_store.as_deref(),
            player_condition_context.as_context(self),
        ) {
            self.send_equip_error(result, None, None, 0, 0);
            return None;
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
            return None;
        }
        if let Some(result) = vendor_buy_required_reputation_block_result(
            required_reputation_faction,
            required_reputation_rank,
            -1,
        ) {
            self.send_buy_error(result, Some(buy.vendor_guid), buy.item_id as u32);
            return None;
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
            return None;
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
            return None;
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
            return None;
        };
        if admission_money < buy_price {
            self.send_buy_error(
                BuyResult::NotEnoughtMoney,
                Some(buy.vendor_guid),
                buy.muid as u32,
            );
            return None;
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
                return None;
            }
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return None;
        }
        let quest_log_item_id = self
            .quest_source_item_quest_log_item_id_like_cpp(buy.item_id as u32)
            .await;

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return None;
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
            return None;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return None;
        };
        let Some(old_gold) = self.resolved_player_money_like_cpp() else {
            return None;
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
                return None;
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
                    return None;
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
                    return None;
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
                return None;
            };
            item_turnin_changes.append(&mut changes);
        }
        let Some(mut planned_currencies) = self.player_currencies_like_cpp() else {
            return None;
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
                return None;
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
            return None;
        };

        // The SQL transaction also owns the turn-ins, returned inventory
        // stacks, currencies and refund metadata. Publish all of that runtime
        // state synchronously before reopening payout admission; cancelling
        // the handler after COMMIT must not leave runtime at the pre-buy state.
        if !self.stage_player_money_change_like_cpp(old_gold, new_gold) {
            self.kick(
                "canonical Player money owner became unavailable after vendor purchase COMMIT",
            );
            return None;
        }
        self.apply_item_turnin_changes(player_guid, map_id, &item_turnin_changes);
        if !self.set_player_currencies_like_cpp(planned_currencies) {
            self.kick("canonical Player currency owner became unavailable after vendor COMMIT");
            return None;
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
            return None;
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
            return None;
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

        Some(VendorItemPurchasePublication {
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
        })
    }
}
