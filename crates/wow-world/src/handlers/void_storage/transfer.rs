//! Transfer operations of void_storage.
//!
//! Divided out of the single inherent impl under #707; every method keeps
//! its name, signature and body.

use super::*;

impl WorldSession {
    pub async fn handle_void_storage_transfer_with_generators_like_cpp(
        &mut self,
        generators: &SessionIdGeneratorsLikeCpp,
        mut pkt: WorldPacket,
    ) {
        let Ok(transfer) = VoidStorageTransfer::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                transfer.npc,
                NPCFlags1::VAULT_KEEPER.bits(),
                0,
            )
            .is_none()
            || !self.void_storage_is_unlocked_like_cpp()
            || self.represented_void_storage_loaded_like_cpp() != Some(true)
        {
            return;
        }

        // These three admission checks intentionally use the request lengths,
        // before invalid GUIDs are skipped, exactly like C++.
        let Some(free_void_slots) = self.represented_void_storage_free_slots_like_cpp() else {
            return;
        };
        if transfer.deposits.len() > free_void_slots {
            self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::Full);
            return;
        }
        // C++ counts only currently empty backpack/equipped-bag slots here,
        // before deposits and before per-item `CanStoreNewItem`. It does not
        // admit a request merely because a later deposit may vacate a slot or
        // the withdrawn item could merge into an existing stack.
        let Some(empty_positions) = self.represented_empty_inventory_positions_like_cpp() else {
            return;
        };
        if transfer.withdrawals.len() > empty_positions.len() {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InventoryFull,
            );
            return;
        }
        let requested_cost =
            (transfer.deposits.len() as u64).saturating_mul(VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP);
        if !self
            .resolved_player_money_like_cpp()
            .is_some_and(|money| money >= requested_cost)
        {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::NotEnoughMoney,
            );
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(port) = self.void_storage_persistence_port_like_cpp() else {
            return;
        };

        let mut planned_deposits = Vec::new();
        let mut used_deposit_guids = HashSet::new();
        let mut reserved_destroyed_guids = HashSet::new();
        let mut reserved_void_slots = HashSet::new();
        for deposit_guid in transfer.deposits {
            if !used_deposit_guids.insert(deposit_guid)
                || reserved_destroyed_guids.contains(&deposit_guid)
            {
                continue;
            }
            let Some((bag, slot, inventory_item)) =
                self.get_inventory_item_by_guid_like_cpp(deposit_guid)
            else {
                continue;
            };
            // Audited 3.4.3 `HandleVoidStorageTransfer` lines 142-151 gates a
            // deposit only with `GetItemByGuid`; it does not inspect stack,
            // unique, quest, bag, or `ITEM_FLAG3_NO_VOID_STORAGE` metadata.
            // Preserve that server behavior here. Issue #114 explicitly keeps
            // broader inventory validation in #52.
            let Some(runtime_item) =
                self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
            else {
                continue;
            };
            let void_item_id = self.next_represented_void_storage_item_id_with_generator_like_cpp(
                generators.void_storage_item.as_ref(),
            );
            let Some(void_slot) = (0
                ..wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP)
                .find(|candidate| {
                    self.represented_void_storage_item_at_like_cpp(*candidate as u8)
                        .is_none()
                        && !reserved_void_slots.contains(candidate)
                })
                .and_then(|slot| u8::try_from(slot).ok())
            else {
                self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::Full);
                return;
            };
            reserved_void_slots.insert(usize::from(void_slot));
            let Some((_, cleared_mainhand_enchantments)) = self
                .inventory_remove_enchantment_persistence_like_cpp(
                    inventory_item.guid,
                    bag == INVENTORY_SLOT_BAG_0 && slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
                )
            else {
                continue;
            };
            let data = runtime_item.data();
            let Some(mut destroyed_items) = self.plan_void_storage_destroyed_items_like_cpp(
                bag,
                slot,
                inventory_item,
                cleared_mainhand_enchantments,
            ) else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InternalError1,
                );
                return;
            };
            // Planning is detached from runtime publication, so emulate C++'s
            // request-order destruction when a bag and one of its children are
            // both listed: a child already claimed by an earlier deposit is no
            // longer part of the later bag destruction, while a child claimed
            // by an earlier bag makes a later explicit deposit invalid.
            destroyed_items.retain(|destroyed| {
                !reserved_destroyed_guids.contains(&destroyed.inventory_item.guid)
            });
            reserved_destroyed_guids.extend(
                destroyed_items
                    .iter()
                    .map(|destroyed| destroyed.inventory_item.guid),
            );
            planned_deposits.push(PlannedVoidDepositLikeCpp {
                destroyed_items,
                void_item: RepresentedVoidStorageItemLikeCpp {
                    item_id: void_item_id,
                    item_entry: runtime_item.object().entry(),
                    creator_guid: data.creator,
                    fixed_scaling_level: runtime_item.get_modifier(ItemModifier::TimewalkerLevel),
                    random_properties_id: data.random_properties_id,
                    random_properties_seed: data.property_seed,
                    context: u8::try_from(data.context).unwrap_or(0),
                },
                void_slot,
            });
        }

        // Accepted failure-only divergence required by issue #114's explicit
        // Done contract: money, inventory and every affected void slot commit
        // in one CharacterDB transaction, and a definite failure leaves
        // runtime unchanged. Unlike C++'s intermediate in-memory mutations,
        // validate and plan every withdrawal before committing any deposit so
        // an item-specific storage failure cannot expose a charged/destroyed
        // deposit without the rest of the request. Successful wire/state order
        // remains C++-compatible and is covered separately by capture/runtime QA.
        let mut removed_entry_order = Vec::new();
        let mut removed_non_bank_counts = HashMap::<u32, u32>::new();
        for destroyed in planned_deposits
            .iter()
            .flat_map(|deposit| deposit.destroyed_items.iter())
        {
            removed_entry_order.push(destroyed.inventory_item.entry_id);
            if wow_entities::is_bank_pos(destroyed.bag, destroyed.slot) {
                continue;
            }
            let count = self
                .resolved_inventory_item_object_like_cpp(destroyed.inventory_item.guid)
                .map_or(0, |item| item.count());
            removed_non_bank_counts
                .entry(destroyed.inventory_item.entry_id)
                .and_modify(|removed| *removed = removed.saturating_add(count))
                .or_insert(count);
        }
        let post_removal_non_bank_counts = removed_entry_order
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|entry_id| {
                let removed = removed_non_bank_counts.get(&entry_id).copied().unwrap_or(0);
                self.represented_non_bank_item_count_like_cpp(entry_id)
                    .map(|count| (entry_id, count.saturating_sub(removed)))
            })
            .collect::<Option<Vec<_>>>();
        let Some(mut post_removal_non_bank_counts) = post_removal_non_bank_counts else {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::InternalError1,
            );
            return;
        };
        post_removal_non_bank_counts.sort_unstable_by_key(|(entry_id, _)| *entry_id);
        let mut quest_persistence_plan = self.begin_item_transfer_quest_persistence_like_cpp(
            &removed_entry_order,
            &post_removal_non_bank_counts,
        );
        let mut planned_withdrawals: Vec<PlannedVoidWithdrawalLikeCpp> = Vec::new();
        let mut used_withdrawal_ids = HashSet::new();
        let mut storage_overlays = Vec::new();
        let mut destination_states = HashMap::<(u8, u8), PlannedVoidDestinationStateLikeCpp>::new();
        let mut planned_container_item_guids = HashMap::<u8, wow_core::ObjectGuid>::new();
        // This includes each equipped bag's top-level `(BAG_0, bag_slot)`
        // position after its child positions. The detached inventory planner
        // removes that parent with `remove_top_level_item`, which unregisters
        // the bag storage, and also omits its `BagTemplateRef`; withdrawals
        // therefore cannot target any slot in a bag destroyed by this request.
        let vacated_inventory_positions = planned_deposits
            .iter()
            .flat_map(|deposit| deposit.destroyed_items.iter())
            .map(|destroyed| (destroyed.bag, destroyed.slot))
            .collect::<Vec<_>>();
        for withdrawal_guid in transfer.withdrawals {
            let void_item_id = withdrawal_guid.counter() as u64;
            if !used_withdrawal_ids.insert(void_item_id) {
                continue;
            }
            let Some((old_void_slot, void_item)) =
                self.represented_void_storage_item_by_id_like_cpp(void_item_id)
            else {
                continue;
            };
            let Some((result, destinations, _)) = self
                .plan_store_new_direct_inventory_item_with_overlays_like_cpp(
                    void_item.item_entry,
                    1,
                    &storage_overlays,
                    &vacated_inventory_positions,
                )
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            };
            if result != wow_constants::InventoryResult::Ok
                || destinations.len() != 1
                || destinations[0].count != 1
            {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InventoryFull,
                );
                return;
            }
            let [bag, slot] = destinations[0].pos.to_be_bytes();
            let quest_log_item_id = self
                .quest_source_item_quest_log_item_id_like_cpp(void_item.item_entry)
                .await;
            if self.plan_item_transfer_withdrawal_quest_persistence_like_cpp(
                &mut quest_persistence_plan,
                void_item.item_entry,
                quest_log_item_id,
                1,
            ) {
                planned_withdrawals.push(PlannedVoidWithdrawalLikeCpp {
                    old_void_slot,
                    void_item,
                    quest_log_item_id,
                    destination: PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem,
                });
                continue;
            }
            let Some((db_guid, item_guid)) = self
                .allocate_item_instance_guids_with_generator_like_cpp(generators.item.as_ref(), 1)
                .and_then(|mut ids| ids.pop())
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InternalError1,
                );
                return;
            };
            let random_properties = self.effective_void_storage_random_properties_like_cpp(
                void_item.random_properties_id,
                void_item.random_properties_seed,
            );
            let existing_inventory_item = (!vacated_inventory_positions.contains(&(bag, slot)))
                .then(|| self.get_inventory_item_by_pos(bag, slot))
                .flatten();
            let previous_state = destination_states.get(&(bag, slot)).cloned();
            let (target, mut item_object, base_enchantments) = if let Some(state) = previous_state {
                (state.target, state.item_object, state.enchantments)
            } else if let Some(inventory_item) = existing_inventory_item {
                let Some(item_object) =
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                else {
                    self.send_void_storage_transfer_result_like_cpp(
                        VoidTransferErrorLikeCpp::InventoryFull,
                    );
                    return;
                };
                let Some((enchantments, _)) = self
                    .inventory_remove_enchantment_persistence_like_cpp(inventory_item.guid, false)
                else {
                    self.send_void_storage_transfer_result_like_cpp(
                        VoidTransferErrorLikeCpp::InventoryFull,
                    );
                    return;
                };
                (
                    PlannedVoidDestinationTargetLikeCpp::Existing(inventory_item),
                    item_object,
                    enchantments,
                )
            } else {
                let context = ItemContext::from_u8(void_item.context).unwrap_or(ItemContext::None);
                let Some(container_guid) = self
                    .void_storage_withdrawal_container_item_guid_like_cpp(
                        bag,
                        &planned_container_item_guids,
                    )
                else {
                    self.send_void_storage_transfer_result_like_cpp(
                        VoidTransferErrorLikeCpp::InventoryFull,
                    );
                    return;
                };
                let mut item_object = self.make_inventory_item_object(
                    item_guid,
                    void_item.item_entry,
                    player_guid,
                    1,
                    self.item_template_max_durability(void_item.item_entry),
                    context,
                    slot,
                );
                // C++ `HandleVoidStorageTransfer` does not pass the stored
                // FixedScalingLevel to `StoreNewItem`; that path recomputes
                // fixed level from the current player instead. Do not restore
                // `void_item.fixed_scaling_level` as a Timewalker modifier.
                if bag != INVENTORY_SLOT_BAG_0 {
                    // C++ `_StoreItem` calls `Bag::StoreItem` before
                    // `SendUpdateToPlayer`; the CREATE therefore already
                    // names the destination bag in ItemData::ContainedIn.
                    item_object.set_contained_in(container_guid);
                    item_object.set_container_guid_and_slot(container_guid, bag);
                }
                (
                    PlannedVoidDestinationTargetLikeCpp::Planned(planned_withdrawals.len()),
                    item_object,
                    Self::void_storage_enchantments_db_string_like_cpp(
                        &[0; wow_entities::MAX_ENCHANTMENT_SLOT],
                    ),
                )
            };
            let is_new_destination = matches!(target, PlannedVoidDestinationTargetLikeCpp::Planned(index) if index == planned_withdrawals.len());
            let create_dynamic_flags = if is_new_destination {
                let template = self.item_storage_template(void_item.item_entry);
                if let Some(template) = template.as_ref() {
                    item_object.set_bonding(template.bonding);
                }
                let flags =
                    void_withdrawal_initial_item_flags_like_cpp(template.as_ref(), bag, slot);
                item_object.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
                flags
            } else {
                0
            };
            let pre_store_dynamic_flags = item_object.item_flags_bits();
            item_object.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
                bag, slot,
            )));
            let store_dynamic_flags = item_object.item_flags_bits();
            let store_dynamic_flags_update =
                (store_dynamic_flags != pre_store_dynamic_flags).then_some(store_dynamic_flags);
            item_object.set_count(item_object.count().saturating_add(1).max(1));
            // A newly constructed destination already has count one; an
            // existing/planned merge gains the withdrawn unit.
            if matches!(target, PlannedVoidDestinationTargetLikeCpp::Planned(index) if index == planned_withdrawals.len())
            {
                item_object.set_count(1);
            }
            let store_stack_count = item_object.count();
            let create_item_object = item_object.clone();
            let mut post_store_item_object = item_object.clone();
            Self::clear_item_publication_changes_like_cpp(&mut post_store_item_object);
            Self::apply_void_withdrawal_post_store_state_like_cpp(
                &mut item_object,
                void_item.creator_guid,
                &random_properties,
            );
            Self::apply_void_withdrawal_post_store_state_like_cpp(
                &mut post_store_item_object,
                void_item.creator_guid,
                &random_properties,
            );
            // C++ creates the temporary source with the void item's context,
            // but `_StoreItem` keeps the destination stack's context when it
            // merges. A brand-new destination already received this context
            // from `make_inventory_item_object` above.
            let enchantments = Self::overwrite_void_storage_random_property_enchantments_like_cpp(
                &base_enchantments,
                &random_properties,
            );

            let destination = match target.clone() {
                PlannedVoidDestinationTargetLikeCpp::Existing(inventory_item) => {
                    PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                        inventory_item,
                        item_object: item_object.clone(),
                        store_stack_count,
                        store_dynamic_flags: store_dynamic_flags_update,
                        post_store_item_object: post_store_item_object.clone(),
                        enchantments: enchantments.clone(),
                    }
                }
                PlannedVoidDestinationTargetLikeCpp::Planned(index)
                    if index < planned_withdrawals.len() =>
                {
                    let Some(target_withdrawal) = planned_withdrawals.get_mut(index) else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InternalError1,
                        );
                        return;
                    };
                    let PlannedVoidWithdrawalDestinationLikeCpp::New {
                        item_guid: target_item_guid,
                        item_state,
                        item_object: target_item,
                        enchantments: target_enchantments,
                        ..
                    } = &mut target_withdrawal.destination
                    else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InternalError1,
                        );
                        return;
                    };
                    // `_StoreItem` keeps the first destination's context when
                    // later temporary items merge into it. The handler still
                    // overwrites the returned stack's creator after each
                    // withdrawal, so only that persisted field follows the
                    // latest item here.
                    item_state.creator_guid = void_item.creator_guid;
                    *target_item = item_object.clone();
                    *target_enchantments = enchantments.clone();
                    PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned {
                        item_guid: *target_item_guid,
                        store_stack_count,
                        store_dynamic_flags: store_dynamic_flags_update,
                        post_store_item_object: post_store_item_object.clone(),
                    }
                }
                PlannedVoidDestinationTargetLikeCpp::Planned(_) => {
                    PlannedVoidWithdrawalDestinationLikeCpp::New {
                        bag,
                        slot,
                        db_guid,
                        item_guid,
                        item_state: void_item.clone(),
                        item_object: item_object.clone(),
                        create_item_object,
                        post_store_item_object,
                        enchantments: enchantments.clone(),
                        create_dynamic_flags,
                    }
                }
            };

            if let PlannedVoidWithdrawalDestinationLikeCpp::New {
                bag,
                slot,
                item_guid,
                item_object,
                ..
            } = &destination
                && *bag == INVENTORY_SLOT_BAG_0
                && self
                    .item_storage_template(item_object.object().entry())
                    .is_some_and(|template| template.container_slots != 0)
            {
                // Sequential C++ `StoreNewItem` has already installed a bag
                // before a later withdrawal is planned inside it.
                planned_container_item_guids.insert(*slot, *item_guid);
            }

            if let Some(overlay) = storage_overlays
                .iter_mut()
                .find(|overlay| overlay.bag == bag && overlay.slot == slot)
            {
                overlay.entry_id = item_object.object().entry();
                overlay.count = item_object.count();
            } else {
                storage_overlays.push(DirectInventoryStorageOverlayLikeCpp {
                    bag,
                    slot,
                    entry_id: item_object.object().entry(),
                    count: item_object.count(),
                });
            }
            destination_states.insert(
                (bag, slot),
                PlannedVoidDestinationStateLikeCpp {
                    target,
                    item_object,
                    enchantments,
                },
            );
            planned_withdrawals.push(PlannedVoidWithdrawalLikeCpp {
                old_void_slot,
                void_item,
                quest_log_item_id,
                destination,
            });
        }

        let planned_quest_statuses =
            self.finish_item_transfer_quest_persistence_like_cpp(quest_persistence_plan);

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let actual_cost =
            (planned_deposits.len() as u64).saturating_mul(VOID_STORAGE_STORE_ITEM_COST_LIKE_CPP);
        let new_money = old_money.saturating_sub(actual_cost);
        let deposits = planned_deposits
            .iter()
            .map(|deposit| wow_persistence::VoidStorageDepositWriteLikeCpp {
                destroyed_items: deposit
                    .destroyed_items
                    .iter()
                    .map(
                        |destroyed| wow_persistence::VoidStorageDestroyedItemWriteLikeCpp {
                            item_db_guid: destroyed.inventory_item.db_guid,
                        },
                    )
                    .collect(),
                void_slot: deposit.void_slot,
                void_item: Self::void_storage_item_write_like_cpp(&deposit.void_item),
            })
            .collect();

        let (total_played_time, _) = self.current_played_time_values_like_cpp();
        let mut planned_container_db_guids = HashMap::<u8, u64>::new();
        let mut withdrawals = Vec::with_capacity(planned_withdrawals.len());
        for withdrawal in &planned_withdrawals {
            let inventory_write = match &withdrawal.destination {
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem => {
                    wow_persistence::VoidStorageWithdrawalInventoryWriteLikeCpp::None
                }
                PlannedVoidWithdrawalDestinationLikeCpp::New {
                    bag,
                    slot,
                    db_guid,
                    item_state,
                    item_object,
                    enchantments,
                    ..
                } => {
                    let Some(container_db_guid) = self
                        .void_storage_withdrawal_container_db_guid_like_cpp(
                            *bag,
                            &planned_container_db_guids,
                        )
                    else {
                        self.send_void_storage_transfer_result_like_cpp(
                            VoidTransferErrorLikeCpp::InventoryFull,
                        );
                        return;
                    };
                    if *bag == INVENTORY_SLOT_BAG_0
                        && self
                            .item_storage_template(item_object.object().entry())
                            .is_some_and(|template| template.container_slots != 0)
                    {
                        // A later withdrawal can be planned inside a bag
                        // created earlier in this same transaction. C++ has
                        // already assigned that bag's item-instance GUID when
                        // its sequential `StoreNewItem` reaches the child.
                        planned_container_db_guids.insert(*slot, *db_guid);
                    }
                    wow_persistence::VoidStorageWithdrawalInventoryWriteLikeCpp::New(
                        wow_persistence::VoidStorageNewInventoryItemWriteLikeCpp {
                            item_db_guid: *db_guid,
                            item_entry: item_state.item_entry,
                            creator_guid: item_state.creator_guid.counter() as u64,
                            count: item_object.count(),
                            enchantments: enchantments.clone(),
                            item_flags: item_object.item_flags_bits(),
                            max_durability: item_object.data().max_durability,
                            total_played_time,
                            random_properties_id: item_object.data().random_properties_id,
                            random_properties_seed: item_object.data().property_seed,
                            context: item_state.context,
                            container_db_guid,
                            inventory_slot: *slot,
                        },
                    )
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                    inventory_item,
                    item_object,
                    enchantments,
                    ..
                } => wow_persistence::VoidStorageWithdrawalInventoryWriteLikeCpp::MergeExisting(
                    self.void_storage_merged_item_write_like_cpp(
                        inventory_item,
                        item_object,
                        enchantments,
                    ),
                ),
                PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned { .. } => {
                    wow_persistence::VoidStorageWithdrawalInventoryWriteLikeCpp::None
                }
            };
            withdrawals.push(wow_persistence::VoidStorageWithdrawalWriteLikeCpp {
                old_void_slot: withdrawal.old_void_slot,
                inventory_write,
            });
        }

        let request = wow_persistence::VoidStorageTransferWriteRequestLikeCpp {
            player_guid: player_guid.counter() as u64,
            money_before: old_money,
            money_after: new_money,
            deposits,
            withdrawals,
            quest_statuses: self.void_storage_quest_status_writes_like_cpp(&planned_quest_statuses),
        };

        let Some(money_persistence) = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                port.persist_void_storage_transfer_like_cpp(request),
                old_money,
                new_money,
                "void-storage transfer",
            )
            .await
        else {
            self.send_void_storage_transfer_result_like_cpp(
                VoidTransferErrorLikeCpp::TransferUnknown,
            );
            return;
        };

        // Publish the complete durable state before reopening payout
        // admission or reaching any cancellation point.
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick("canonical Player money owner became unavailable after void-storage transfer COMMIT");
            return;
        }
        let mut added_items = Vec::new();
        let mut removed_items = Vec::new();
        let mut destroyed_deposit_items = Vec::new();
        let mut withdrawal_item_publications = Vec::new();
        let mut collection_updates = Vec::new();
        let mut changed_quest_ids = Vec::new();
        let mut added_changed_quest_ids = Vec::new();
        let map_id = self.player_map_id_like_cpp();
        for deposit in &planned_deposits {
            let parent = deposit
                .destroyed_items
                .last()
                .expect("every void deposit includes its source item");
            let parent_position = (parent.bag, parent.slot);
            // The CharacterDB transaction is already durable, so retiring an
            // Item-owned Loot cannot leak a release on validation failure or
            // rollback. C++ `DestroyItem` destroys the Item and its owned
            // `Loot`; Rust keeps that represented Loot separately and must
            // retire only the child/parent objects this committed deposit is
            // about to remove.
            for destroyed in &deposit.destroyed_items {
                self.retire_committed_destroyed_item_loot_like_cpp(
                    destroyed.inventory_item.guid,
                    player_guid,
                );
            }
            let Some((destroyed_guids, deposit_changed_quest_ids)) = self
                .apply_committed_void_storage_destroyed_items_like_cpp(&deposit.destroyed_items)
            else {
                self.send_void_storage_transfer_result_like_cpp(
                    VoidTransferErrorLikeCpp::InternalError1,
                );
                return;
            };
            changed_quest_ids.extend(deposit_changed_quest_ids);
            destroyed_deposit_items.push((parent_position, destroyed_guids));
            let inserted_slot =
                self.add_represented_void_storage_item_like_cpp(deposit.void_item.clone());
            debug_assert_eq!(inserted_slot, Some(deposit.void_slot));
            added_items.push(self.represented_void_storage_item_packet_like_cpp(
                deposit.void_slot,
                &deposit.void_item,
            ));
        }
        for withdrawal in &planned_withdrawals {
            let removed =
                self.delete_represented_void_storage_item_like_cpp(withdrawal.old_void_slot);
            debug_assert_eq!(removed.as_ref(), Some(&withdrawal.void_item));
            match &withdrawal.destination {
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem => {
                    added_changed_quest_ids.extend(
                        self.apply_quest_item_added_bound_state_like_cpp(
                            withdrawal.void_item.item_entry,
                            withdrawal.quest_log_item_id,
                            1,
                        ),
                    );
                }
                PlannedVoidWithdrawalDestinationLikeCpp::New {
                    bag,
                    slot,
                    db_guid,
                    item_guid,
                    item_object,
                    create_item_object,
                    post_store_item_object,
                    create_dynamic_flags,
                    ..
                } => {
                    let inserted = self.apply_committed_new_inventory_item_at_like_cpp(
                        *bag,
                        *slot,
                        InventoryItem {
                            guid: *item_guid,
                            entry_id: item_object.object().entry(),
                            db_guid: *db_guid,
                            inventory_type: self
                                .item_template_inventory_type(item_object.object().entry()),
                        },
                        item_object.clone(),
                    );
                    debug_assert!(inserted);
                    collection_updates
                        .extend(self.on_item_added_to_collection_like_cpp(item_object));
                    let container_slots = self
                        .item_storage_template(item_object.object().entry())
                        .map_or(0, |template| u32::from(template.container_slots));
                    withdrawal_item_publications.push(
                        Self::new_void_withdrawal_item_publication_like_cpp(
                            create_item_object,
                            post_store_item_object,
                            *create_dynamic_flags,
                            container_slots,
                            map_id,
                        ),
                    );
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergeExisting {
                    inventory_item,
                    item_object,
                    store_stack_count,
                    store_dynamic_flags,
                    post_store_item_object,
                    ..
                } => {
                    let updated = self
                        .update_inventory_item_object_like_cpp(inventory_item.guid, |target| {
                            *target = item_object.clone()
                        });
                    debug_assert!(updated);
                    collection_updates
                        .extend(self.on_item_added_to_collection_like_cpp(item_object));
                    withdrawal_item_publications.extend(
                        Self::merged_void_withdrawal_item_publications_like_cpp(
                            inventory_item.guid,
                            *store_stack_count,
                            *store_dynamic_flags,
                            post_store_item_object,
                            map_id,
                        ),
                    );
                    self.refresh_inventory_item_enchantment_duration_refs_like_cpp(
                        inventory_item.guid,
                    );
                }
                PlannedVoidWithdrawalDestinationLikeCpp::MergedIntoPlanned {
                    item_guid,
                    store_stack_count,
                    store_dynamic_flags,
                    post_store_item_object,
                    ..
                } => {
                    withdrawal_item_publications.extend(
                        Self::merged_void_withdrawal_item_publications_like_cpp(
                            *item_guid,
                            *store_stack_count,
                            *store_dynamic_flags,
                            post_store_item_object,
                            map_id,
                        ),
                    );
                }
            }
            if !matches!(
                &withdrawal.destination,
                PlannedVoidWithdrawalDestinationLikeCpp::QuestBoundNoItem
            ) {
                added_changed_quest_ids.extend(
                    self.apply_quest_item_added_non_bound_state_like_cpp(
                        withdrawal.void_item.item_entry,
                        withdrawal.quest_log_item_id,
                        1,
                    ),
                );
            }
            removed_items.push(wow_core::ObjectGuid::create_item(
                self.realm_id(),
                withdrawal.void_item.item_id as i64,
            ));
        }
        changed_quest_ids.extend(added_changed_quest_ids.iter().copied());
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        added_changed_quest_ids.sort_unstable();
        added_changed_quest_ids.dedup();
        let planned_changed_quest_ids = planned_quest_statuses
            .iter()
            .map(|status| status.quest_id)
            .collect::<Vec<_>>();
        debug_assert_eq!(changed_quest_ids, planned_changed_quest_ids);
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            generators.item.as_ref(),
        )
        .await;
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
        self.publish_void_storage_item_lifecycle_like_cpp(
            map_id,
            destroyed_deposit_items,
            withdrawal_item_publications,
        );
        self.publish_quest_item_added_status_changes_like_cpp(&added_changed_quest_ids);
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
        for withdrawal in &planned_withdrawals {
            if let PlannedVoidWithdrawalDestinationLikeCpp::New {
                bag,
                slot,
                item_guid,
                ..
            } = &withdrawal.destination
            {
                if *bag == INVENTORY_SLOT_BAG_0 {
                    self.send_player_values_update_from_entity_bridge(
                        &[(*slot, *item_guid)],
                        &[],
                        &[],
                        &[],
                        None,
                    );
                } else {
                    self.send_bag_slot_values_update_like_cpp(*bag, *slot);
                }
            }
        }
        self.send_packet(&VoidStorageTransferChanges {
            removed_items,
            added_items,
        });
        self.send_void_storage_transfer_result_like_cpp(VoidTransferErrorLikeCpp::NoError);
    }

    #[cfg(test)]
    pub async fn handle_void_storage_transfer(&mut self, pkt: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_void_storage_transfer_with_generators_like_cpp(&generators, pkt)
            .await;
    }
}
