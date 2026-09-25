//! Disenchant winner storage and claim publication helpers.

use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) async fn store_represented_disenchant_loot_winner_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
    ) -> bool {
        let Some(template) = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(entry.item_id))
        else {
            return false;
        };
        let Some((disenchant_id, _)) = self.item_disenchant_loot_with_catalogs_like_cpp(
            item_valuation,
            entry.item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        ) else {
            return false;
        };

        let disenchant_entries = self
            .generate_represented_disenchant_loot_template_entries_like_cpp(
                disenchant_id,
                winner_guid,
            )
            .await;
        if disenchant_entries.is_empty() {
            return false;
        }

        if self.player_guid() == Some(winner_guid) {
            return self
                .store_direct_disenchant_batch_with_generator_like_cpp(
                    item_guid_generator,
                    &disenchant_entries,
                    dungeon_encounter_id,
                    claim,
                    claim.map(|_| LootItemClaimCommitContextLikeCpp {
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        player_guid: winner_guid,
                        free_for_all: entry.flags.freeforall,
                    }),
                )
                .await;
        }

        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                disenchant_entries,
                true,
                claim.cloned(),
            )
            .await
        {
            MasterLootGiveResult::Stored => true,
            MasterLootGiveResult::StoreFailed(error) => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    error,
                    "represented disenchant loot winner batch failed in target session"
                );
                false
            }
            MasterLootGiveResult::TargetMismatch => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    "represented disenchant loot winner target was not connected"
                );
                false
            }
        }
    }

    pub(in crate::handlers::loot) async fn store_direct_disenchant_batch_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entries: &[LootEntry],
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if loot_entries.is_empty()
            || loot_entries
                .iter()
                .any(|entry| entry.item_id == 0 || entry.quantity == 0)
        {
            return false;
        }
        let durable_item_fanout = match (claim, claim_commit_context) {
            (Some(claim), Some(context)) => {
                let Some(fanout) = self.prepare_durable_loot_item_fanout_like_cpp(claim, context)
                else {
                    return false;
                };
                Some(fanout)
            }
            (None, None) => None,
            _ => return false,
        };

        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let grant_count = loot_entries.len();
            let runtime_inventory_applied =
                claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = claim_commit_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(|(context, runtime_inventory_applied)| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid: context.owner_guid,
                            loot_list_id: context.loot_list_id,
                            player_guid: context.player_guid,
                            item_owner_auto_release: false,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                });
            let Ok(persistence) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    // Model the asynchronous commit boundary so cancellation
                    // regressions exercise the same ownership shape as SQL.
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    grants.fetch_add(grant_count, Ordering::SeqCst);
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(persistence.await, Ok(Ok(()))) {
                return false;
            }
            for (slot, entry) in loot_entries.iter().enumerate() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    entry,
                    0,
                    0,
                    u8::try_from(slot).unwrap_or(0),
                    entry.quantity,
                    entry.quantity,
                    false,
                    dungeon_encounter_id,
                );
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }

        let Some(inventory_persistence) = self.player_inventory_persistence_port_like_cpp() else {
            return false;
        };

        // `CanStoreNewItem`'s max-count checks must see all generated stacks
        // of the same material, not each temporary LootItem in isolation.
        let mut quantity_by_item = HashMap::<u32, u32>::new();
        for entry in loot_entries {
            let Some(total) = quantity_by_item
                .get(&entry.item_id)
                .copied()
                .unwrap_or(0)
                .checked_add(entry.quantity)
            else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            quantity_by_item.insert(entry.item_id, total);
        }
        for (item_id, count) in quantity_by_item {
            let Some((store_result, _, _)) =
                self.plan_store_new_direct_inventory_item(item_id, count)
            else {
                self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                return false;
            };
            if store_result != InventoryResult::Ok {
                self.send_equip_error(store_result, None, None, 0, 0);
                return false;
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        let mut planned_existing_stacks = Vec::<PlannedDisenchantExistingStack>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();
        let mut planned_grants = Vec::<PlannedDisenchantGrant>::new();

        for loot_entry in loot_entries {
            let random_properties = {
                let mut rng = self.represented_runtime_subrng_like_cpp();
                self.generate_loot_store_random_properties_with_rng_like_cpp(
                    loot_entry.item_id,
                    &mut rng,
                )
            };
            let max_stack = self
                .item_storage_template(loot_entry.item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);
            let mut remaining = loot_entry.quantity;
            let mut existing_pushes = Vec::new();
            let mut new_pushes = Vec::new();

            // Existing backpack stacks are consumed first, matching the
            // direct StoreNewItem path represented in this server.
            for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
                if remaining == 0 {
                    break;
                }
                let Some(existing) = self.resolved_inventory_item_like_cpp(slot) else {
                    continue;
                };
                if existing.entry_id != loot_entry.item_id {
                    continue;
                }
                let Some(existing_object) =
                    self.resolved_inventory_item_object_like_cpp(existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                if !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    &existing_object,
                ) {
                    continue;
                }

                let current_count = planned_existing_stacks
                    .iter()
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
                    .unwrap_or_else(|| existing_object.count());
                let added_count = max_stack.saturating_sub(current_count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                let new_count = current_count.saturating_add(added_count);
                if let Some(planned) = planned_existing_stacks
                    .iter_mut()
                    .find(|planned| planned.slot == slot)
                {
                    planned.new_count = new_count;
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        loot_entry.item_id,
                        slot,
                        &existing_object,
                    );
                    planned_existing_stacks.push(PlannedDisenchantExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
                }
                existing_pushes.push(PlannedDisenchantExistingPush {
                    slot,
                    item_guid: existing.guid,
                    added_count,
                    new_count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            // A second generated LootItem for the same material may continue
            // a new stack already planned earlier in this same transaction.
            for (stack_index, stack) in planned_new_stacks.iter_mut().enumerate() {
                if remaining == 0 {
                    break;
                }
                if stack.entry_id != loot_entry.item_id
                    || stack.random_properties_id != random_properties.id
                    || stack.random_properties_seed != random_properties.seed
                    || stack.item_context != loot_entry.item_context
                {
                    continue;
                }
                let added_count = max_stack.saturating_sub(stack.count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                stack.count = stack.count.saturating_add(added_count);
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count,
                    new_count: stack.count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            while remaining > 0 {
                let Some(slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                    self.resolved_inventory_items_like_cpp()
                        .is_some_and(|items| !items.contains_key(slot))
                        && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                }) else {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                };
                let count = remaining.min(max_stack);
                let stack_index = planned_new_stacks.len();
                planned_new_stacks.push(PlannedLootNewStack {
                    slot,
                    entry_id: loot_entry.item_id,
                    count,
                    max_durability: self.item_template_max_durability(loot_entry.item_id),
                    dynamic_flags: self
                        .stored_new_item_dynamic_flags_like_cpp(loot_entry.item_id, slot),
                    random_properties_id: random_properties.id,
                    random_properties_seed: random_properties.seed,
                    item_context: loot_entry.item_context,
                });
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count: count,
                    new_count: count,
                });
                remaining = remaining.saturating_sub(count);
            }

            planned_grants.push(PlannedDisenchantGrant {
                entry: loot_entry.clone(),
                random_properties,
                existing_pushes,
                new_pushes,
            });
        }

        let mut created_new_stacks = Vec::with_capacity(planned_new_stacks.len());
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) = self.allocate_item_instance_guids_with_generator_like_cpp(
                item_guid_generator,
                planned_new_stacks.len(),
            ) else {
                warn!(
                    count = planned_new_stacks.len(),
                    "disenchant item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let persistence_request =
            wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDisenchantBatch(
                wow_persistence::LootDisenchantBatchPersistenceLikeCpp {
                    existing_stacks: planned_existing_stacks
                        .iter()
                        .map(
                            |stack| wow_persistence::LootExistingStackPersistenceLikeCpp {
                                item_guid: stack.db_guid,
                                new_count: stack.new_count,
                                dynamic_flags: stack.flags_changed.then_some(stack.dynamic_flags),
                            },
                        )
                        .collect(),
                    new_stacks: created_new_stacks
                        .iter()
                        .map(|(stack, db_guid, _)| {
                            wow_persistence::LootNewStackPersistenceLikeCpp {
                                item_guid: *db_guid,
                                entry_id: stack.entry_id,
                                owner_guid: player_guid.counter() as u64,
                                count: stack.count,
                                max_durability: stack.max_durability,
                                dynamic_flags: stack.dynamic_flags,
                                random_properties_id: stack.random_properties_id,
                                random_properties_seed: stack.random_properties_seed,
                                item_context: stack.item_context,
                                slot: stack.slot,
                            }
                        })
                        .collect(),
                },
            );

        let runtime_inventory_applied =
            claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = claim_commit_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(|(context, runtime_inventory_applied)| {
                (
                    self.begin_durable_item_loot_persistence_like_cpp(),
                    DurableItemLootCompletionLikeCpp {
                        owner_guid: context.owner_guid,
                        loot_list_id: context.loot_list_id,
                        player_guid: context.player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: None,
                        durable_item_money_notified_amount: None,
                        durable_item_money_balance_applied: None,
                        item_fanout: durable_item_fanout.clone(),
                        runtime_inventory_applied,
                    },
                )
            });
        let persistence = match spawn_loot_item_persistence_worker_like_cpp(
            async move {
                inventory_persistence
                    .persist_inventory_mutation_like_cpp(persistence_request)
                    .await
            },
            claim.cloned(),
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "disenchant claim closed before persistence started");
                return false;
            }
        };
        match persistence.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                warn!(?error, "disenchant material batch transaction failed");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
            Err(error) => {
                warn!(?error, "disenchant material batch worker terminated");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        }

        for stack in &planned_existing_stacks {
            let mut updates = vec![ItemObjectUpdateLikeCpp::SetCount(stack.new_count)];
            if stack.flags_changed {
                updates.push(ItemObjectUpdateLikeCpp::ReplaceAllItemFlags(
                    ItemFieldFlags::from_bits_retain(stack.dynamic_flags),
                ));
            }
            let _ = self.apply_inventory_item_object_updates_like_cpp(stack.item_guid, &updates);
        }

        let mut collection_updates = Vec::new();
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
            let mut item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                loot_item_context(stack.item_context),
                stack.slot,
            );
            self.apply_stored_new_item_flags_like_cpp(stack.entry_id, stack.slot, &mut item_object);
            if stack.random_properties_id != 0 {
                item_object.set_random_properties_id(stack.random_properties_id);
            }
            if stack.random_properties_seed != 0 {
                item_object.set_property_seed(stack.random_properties_seed);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        if let Some(runtime_inventory_applied) = runtime_inventory_applied {
            runtime_inventory_applied.store(true, Ordering::Release);
        }

        for grant in &planned_grants {
            let quest_log_item_id = self
                .load_creature_item_template_addon_loot_metadata_like_cpp(grant.entry.item_id)
                .await
                .quest_log_item_id
                .try_into()
                .unwrap_or(0);
            let mut changed_quest_ids = self
                .apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                    grant.entry.item_id,
                    quest_log_item_id,
                    grant.entry.quantity,
                )
                .await;
            self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
                .await;
        }

        let map_id = self.player_map_id_like_cpp();
        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: stack.dynamic_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: stack.item_context,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }
        for stack in &planned_existing_stacks {
            let update = if stack.flags_changed {
                UpdateObject::item_stack_count_and_flags_update(
                    stack.item_guid,
                    map_id,
                    stack.new_count,
                    stack.dynamic_flags,
                )
            } else {
                UpdateObject::item_stack_count_update(stack.item_guid, map_id, stack.new_count)
            };
            self.send_packet(&update);
        }

        // C++ writes each material's item update on the instance connection
        // before `SendNewItem` routes its push result to the realm connection.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            let _ = self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            );
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        for grant in &planned_grants {
            for push in &grant.existing_pushes {
                self.send_loot_item_push_result(
                    player_guid,
                    push.item_guid,
                    &grant.entry,
                    grant.random_properties.id,
                    grant.random_properties.seed,
                    push.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
            for push in &grant.new_pushes {
                let (stack, _, item_guid) = &created_new_stacks[push.stack_index];
                self.send_loot_item_push_result(
                    player_guid,
                    *item_guid,
                    &grant.entry,
                    stack.random_properties_id,
                    stack.random_properties_seed,
                    stack.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
        }

        // `Loot::AutoStore` completes every realm-routed `SendNewItem` before
        // `LootRoll::Finish` emits the original slot removal on the instance
        // connection. Do not publish the later instance packets without the
        // writer acknowledgement; reconnect will reload the durable grant.
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        // C++ `Loot::AutoStore` performs `StoreNewItem` and `SendNewItem` for
        // every generated material. Only after `AutoStore` returns does
        // `LootRoll::Finish` call `NotifyItemRemoved` for the original loot.
        // SQL and the claim were already committed by the detached worker.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !created_new_stacks.is_empty() {
            let changed_slots = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect::<Vec<_>>();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
}
