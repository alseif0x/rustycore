//! Direct loot item storage and publication helpers.

use super::*;
use wow_entities::Item;
mod disenchant;

impl WorldSession {
    pub(in crate::handlers::loot) async fn store_direct_loot_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            None,
            None,
            None,
        )
        .await
    }

    /// Apply the item state established by C++ `Player::StoreNewItem` and
    /// `_StoreItem` before the item is persisted or sent to the client.
    fn apply_stored_new_item_flags_like_cpp(&self, item_id: u32, slot: u8, item: &mut Item) {
        if let Some(template) = self.item_storage_template(item_id) {
            item.set_bonding(template.bonding);
        }
        item.set_item_flag(ItemFieldFlags::NEW_ITEM);
        item.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
    }

    pub(in crate::handlers::loot) fn stored_new_item_dynamic_flags_like_cpp(
        &self,
        item_id: u32,
        slot: u8,
    ) -> u32 {
        let mut item = Item::new(0);
        self.apply_stored_new_item_flags_like_cpp(item_id, slot, &mut item);
        item.item_flags_bits()
    }

    /// C++ `_StoreItem` binds the destination object before incrementing an
    /// existing stack. Unlike `StoreNewItem`, that historical object must not
    /// acquire `ITEM_FIELD_FLAG_NEW_ITEM` merely because more items arrived.
    pub(in crate::handlers::loot) fn stored_existing_item_dynamic_flags_like_cpp(
        &self,
        item_id: u32,
        slot: u8,
        existing: &Item,
    ) -> u32 {
        let mut planned = existing.clone();
        if let Some(template) = self.item_storage_template(item_id) {
            planned.set_bonding(template.bonding);
        }
        planned.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
        planned.item_flags_bits()
    }

    pub(in crate::handlers::loot) async fn store_direct_loot_item_from_owner_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> bool {
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            item_guid_generator,
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            None,
            None,
        )
        .await
    }

    pub(in crate::handlers::loot) async fn store_direct_loot_item_with_source_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let item_id = loot_entry.item_id;
        let count = loot_entry.quantity;
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
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
        // C++ Loot::AutoStore validates CanStoreNewItem before StoreNewItem.
        // That ordering still applies when StoreNewItem later converts a
        // quest-bound Item into objective credit and returns nullptr.
        let Some((store_result, mut store_dest, _)) =
            self.plan_store_new_direct_inventory_item(item_id, count)
        else {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return false;
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return false;
        }
        let quest_log_item_id = self
            .load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
            .await
            .quest_log_item_id
            .try_into()
            .unwrap_or(0);
        let bound_objective_plan = self
            .plan_quest_source_item_bound_objective_persistence_like_cpp(
                item_id,
                quest_log_item_id,
                count,
            );
        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let materializes_inventory_item = bound_objective_plan.is_none();
            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
            let Ok(worker) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    if materializes_inventory_item {
                        grants.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(worker.await, Ok(Ok(()))) {
                return false;
            }
            if let Some(plan) = bound_objective_plan.as_ref() {
                let applied = self
                    .apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
                        item_guid_generator,
                        item_id,
                        quest_log_item_id,
                        count,
                    )
                    .await;
                debug_assert!(applied.as_ref().is_some_and(|result| result.no_grant));
                debug_assert!(plan.statuses.iter().all(|planned| {
                    self.quest_test_fixture_like_cpp
                        .player_quests
                        .get(&planned.quest_id)
                        .is_some_and(|actual| {
                            actual.status == planned.status
                                && actual.objective_counts == planned.objective_counts
                        })
                }));
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if bound_objective_plan.is_none() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    loot_entry,
                    0,
                    0,
                    0,
                    count,
                    count,
                    false,
                    dungeon_encounter_id,
                );
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }
        let Some(inventory_persistence) = self.player_inventory_persistence_port_like_cpp() else {
            return false;
        };
        if let Some(bound_objective_plan) = bound_objective_plan {
            let persistence_request =
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootQuestBoundProgress(
                    wow_persistence::LootQuestBoundProgressPersistenceLikeCpp {
                        owner_guid: player_guid.counter() as u64,
                        quest_statuses: self.represented_quest_status_persistence_rows_like_cpp(
                            &bound_objective_plan.statuses,
                        ),
                        stored_item_source: stored_item_loot_source.map(|item_guid| {
                            wow_persistence::StoredItemLootSourcePersistenceLikeCpp {
                                item_guid: item_guid.counter() as u64,
                                item_id,
                                count,
                                loot_list_id: u32::from(loot_entry.loot_list_id),
                            }
                        }),
                    },
                );

            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
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
                    warn!(
                        ?error,
                        "LootItem: quest-bound claim closed before persistence started"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            };
            match persistence.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    warn!(?error, "LootItem: quest-bound objective transaction failed");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                Err(error) => {
                    warn!(
                        ?error,
                        "LootItem: detached quest-bound transaction worker terminated"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            }

            let applied = self
                .apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
                    item_guid_generator,
                    item_id,
                    quest_log_item_id,
                    count,
                )
                .await;
            if !applied.as_ref().is_some_and(|result| result.no_grant)
                || !self
                    .player_quest_gameplay_snapshot_like_cpp()
                    .is_some_and(|state| {
                        bound_objective_plan.statuses.iter().all(|planned| {
                            state
                                .statuses_like_cpp()
                                .get(&planned.quest_id)
                                .is_some_and(|actual| {
                                    actual.status == planned.status
                                        && actual.objective_counts == planned.objective_counts
                                })
                        })
                    })
            {
                self.kick("durable quest-bound loot state diverged; relog required");
                return true;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            self.sync_player_registry_state_like_cpp();
            return true;
        }
        let store_random_properties = {
            let mut rng = self.represented_runtime_subrng_like_cpp();
            self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, &mut rng)
        };

        if store_dest.iter().any(|dest| {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            bag == u8::from(INVENTORY_SLOT_BAG_0)
                && self
                    .resolved_inventory_item_like_cpp(slot)
                    .is_some_and(|existing| {
                        self.resolved_inventory_item_object_like_cpp(existing.guid)
                            .is_some_and(|item| {
                                !loot_store_data_can_stack_with_item(
                                    loot_entry,
                                    store_random_properties,
                                    &item,
                                )
                            })
                    })
        }) {
            let Some(compatible_dest) = self.plan_direct_loot_item_preserving_cpp_store_metadata(
                loot_entry,
                store_random_properties,
            ) else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            store_dest = compatible_dest;
        }

        let mut planned_existing_counts = Vec::<PlannedDirectLootExistingStack>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();

        for dest in store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            let max_stack = self
                .item_storage_template(item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);

            if let Some(existing) = self.resolved_inventory_item_like_cpp(slot) {
                let Some(existing_object) =
                    self.resolved_inventory_item_object_like_cpp(existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                let base_count = planned_existing_counts
                    .iter()
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
                    .unwrap_or_else(|| existing_object.count());
                let new_count = base_count.saturating_add(dest.count);
                if existing.entry_id != item_id
                    || new_count > max_stack
                    || !loot_store_data_can_stack_with_item(
                        loot_entry,
                        store_random_properties,
                        &existing_object,
                    )
                {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                if let Some(existing_plan) = planned_existing_counts
                    .iter_mut()
                    .find(|planned| planned.slot == slot)
                {
                    existing_plan.new_count = new_count;
                    existing_plan.added_count =
                        existing_plan.added_count.saturating_add(dest.count);
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        item_id,
                        slot,
                        &existing_object,
                    );
                    planned_existing_counts.push(PlannedDirectLootExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        added_count: dest.count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
                }
                continue;
            }

            if let Some(stack) = planned_new_stacks
                .iter_mut()
                .find(|stack| stack.slot == slot)
            {
                if stack.entry_id == item_id
                    && stack.random_properties_id == store_random_properties.id
                    && stack.random_properties_seed == store_random_properties.seed
                    && stack.item_context == loot_entry.item_context
                    && stack.count.saturating_add(dest.count) <= max_stack
                {
                    stack.count = stack.count.saturating_add(dest.count);
                    continue;
                }
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            planned_new_stacks.push(PlannedLootNewStack {
                slot,
                entry_id: item_id,
                count: dest.count,
                max_durability: self.item_template_max_durability(item_id),
                dynamic_flags: self.stored_new_item_dynamic_flags_like_cpp(item_id, slot),
                random_properties_id: store_random_properties.id,
                random_properties_seed: store_random_properties.seed,
                item_context: loot_entry.item_context,
            });
        }

        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) = self.allocate_item_instance_guids_with_generator_like_cpp(
                item_guid_generator,
                planned_new_stacks.len(),
            ) else {
                warn!(
                    count = planned_new_stacks.len(),
                    "loot item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        // Item-container loot is a move between two durable stores. The
        // semantic request keeps source deletion in the same transaction as
        // every destination stack, so a crash cannot duplicate or lose it.
        let persistence_request =
            wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::LootDirectItemGrant(
                wow_persistence::LootDirectItemGrantPersistenceLikeCpp {
                    existing_stacks: planned_existing_counts
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
                    stored_item_source: stored_item_loot_source.map(|item_guid| {
                        wow_persistence::StoredItemLootSourcePersistenceLikeCpp {
                            item_guid: item_guid.counter() as u64,
                            item_id,
                            count,
                            loot_list_id: u32::from(loot_entry.loot_list_id),
                        }
                    }),
                },
            );

        let durable_claim = claim.cloned();
        let durable_completion_context = stored_item_loot_source
            .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
            .or_else(|| {
                claim_commit_context.map(|context| {
                    (
                        context.owner_guid,
                        context.loot_list_id,
                        context.player_guid,
                        false,
                    )
                })
            });
        let runtime_inventory_applied =
            durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = durable_completion_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(
                |(
                    (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                    runtime_inventory_applied,
                )| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid,
                            loot_list_id,
                            player_guid,
                            item_owner_auto_release,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                },
            );
        let persistence = match spawn_loot_item_persistence_worker_like_cpp(
            async move {
                inventory_persistence
                    .persist_inventory_mutation_like_cpp(persistence_request)
                    .await
            },
            durable_claim,
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "LootItem: claim closed before persistence started");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        let persistence_result = match persistence.await {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    ?error,
                    "LootItem: detached store transaction worker terminated"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        if let Err(e) = persistence_result {
            warn!("LootItem: store transaction failed: {e:?}");
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
        }

        for stack in &planned_existing_counts {
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

        let mut changed_quest_ids = self
            .apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
                item_guid_generator,
                item_id,
                quest_log_item_id,
                count,
            )
            .await;
        self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
            .await;

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

        for stack in &planned_existing_counts {
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

        // The worker committed SQL and the authority claim before runtime
        // publication. C++ `StoreNewItem` sends the stored item's update,
        // `Player::StoreLootItem` then notifies removal, and only afterwards
        // does `SendNewItem` emit `SMSG_ITEM_PUSH_RESULT`.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
        }

        for stack in &planned_existing_counts {
            self.send_loot_item_push_result(
                player_guid,
                stack.item_guid,
                loot_entry,
                store_random_properties.id,
                store_random_properties.seed,
                stack.slot,
                stack.added_count,
                stack.new_count,
                false,
                dungeon_encounter_id,
            );
        }

        for (stack, _, item_guid) in &created_new_stacks {
            self.send_loot_item_push_result(
                player_guid,
                *item_guid,
                loot_entry,
                stack.random_properties_id,
                stack.random_properties_seed,
                stack.slot,
                stack.count,
                stack.count,
                false,
                dungeon_encounter_id,
            );
        }

        if (!created_new_stacks.is_empty() || !collection_updates.is_empty())
            && !self
                .wait_for_realm_send_before_instance_update_like_cpp()
                .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
        }

        if !created_new_stacks.is_empty() {
            let changed_slots: Vec<_> = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }

        self.sync_player_registry_state_like_cpp();
        true
    }

    fn plan_direct_loot_item_preserving_cpp_store_metadata(
        &self,
        loot_entry: &LootEntry,
        random_properties: LootStoreRandomProperties,
    ) -> Option<Vec<ItemPosCount>> {
        let max_stack = self
            .item_storage_template(loot_entry.item_id)
            .map(|template| template.max_stack_size)
            .unwrap_or(1)
            .max(1);
        let mut remaining = loot_entry.quantity;
        let mut dest = Vec::new();

        let mut existing_slots: Vec<u8> = self
            .resolved_inventory_items_like_cpp()?
            .keys()
            .copied()
            .collect();
        existing_slots.sort_unstable();
        for slot in existing_slots {
            if remaining == 0 {
                break;
            }
            let Some(existing) = self.resolved_inventory_item_like_cpp(slot) else {
                continue;
            };
            let Some(existing_object) = self.resolved_inventory_item_object_like_cpp(existing.guid)
            else {
                continue;
            };
            if existing.entry_id != loot_entry.item_id
                || !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    &existing_object,
                )
                || existing_object.count() >= max_stack
            {
                continue;
            }
            let can_add = max_stack
                .saturating_sub(existing_object.count())
                .min(remaining);
            if can_add > 0 {
                dest.push(ItemPosCount::new(
                    make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                    can_add,
                ));
                remaining = remaining.saturating_sub(can_add);
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
            if remaining == 0 {
                break;
            }
            if self
                .resolved_inventory_items_like_cpp()
                .is_none_or(|items| items.contains_key(&slot))
            {
                continue;
            }
            let quantity = max_stack.min(remaining);
            dest.push(ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                quantity,
            ));
            remaining = remaining.saturating_sub(quantity);
        }

        (remaining == 0).then_some(dest)
    }

    pub(in crate::handlers::loot) async fn destroy_fully_looted_direct_item(
        &mut self,
        item_guid: ObjectGuid,
    ) {
        self.destroy_direct_item_count_after_loot_release_like_cpp(item_guid, None)
            .await;
    }
}
