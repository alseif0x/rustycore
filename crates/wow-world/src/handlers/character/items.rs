// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory storage, equip/swap, destroy, durability and item modification.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::{CharStatements, SqlTransaction};

use super::*;

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
    pub(super) fn creature_virtual_items_from_row_like_cpp(
        &mut self,
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

        if let Some(store) = self.creature_equipment_store_like_cpp().cloned() {
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

    /// Handle CMSG_SAVE_EQUIPMENT_SET.
    ///
    /// C++ validates the equipment/transmog payload, normalizes ignored slots,
    /// then calls `Player::SetEquipmentSet`. Rust mirrors the in-memory dirty
    /// state and new-set `SMSG_EQUIPMENT_SET_ID`; the next full player save
    /// appends `_SaveEquipmentSets`-shaped statements to its transaction.
    pub async fn handle_save_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match SaveEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SaveEquipmentSet: {error}");
                return;
            }
        };

        let Some(saved) = self.save_represented_equipment_set_like_cpp(request.set) else {
            return;
        };

        if saved.generated_new_guid {
            self.send_packet(&EquipmentSetId {
                guid: saved.guid,
                set_type: saved.raw_set_type,
                set_id: saved.set_id,
            });
        }
    }

    /// Handle CMSG_ASSIGN_EQUIPMENT_SET_SPEC.
    ///
    /// C++ `Player::AssignEquipmentSetToSpec` only mutates the first equipment
    /// set whose client SetID matches and does not send an immediate response.
    /// The represented container keeps the same in-memory assignment/state
    /// semantics before the next full player-save transaction persists them.
    pub async fn handle_assign_equipment_set_spec(&mut self, mut pkt: WorldPacket) {
        let request = match AssignEquipmentSetSpec::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad AssignEquipmentSetSpec: {error}");
                return;
            }
        };

        let _assigned = self
            .assign_represented_equipment_set_to_spec_like_cpp(request.set_id, request.spec_index);
    }

    /// Handle CMSG_DELETE_EQUIPMENT_SET.
    ///
    /// C++ marks existing equipment/transmog sets as deleted unless the set was
    /// still new in memory, in which case it removes it immediately. The DB
    /// delete happens later in `_SaveEquipmentSets`.
    pub async fn handle_delete_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match DeleteEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad DeleteEquipmentSet: {error}");
                return;
            }
        };

        let _deleted = self.delete_represented_equipment_set_like_cpp(request.id);
    }

    /// Handle CMSG_USE_EQUIPMENT_SET.
    ///
    /// C++ `HandleUseEquipmentSet` iterates all 19 equipment slots, skips the
    /// ignored GUID sentinel and non-weapon slots in combat, then uses
    /// `GetItemByGuid` + `SwapItem` / `CanStoreItem` to move gear. This slice
    /// mirrors the represented direct-inventory state and the result packet;
    /// full nested-container validation, `CanEquipItem`, DB writes, and item
    /// update fanout remain later inventory-runtime work.
    pub async fn handle_use_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match UseEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad UseEquipmentSet: {error}");
                return;
            }
        };

        let represented_item_mods_changed = self.use_represented_equipment_set_like_cpp(&request);
        if represented_item_mods_changed {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        self.send_packet(&UseEquipmentSetResult {
            guid: request.guid,
            reason: 0,
        });
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
        let source_object = self.inventory_item_objects_like_cpp().get(&source.guid)?;
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

        let destination_count = destinations.len();
        let mut existing_updates = Vec::new();
        let mut moved_destination = None;
        let mut planned_count = 0u32;
        for destination in destinations {
            let [bag, slot] = destination.pos.to_be_bytes();
            if destination.count == 0 {
                return Some(Err(InventoryResult::InternalBagError));
            }
            planned_count = match planned_count.checked_add(destination.count) {
                Some(count) => count,
                None => return Some(Err(InventoryResult::InternalBagError)),
            };

            if bag == source_bag && slot == source_slot {
                if destination_count == 1 {
                    // C++ HandleAutoStoreBagItemOpcode treats the one-slot
                    // autostore result as a no-op and clears the client's grey
                    // item state with EQUIP_ERR_INTERNAL_BAG_ERROR.
                    return Some(Err(InventoryResult::InternalBagError));
                }
                if moved_destination
                    .replace((bag, slot, destination.count))
                    .is_some()
                {
                    return Some(Err(InventoryResult::InternalBagError));
                }
                continue;
            }
            if let Some(existing) = self.get_inventory_item_by_pos(bag, slot) {
                if existing.guid == source.guid || existing.entry_id != source.entry_id {
                    return Some(Err(InventoryResult::CantStack));
                }
                let Some(existing_object) =
                    self.inventory_item_objects_like_cpp().get(&existing.guid)
                else {
                    return Some(Err(InventoryResult::ItemNotFound));
                };
                let Some(new_count) = existing_object.count().checked_add(destination.count) else {
                    return Some(Err(InventoryResult::InternalBagError));
                };
                let max_stack = self
                    .item_storage_template(existing.entry_id)
                    .map_or(1, |template| template.max_stack_size.max(1));
                if new_count > max_stack {
                    return Some(Err(InventoryResult::CantStack));
                }
                existing_updates.push(ExistingStorageStackUpdateLikeCpp {
                    item: existing,
                    bag,
                    slot,
                    new_count,
                });
            } else if moved_destination
                .replace((bag, slot, destination.count))
                .is_some()
            {
                // One Item instance can supply merges plus at most one remainder stack.
                return Some(Err(InventoryResult::InternalBagError));
            }
        }

        if planned_count != source_count {
            return Some(Err(InventoryResult::InternalBagError));
        }

        Some(Ok(InventoryStorageMovePlanLikeCpp {
            source_bag,
            source_slot,
            source,
            source_count,
            existing_updates,
            moved_destination,
        }))
    }

    pub(crate) fn inventory_container_db_guid_like_cpp(&self, bag: u8) -> Option<u64> {
        if bag == INVENTORY_SLOT_BAG_0 {
            Some(0)
        } else {
            self.inventory_items_like_cpp()
                .get(&bag)
                .map(|item| item.db_guid)
        }
    }

    pub(super) async fn execute_inventory_storage_move_like_cpp(
        &mut self,
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
        let current_non_bank_count =
            self.represented_non_bank_item_count_like_cpp(plan.source.entry_id);
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
            if let Some(mut item) = self
                .inventory_item_objects_like_cpp()
                .get(&update.item.guid)
                .cloned()
            {
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
            && let Some(mut item) = self
                .inventory_item_objects_like_cpp()
                .get(&plan.source.guid)
                .cloned()
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
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&update.item.guid)
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
                item,
                update.new_count,
                planned_flags(update.item.guid, item.item_flags_bits()),
                enchantments,
                self.item_effect_count_like_cpp(update.item.entry_id),
            ));
        }
        if let Some((_, _, moved_count)) = plan.moved_destination {
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&plan.source.guid)
            else {
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
                item,
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
            self.update_inventory_item_object_like_cpp(*item_guid, |item| {
                item.set_binding(true);
            });
            self.send_item_dynamic_flags_values_update_like_cpp(*item_guid);
        }
        for update in &plan.existing_updates {
            self.update_inventory_item_object_like_cpp(update.item.guid, |item| {
                item.set_count(update.new_count);
            });
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
                .inventory_item_objects_like_cpp()
                .get(&plan.source.guid)
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
                self.update_inventory_item_object_like_cpp(plan.source.guid, |item| {
                    item.set_count(moved_count);
                });
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
        if source_leaves_position
            && plan.source_bag == INVENTORY_SLOT_BAG_0
            && plan.source_slot < 19
        {
            self.send_stat_update();
        }
        if represented_item_mods_changed {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        if source_leaves_position && plan.source_bag == INVENTORY_SLOT_BAG_0 {
            if plan.source_slot < wow_entities::EQUIPMENT_SLOT_END {
                self.record_represented_titan_grip_penalty_action_like_cpp();
            }
            self.record_represented_avg_equipped_item_level_update_like_cpp();
        }
        self.sync_object_accessor_player();
        if apply_obtain_spells {
            let _ = self
                .apply_inventory_item_obtain_spells_like_cpp(plan.source.entry_id)
                .await;
        }

        let mut changed_quest_ids =
            if quest_checks == InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved {
                self.apply_quest_item_removed_like_cpp(plan.source.entry_id)
            } else {
                Vec::new()
            };
        if runs_added_quest_check {
            changed_quest_ids.extend(
                self.apply_quest_item_added_objective_progress_like_cpp(
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

    /// Handle CMSG_LIST_INVENTORY — player opens vendor window.
    ///
    /// Queries npc_vendor for the creature's items (including reference vendors, item_id < 0)
    /// and sends SMSG_VENDOR_INVENTORY. Entry is resolved from the visibility tracker or,
    /// if missing, from world.creature by GUID (fallback when NPC not in tracker).
    pub async fn handle_list_inventory(&mut self, hello: Hello) {
        let vendor_guid = hello.unit;
        info!(
            "ListInventory for {:?} from account {}",
            vendor_guid, self.account_id
        );

        let vendor_catalog = match self.vendor_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        // Resolve creature entry: first from map-owned creature state, then fallback from DB by spawn GUID.
        let entry = match self.mutate_world_creature(vendor_guid, |creature| {
            creature.pause_interaction_movement_like_cpp();
            creature.entry()
        }) {
            Some(entry) => entry,
            None => {
                let fallback = match tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    vendor_catalog
                        .load_creature_entry_by_spawn_like_cpp(vendor_guid.low_value() as u64),
                )
                .await
                {
                    Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(entry)) => Some(entry),
                    _ => None,
                };
                match fallback {
                    Some(e) => {
                        info!("Vendor entry {} resolved from DB (GUID not in tracker)", e);
                        e
                    }
                    None => {
                        info!(
                            "Vendor GUID {:?} not in tracker and not found in creature table",
                            vendor_guid
                        );
                        self.send_packet(&VendorInventory {
                            vendor_guid,
                            reason: 0,
                            items: vec![],
                        });
                        return;
                    }
                }
            }
        };

        // Load all items: direct rows + expand reference vendors (npc_vendor.item < 0).
        let mut items = Vec::new();
        let mut raw_slot = 0i32;
        let mut expanded = std::collections::HashSet::<u32>::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(entry);
        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let player_condition_object = self.build_condition_player_object_like_cpp();
        let vendor_condition_object = self.build_condition_creature_object_like_cpp(vendor_guid);
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();

        'vendor_expansion: while let Some(vendor_entry) = queue.pop_front() {
            if !expanded.insert(vendor_entry) {
                continue; // already expanded (avoid cycles)
            }
            let rows = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                vendor_catalog.load_vendor_rows_like_cpp(entry, vendor_entry),
            )
            .await
            {
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Loaded(rows)) => rows,
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Missing) => Vec::new(),
                Ok(wow_persistence::VendorCatalogOutcomeLikeCpp::Failed { reason }) => {
                    warn!("Vendor query failed for entry {vendor_entry}: {reason}");
                    continue;
                }
                Err(_) => {
                    warn!("Vendor query timed out for entry {vendor_entry}");
                    continue;
                }
            };

            for row in rows {
                let item_id = row.item_id;
                let maxcount = row.max_count;
                let extended_cost = row.extended_cost as i32;
                let item_type = i32::from(row.item_type);
                let buy_price = row.buy_price;
                let durability = row.max_durability as i32;
                let stack_count = row.buy_count as i32;
                let do_not_filter = row.do_not_filter;
                let incr_time = row.incr_time;
                let player_condition_id = row.player_condition_id;
                let has_vendor_conditions = row.has_vendor_conditions;

                // Solo enviar items con ID válido; 0 o negativo el cliente lo muestra como ? y nombre vacío
                // Ademas filtrar items que no existen en Item.db2, matching
                // C++ `SendListInventory` item-template validation
                // (`Handlers/ItemHandler.cpp:617-626`).
                // Items 58260, 58274, etc. no están en Item.db2 de este cliente → se omiten.
                if item_id > 0 {
                    let muid = raw_slot.saturating_add(1);
                    raw_slot = raw_slot.saturating_add(1);
                    if item_type == ItemVendorType::Currency as i32 {
                        if vendor_list_should_skip_currency_row(
                            self.currency_types_store().map(|store| store.as_ref()),
                            item_id,
                            extended_cost,
                        ) {
                            continue;
                        }
                        items.push(VendorItem {
                            muid,
                            item_id,
                            item_type,
                            quantity: 0,
                            price: 0,
                            durability: 0,
                            stack_count: maxcount,
                            extended_cost,
                            player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                                player_condition_id,
                                player_condition_store.as_deref(),
                                Some(player_condition_context.as_context(self)),
                            ),
                            locked: false,
                            do_not_filter,
                            refundable: false,
                        });
                        if vendor_list_reaches_cpp_item_limit(items.len()) {
                            break 'vendor_expansion;
                        }
                        continue;
                    }
                    let item_known = self
                        .item_store()
                        .map_or(true, |s| s.get(item_id as u32).is_some());
                    if !item_known {
                        info!(
                            "Vendor item {} not in Item.db2 (entry {}), skipping",
                            item_id, vendor_entry
                        );
                        continue;
                    }
                    let current_count = self.vendor_item_current_count(
                        vendor_guid,
                        item_id as u32,
                        maxcount.max(0) as u32,
                        incr_time,
                        stack_count.max(1) as u32,
                    );
                    if vendor_list_should_skip_sold_out(maxcount, current_count, self.security > 0)
                    {
                        continue;
                    }
                    let template = self.item_storage_template(item_id as u32);
                    let sparse_template = self
                        .item_stats_store()
                        .and_then(|store| store.sparse_template(item_id as u32));
                    if vendor_list_should_skip_allowed_class(
                        sparse_template.map(|template| template.allowable_class),
                        sparse_template.map(|template| template.bonding),
                        self.player_class_like_cpp(),
                        self.security > 0,
                    ) {
                        continue;
                    }
                    if vendor_list_should_skip_faction_flags(
                        sparse_template.map(|template| template.flags[1]),
                        player_team_for_race_cpp(self.player_race_like_cpp()),
                        self.security > 0,
                    ) {
                        continue;
                    }
                    if has_vendor_conditions {
                        let Some(store) = condition_store.as_ref() else {
                            continue;
                        };
                        let (vendor_object, vendor_unit_snapshot) = vendor_condition_object
                            .as_ref()
                            .map(|(object, snapshot)| (Some(object), Some(*snapshot)))
                            .unwrap_or((None, None));
                        if !Self::vendor_item_conditions_meet_like_cpp(
                            store.as_ref(),
                            entry,
                            item_id as u32,
                            player_condition_object.as_ref(),
                            vendor_object,
                            player_unit_snapshot,
                            player_snapshot,
                            vendor_unit_snapshot,
                            player_condition_store.as_deref(),
                            Some(player_condition_context.as_context(self)),
                        ) {
                            warn!(
                                "Vendor item condition not met for creature entry {} item {}",
                                entry, item_id
                            );
                            continue;
                        }
                    }
                    let refundable = vendor_list_item_refundable(
                        template.as_ref().map(|template| template.flags),
                        template.as_ref().map(|template| template.max_stack_size),
                        extended_cost,
                    );
                    items.push(VendorItem {
                        muid,
                        item_id,
                        item_type,
                        quantity: if maxcount == 0 {
                            -1
                        } else {
                            current_count as i32
                        },
                        price: buy_price,
                        durability,
                        stack_count: stack_count.max(1),
                        extended_cost,
                        player_condition_failed: vendor_player_condition_failed_id_like_cpp(
                            player_condition_id,
                            player_condition_store.as_deref(),
                            Some(player_condition_context.as_context(self)),
                        ),
                        locked: false,
                        do_not_filter,
                        refundable,
                    });
                    if vendor_list_reaches_cpp_item_limit(items.len()) {
                        break 'vendor_expansion;
                    }
                } else if item_id < 0 {
                    let ref_entry = (-item_id) as u32;
                    queue.push_back(ref_entry);
                }
            }
        }

        let item_ids: Vec<i32> = items.iter().map(|i| i.item_id).collect();
        info!(
            "Sending vendor inventory: {} items for entry {} (item_ids: {:?})",
            items.len(),
            entry,
            item_ids
        );
        self.send_packet(&VendorInventory {
            vendor_guid,
            reason: 0,
            items,
        });
    }

    pub(super) fn has_item_count_direct_inventory(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        let mut current_count = 0_u32;
        let mut slots: Vec<_> = self.inventory_items_like_cpp().iter().collect();
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
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&inventory_item.guid)
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

        let mut remaining = count;
        let mut changes = Vec::new();
        let mut slots: Vec<_> = self.inventory_items_like_cpp().iter().collect();
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
            let Some(item) = self
                .inventory_item_objects_like_cpp()
                .get(&inventory_item.guid)
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

    pub(crate) fn append_item_turnin_statements(
        char_db: &wow_database::CharacterDatabase,
        tx: &mut SqlTransaction,
        player_guid: ObjectGuid,
        changes: &[ExtendedCostItemTurninChange],
    ) {
        for change in changes {
            match *change {
                ExtendedCostItemTurninChange::Update {
                    db_guid, new_count, ..
                } => {
                    let mut stmt = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
                    stmt.set_u32(0, new_count);
                    stmt.set_u64(1, db_guid);
                    tx.append(stmt);
                }
                ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                    let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
                    del_inv.set_u64(0, player_guid.counter() as u64);
                    del_inv.set_u64(1, db_guid);
                    tx.append(del_inv);

                    let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
                    del_item.set_u64(0, db_guid);
                    tx.append(del_item);
                }
            }
        }
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
                    self.update_inventory_item_object_like_cpp(item_guid, |item| {
                        item.set_count(new_count);
                    });
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
            self.sync_object_accessor_player();
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

    fn validate_inventory_swap_target_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        swap: bool,
        require_exact_destination: bool,
    ) -> Option<(InventoryResult, InventorySwapTargetLikeCpp)> {
        let source = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let source_count = self
            .inventory_item_objects_like_cpp()
            .get(&source.guid)?
            .count();
        let destination_pos = wow_entities::make_item_pos(destination_bag, destination_slot);

        if is_inventory_pos(destination_bag, destination_slot) {
            let (mut result, destinations, _) = self
                .plan_store_existing_inventory_item_at_like_cpp(
                    source_bag,
                    source_slot,
                    destination_bag,
                    destination_slot,
                    swap,
                )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Inventory));
        }
        if is_bank_pos(destination_bag, destination_slot) {
            let (mut result, destinations) = self.plan_bank_existing_inventory_item_at_like_cpp(
                source_bag,
                source_slot,
                destination_bag,
                destination_slot,
                swap,
            )?;
            if require_exact_destination
                && result == InventoryResult::Ok
                && (destinations.len() != 1
                    || destinations[0].pos != destination_pos
                    || destinations[0].count != source_count)
            {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Bank));
        }
        if is_equipment_pos(destination_bag, destination_slot) {
            let (mut result, dest) = self.plan_equip_existing_inventory_item_like_cpp(
                source_bag,
                source_slot,
                destination_slot,
                swap,
            )?;
            if result == InventoryResult::Ok && dest != destination_pos {
                result = InventoryResult::InternalBagError;
            }
            return Some((result, InventorySwapTargetLikeCpp::Equipment { dest }));
        }

        Some((InventoryResult::Ok, InventorySwapTargetLikeCpp::None))
    }

    async fn execute_inventory_swap_positions_like_cpp(&mut self, src: u16, dst: u16) {
        let mut pending = VecDeque::from([(src, dst)]);
        let mut steps = 0usize;
        while let Some((step_src, step_dst)) = pending.pop_front() {
            steps += 1;
            if steps > 4 {
                self.send_equip_error(InventoryResult::InternalBagError, None, None, 0, 0);
                return;
            }
            match self
                .execute_inventory_swap_step_like_cpp(step_src, step_dst)
                .await
            {
                InventorySwapStepLikeCpp::Done => {}
                InventorySwapStepLikeCpp::ChildRedirect {
                    first_src,
                    first_dst,
                    second_src,
                    second_dst,
                } => {
                    pending.push_front((second_src, second_dst));
                    pending.push_front((first_src, first_dst));
                }
            }
        }
    }

    fn validate_inventory_redirected_empty_move_like_cpp(
        &self,
        src: u16,
        dst: u16,
    ) -> Result<Option<u32>, InventoryResult> {
        let Some(preflight) = self.plan_inventory_swap_preflight_like_cpp(src, dst) else {
            return Err(InventoryResult::InternalBagError);
        };
        match preflight.result {
            SwapItemPreflightResult::NoSource => return Ok(None),
            SwapItemPreflightResult::Error(result) => return Err(result),
            SwapItemPreflightResult::ChildRedirect { .. } => {
                return Err(InventoryResult::InternalBagError);
            }
            SwapItemPreflightResult::Continue => {}
        }

        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        if self.get_inventory_item_by_pos(dst_bag, dst_slot).is_some() {
            return Err(InventoryResult::InternalBagError);
        }
        let source = self
            .get_inventory_item_by_pos(src_bag, src_slot)
            .ok_or(InventoryResult::ItemNotFound)?;
        let count = self
            .inventory_item_objects_like_cpp()
            .get(&source.guid)
            .map(wow_entities::Item::count)
            .ok_or(InventoryResult::ItemNotFound)?;
        let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
            src_bag, src_slot, dst_bag, dst_slot, false, true,
        ) else {
            return Err(InventoryResult::InternalBagError);
        };
        if result != InventoryResult::Ok {
            return Err(result);
        }
        if matches!(target, InventorySwapTargetLikeCpp::None) {
            return Err(InventoryResult::InternalBagError);
        }
        Ok(Some(count))
    }

    /// Validate the two recursive `Player::SwapItem` moves against a temporary
    /// overlay before `AutoUnequipChildItem` is persisted. C++ performs those
    /// calls synchronously; preserving that observable order in Rust must not
    /// leave the child hidden when a later dead/combat/charmed/unequip/equip
    /// gate rejects either move.
    pub(super) fn plan_inventory_child_redirect_like_cpp(
        &mut self,
        child_bag: u8,
        child_slot: u8,
        first_src: u16,
        first_dst: u16,
        second_src: u16,
        second_dst: u16,
    ) -> Result<u8, InventoryResult> {
        let child_move = self
            .plan_inventory_storage_move_like_cpp(
                child_bag,
                child_slot,
                INVENTORY_SLOT_BAG_0,
                NULL_SLOT,
                InventoryStorageTargetLikeCpp::Inventory,
            )
            .ok_or(InventoryResult::ItemNotFound)??;
        if !child_move.existing_updates.is_empty() {
            return Err(InventoryResult::InternalBagError);
        }
        let Some((hidden_bag, hidden_slot, hidden_count)) = child_move.moved_destination else {
            return Err(InventoryResult::InternalBagError);
        };
        if hidden_bag != INVENTORY_SLOT_BAG_0
            || !is_child_equipment_pos(hidden_bag, hidden_slot)
            || hidden_count != child_move.source_count
        {
            return Err(InventoryResult::InternalBagError);
        }
        if !self.apply_committed_inventory_item_relocation_like_cpp(
            child_bag,
            child_slot,
            hidden_bag,
            hidden_slot,
            hidden_count,
        ) {
            return Err(InventoryResult::InternalBagError);
        }

        let first_count =
            self.validate_inventory_redirected_empty_move_like_cpp(first_src, first_dst);
        let mut first_applied_count = None;
        let validation = match first_count {
            Ok(Some(count)) => {
                let [first_src_bag, first_src_slot] = first_src.to_be_bytes();
                let [first_dst_bag, first_dst_slot] = first_dst.to_be_bytes();
                if self.apply_committed_inventory_item_relocation_like_cpp(
                    first_src_bag,
                    first_src_slot,
                    first_dst_bag,
                    first_dst_slot,
                    count,
                ) {
                    first_applied_count = Some(count);
                    self.validate_inventory_redirected_empty_move_like_cpp(second_src, second_dst)
                        .map(|_| ())
                } else {
                    Err(InventoryResult::InternalBagError)
                }
            }
            Ok(None) => self
                .validate_inventory_redirected_empty_move_like_cpp(second_src, second_dst)
                .map(|_| ()),
            Err(result) => Err(result),
        };

        let first_rolled_back = if let Some(count) = first_applied_count {
            let [first_src_bag, first_src_slot] = first_src.to_be_bytes();
            let [first_dst_bag, first_dst_slot] = first_dst.to_be_bytes();
            self.apply_committed_inventory_item_relocation_like_cpp(
                first_dst_bag,
                first_dst_slot,
                first_src_bag,
                first_src_slot,
                count,
            )
        } else {
            true
        };
        debug_assert!(first_rolled_back);
        let child_rolled_back = self.apply_committed_inventory_item_relocation_like_cpp(
            hidden_bag,
            hidden_slot,
            child_bag,
            child_slot,
            hidden_count,
        );
        debug_assert!(child_rolled_back);
        if !first_rolled_back || !child_rolled_back {
            return Err(InventoryResult::InternalBagError);
        }

        validation.map(|()| hidden_slot)
    }

    /// Current upstream TrinityCore calls `Player::AutoUnequipChildItem`
    /// before recursively continuing either child redirect in
    /// `Player::SwapItem`. The legacy 3.4.3 snapshot omitted that call and
    /// recurses on the unchanged equipped child forever. Persist the child in
    /// the already validated reserved slot so both queued moves observe its
    /// equipment position as empty.
    async fn execute_inventory_auto_unequip_child_item_like_cpp(
        &mut self,
        child_bag: u8,
        child_slot: u8,
        child_guid: ObjectGuid,
        hidden_slot: u8,
    ) -> bool {
        if is_child_equipment_pos(child_bag, child_slot) {
            return true;
        }

        self.execute_inventory_storage_move_like_cpp(
            child_bag,
            child_slot,
            INVENTORY_SLOT_BAG_0,
            hidden_slot,
            InventoryStorageTargetLikeCpp::Inventory,
            InventoryStorageQuestChecksLikeCpp::None,
            None,
        )
        .await;

        self.get_inventory_item_by_guid_like_cpp(child_guid)
            .is_some_and(|(bag, slot, _)| bag == INVENTORY_SLOT_BAG_0 && slot == hidden_slot)
    }

    /// Current upstream C++ `Player::CanEquipChildItem`. Rust represents the
    /// parent/child link with the child's `CHILD` flag plus creator GUID; the
    /// DB2 row supplies the visible equipment slot. When that slot is busy,
    /// validate where the displaced item can be stored before moving the
    /// parent, preserving C++'s no-partial-parent-move failure contract.
    pub(super) fn plan_inventory_equip_child_like_cpp(
        &self,
        parent_bag: u8,
        parent_slot: u8,
        parent_guid: ObjectGuid,
    ) -> Result<Option<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
        let Some(parent) = self.inventory_item_objects_like_cpp().get(&parent_guid) else {
            return Ok(None);
        };
        let Some(child_equipment) =
            self.item_child_equipment_for_parent_like_cpp(parent.object().entry())
        else {
            return Ok(None);
        };
        let destination_slot = child_equipment.child_item_equip_slot;
        if !is_equipment_pos(INVENTORY_SLOT_BAG_0, destination_slot) {
            return Err(InventoryResult::NotEquippable);
        }
        let Some(child) = self
            .inventory_item_objects_like_cpp()
            .values()
            .find(|item| {
                item.has_item_flag(ItemFieldFlags::CHILD)
                    && item.data().creator == parent_guid
                    && (child_equipment.child_item_id <= 0
                        || item.object().entry() == child_equipment.child_item_id as u32)
            })
        else {
            return Ok(None);
        };
        let child_guid = child.object().guid();
        if self
            .get_inventory_item_by_guid_like_cpp(child_guid)
            .is_some_and(|(bag, slot, _)| bag == INVENTORY_SLOT_BAG_0 && slot == destination_slot)
        {
            return Ok(None);
        }

        let Some(displaced) =
            self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, destination_slot)
        else {
            return Ok(Some(InventoryEquipChildPlanLikeCpp {
                child_guid,
                destination_slot,
                displaced_storage: None,
            }));
        };
        let displaced_object = self.inventory_item_objects_like_cpp().get(&displaced.guid);
        let displaced_proto = self.item_storage_template(displaced.entry_id);
        let child_is_bag = self
            .item_storage_template(child.object().entry())
            .is_some_and(|template| template.container_slots > 0);
        let can_unequip = self.can_unequip_inventory_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            destination_slot,
            !child_is_bag,
            displaced_object,
            displaced_proto.as_ref(),
            self.direct_item_contains_items(displaced.guid),
        );
        if can_unequip != InventoryResult::Ok {
            return Err(can_unequip);
        }

        let displaced_storage = if is_inventory_pos(parent_bag, parent_slot) {
            let mut last_result = InventoryResult::InvFull;
            let mut destination = None;
            for (bag, slot) in [(parent_bag, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _, _)) = self.plan_store_existing_inventory_item_at_like_cpp(
                    INVENTORY_SLOT_BAG_0,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                last_result = result;
                if result == InventoryResult::Ok {
                    destination = Some((bag, slot, InventoryStorageTargetLikeCpp::Inventory));
                    break;
                }
            }
            destination.ok_or(last_result)?
        } else if is_bank_pos(parent_bag, parent_slot) {
            let mut last_result = InventoryResult::BankFull;
            let mut destination = None;
            for (bag, slot) in [(parent_bag, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _)) = self.plan_bank_existing_inventory_item_at_like_cpp(
                    INVENTORY_SLOT_BAG_0,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                last_result = result;
                if result == InventoryResult::Ok {
                    destination = Some((bag, slot, InventoryStorageTargetLikeCpp::Bank));
                    break;
                }
            }
            destination.ok_or(last_result)?
        } else {
            return Err(InventoryResult::CantSwap);
        };

        Ok(Some(InventoryEquipChildPlanLikeCpp {
            child_guid,
            destination_slot,
            displaced_storage: Some(displaced_storage),
        }))
    }

    /// Current upstream C++ `Player::EquipChildItem`, executed only after the
    /// parent move and its preflight have succeeded.
    async fn execute_inventory_equip_child_like_cpp(
        &mut self,
        plan: InventoryEquipChildPlanLikeCpp,
    ) -> bool {
        if self
            .get_inventory_item_by_guid_like_cpp(plan.child_guid)
            .is_some_and(|(bag, slot, _)| {
                bag == INVENTORY_SLOT_BAG_0 && slot == plan.destination_slot
            })
        {
            return true;
        }

        if let Some((bag, slot, target)) = plan.displaced_storage {
            self.execute_inventory_storage_move_like_cpp(
                INVENTORY_SLOT_BAG_0,
                plan.destination_slot,
                bag,
                slot,
                target,
                InventoryStorageQuestChecksLikeCpp::None,
                None,
            )
            .await;
            if self
                .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, plan.destination_slot)
                .is_some()
            {
                return false;
            }
        }

        let Some((child_bag, child_slot, _)) =
            self.get_inventory_item_by_guid_like_cpp(plan.child_guid)
        else {
            return false;
        };
        self.execute_inventory_equip_to_empty_raw_like_cpp(
            child_bag,
            child_slot,
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, plan.destination_slot),
        )
        .await;
        self.get_inventory_item_by_guid_like_cpp(plan.child_guid)
            .is_some_and(|(bag, slot, _)| {
                bag == INVENTORY_SLOT_BAG_0 && slot == plan.destination_slot
            })
    }

    async fn execute_inventory_swap_step_like_cpp(
        &mut self,
        src: u16,
        dst: u16,
    ) -> InventorySwapStepLikeCpp {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let source = self.get_inventory_item_by_pos(src_bag, src_slot);
        let destination = self.get_inventory_item_by_pos(dst_bag, dst_slot);
        let source_guid = source.as_ref().map(|item| item.guid);
        let destination_guid = destination.as_ref().map(|item| item.guid);

        let Some(preflight) = self.plan_inventory_swap_preflight_like_cpp(src, dst) else {
            return InventorySwapStepLikeCpp::Done;
        };
        match preflight.result {
            SwapItemPreflightResult::NoSource => return InventorySwapStepLikeCpp::Done,
            SwapItemPreflightResult::ChildRedirect {
                first_src,
                first_dst,
                second_src,
                second_dst,
            } => {
                let child = source
                    .as_ref()
                    .filter(|item| {
                        is_equipment_pos(src_bag, src_slot)
                            && self
                                .inventory_item_objects_like_cpp()
                                .get(&item.guid)
                                .is_some_and(|object| object.has_item_flag(ItemFieldFlags::CHILD))
                    })
                    .map(|item| (src_bag, src_slot, item.guid))
                    .or_else(|| {
                        destination
                            .as_ref()
                            .filter(|item| {
                                is_equipment_pos(dst_bag, dst_slot)
                                    && self
                                        .inventory_item_objects_like_cpp()
                                        .get(&item.guid)
                                        .is_some_and(|object| {
                                            object.has_item_flag(ItemFieldFlags::CHILD)
                                        })
                            })
                            .map(|item| (dst_bag, dst_slot, item.guid))
                    });
                let Some((child_bag, child_slot, child_guid)) = child else {
                    return InventorySwapStepLikeCpp::Done;
                };
                let hidden_slot = match self.plan_inventory_child_redirect_like_cpp(
                    child_bag, child_slot, first_src, first_dst, second_src, second_dst,
                ) {
                    Ok(hidden_slot) => hidden_slot,
                    Err(result) => {
                        self.send_equip_error(result, source_guid, destination_guid, 0, 0);
                        return InventorySwapStepLikeCpp::Done;
                    }
                };
                if !self
                    .execute_inventory_auto_unequip_child_item_like_cpp(
                        child_bag,
                        child_slot,
                        child_guid,
                        hidden_slot,
                    )
                    .await
                {
                    return InventorySwapStepLikeCpp::Done;
                }
                return InventorySwapStepLikeCpp::ChildRedirect {
                    first_src,
                    first_dst,
                    second_src,
                    second_dst,
                };
            }
            SwapItemPreflightResult::Error(result) => {
                self.send_equip_error(result, source_guid, destination_guid, 0, 0);
                return InventorySwapStepLikeCpp::Done;
            }
            SwapItemPreflightResult::Continue => {}
        }

        let Some(source) = source else {
            return InventorySwapStepLikeCpp::Done;
        };
        let source_limit_category = self
            .item_storage_template(source.entry_id)
            .map_or(0, |template| template.item_limit_category);

        let Some(destination) = destination else {
            let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
                src_bag, src_slot, dst_bag, dst_slot, false, true,
            ) else {
                return InventorySwapStepLikeCpp::Done;
            };
            if result != InventoryResult::Ok {
                self.send_equip_error(result, Some(source.guid), None, 0, source_limit_category);
                return InventorySwapStepLikeCpp::Done;
            }
            match target {
                InventorySwapTargetLikeCpp::Inventory => {
                    self.execute_inventory_storage_move_like_cpp(
                        src_bag,
                        src_slot,
                        dst_bag,
                        dst_slot,
                        InventoryStorageTargetLikeCpp::Inventory,
                        InventoryStorageQuestChecksLikeCpp::None,
                        None,
                    )
                    .await;
                }
                InventorySwapTargetLikeCpp::Bank => {
                    self.execute_inventory_storage_move_like_cpp(
                        src_bag,
                        src_slot,
                        dst_bag,
                        dst_slot,
                        InventoryStorageTargetLikeCpp::Bank,
                        InventoryStorageQuestChecksLikeCpp::None,
                        None,
                    )
                    .await;
                }
                InventorySwapTargetLikeCpp::Equipment { dest } => {
                    self.execute_inventory_equip_to_empty_like_cpp(src_bag, src_slot, dest)
                        .await;
                }
                InventorySwapTargetLikeCpp::None => {}
            }
            return InventorySwapStepLikeCpp::Done;
        };

        let source_is_bag = self
            .item_storage_template(source.entry_id)
            .is_some_and(|template| template.container_slots > 0);
        let destination_is_bag = self
            .item_storage_template(destination.entry_id)
            .is_some_and(|template| template.container_slots > 0);
        if !source_is_bag && !destination_is_bag && source.entry_id == destination.entry_id {
            let Some((result, target)) = self.validate_inventory_swap_target_like_cpp(
                src_bag, src_slot, dst_bag, dst_slot, false, false,
            ) else {
                return InventorySwapStepLikeCpp::Done;
            };
            if result == InventoryResult::Ok && !matches!(target, InventorySwapTargetLikeCpp::None)
            {
                self.execute_inventory_stack_merge_like_cpp(
                    src_bag,
                    src_slot,
                    dst_bag,
                    dst_slot,
                    source,
                    destination,
                )
                .await;
                return InventorySwapStepLikeCpp::Done;
            }
        }

        let Some((source_result, source_target)) = self.validate_inventory_swap_target_like_cpp(
            src_bag, src_slot, dst_bag, dst_slot, true, true,
        ) else {
            return InventorySwapStepLikeCpp::Done;
        };
        if source_result != InventoryResult::Ok {
            self.send_equip_error(
                source_result,
                Some(source.guid),
                Some(destination.guid),
                0,
                source_limit_category,
            );
            return InventorySwapStepLikeCpp::Done;
        }
        let destination_limit_category = self
            .item_storage_template(destination.entry_id)
            .map_or(0, |template| template.item_limit_category);
        let Some((destination_result, destination_target)) = self
            .validate_inventory_swap_target_like_cpp(
                dst_bag, dst_slot, src_bag, src_slot, true, true,
            )
        else {
            return InventorySwapStepLikeCpp::Done;
        };
        if destination_result != InventoryResult::Ok {
            self.send_equip_error(
                destination_result,
                Some(destination.guid),
                Some(source.guid),
                0,
                destination_limit_category,
            );
            return InventorySwapStepLikeCpp::Done;
        }

        self.execute_inventory_real_swap_like_cpp(
            src_bag,
            src_slot,
            dst_bag,
            dst_slot,
            source,
            destination,
            source_target,
            destination_target,
        )
        .await;
        InventorySwapStepLikeCpp::Done
    }

    async fn execute_inventory_equip_to_empty_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return;
        };
        let child_plan =
            match self.plan_inventory_equip_child_like_cpp(source_bag, source_slot, source.guid) {
                Ok(plan) => plan,
                Err(result) => {
                    self.send_equip_error(result, Some(source.guid), None, 0, 0);
                    return;
                }
            };
        let source_guid = source.guid;
        self.execute_inventory_equip_to_empty_raw_like_cpp(source_bag, source_slot, destination)
            .await;
        let [destination_bag, destination_slot] = destination.to_be_bytes();
        if !self
            .get_inventory_item_by_guid_like_cpp(source_guid)
            .is_some_and(|(bag, slot, _)| bag == destination_bag && slot == destination_slot)
        {
            return;
        }
        if let Some(plan) = child_plan {
            let _ = self.execute_inventory_equip_child_like_cpp(plan).await;
        }
        self.execute_inventory_auto_unequip_offhand_if_need_like_cpp()
            .await;
    }

    async fn execute_inventory_equip_to_empty_raw_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination: u16,
    ) {
        let [destination_bag, destination_slot] = destination.to_be_bytes();
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(source) = self.get_inventory_item_by_pos(source_bag, source_slot) else {
            return;
        };
        let Some(runtime_item) = self
            .inventory_item_objects_like_cpp()
            .get(&source.guid)
            .cloned()
        else {
            return;
        };
        let Some((enchantments, cleared_enchantments)) = self
            .inventory_remove_enchantment_persistence_like_cpp(
                source.guid,
                source_bag == INVENTORY_SLOT_BAG_0
                    && source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        else {
            return;
        };
        let mut planned_item = runtime_item.clone();
        bind_inventory_item_for_destination_like_cpp(&mut planned_item, destination);
        let dynamic_flags_changed =
            item_dynamic_flags_changed_like_cpp(&runtime_item, &planned_item);
        for slot in &cleared_enchantments {
            planned_item.clear_enchantment(*slot);
        }
        let mutable = item_storage_mutable_persistence_like_cpp(
            source.db_guid,
            &planned_item,
            planned_item.count(),
            planned_item.item_flags_bits(),
            enchantments,
            self.item_effect_count_like_cpp(source.entry_id),
        );
        let Some(container_db_guid) = self.inventory_container_db_guid_like_cpp(destination_bag)
        else {
            return;
        };
        let Some(inventory_port) = self.player_inventory_persistence_port_like_cpp() else {
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                None,
                0,
                0,
            );
            return;
        };
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::Equip(
            wow_persistence::InventoryEquipPersistenceLikeCpp {
                mutable_item: mutable,
                delete_source_link_owner_guid: player_guid.counter() as u64,
                delete_source_link_item_guid: source.db_guid,
                destination_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: container_db_guid,
                    slot: destination_slot,
                    item_guid: source.db_guid,
                },
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
                destination_bag,
                destination_slot,
                error = %reason,
                "inventory equip transaction failed; runtime left unchanged"
            );
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                None,
                0,
                0,
            );
            return;
        }

        let removed_mods = self.apply_inventory_item_remove_side_effects_like_cpp(
            source_bag,
            source_slot,
            source.guid,
            &cleared_enchantments,
        );
        let relocated = self.apply_committed_inventory_item_relocation_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
            runtime_item.count(),
        );
        debug_assert!(relocated);
        self.update_inventory_item_object_like_cpp(source.guid, |item| {
            item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                planned_item.item_flags_bits(),
            ));
        });
        let added_mods = self.apply_inventory_item_store_side_effects_like_cpp(
            destination_bag,
            destination_slot,
            source.guid,
        );
        self.publish_inventory_position_changes_like_cpp(&[
            (source_bag, source_slot),
            (destination_bag, destination_slot),
        ]);
        self.send_item_relocation_values_update_like_cpp(source.guid, true, &cleared_enchantments);
        if dynamic_flags_changed {
            // C++ VisualizeItem dirties ITEM_DATA_DYNAMIC_FLAGS when the
            // destination applies OnEquip/OnAcquire binding.
            self.send_item_dynamic_flags_values_update_like_cpp(source.guid);
        }
        if removed_mods || added_mods {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        self.record_represented_titan_grip_penalty_action_like_cpp();
        self.record_represented_avg_equipped_item_level_update_like_cpp();
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
    }

    /// C++ `Player::AutoUnequipOffhandIfNeed` after an equipment move. The
    /// normal two-hand equip path has already proved `CanStoreItem`, so use the
    /// same persisted storage executor rather than the older runtime-only
    /// represented helper.
    async fn execute_inventory_auto_unequip_offhand_if_need_like_cpp(&mut self) {
        let Some(reason) = self.represented_auto_unequip_offhand_reason_like_cpp(false) else {
            return;
        };
        let Some(offhand) = self
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND)
        else {
            return;
        };
        let offhand_guid = offhand.guid;
        let offhand_entry = offhand.entry_id;

        self.execute_inventory_storage_move_like_cpp(
            INVENTORY_SLOT_BAG_0,
            wow_entities::EQUIPMENT_SLOT_OFFHAND,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
            InventoryStorageQuestChecksLikeCpp::None,
            None,
        )
        .await;

        if self
            .get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, wow_entities::EQUIPMENT_SLOT_OFFHAND)
            .is_some_and(|item| item.guid == offhand_guid)
        {
            return;
        }
        let stored_destination = self
            .get_inventory_item_by_guid_like_cpp(offhand_guid)
            .map(|(bag, slot, _)| (bag, slot));
        self.record_represented_auto_unequip_offhand_request_like_cpp(
            RepresentedAutoUnequipOffhandLikeCpp {
                item_guid: offhand_guid,
                item_entry: offhand_entry,
                reason,
                stored_destination,
                needs_mail_fallback: false,
            },
        );
    }

    async fn execute_inventory_stack_merge_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        source: InventoryItem,
        destination: InventoryItem,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(source_object) = self
            .inventory_item_objects_like_cpp()
            .get(&source.guid)
            .cloned()
        else {
            return;
        };
        let Some(destination_object) = self
            .inventory_item_objects_like_cpp()
            .get(&destination.guid)
            .cloned()
        else {
            return;
        };
        let max_stack = self
            .item_storage_template(source.entry_id)
            .map_or(1, |template| template.max_stack_size.max(1));
        if destination_object.count() >= max_stack {
            return;
        }
        let total = source_object
            .count()
            .saturating_add(destination_object.count());
        let destination_count = total.min(max_stack);
        let source_count = total.saturating_sub(destination_count);
        let Some((source_enchantments, source_cleared)) = self
            .inventory_remove_enchantment_persistence_like_cpp(
                source.guid,
                source_bag == INVENTORY_SLOT_BAG_0
                    && source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        else {
            return;
        };
        let Some((destination_enchantments, _)) =
            self.inventory_remove_enchantment_persistence_like_cpp(destination.guid, false)
        else {
            return;
        };
        let source_mutable = item_storage_mutable_persistence_like_cpp(
            source.db_guid,
            &source_object,
            source_count,
            source_object.item_flags_bits(),
            source_enchantments,
            self.item_effect_count_like_cpp(source.entry_id),
        );
        let destination_mutable = item_storage_mutable_persistence_like_cpp(
            destination.db_guid,
            &destination_object,
            destination_count,
            destination_object.item_flags_bits(),
            destination_enchantments,
            self.item_effect_count_like_cpp(destination.entry_id),
        );
        let Some(inventory_port) = self.player_inventory_persistence_port_like_cpp() else {
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                Some(destination.guid),
                0,
                0,
            );
            return;
        };
        let source_persistence = if source_count > 0 {
            wow_persistence::InventoryStackMergeSourcePersistenceLikeCpp::Retained(source_mutable)
        } else {
            wow_persistence::InventoryStackMergeSourcePersistenceLikeCpp::FullyMerged {
                item_guid: source.db_guid,
            }
        };
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::StackMerge(
            wow_persistence::InventoryStackMergePersistenceLikeCpp {
                owner_guid: player_guid.counter() as u64,
                destination_item: destination_mutable,
                source: source_persistence,
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
                destination_bag,
                destination_slot,
                error = %reason,
                "inventory stack merge transaction failed; runtime left unchanged"
            );
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                Some(destination.guid),
                0,
                0,
            );
            return;
        }

        self.update_inventory_item_object_like_cpp(destination.guid, |item| {
            item.set_count(destination_count);
        });
        self.send_packet(&UpdateObject::item_stack_count_update(
            destination.guid,
            self.player_map_id_like_cpp(),
            destination_count,
        ));
        if source_count > 0 {
            self.update_inventory_item_object_like_cpp(source.guid, |item| {
                item.set_count(source_count);
                for slot in &source_cleared {
                    item.clear_enchantment(*slot);
                }
            });
            self.send_packet(&UpdateObject::item_stack_count_update(
                source.guid,
                self.player_map_id_like_cpp(),
                source_count,
            ));
        } else {
            let removed_mods = self.apply_inventory_item_remove_side_effects_like_cpp(
                source_bag,
                source_slot,
                source.guid,
                &source_cleared,
            );
            let removed = self.apply_committed_inventory_item_removal_like_cpp(
                source_bag,
                source_slot,
                source.guid,
            );
            debug_assert!(removed);
            self.send_packet(&UpdateObject::destroy_objects(
                vec![source.guid],
                self.player_map_id_like_cpp(),
            ));
            self.publish_inventory_position_changes_like_cpp(&[(source_bag, source_slot)]);
            if removed_mods {
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        }
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        if is_equipment_pos(destination_bag, destination_slot) {
            self.execute_inventory_auto_unequip_offhand_if_need_like_cpp()
                .await;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn plan_inventory_real_swap_children_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        source_guid: ObjectGuid,
        destination_bag: u8,
        destination_slot: u8,
        destination_guid: ObjectGuid,
    ) -> Result<Vec<InventoryEquipChildPlanLikeCpp>, InventoryResult> {
        let mut plans = Vec::new();
        if is_equipment_pos(destination_bag, destination_slot) {
            if let Some(plan) =
                self.plan_inventory_equip_child_like_cpp(source_bag, source_slot, source_guid)?
            {
                plans.push(plan);
            }
        }
        if is_equipment_pos(source_bag, source_slot) {
            if let Some(plan) = self.plan_inventory_equip_child_like_cpp(
                destination_bag,
                destination_slot,
                destination_guid,
            )? {
                plans.push(plan);
            }
        }
        Ok(plans)
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_inventory_real_swap_like_cpp(
        &mut self,
        source_bag: u8,
        source_slot: u8,
        destination_bag: u8,
        destination_slot: u8,
        source: InventoryItem,
        destination: InventoryItem,
        _source_target: InventorySwapTargetLikeCpp,
        _destination_target: InventorySwapTargetLikeCpp,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(source_object) = self
            .inventory_item_objects_like_cpp()
            .get(&source.guid)
            .cloned()
        else {
            return;
        };
        let Some(destination_object) = self
            .inventory_item_objects_like_cpp()
            .get(&destination.guid)
            .cloned()
        else {
            return;
        };
        let child_plans = match self.plan_inventory_real_swap_children_like_cpp(
            source_bag,
            source_slot,
            source.guid,
            destination_bag,
            destination_slot,
            destination.guid,
        ) {
            Ok(plans) => plans,
            Err(result) => {
                self.send_equip_error(result, Some(source.guid), Some(destination.guid), 0, 0);
                return;
            }
        };
        let Some(source_container_db_guid) = self.inventory_container_db_guid_like_cpp(source_bag)
        else {
            return;
        };
        let Some(destination_container_db_guid) =
            self.inventory_container_db_guid_like_cpp(destination_bag)
        else {
            return;
        };
        let Some((source_enchantments, source_cleared)) = self
            .inventory_remove_enchantment_persistence_like_cpp(
                source.guid,
                source_bag == INVENTORY_SLOT_BAG_0
                    && source_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        else {
            return;
        };
        let Some((destination_enchantments, destination_cleared)) = self
            .inventory_remove_enchantment_persistence_like_cpp(
                destination.guid,
                destination_bag == INVENTORY_SLOT_BAG_0
                    && destination_slot == wow_entities::EQUIPMENT_SLOT_MAINHAND,
            )
        else {
            return;
        };

        let source_destination_pos = wow_entities::make_item_pos(destination_bag, destination_slot);
        let destination_source_pos = wow_entities::make_item_pos(source_bag, source_slot);
        let mut planned_source = source_object.clone();
        bind_inventory_item_for_destination_like_cpp(&mut planned_source, source_destination_pos);
        let source_dynamic_flags_changed =
            item_dynamic_flags_changed_like_cpp(&source_object, &planned_source);
        for slot in &source_cleared {
            planned_source.clear_enchantment(*slot);
        }
        let mut planned_destination = destination_object.clone();
        bind_inventory_item_for_destination_like_cpp(
            &mut planned_destination,
            destination_source_pos,
        );
        let destination_dynamic_flags_changed =
            item_dynamic_flags_changed_like_cpp(&destination_object, &planned_destination);
        for slot in &destination_cleared {
            planned_destination.clear_enchantment(*slot);
        }

        let source_mutable = item_storage_mutable_persistence_like_cpp(
            source.db_guid,
            &planned_source,
            planned_source.count(),
            planned_source.item_flags_bits(),
            source_enchantments,
            self.item_effect_count_like_cpp(source.entry_id),
        );
        let destination_mutable = item_storage_mutable_persistence_like_cpp(
            destination.db_guid,
            &planned_destination,
            planned_destination.count(),
            planned_destination.item_flags_bits(),
            destination_enchantments,
            self.item_effect_count_like_cpp(destination.entry_id),
        );

        // C++ exchanges the contents when an empty bag outside a bag slot is
        // swapped with a non-empty equipped/bank bag. Persist those child
        // container changes in the same transaction as the two bag positions.
        let source_template = self.item_storage_template(source.entry_id);
        let destination_template = self.item_storage_template(destination.entry_id);
        let source_children = self
            .inventory_item_objects_like_cpp()
            .values()
            .filter(|item| item.container_guid() == source.guid)
            .map(|item| (item.slot(), item.object().guid(), item.object().entry()))
            .collect::<Vec<_>>();
        let destination_children = self
            .inventory_item_objects_like_cpp()
            .values()
            .filter(|item| item.container_guid() == destination.guid)
            .map(|item| (item.slot(), item.object().guid(), item.object().entry()))
            .collect::<Vec<_>>();
        let bag_exchange = match (source_template.as_ref(), destination_template.as_ref()) {
            (Some(source_proto), Some(destination_proto))
                if source_proto.container_slots > 0 && destination_proto.container_slots > 0 =>
            {
                if source_children.is_empty()
                    && !wow_entities::is_bag_pos(destination_source_pos)
                    && !destination_children.is_empty()
                {
                    Some((
                        source.guid,
                        source.db_guid,
                        source_proto,
                        destination.guid,
                        destination_children.clone(),
                    ))
                } else if destination_children.is_empty()
                    && !wow_entities::is_bag_pos(source_destination_pos)
                    && !source_children.is_empty()
                {
                    Some((
                        destination.guid,
                        destination.db_guid,
                        destination_proto,
                        source.guid,
                        source_children.clone(),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };
        let mut child_moves = Vec::new();
        if let Some((empty_guid, empty_db_guid, empty_proto, full_guid, mut children)) =
            bag_exchange
        {
            children.sort_by_key(|(slot, guid, _)| (*slot, guid.counter()));
            if children.len() > usize::from(empty_proto.container_slots) {
                self.send_equip_error(
                    InventoryResult::CantSwap,
                    Some(source.guid),
                    Some(destination.guid),
                    0,
                    0,
                );
                return;
            }
            for (to_slot, (from_slot, child_guid, child_entry)) in children.into_iter().enumerate()
            {
                let Some(child_proto) = self.item_storage_template(child_entry) else {
                    self.send_equip_error(
                        InventoryResult::BagInBag,
                        Some(source.guid),
                        Some(destination.guid),
                        0,
                        0,
                    );
                    return;
                };
                if !item_can_go_into_bag(&child_proto, empty_proto) {
                    self.send_equip_error(
                        InventoryResult::BagInBag,
                        Some(source.guid),
                        Some(destination.guid),
                        0,
                        0,
                    );
                    return;
                }
                child_moves.push((
                    child_guid,
                    child_guid.counter() as u64,
                    full_guid,
                    empty_guid,
                    empty_db_guid,
                    from_slot,
                    to_slot as u8,
                ));
            }
        }

        let Some(inventory_port) = self.player_inventory_persistence_port_like_cpp() else {
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                Some(destination.guid),
                0,
                0,
            );
            return;
        };
        let child_links = child_moves
            .iter()
            .map(|(_, child_db_guid, _, _, empty_db_guid, _, to_slot)| {
                wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: *empty_db_guid,
                    slot: *to_slot,
                    item_guid: *child_db_guid,
                }
            })
            .collect();
        let request = wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::Swap(
            wow_persistence::InventorySwapPersistenceLikeCpp {
                source_item: source_mutable,
                destination_item: destination_mutable,
                child_links,
                source_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: destination_container_db_guid,
                    slot: destination_slot,
                    item_guid: source.db_guid,
                },
                destination_link: wow_persistence::InventoryLinkPersistenceLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    bag_guid: source_container_db_guid,
                    slot: source_slot,
                    item_guid: destination.db_guid,
                },
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
                destination_bag,
                destination_slot,
                error = %reason,
                "inventory real swap transaction failed; runtime left unchanged"
            );
            self.send_equip_error(
                InventoryResult::InternalBagError,
                Some(source.guid),
                Some(destination.guid),
                0,
                0,
            );
            return;
        }

        let removed_source_mods = self.apply_inventory_item_remove_side_effects_like_cpp(
            source_bag,
            source_slot,
            source.guid,
            &source_cleared,
        );
        let removed_destination_mods = self.apply_inventory_item_remove_side_effects_like_cpp(
            destination_bag,
            destination_slot,
            destination.guid,
            &destination_cleared,
        );
        for (child_guid, _, _, empty_guid, _, _, to_slot) in &child_moves {
            self.update_inventory_item_object_like_cpp(*child_guid, |item| {
                relocate_bag_exchange_child_like_cpp(item, *empty_guid, *to_slot);
            });
        }
        let swapped = self.apply_committed_inventory_item_swap_like_cpp(
            source_bag,
            source_slot,
            destination_bag,
            destination_slot,
        );
        debug_assert!(swapped);
        self.update_inventory_item_object_like_cpp(source.guid, |item| {
            item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                planned_source.item_flags_bits(),
            ));
        });
        self.update_inventory_item_object_like_cpp(destination.guid, |item| {
            item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                planned_destination.item_flags_bits(),
            ));
        });
        let added_source_mods = self.apply_inventory_item_store_side_effects_like_cpp(
            destination_bag,
            destination_slot,
            source.guid,
        );
        let added_destination_mods = self.apply_inventory_item_store_side_effects_like_cpp(
            source_bag,
            source_slot,
            destination.guid,
        );

        self.publish_inventory_position_changes_like_cpp(&[
            (source_bag, source_slot),
            (destination_bag, destination_slot),
        ]);
        self.send_item_relocation_values_update_like_cpp(source.guid, true, &source_cleared);
        if source_dynamic_flags_changed {
            self.send_item_dynamic_flags_values_update_like_cpp(source.guid);
        }
        self.send_item_relocation_values_update_like_cpp(
            destination.guid,
            true,
            &destination_cleared,
        );
        if destination_dynamic_flags_changed {
            self.send_item_dynamic_flags_values_update_like_cpp(destination.guid);
        }
        for (child_guid, _, full_guid, empty_guid, _, from_slot, to_slot) in &child_moves {
            self.send_item_relocation_values_update_like_cpp(*child_guid, false, &[]);
            self.send_bag_object_slot_values_update_like_cpp(*full_guid, *from_slot);
            self.send_bag_object_slot_values_update_like_cpp(*empty_guid, *to_slot);
        }
        if removed_source_mods
            || removed_destination_mods
            || added_source_mods
            || added_destination_mods
        {
            self.send_represented_item_bonus_player_stat_update_like_cpp();
        }
        // Preserve the local 3.4.3 C++ ordering in Player::SwapItem: exchange
        // bag contents first, then inspect the items still contained by bags
        // that occupied src/dst bag slots. Snapshotting the pre-exchange
        // contents here would release loot in cases where C++ does not.
        let source_moved_bag_has_active_loot = wow_entities::is_bag_pos(destination_source_pos)
            && self.represented_bag_contains_active_item_loot_like_cpp(source.guid);
        let destination_moved_bag_has_active_loot =
            wow_entities::is_bag_pos(source_destination_pos)
                && self.represented_bag_contains_active_item_loot_like_cpp(destination.guid);
        if source_moved_bag_has_active_loot || destination_moved_bag_has_active_loot {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.record_represented_titan_grip_penalty_action_like_cpp();
        self.record_represented_avg_equipped_item_level_update_like_cpp();
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        for plan in child_plans {
            let _ = self.execute_inventory_equip_child_like_cpp(plan).await;
        }
        self.execute_inventory_auto_unequip_offhand_if_need_like_cpp()
            .await;
    }

    fn publish_inventory_position_changes_like_cpp(&mut self, positions: &[(u8, u8)]) {
        let mut unique_positions = positions.to_vec();
        unique_positions.sort_unstable();
        unique_positions.dedup();
        let mut top_level_changes = Vec::new();
        let mut visible_item_changes = Vec::new();
        let mut virtual_item_changes = Vec::new();
        let mut gear_changed = false;

        for (bag, slot) in unique_positions {
            if bag != INVENTORY_SLOT_BAG_0 {
                self.send_bag_slot_values_update_like_cpp(bag, slot);
                continue;
            }
            let item = self.get_inventory_item_by_pos(bag, slot);
            top_level_changes.push((
                slot,
                item.as_ref().map_or(ObjectGuid::EMPTY, |item| item.guid),
            ));
            if slot < 19 {
                gear_changed = true;
                visible_item_changes.push((
                    slot,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((
                    slot - 15,
                    item.as_ref().map_or(0, |item| item.entry_id as i32),
                    0,
                    0,
                ));
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
        if gear_changed {
            self.send_stat_update();
        }
    }

    /// Handle CMSG_SWAP_INV_ITEM: drag-and-drop item between two inventory slots.
    pub async fn handle_swap_inv_item(&mut self, swap: SwapInvItem) {
        if swap.inv_update.items.len() != 2 {
            warn!(
                "HandleSwapInvItemOpcode - Invalid itemCount ({})",
                swap.inv_update.items.len()
            );
            return;
        }

        if self.player_guid().is_none() || swap.src_slot == swap.dst_slot {
            return;
        }
        if !self.is_valid_inventory_pos_like_cpp(INVENTORY_SLOT_BAG_0, swap.src_slot, true) {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return;
        }
        if !self.is_valid_inventory_pos_like_cpp(INVENTORY_SLOT_BAG_0, swap.dst_slot, true) {
            self.send_equip_error(InventoryResult::WrongSlot, None, None, 0, 0);
            return;
        }
        if (is_bank_pos(INVENTORY_SLOT_BAG_0, swap.src_slot)
            || is_bank_pos(INVENTORY_SLOT_BAG_0, swap.dst_slot))
            && !self.represented_can_use_current_bank_like_cpp()
        {
            return;
        }
        self.execute_inventory_swap_positions_like_cpp(
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.src_slot),
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.dst_slot),
        )
        .await;
    }

    /// Handle CMSG_AUTO_EQUIP_ITEM: right-click to auto-equip/unequip an item.
    pub async fn handle_auto_equip_item(&mut self, equip: AutoEquipItem) {
        if equip.inv_update.items.len() != 1 {
            warn!(
                "HandleAutoEquipItemOpcode - Invalid itemCount ({})",
                equip.inv_update.items.len()
            );
            return;
        }

        if self.player_guid().is_none() {
            return;
        }
        // Audited legacy 3.4.3 and current upstream TrinityCore intentionally
        // do not call WorldSession::CanUseBank in AutoEquipItem. The explicit
        // bank-interaction gates belong to HandleSwapInvItem/HandleSwapItem;
        // preserve the observable C++ handler contract here.
        let Some(source) = self.get_inventory_item_by_pos(equip.pack_slot, equip.slot) else {
            return;
        };
        let source_is_bag = self
            .item_storage_template(source.entry_id)
            .is_some_and(|template| template.container_slots > 0);
        let Some((result, destination)) = self.plan_equip_existing_inventory_item_like_cpp(
            equip.pack_slot,
            equip.slot,
            NULL_SLOT,
            !source_is_bag,
        ) else {
            return;
        };
        if result != InventoryResult::Ok {
            self.send_equip_error(result, Some(source.guid), None, 0, 0);
            return;
        }
        let source_pos = wow_entities::make_item_pos(equip.pack_slot, equip.slot);
        if source_pos == destination {
            return;
        }

        // C++ HandleAutoEquipItemOpcode first tries to place the displaced
        // equipment item back at the source position, then in the source bag,
        // and finally anywhere. Player::SwapItem only covers the first case,
        // so preserve the two fallback searches before executing the equip.
        let [destination_bag, destination_slot] = destination.to_be_bytes();
        let Some(displaced) = self.get_inventory_item_by_pos(destination_bag, destination_slot)
        else {
            self.execute_inventory_swap_positions_like_cpp(source_pos, destination)
                .await;
            return;
        };
        let displaced_is_child = self
            .inventory_item_objects_like_cpp()
            .get(&displaced.guid)
            .is_some_and(|item| item.has_item_flag(ItemFieldFlags::CHILD));
        if displaced_is_child {
            self.execute_inventory_swap_positions_like_cpp(source_pos, destination)
                .await;
            return;
        }

        let Some(preflight) = self.plan_inventory_swap_preflight_like_cpp(source_pos, destination)
        else {
            return;
        };
        match preflight.result {
            SwapItemPreflightResult::NoSource => return,
            SwapItemPreflightResult::ChildRedirect { .. } => {
                self.execute_inventory_swap_positions_like_cpp(source_pos, destination)
                    .await;
                return;
            }
            SwapItemPreflightResult::Error(result) => {
                self.send_equip_error(result, Some(source.guid), Some(displaced.guid), 0, 0);
                return;
            }
            SwapItemPreflightResult::Continue => {}
        }

        let exact_displaced_result = self
            .validate_inventory_swap_target_like_cpp(
                destination_bag,
                destination_slot,
                equip.pack_slot,
                equip.slot,
                true,
                true,
            )
            .map_or(InventoryResult::CantSwap, |(result, _)| result);
        if exact_displaced_result == InventoryResult::Ok {
            self.execute_inventory_swap_positions_like_cpp(source_pos, destination)
                .await;
            return;
        }

        let mut fallback_error = exact_displaced_result;
        let fallback = if is_inventory_pos(equip.pack_slot, equip.slot) {
            let mut fallback = None;
            for (bag, slot) in [(equip.pack_slot, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _, _)) = self.plan_store_existing_inventory_item_at_like_cpp(
                    destination_bag,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                fallback_error = result;
                if result == InventoryResult::Ok {
                    fallback = Some((bag, slot, InventoryStorageTargetLikeCpp::Inventory));
                    break;
                }
            }
            fallback
        } else if is_bank_pos(equip.pack_slot, equip.slot) {
            let mut fallback = None;
            for (bag, slot) in [(equip.pack_slot, NULL_SLOT), (NULL_BAG, NULL_SLOT)] {
                let Some((result, _)) = self.plan_bank_existing_inventory_item_at_like_cpp(
                    destination_bag,
                    destination_slot,
                    bag,
                    slot,
                    true,
                ) else {
                    continue;
                };
                fallback_error = result;
                if result == InventoryResult::Ok {
                    fallback = Some((bag, slot, InventoryStorageTargetLikeCpp::Bank));
                    break;
                }
            }
            fallback
        } else {
            None
        };
        let Some((fallback_bag, fallback_slot, fallback_target)) = fallback else {
            self.send_equip_error(
                fallback_error,
                Some(displaced.guid),
                Some(source.guid),
                0,
                0,
            );
            return;
        };

        self.execute_inventory_storage_move_like_cpp(
            destination_bag,
            destination_slot,
            fallback_bag,
            fallback_slot,
            fallback_target,
            InventoryStorageQuestChecksLikeCpp::None,
            None,
        )
        .await;
        if self
            .get_inventory_item_by_pos(destination_bag, destination_slot)
            .is_some()
        {
            return;
        }
        self.execute_inventory_equip_to_empty_like_cpp(equip.pack_slot, equip.slot, destination)
            .await;
    }

    /// Handle CMSG_AUTO_EQUIP_ITEM_SLOT.
    ///
    /// C++ treats this as an explicit GUID + destination equipment-slot swap:
    /// it requires exactly one `InvUpdate` source position, verifies that the
    /// GUID still lives at that source position, rejects src==dst, then calls
    /// `Player::SwapItem` with the packed source position.
    pub async fn handle_auto_equip_item_slot(&mut self, equip: AutoEquipItemSlot) {
        if self.player_guid().is_none() {
            return;
        }

        if equip.inv_update.items.len() != 1
            || !is_equipment_pos(INVENTORY_SLOT_BAG_0, equip.item_dst_slot)
        {
            return;
        }

        let (container_slot, src_slot) = equip.inv_update.items[0];
        let Some((actual_bag, actual_slot, _item)) =
            self.get_inventory_item_by_guid_like_cpp(equip.item)
        else {
            return;
        };

        if actual_bag != container_slot || actual_slot != src_slot {
            return;
        }

        let source = wow_entities::make_item_pos(container_slot, src_slot);
        let destination = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, equip.item_dst_slot);
        if source == destination {
            return;
        }
        self.execute_inventory_swap_positions_like_cpp(source, destination)
            .await;
    }

    /// Handle CMSG_SWAP_ITEM: C++ container-aware swap between two positions.
    /// C++ ref: `WorldSession::HandleSwapItem`
    /// (`Handlers/ItemHandler.cpp:130-173`).
    pub async fn handle_swap_item(&mut self, swap: wow_packet::packets::item::SwapItem) {
        if swap.inv_update.items.len() != 2 {
            warn!(
                "HandleSwapItem - Invalid itemCount ({})",
                swap.inv_update.items.len()
            );
            return;
        }

        if self.player_guid().is_none() {
            return;
        }
        let source = wow_entities::make_item_pos(swap.container_slot_a, swap.slot_a);
        let destination = wow_entities::make_item_pos(swap.container_slot_b, swap.slot_b);
        if source == destination {
            return;
        }
        if !self.is_valid_inventory_pos_like_cpp(swap.container_slot_a, swap.slot_a, true) {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return;
        }
        if !self.is_valid_inventory_pos_like_cpp(swap.container_slot_b, swap.slot_b, true) {
            self.send_equip_error(InventoryResult::WrongSlot, None, None, 0, 0);
            return;
        }
        if (is_bank_pos(swap.container_slot_a, swap.slot_a)
            || is_bank_pos(swap.container_slot_b, swap.slot_b))
            && !self.represented_can_use_current_bank_like_cpp()
        {
            return;
        }
        self.execute_inventory_swap_positions_like_cpp(source, destination)
            .await;
    }

    /// Handle CMSG_AUTO_STORE_BAG_ITEM: right-click to store item in bag/backpack.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreBagItemOpcode`
    /// (`Handlers/ItemHandler.cpp:699-743`).
    pub async fn handle_auto_store_bag_item(
        &mut self,
        store: wow_packet::packets::item::AutoStoreBagItem,
    ) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        if !store.inv_update.items.is_empty() {
            warn!(
                "HandleAutoStoreBagItemOpcode - Invalid itemCount ({})",
                store.inv_update.items.len()
            );
            return;
        }

        debug!(
            "AutoStoreBagItem: src container={} slot={} dst container={} for {:?}",
            store.container_slot_a, store.slot_a, store.container_slot_b, player_guid
        );

        // Audited legacy 3.4.3 and current upstream TrinityCore likewise do
        // not call WorldSession::CanUseBank in AutoStoreBagItem. Do not add a
        // Rust-only rejection that would diverge from the cited handler.
        let Some(source) = self.get_inventory_item_by_pos(store.container_slot_a, store.slot_a)
        else {
            return;
        };

        if !self.is_valid_inventory_pos_like_cpp(store.container_slot_b, NULL_SLOT, false) {
            self.send_equip_error(InventoryResult::WrongSlot, Some(source.guid), None, 0, 0);
            return;
        }

        let runtime_item = self.inventory_item_objects_like_cpp().get(&source.guid);
        let proto = self.item_storage_template(source.entry_id);
        let source_pos = wow_entities::make_item_pos(store.container_slot_a, store.slot_a);
        if is_equipment_pos(store.container_slot_a, store.slot_a)
            || wow_entities::is_bag_pos(source_pos)
        {
            let result = self.can_unequip_inventory_item_at_like_cpp(
                store.container_slot_a,
                store.slot_a,
                !wow_entities::is_bag_pos(source_pos),
                runtime_item,
                proto.as_ref(),
                self.direct_item_contains_items(source.guid),
            );
            if result != InventoryResult::Ok {
                self.send_equip_error(result, Some(source.guid), None, 0, 0);
                return;
            }
        }

        self.execute_inventory_storage_move_like_cpp(
            store.container_slot_a,
            store.slot_a,
            store.container_slot_b,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Inventory,
            InventoryStorageQuestChecksLikeCpp::None,
            None,
        )
        .await;
    }

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

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .cloned();
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
            let planned_quest_statuses =
                self.plan_destroyed_inventory_quest_persistence_like_cpp(&[
                    DestroyQuestItemLikeCpp {
                        bag,
                        slot,
                        entry_id: item.entry_id,
                        count: removed_count,
                    },
                ]);
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

            self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
                item_object.set_count(new_count);
            });
            let changed_quest_ids = self.apply_quest_item_removed_like_cpp(item.entry_id);
            debug_assert_eq!(
                changed_quest_ids.len(),
                planned_quest_statuses.len(),
                "partial destroy quest persistence must match committed runtime removal"
            );
            self.sync_object_accessor_player();
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

    pub(super) fn plan_destroyed_inventory_quest_persistence_like_cpp(
        &self,
        destroyed_items: &[DestroyQuestItemLikeCpp],
    ) -> Vec<crate::handlers::quest::PlayerQuestStatus> {
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
                let current = self.represented_non_bank_item_count_like_cpp(entry_id);
                let removed = removed_non_bank_counts.get(&entry_id).copied().unwrap_or(0);
                (entry_id, current.saturating_sub(removed))
            })
            .collect::<Vec<_>>();
        self.plan_item_transfer_quest_persistence_like_cpp(
            &removed_entries_in_order,
            &post_removal_non_bank_counts,
            &[],
        )
    }

    /// Handle CMSG_CANCEL_TEMP_ENCHANTMENT.
    ///
    /// C++ ref: `WorldSession::HandleCancelTempEnchantmentOpcode`.
    pub async fn handle_cancel_temp_enchantment(&mut self, cancel: CancelTempEnchantment) {
        let Ok(slot) = u8::try_from(cancel.slot) else {
            return;
        };
        if !is_equipment_pos(INVENTORY_SLOT_BAG_0, slot) {
            return;
        }

        let Some(item) = self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, slot) else {
            return;
        };
        let Some(runtime_item) = self.inventory_item_objects_like_cpp().get(&item.guid) else {
            return;
        };
        if runtime_item.data().enchantments[EnchantmentSlot::EnhancementTemporary as usize].id == 0
        {
            return;
        }

        let _ = self.apply_current_player_item_enchantment_plan_like_cpp(
            item.guid,
            EnchantmentSlot::EnhancementTemporary,
            wow_entities::ApplyEnchantmentArgs::remove(),
        );
        self.update_inventory_item_object_like_cpp(item.guid, |item| {
            item.clear_enchantment(EnchantmentSlot::EnhancementTemporary);
        });
        self.sync_object_accessor_player();
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
        let descendants = self.represented_inventory_descendants_postorder_like_cpp(item.guid);
        let descendant_runtime = descendants
            .iter()
            .map(|(child_bag, child_slot, child)| {
                (
                    *child_bag,
                    *child_slot,
                    child.clone(),
                    self.inventory_item_objects_like_cpp()
                        .get(&child.guid)
                        .cloned(),
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
        let planned_quest_statuses =
            self.plan_destroyed_inventory_quest_persistence_like_cpp(&destroyed_quest_items);

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
            changed_quest_ids.extend(self.apply_quest_item_removed_like_cpp(child.entry_id));
        }

        let represented_item_mods_changed =
            self.apply_inventory_item_remove_side_effects_like_cpp(bag, slot, item.guid, &[]);

        let removed = self.apply_committed_inventory_item_removal_like_cpp(bag, slot, item.guid);
        debug_assert!(removed);
        destroyed_guids.push(item.guid);
        changed_quest_ids.extend(self.apply_quest_item_removed_like_cpp(item.entry_id));
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        debug_assert_eq!(
            changed_quest_ids.len(),
            planned_quest_statuses.len(),
            "recursive destroy quest persistence must match child/parent runtime removals"
        );
        self.sync_object_accessor_player();
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

            if slot < 19 {
                self.send_stat_update();
            }
            if represented_item_mods_changed {
                self.send_represented_item_bonus_player_stat_update_like_cpp();
            }
        } else {
            self.send_bag_slot_values_update_like_cpp(bag, slot);
        }

        true
    }

    pub(super) fn player_stat_changes_with_represented_item_bonuses_like_cpp(
        &mut self,
        include_represented_item_bonuses: bool,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return None,
        };

        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let level = self.player_level_like_cpp();

        if race == 0 || class == 0 || level == 0 {
            return None; // Not fully logged in yet
        }

        let gear = self.represented_player_gear_stats_like_cpp(include_represented_item_bonuses);
        let projection = self.player_stat_system_projection_like_cpp(race, class, level, &gear)?;

        let computed_max_health_u32 = max_health_u32_like_cpp(projection.max_health);
        let (health, max_health_for_update) = self
            .sync_canonical_player_max_health_like_cpp(computed_max_health_u32)
            .unwrap_or_else(|| {
                let current = self.player_health_like_cpp().min(computed_max_health_u32);
                self.set_player_health_like_cpp(current, computed_max_health_u32);
                (current, computed_max_health_u32)
            });
        let health = i64::from(health);
        let max_health = i64::from(max_health_for_update);

        // Existing represented unarmed damage bridge consumes the total AP
        // after C++ base and total-value modifiers have been separated.
        let ap_f = projection.total_attack_power as f32;
        let base_dmg = ap_f / 14.0 * 2.0;
        let min_d = (base_dmg + 1.0).max(1.0);
        let max_d = min_d + 1.0;

        let rap_f = projection.total_ranged_attack_power as f32;
        let (min_rd, max_rd) = if rap_f > 0.0 {
            let rd = rap_f / 14.0 * 2.8;
            ((rd + 1.0).max(1.0), rd + 3.0)
        } else {
            (0.0, 0.0)
        };

        // Power for slot 0 (mana/rage/energy/runic). Keep current power from
        // the runtime player and update only the max, like C++ `SetMaxPower`.
        let primary_power_type = primary_power_type_for_class_like_cpp(class);
        let computed_max_power0 = primary_max_power_for_class_like_cpp(class, projection.max_mana);
        let base_mana = if primary_power_type == PowerType::Mana {
            projection.base_mana
        } else {
            0
        };
        let (power0, max_power0) = self
            .sync_canonical_player_primary_power_max_like_cpp(
                primary_power_type,
                computed_max_power0,
                base_mana,
            )
            .or_else(|| {
                self.canonical_player_power_snapshot_like_cpp(primary_power_type)
                    .map(|(current, _)| {
                        (current.max(0).min(computed_max_power0), computed_max_power0)
                    })
            })
            .unwrap_or((computed_max_power0, computed_max_power0));

        // Mana regeneration is outside DATASTATS.1's audited fields; preserve
        // the existing represented bridge until the regen GameTables land.
        // spirit_regen = 0.001 + sqrt(INT) * SPI * class_coeff
        let class_regen_coeff: f32 = match class {
            2 => 0.044,  // Paladin
            3 => 0.030,  // Hunter
            5 => 0.033,  // Priest
            7 => 0.044,  // Shaman
            8 => 0.035,  // Mage
            9 => 0.033,  // Warlock
            11 => 0.044, // Druid
            _ => 0.0,    // Warrior, Rogue, DK (no mana)
        };
        let spirit_regen = if class_regen_coeff > 0.0 {
            0.001
                + (projection.stats[3] as f32).max(0.0).sqrt()
                    * projection.stats[4] as f32
                    * class_regen_coeff
        } else {
            0.0
        };

        let expertise_value =
            gear.combat_ratings[23] as f32 * self.combat_rating_multiplier_like_cpp(level, 23);

        // ── Shield block value (from STR, for shield classes) ──
        let mut shield_block_value = match class {
            1 | 2 | 7 => ((projection.stats[0] as f32 * 0.5 - 10.0).max(0.0)) as i32,
            _ => 0,
        };
        shield_block_value = shield_block_value
            .saturating_add(gear.shield_block_base_mod)
            .saturating_add(i32::try_from(gear.shield_block_value).unwrap_or(i32::MAX));
        // C++ `Player::UpdateManaRegen` stores MP5 bonuses as per-second
        // values in both normal and interrupted flat regen fields.
        let represented_mana_regen_per_second = gear.mana_regen_bonus as f32 / 5.0;

        let changes = PlayerStatChanges {
            health,
            max_health,
            min_damage: min_d,
            max_damage: max_d,
            base_mana,
            base_health: projection.create_health,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_mod_neg: 0,
            attack_power_multiplier: 0.0,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: 0.0,
            min_ranged_damage: min_rd,
            max_ranged_damage: max_rd,
            power0,
            max_power0,
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            armor: projection.armor,
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            parry_pct: projection.parry_pct,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            // Mana regen
            mana_regen: spirit_regen + represented_mana_regen_per_second,
            mana_regen_combat: represented_mana_regen_per_second,
            mana_regen_mp5: 0.0,
            // Expertise
            mainhand_expertise: expertise_value,
            offhand_expertise: expertise_value,
            // Extended parent 38 fields
            ranged_expertise: 0.0,
            combat_rating_expertise: expertise_value,
            dodge_from_attr: projection.dodge_from_attr,
            parry_from_attr: projection.parry_from_attr,
            offhand_crit_pct: projection.offhand_crit_pct,
            shield_block: shield_block_value,
            shield_block_crit_pct: 0.0,
            mod_healing_pct: 1.0,
            mod_healing_done_pct: 1.0,
            mod_periodic_healing_pct: 1.0,
            mod_spell_power_pct: 1.0,
        };

        if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
            info!(
                guid = ?player_guid,
                power_type = ?primary_power_type,
                current_power0 = power0,
                max_power0,
                base_mana,
                "RUST_STAT_POWER_UPDATE"
            );
        }

        debug!(
            "Stat update for {:?}: HP={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} SP={} Crit={:.1}% SCrit={:.1}% Dodge={:.1}% Parry={:.1}% Exp={:.1} ManaRegen={:.1}",
            player_guid,
            max_health,
            projection.attack_power,
            projection.stats,
            projection.armor,
            gear.spell_power,
            projection.crit_pct,
            projection.spell_crit_pct[0],
            projection.dodge_pct,
            projection.parry_pct,
            expertise_value,
            spirit_regen
        );

        Some((player_guid, changes))
    }

    pub(super) fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self) {
        let Some((player_guid, changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return;
        };

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }
}
