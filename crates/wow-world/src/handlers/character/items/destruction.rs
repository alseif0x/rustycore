// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory item destruction and recursive persistence.

use super::*;

impl WorldSession {
    /// Handle CMSG_DESTROY_ITEM: delete an item from inventory.
    pub async fn handle_destroy_item(
        &mut self,
        destroy: wow_packet::packets::item::DestroyItemPkt,
    ) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(
            "DestroyItem: container={} slot={} count={} for {:?}",
            destroy.container_id, destroy.slot_num, destroy.count, player_guid
        );

        let bag = destroy.container_id;
        let slot = destroy.slot_num;
        let item = match self.get_inventory_item_by_pos(bag, slot) {
            Some(item) => item,
            None => {
                self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                return;
            }
        };

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item.guid);
        let item_proto = self.item_storage_template(item.entry_id);
        let unequip_result = self.can_unequip_inventory_item_at_like_cpp(
            bag,
            slot,
            false,
            runtime_item.as_ref(),
            item_proto.as_ref(),
            self.direct_item_contains_items(item.guid),
        );
        if unequip_result != InventoryResult::Ok {
            self.send_packet_realm(&InventoryChangeFailure::error(unequip_result));
            return;
        }

        if self
            .item_template_flags(item.entry_id)
            .is_some_and(|flags| flags.contains(ItemFlags::NO_USER_DESTROY))
        {
            self.send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::DropBoundItem,
            ));
            return;
        }

        // Delete from DB
        let inventory_port = match self.player_inventory_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let count_action = runtime_item
            .as_ref()
            .map(|item_object| {
                destroy_item_count_action(
                    item_object.count(),
                    u32::try_from(destroy.count).unwrap_or(u32::MAX),
                )
            })
            .unwrap_or(DestroyItemCountAction::FullStack);

        if let DestroyItemCountAction::PartialStack { new_count } = count_action {
            let removed_count = runtime_item
                .as_ref()
                .map(|item_object| item_object.count().saturating_sub(new_count))
                .unwrap_or(0);
            let Some(planned_quest_statuses) = self
                .plan_destroyed_inventory_quest_persistence_like_cpp(&[DestroyQuestItemLikeCpp {
                    bag,
                    slot,
                    entry_id: item.entry_id,
                    count: removed_count,
                }])
            else {
                return;
            };
            let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::PartialDestroy(
                wow_persistence::InventoryPartialDestroyPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    new_count,
                    quest_statuses: self.represented_quest_status_persistence_rows_like_cpp(
                        &planned_quest_statuses,
                    ),
                },
            );
            let outcome = inventory_port
                .persist_inventory_mutation_like_cpp(request)
                .await;
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
            {
                warn!(error = %reason, "DestroyItem: update partial stack count failed");
                self.send_packet_realm(&InventoryChangeFailure::error(
                    InventoryResult::InternalBagError,
                ));
                return;
            }

            let _ = self.apply_inventory_item_object_updates_like_cpp(
                item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
            );
            let Some(changed_quest_ids) = self.apply_quest_item_removed_like_cpp(item.entry_id)
            else {
                return;
            };
            debug_assert_eq!(
                changed_quest_ids.len(),
                planned_quest_statuses.len(),
                "partial destroy quest persistence must match committed runtime removal"
            );
            self.send_packet(&UpdateObject::item_stack_count_update(
                item.guid,
                self.player_map_id_like_cpp(),
                new_count,
            ));
            info!(
                "Destroyed partial item entry={} at ({},{}) count={} for {:?}",
                item.entry_id, bag, slot, destroy.count, player_guid
            );
            return;
        }

        let destroyed_entry_id = item.entry_id;
        if self
            .destroy_inventory_full_stack_by_pos_like_cpp(
                bag,
                slot,
                item,
                runtime_item,
                "DestroyItem",
            )
            .await
        {
            info!(
                "Destroyed item entry={} at ({},{}) for {:?}",
                destroyed_entry_id, bag, slot, player_guid
            );
        }
    }

    pub(in crate::handlers::character) fn plan_destroyed_inventory_quest_persistence_like_cpp(
        &self,
        destroyed_items: &[DestroyQuestItemLikeCpp],
    ) -> Option<Vec<crate::handlers::quest::PlayerQuestStatus>> {
        let removed_entries_in_order = destroyed_items
            .iter()
            .map(|item| item.entry_id)
            .collect::<Vec<_>>();
        let mut removed_non_bank_counts = HashMap::<u32, u32>::new();
        for item in destroyed_items {
            if !is_bank_pos(item.bag, item.slot) {
                removed_non_bank_counts
                    .entry(item.entry_id)
                    .and_modify(|count| *count = count.saturating_add(item.count))
                    .or_insert(item.count);
            }
        }
        let post_removal_non_bank_counts = removed_entries_in_order
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|entry_id| {
                let current = self.represented_non_bank_item_count_like_cpp(entry_id)?;
                let removed = removed_non_bank_counts.get(&entry_id).copied().unwrap_or(0);
                Some((entry_id, current.saturating_sub(removed)))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(self.plan_item_transfer_quest_persistence_like_cpp(
            &removed_entries_in_order,
            &post_removal_non_bank_counts,
            &[],
        ))
    }

    /// C++ `Player::DestroyItem(bag, slot, update=true)` for a full-stack item.
    pub(crate) async fn destroy_inventory_full_stack_by_pos_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: crate::session::InventoryItem,
        runtime_item: Option<wow_entities::Item>,
        context: &str,
    ) -> bool {
        self.destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
            bag,
            slot,
            item,
            runtime_item,
            None,
            context,
        )
        .await
    }

    pub(crate) async fn destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: crate::session::InventoryItem,
        runtime_item: Option<wow_entities::Item>,
        expected_owner_db_guid: Option<u64>,
        context: &str,
    ) -> bool {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return false,
        };
        let inventory_port = match self.player_inventory_persistence_port_like_cpp() {
            Some(port) => port,
            None => return false,
        };

        // C++ Player::DestroyItem recursively destroys a bag's contents before
        // deleting the bag itself. Keep all corresponding character rows in a
        // single transaction so a persistence failure cannot orphan children
        // or expose a partially destroyed runtime graph.
        let Some(descendants) =
            self.represented_inventory_descendants_postorder_like_cpp(item.guid)
        else {
            return false;
        };
        let descendant_runtime = descendants
            .iter()
            .map(|(child_bag, child_slot, child)| {
                (
                    *child_bag,
                    *child_slot,
                    child.clone(),
                    self.resolved_inventory_item_object_like_cpp(child.guid),
                )
            })
            .collect::<Vec<_>>();

        let mut destroyed_quest_items = descendant_runtime
            .iter()
            .map(
                |(child_bag, child_slot, child, child_runtime)| DestroyQuestItemLikeCpp {
                    bag: *child_bag,
                    slot: *child_slot,
                    entry_id: child.entry_id,
                    count: child_runtime
                        .as_ref()
                        .map(wow_entities::Item::count)
                        .unwrap_or(1),
                },
            )
            .collect::<Vec<_>>();
        destroyed_quest_items.push(DestroyQuestItemLikeCpp {
            bag,
            slot,
            entry_id: item.entry_id,
            count: runtime_item
                .as_ref()
                .map(wow_entities::Item::count)
                .unwrap_or(1),
        });
        let Some(planned_quest_statuses) =
            self.plan_destroyed_inventory_quest_persistence_like_cpp(&destroyed_quest_items)
        else {
            return false;
        };

        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());

        let nodes = descendant_runtime
            .iter()
            .map(|(_, _, child, _)| child.db_guid)
            .chain(std::iter::once(item.db_guid))
            .map(
                |db_guid| wow_persistence::InventoryDestroyNodePersistenceLikeCpp {
                    item_guid: db_guid,
                    expected_owner_db_guid: (db_guid == item.db_guid)
                        .then_some(expected_owner_db_guid)
                        .flatten(),
                },
            )
            .collect();
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::GraphDestroy(
            wow_persistence::InventoryGraphDestroyPersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                nodes,
                quest_statuses: self
                    .represented_quest_status_persistence_rows_like_cpp(&planned_quest_statuses),
            },
        );
        let outcome = inventory_port
            .persist_inventory_mutation_like_cpp(request)
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(error = %reason, "{context}: delete transaction failed");
            self.send_packet_realm(&InventoryChangeFailure::error(
                InventoryResult::InternalBagError,
            ));
            return false;
        }

        if let Some(expected_owner_db_guid) = expected_owner_db_guid {
            match self
                .uncage_item_state_like_cpp(expected_owner_db_guid, item.db_guid)
                .await
            {
                wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state)
                    if state.owner_guid.is_none() && !state.inventory_linked => {}
                wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state) => {
                    warn!(
                        item_guid = item.db_guid,
                        owner_guid = ?state.owner_guid,
                        inventory_linked = state.inventory_linked,
                        "{context}: guarded item deletion did not reach its durable postcondition"
                    );
                    return false;
                }
                wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!(
                        item_guid = item.db_guid,
                        %reason,
                        "{context}: failed to verify guarded item deletion"
                    );
                    return false;
                }
            }
        }

        let mut destroyed_guids = Vec::with_capacity(descendant_runtime.len() + 1);
        let mut changed_quest_ids = Vec::new();
        for (child_bag, child_slot, child, child_runtime) in descendant_runtime {
            let should_expire_child_refund = child_runtime
                .as_ref()
                .is_some_and(|item_object| item_object.is_refundable());
            let _ = self.apply_inventory_item_remove_side_effects_like_cpp(
                child_bag,
                child_slot,
                child.guid,
                &[],
            );
            let removed = self
                .apply_committed_inventory_item_removal_like_cpp(child_bag, child_slot, child.guid);
            debug_assert!(removed);
            if should_expire_child_refund {
                self.send_packet(&ItemExpirePurchaseRefund {
                    item_guid: child.guid,
                });
            }
            destroyed_guids.push(child.guid);
            let Some(changed) = self.apply_quest_item_removed_like_cpp(child.entry_id) else {
                return false;
            };
            changed_quest_ids.extend(changed);
        }

        let represented_item_mods_changed =
            self.apply_inventory_item_remove_side_effects_like_cpp(bag, slot, item.guid, &[]);

        let removed = self.apply_committed_inventory_item_removal_like_cpp(bag, slot, item.guid);
        debug_assert!(removed);
        destroyed_guids.push(item.guid);
        let Some(changed) = self.apply_quest_item_removed_like_cpp(item.entry_id) else {
            return false;
        };
        changed_quest_ids.extend(changed);
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        debug_assert_eq!(
            changed_quest_ids.len(),
            planned_quest_statuses.len(),
            "recursive destroy quest persistence must match child/parent runtime removals"
        );
        self.sync_player_registry_state_like_cpp();

        self.send_packet(&UpdateObject::destroy_objects(
            destroyed_guids,
            self.player_map_id_like_cpp(),
        ));

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            let inv_slot_changes = vec![(slot, ObjectGuid::EMPTY)];
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();

            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &inv_slot_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 || represented_item_mods_changed {
                self.send_stat_update();
            }
        } else {
            self.send_bag_slot_values_update_like_cpp(bag, slot);
        }

        true
    }
}
