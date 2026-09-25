// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory storage, equip/swap, destroy, durability and item modification.

use super::*;
mod destruction;
mod equipment_sets;
mod handlers;
mod inventory_moves;
pub(super) mod login_load;

pub(crate) fn item_turnin_persistence_rows_like_cpp(
    player_guid: ObjectGuid,
    changes: &[ExtendedCostItemTurninChange],
) -> Vec<wow_persistence::VendorItemTurninPersistenceLikeCpp> {
    changes
        .iter()
        .map(|change| match *change {
            ExtendedCostItemTurninChange::Update {
                db_guid, new_count, ..
            } => wow_persistence::VendorItemTurninPersistenceLikeCpp::Update {
                item_guid: db_guid,
                new_count,
            },
            ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                wow_persistence::VendorItemTurninPersistenceLikeCpp::Delete {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: db_guid,
                }
            }
        })
        .collect()
}

impl WorldSession {
    pub(super) fn creature_virtual_items_from_row_with_catalogs_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        persisted_equipment_id: i16,
    ) -> CreatureEquipmentCreateFieldsLikeCpp {
        let mut equipment_id = persisted_equipment_id;
        let original_equipment_id = i8::try_from(equipment_id).unwrap_or(0);
        if equipment_id == 0 {
            return CreatureEquipmentCreateFieldsLikeCpp {
                selected_equipment_id: 0,
                original_equipment_id: 0,
                virtual_items: [(0, 0, 0); 3],
            };
        }

        {
            let store = &catalogs.equipment;
            let equipment = if equipment_id == -1 {
                let count = store.len_for_entry(entry);
                if count == 0 {
                    None
                } else {
                    let index = self.represented_urand_u32_like_cpp(0, (count - 1) as u32) as usize;
                    store.nth_for_entry(entry, index).map(|(id, info)| {
                        equipment_id = i16::from(id);
                        info
                    })
                }
            } else {
                u8::try_from(equipment_id)
                    .ok()
                    .and_then(|id| store.get(entry, id))
            };

            if let Some(equipment) = equipment {
                let selected_equipment_id = u8::try_from(equipment_id).unwrap_or(0);
                return CreatureEquipmentCreateFieldsLikeCpp {
                    selected_equipment_id,
                    original_equipment_id,
                    virtual_items: equipment.items.map(|item| {
                        (
                            i32::try_from(item.item_id).unwrap_or(0),
                            item.appearance_mod_id,
                            item.item_visual,
                        )
                    }),
                };
            }
        }

        CreatureEquipmentCreateFieldsLikeCpp {
            selected_equipment_id: 0,
            original_equipment_id: 0,
            virtual_items: [(0, 0, 0); 3],
        }
    }

    #[cfg(test)]
    pub(super) fn creature_virtual_items_from_row_like_cpp(
        &mut self,
        entry: u32,
        persisted_equipment_id: i16,
    ) -> CreatureEquipmentCreateFieldsLikeCpp {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.creature_virtual_items_from_row_with_catalogs_like_cpp(
            &catalogs,
            entry,
            persisted_equipment_id,
        )
    }

    pub(super) fn plan_inventory_storage_move_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
    ) -> Option<Result<InventoryStorageMovePlanLikeCpp, InventoryResult>> {
        let source = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_object = self.resolved_inventory_item_object_like_cpp(source.guid)?;
        let source_count = source_object.count();
        let moving_to_bank = target == InventoryStorageTargetLikeCpp::Bank;
        let (result, destinations) = if moving_to_bank {
            self.plan_bank_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                false,
            )?
        } else {
            let (result, destinations, _) = self.plan_store_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                false,
            )?;
            (result, destinations)
        };
        if result != InventoryResult::Ok {
            return Some(Err(result));
        }

        Some(wow_entities::plan_inventory_storage_move_like_cpp(
            source_bag,
            source_slot,
            source,
            source_count,
            &destinations,
            |bag, slot| self.get_inventory_item_by_pos(bag, slot),
            |guid| {
                self.resolved_inventory_item_object_like_cpp(guid)
                    .map(|item| item.count())
            },
            |entry_id| {
                self.item_storage_template(entry_id)
                    .map(|template| template.max_stack_size)
            },
        ))
    }

    pub(crate) fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        if bag == INVENTORY_SLOT_BAG_0 {
            Some(0)
        } else {
            self.resolved_inventory_item_like_cpp(bag)
                .map(|item| item.db_guid)
        }
    }

    pub(super) async fn execute_inventory_storage_move_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        target: InventoryStorageTargetLikeCpp,
        quest_checks: InventoryStorageQuestChecksLikeCpp,
        represented_move: Option<RepresentedBankItemMoveLikeCpp>,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let source_item = self.get_inventory_item_by_pos(source_bag, source_slot);
        let source_guid = source_item.as_ref().map(|item| item.guid);
        let source_limit_category = source_item
            .as_ref()
            .and_then(|item| self.item_storage_template(item.entry_id))
            .map(|template| template.item_limit_category)
            .unwrap_or(0);
        let plan = match self.plan_inventory_storage_move_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            target,
        ) {
            Some(Ok(plan)) => plan,
            Some(Err(result)) => {
                self.send_equip_error(result, source_guid, None, 0, source_limit_category);
                return;
            }
            None => return,
        };
        let Some(inventory_port) = self.player_inventory_persistence_port_like_cpp() else {
            return;
        };

        let source_stays_in_place = plan
            .moved_destination
            .is_some_and(|(bag, slot, _)| bag == plan.source_bag && slot == plan.source_slot);
        let (moving_to_bank, moving_from_bank) = inventory_storage_move_quest_directions_like_cpp(
            plan.source_bag,
            plan.source_slot,
            target,
        );
        let runs_added_quest_check = quest_checks
            == InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded
            && moving_from_bank;
        let quest_log_item_id = if runs_added_quest_check {
            self.quest_source_item_quest_log_item_id_like_cpp(plan.source.entry_id)
                .await
        } else {
            0
        };
        let added_quest_count = if runs_added_quest_check {
            bank_store_item_added_quest_count_like_cpp(&plan)
        } else {
            0
        };
        let apply_obtain_spells = plan
            .moved_destination
            .is_some_and(|(bag, _, _)| bank_store_destination_applies_obtain_spells_like_cpp(bag))
            || plan.existing_updates.iter().any(|update| {
                self.get_inventory_item_by_guid_like_cpp(update.item.guid)
                    .is_some_and(|(bag, _, _)| {
                        bank_store_destination_applies_obtain_spells_like_cpp(bag)
                    })
            });
        let Some(current_non_bank_count) =
            self.represented_non_bank_item_count_like_cpp(plan.source.entry_id)
        else {
            return;
        };
        let post_move_non_bank_count = if quest_checks
            == InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved
            && moving_to_bank
            && !is_bank_pos(plan.source_bag, plan.source_slot)
        {
            current_non_bank_count.saturating_sub(plan.source_count)
        } else {
            current_non_bank_count
        };
        let planned_quest_statuses = match quest_checks {
            InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved => self
                .plan_bank_item_quest_persistence_like_cpp(
                    plan.source.entry_id,
                    0,
                    true,
                    post_move_non_bank_count,
                    0,
                ),
            InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded if moving_from_bank => self
                .plan_bank_item_quest_persistence_like_cpp(
                    plan.source.entry_id,
                    quest_log_item_id,
                    false,
                    post_move_non_bank_count,
                    added_quest_count,
                ),
            InventoryStorageQuestChecksLikeCpp::None
            | InventoryStorageQuestChecksLikeCpp::AutoStoreBankItemAdded => Vec::new(),
        };
        let enchantment_persistence = plan.moved_destination.and_then(|_| {
            self.inventory_remove_enchantment_persistence_like_cpp(
                plan.source.guid,
                !source_stays_in_place
                    && plan.source_bag == INVENTORY_SLOT_BAG_0
                    && plan.source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        });
        let mut binding_updates = Vec::new();
        for update in &plan.existing_updates {
            if let Some(mut item) = self.resolved_inventory_item_object_like_cpp(update.item.guid) {
                let old_flags = item.item_flags_bits();
                item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
                    update.bag,
                    update.slot,
                )));
                if item.item_flags_bits() != old_flags {
                    binding_updates.push((
                        update.item.guid,
                        update.item.db_guid,
                        item.item_flags_bits(),
                    ));
                }
            }
        }
        if let Some((destination_bag, destination_slot, _)) = plan.moved_destination
            && let Some(mut item) = self.resolved_inventory_item_object_like_cpp(plan.source.guid)
        {
            let old_flags = item.item_flags_bits();
            item.bind_if_stored(wow_entities::is_bag_pos(wow_entities::make_item_pos(
                destination_bag,
                destination_slot,
            )));
            if item.item_flags_bits() != old_flags {
                binding_updates.push((
                    plan.source.guid,
                    plan.source.db_guid,
                    item.item_flags_bits(),
                ));
            }
        }

        let planned_flags = |item_guid: ObjectGuid, fallback: u32| {
            binding_updates
                .iter()
                .find(|(guid, _, _)| *guid == item_guid)
                .map_or(fallback, |(_, _, flags)| *flags)
        };
        let mut mutable_persistence = Vec::new();
        for update in &plan.existing_updates {
            let Some(item) = self.resolved_inventory_item_object_like_cpp(update.item.guid) else {
                self.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(update.item.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            let Some((enchantments, _)) =
                self.inventory_remove_enchantment_persistence_like_cpp(update.item.guid, false)
            else {
                self.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(update.item.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            mutable_persistence.push(item_storage_mutable_persistence_like_cpp(
                update.item.db_guid,
                &item,
                update.new_count,
                planned_flags(update.item.guid, item.item_flags_bits()),
                enchantments,
                self.item_effect_count_like_cpp(update.item.entry_id),
            ));
        }
        if let Some((_, _, moved_count)) = plan.moved_destination {
            let Some(item) = self.resolved_inventory_item_object_like_cpp(plan.source.guid) else {
                self.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(plan.source.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            let Some((enchantments, _)) = enchantment_persistence.as_ref() else {
                self.send_equip_error(
                    InventoryResult::ItemNotFound,
                    Some(plan.source.guid),
                    None,
                    0,
                    source_limit_category,
                );
                return;
            };
            mutable_persistence.push(item_storage_mutable_persistence_like_cpp(
                plan.source.db_guid,
                &item,
                moved_count,
                planned_flags(plan.source.guid, item.item_flags_bits()),
                enchantments.clone(),
                self.item_effect_count_like_cpp(plan.source.entry_id),
            ));
        }

        let destination_link = match plan.moved_destination {
            Some((destination_bag, destination_slot, _)) if !source_stays_in_place => {
                let Some(container_db_guid) =
                    self.inventory_container_db_guid_like_cpp(destination_bag)
                else {
                    self.send_equip_error(
                        InventoryResult::WrongBagType,
                        Some(plan.source.guid),
                        None,
                        0,
                        source_limit_category,
                    );
                    return;
                };
                Some(wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: container_db_guid,
                    slot: destination_slot,
                    item_guid: plan.source.db_guid,
                })
            }
            _ => None,
        };
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::StorageMove(
            wow_persistence::InventoryStorageMovePersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                mutable_items: mutable_persistence,
                delete_source_link_item_guid: (!source_stays_in_place)
                    .then_some(plan.source.db_guid),
                destination_link,
                fully_merged_source_item_guid: plan
                    .moved_destination
                    .is_none()
                    .then_some(plan.source.db_guid),
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
            warn!(
                source_bag,
                source_slot,
                item_guid = plan.source.db_guid,
                error = %reason,
                "bank storage transaction failed; runtime left unchanged"
            );
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(plan.source.guid),
                None,
                0,
                source_limit_category,
            );
            return;
        }

        let map_id = self.player_map_id_like_cpp();
        for (item_guid, _, _) in &binding_updates {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                *item_guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetBinding(true)],
            );
            self.send_item_dynamic_flags_values_update_like_cpp(*item_guid);
        }
        for update in &plan.existing_updates {
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                update.item.guid,
                &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(
                    update.new_count,
                )],
            );
            self.send_packet(&UpdateObject::item_stack_count_update(
                update.item.guid,
                map_id,
                update.new_count,
            ));
            self.refresh_inventory_item_enchantment_duration_refs_like_cpp(update.item.guid);
        }

        let source_leaves_position = !source_stays_in_place;
        let source_dynamic_flags2_changed = source_leaves_position
            && plan.source_bag == INVENTORY_SLOT_BAG_0
            && plan.source_slot < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(plan.source.guid)
                .is_some_and(|item| item.has_item_flag2(wow_constants::ItemFieldFlags2::EQUIPPED));
        if source_stays_in_place {
            self.remove_inventory_item_duration_refs_like_cpp(plan.source.guid);
            self.remove_inventory_tradeable_item_like_cpp(plan.source.guid);
        }
        let represented_item_mods_changed = if source_leaves_position {
            self.apply_inventory_item_remove_side_effects_like_cpp(
                plan.source_bag,
                plan.source_slot,
                plan.source.guid,
                enchantment_persistence
                    .as_ref()
                    .map(|(_, slots)| slots.as_slice())
                    .unwrap_or_default(),
            )
        } else {
            false
        };

        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        if source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 {
            top_level_changes.push((plan.source_slot, ObjectGuid::EMPTY));
            if plan.source_slot < 19 {
                visible_item_changes.push((plan.source_slot, 0, 0, 0));
            }
            if (15..=17).contains(&plan.source_slot) {
                virtual_item_changes.push((plan.source_slot - 15, 0, 0, 0));
            }
        }
        if let Some((destination_bag, destination_slot, moved_count)) = plan.moved_destination {
            if source_stays_in_place {
                let _ = self.apply_inventory_item_object_updates_like_cpp(
                    plan.source.guid,
                    &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(moved_count)],
                );
                self.add_inventory_item_duration_refs_like_cpp(plan.source.guid);
                self.send_packet(&UpdateObject::item_stack_count_update(
                    plan.source.guid,
                    map_id,
                    moved_count,
                ));
            } else {
                // All possible failure conditions were checked before the commit.
                let relocated = self.apply_committed_inventory_item_relocation_like_cpp(
                    plan.source_bag,
                    plan.source_slot,
                    destination_bag,
                    destination_slot,
                    moved_count,
                );
                debug_assert!(relocated);
                self.add_inventory_item_duration_refs_like_cpp(plan.source.guid);
                if destination_bag == INVENTORY_SLOT_BAG_0 {
                    top_level_changes.push((destination_slot, plan.source.guid));
                }
                self.send_item_relocation_values_update_like_cpp(
                    plan.source.guid,
                    source_dynamic_flags2_changed,
                    enchantment_persistence
                        .as_ref()
                        .map(|(_, slots)| slots.as_slice())
                        .unwrap_or_default(),
                );
                if moved_count != plan.source_count {
                    self.send_packet(&UpdateObject::item_stack_count_update(
                        plan.source.guid,
                        map_id,
                        moved_count,
                    ));
                }
                if plan.source_bag != INVENTORY_SLOT_BAG_0 {
                    self.send_bag_slot_values_update_like_cpp(plan.source_bag, plan.source_slot);
                }
                if destination_bag != INVENTORY_SLOT_BAG_0 {
                    self.send_bag_slot_values_update_like_cpp(destination_bag, destination_slot);
                }
            }
        } else {
            let removed = self.apply_committed_inventory_item_removal_like_cpp(
                plan.source_bag,
                plan.source_slot,
                plan.source.guid,
            );
            debug_assert!(removed);
            self.send_packet(&UpdateObject::destroy_objects(
                vec![plan.source.guid],
                map_id,
            ));
            if plan.source_bag != INVENTORY_SLOT_BAG_0 {
                self.send_bag_slot_values_update_like_cpp(plan.source_bag, plan.source_slot);
            }
        }
        if !top_level_changes.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &top_level_changes,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if (source_leaves_position
            && plan.source_bag == INVENTORY_SLOT_BAG_0
            && plan.source_slot < 19)
            || represented_item_mods_changed
        {
            self.send_stat_update();
        }
        if source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 {
            if plan.source_slot < wow_entities::EQUIPMENT_SLOT_END {
                self.record_represented_titan_grip_penalty_action_like_cpp();
            }
            self.record_represented_avg_equipped_item_level_update_like_cpp();
        }
        if apply_obtain_spells {
            let _ = self
                .apply_inventory_item_obtain_spells_with_generator_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    plan.source.entry_id,
                )
                .await;
        }

        let mut changed_quest_ids = if quest_checks
            == InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved
        {
            let Some(changed) = self.apply_quest_item_removed_like_cpp(plan.source.entry_id) else {
                return;
            };
            changed
        } else {
            Vec::new()
        };
        if runs_added_quest_check {
            changed_quest_ids.extend(
                self.apply_quest_item_added_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                    plan.source.entry_id,
                    quest_log_item_id,
                    added_quest_count,
                )
                .await,
            );
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        debug_assert_eq!(
            changed_quest_ids.len(),
            planned_quest_statuses.len(),
            "bank quest persistence plan must match committed runtime removal"
        );
        if let Some(represented_move) = represented_move {
            self.record_represented_bank_item_move_like_cpp(represented_move);
        }
    }

    pub(super) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return false;
        };
        let mut current_count = 0_u32;
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (_, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }
            current_count = current_count.saturating_add(item.count());
            if current_count >= count {
                return true;
            }
        }

        false
    }

    pub(crate) fn plan_destroy_item_count_direct_inventory(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        if count == 0 {
            return Some(Vec::new());
        }

        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let mut remaining = count;
        let mut changes = Vec::new();
        let mut slots: Vec<_> = inventory_items.iter().collect();
        slots.sort_by_key(|&(slot, _)| {
            let slot = *slot;
            if slot >= 19 {
                u16::from(slot)
            } else {
                1000 + u16::from(slot)
            }
        });

        for (&slot, inventory_item) in slots {
            if inventory_item.entry_id != item_entry {
                continue;
            }
            let Some(item) = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
            else {
                continue;
            };
            if item.is_in_trade() {
                continue;
            }

            let item_count = item.count();
            if item_count <= remaining {
                remaining -= item_count;
                changes.push(ExtendedCostItemTurninChange::Delete {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                });
            } else {
                changes.push(ExtendedCostItemTurninChange::Update {
                    slot,
                    item_guid: inventory_item.guid,
                    db_guid: inventory_item.db_guid,
                    new_count: item_count - remaining,
                });
                remaining = 0;
            }

            if remaining == 0 {
                return Some(changes);
            }
        }

        None
    }

    pub(crate) fn apply_item_turnin_changes(
        &mut self,
        _player_guid: ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        let mut cleared_slots = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut send_stat_update = false;

        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    item_guid,
                    new_count,
                    ..
                } => {
                    let _ = self.apply_inventory_item_object_updates_like_cpp(
                        item_guid,
                        &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(new_count)],
                    );
                    self.send_packet(&UpdateObject::item_stack_count_update(
                        item_guid, map_id, new_count,
                    ));
                }
                ExtendedCostItemTurninChange::Delete {
                    slot, item_guid, ..
                } => {
                    self.remove_inventory_item_like_cpp(slot);
                    self.remove_inventory_item_object(item_guid);
                    cleared_slots.push((slot, ObjectGuid::EMPTY));
                    if (slot as usize) < 19 {
                        visible_item_changes.push((slot, 0i32, 0u16, 0u16));
                        send_stat_update = true;
                    }
                    if (15..=17).contains(&slot) {
                        virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
                    }
                }
            }
        }

        if !cleared_slots.is_empty() {
            self.send_player_values_update_from_entity_bridge(
                &cleared_slots,
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );
        }
        if send_stat_update {
            self.send_stat_update();
        }
    }
}
