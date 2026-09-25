// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-source item grants used during quest acceptance.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(super) async fn store_quest_source_item_like_cpp(
        &mut self,
        entry_id: u32,
        quantity: u32,
        dest: &[ItemPosCount],
    ) -> Option<QuestSourceItemStoreOutcomeLikeCpp> {
        let generator = self.item_guid_generator_like_cpp_for_bridge()?;
        self.store_quest_source_item_with_generator_like_cpp(
            generator.as_ref(),
            entry_id,
            quantity,
            dest,
        )
        .await
    }

    pub(super) async fn store_quest_source_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        entry_id: u32,
        quantity: u32,
        dest: &[ItemPosCount],
    ) -> Option<QuestSourceItemStoreOutcomeLikeCpp> {
        let Some(player_guid) = self.player_guid() else {
            return None;
        };
        if dest.is_empty() {
            return None;
        }
        let quest_log_item_id = self
            .quest_source_item_quest_log_item_id_like_cpp(entry_id)
            .await;
        let completion_evidence_start = self
            .represented_quest_complete_status_updates_like_cpp()
            .len();
        if let Some(bound_preflight) = self
            .apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
                item_guid_generator,
                entry_id,
                quest_log_item_id,
                quantity,
            )
            .await
        {
            for quest_id in bound_preflight.changed_quest_ids {
                self.save_represented_quest_status_like_cpp(quest_id).await;
            }
            if bound_preflight.no_grant {
                self.save_represented_quest_statuses_completed_after_like_cpp(
                    completion_evidence_start,
                )
                .await;
                return Some(QuestSourceItemStoreOutcomeLikeCpp::BoundObjectiveNoGrant);
            }
        }

        #[derive(Clone, Copy)]
        struct ExistingStackUpdate {
            item_guid: ObjectGuid,
            new_count: u32,
            should_bind: bool,
            pos: u16,
        }

        #[derive(Clone, Copy)]
        struct NewStack {
            bag: u8,
            slot: u8,
            db_guid: u64,
            item_guid: ObjectGuid,
            stack_count: u32,
            max_durability: u32,
            item_flags: u32,
            contained_in: ObjectGuid,
        }

        let mut existing_updates: Vec<ExistingStackUpdate> = Vec::new();
        let mut new_stacks: Vec<NewStack> = Vec::new();
        let mut persistence_existing_stacks = Vec::new();
        let mut persistence_new_stacks = Vec::new();
        let source_item_bonding = self
            .item_storage_template(entry_id)
            .map(|template| template.bonding);
        let mut last_item_guid = ObjectGuid::EMPTY;
        let mut last_bag = u8::from(wow_entities::INVENTORY_SLOT_BAG_0);
        let mut last_slot = 0;
        let mut last_count_in_stack = 0;
        let new_item_count = dest
            .iter()
            .filter(|dest| {
                let bag = (dest.pos >> 8) as u8;
                let slot = (dest.pos & 0x00FF) as u8;
                self.get_inventory_item_by_pos(bag, slot).is_none()
            })
            .count();
        let Some(allocated_new_item_guids) = self
            .allocate_item_instance_guids_with_generator_like_cpp(
                item_guid_generator,
                new_item_count,
            )
        else {
            warn!(
                account = self.account_id,
                entry_id,
                count = new_item_count,
                "QuestConfirmAccept: process-wide item GUID allocator is unavailable"
            );
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return None;
        };
        let mut allocated_new_item_guids = allocated_new_item_guids.into_iter();

        for dest in dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;

            if let Some(inv_item) = self.get_inventory_item_by_pos(bag, slot) {
                let Some(existing_item) =
                    self.resolved_inventory_item_object_like_cpp(inv_item.guid)
                else {
                    warn!(
                        account = self.account_id,
                        slot,
                        entry_id,
                        "QuestConfirmAccept: missing runtime item object for source item stack"
                    );
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return None;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let existing_flags = existing_item.item_flags_bits();
                let should_bind = source_item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                persistence_existing_stacks.push(
                    wow_persistence::QuestItemExistingStackPersistenceLikeCpp {
                        item_guid: inv_item.db_guid,
                        new_count,
                        dynamic_flags: (should_bind && !existing_item.is_soul_bound())
                            .then_some(existing_flags | ItemFieldFlags::SOULBOUND.bits()),
                    },
                );
                existing_updates.push(ExistingStackUpdate {
                    item_guid: inv_item.guid,
                    new_count,
                    should_bind,
                    pos: dest.pos,
                });
                last_item_guid = inv_item.guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = new_count;
            } else {
                let (inventory_bag_db_guid, contained_in) = if bag
                    == u8::from(wow_entities::INVENTORY_SLOT_BAG_0)
                {
                    (0, player_guid)
                } else if let Some(bag_inventory_item) = self.resolved_inventory_item_like_cpp(bag)
                {
                    (bag_inventory_item.db_guid, bag_inventory_item.guid)
                } else {
                    warn!(
                        account = self.account_id,
                        bag,
                        slot,
                        entry_id,
                        "QuestConfirmAccept: represented source item destination references missing bag"
                    );
                    self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                    return None;
                };

                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!(
                        account = self.account_id,
                        entry_id,
                        "QuestConfirmAccept: preallocated item GUID count did not match store plan"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return None;
                };
                let max_durability = self.item_template_max_durability(entry_id);
                let should_bind = source_item_bonding.is_some_and(|bonding| {
                    matches!(bonding, ItemBondingType::OnAcquire | ItemBondingType::Quest)
                        || (bonding == ItemBondingType::OnEquip && is_bag_pos(dest.pos))
                });
                let item_flags = if should_bind {
                    ItemFieldFlags::SOULBOUND.bits()
                } else {
                    0
                };

                persistence_new_stacks.push(wow_persistence::QuestItemNewStackPersistenceLikeCpp {
                    item_guid: db_guid,
                    entry_id,
                    owner_guid: player_guid.counter() as u64,
                    count: dest.count,
                    max_durability,
                    dynamic_flags: item_flags,
                    bag_guid: inventory_bag_db_guid,
                    slot,
                });

                new_stacks.push(NewStack {
                    bag,
                    slot,
                    db_guid,
                    item_guid,
                    stack_count: dest.count,
                    max_durability,
                    item_flags,
                    contained_in,
                });
                last_item_guid = item_guid;
                last_bag = bag;
                last_slot = slot;
                last_count_in_stack = dest.count;
            }
        }

        if let Some(port) = self.player_inventory_persistence_port_like_cpp() {
            let outcome = port
                .persist_inventory_mutation_like_cpp(
                    wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestItemGrant(
                        wow_persistence::QuestItemGrantPersistenceLikeCpp {
                            existing_stacks: persistence_existing_stacks,
                            new_stacks: persistence_new_stacks,
                        },
                    ),
                )
                .await;
            match outcome {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                    warn!(account = self.account_id, entry_id, error = %reason,
                        "QuestConfirmAccept: source item StoreNewItem transaction failed");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return None;
                }
                wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(account = self.account_id, entry_id, error = %reason,
                        "QuestConfirmAccept: source item StoreNewItem commit outcome is unknown");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return None;
                }
            }
        }

        for update in &existing_updates {
            let mut item_updates = vec![wow_entities::ItemObjectUpdateLikeCpp::SetCount(
                update.new_count,
            )];
            if let Some(bonding) = source_item_bonding {
                item_updates.push(wow_entities::ItemObjectUpdateLikeCpp::SetBonding(bonding));
                if update.should_bind {
                    item_updates.push(wow_entities::ItemObjectUpdateLikeCpp::BindIfStored(
                        is_bag_pos(update.pos),
                    ));
                }
            }
            let _ =
                self.apply_inventory_item_object_updates_like_cpp(update.item_guid, &item_updates);
        }

        let inventory_type = self.item_template_inventory_type(entry_id);
        for stack in &new_stacks {
            if stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                self.insert_inventory_item_like_cpp(
                    stack.slot,
                    InventoryItem {
                        guid: stack.item_guid,
                        entry_id,
                        db_guid: stack.db_guid,
                        inventory_type,
                    },
                );
            }
            let mut item_object = self.make_inventory_item_object(
                stack.item_guid,
                entry_id,
                player_guid,
                stack.stack_count,
                stack.max_durability,
                ItemContext::None,
                stack.slot,
            );
            if stack.bag != u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                item_object.set_container_guid_and_slot(stack.contained_in, stack.bag);
            }
            if let Some(bonding) = source_item_bonding {
                item_object.set_bonding(bonding);
                item_object.bind_if_stored(is_bag_pos(wow_entities::make_item_pos(
                    stack.bag, stack.slot,
                )));
            }
            self.insert_inventory_item_object(item_object);
        }

        let map_id = self.player_map_id_like_cpp();
        if !new_stacks.is_empty() {
            let item_creates = new_stacks
                .iter()
                .map(|stack| ItemCreateData {
                    item_guid: stack.item_guid,
                    entry_id: entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: stack.contained_in,
                    stack_count: stack.stack_count,
                    dynamic_flags: stack.item_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: 0,
                    random_properties_id: 0,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: ItemContext::None as u8,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for update in &existing_updates {
            self.send_packet(&UpdateObject::item_stack_count_update(
                update.item_guid,
                map_id,
                update.new_count,
            ));
        }

        if !new_stacks.is_empty() {
            let changed_slots: Vec<_> = new_stacks
                .iter()
                .filter(|stack| stack.bag == u8::from(wow_entities::INVENTORY_SLOT_BAG_0))
                .map(|stack| (stack.slot, stack.item_guid))
                .collect();
            if !changed_slots.is_empty() {
                self.send_player_values_update_from_entity_bridge(
                    &changed_slots,
                    &[],
                    &[],
                    &[],
                    None,
                );
            }
        }

        let quantity_in_inventory = self
            .represented_inventory_item_counts_like_cpp()?
            .get(&entry_id)
            .copied()
            .unwrap_or(0);
        let changed_non_bound_quest_ids = self
            .apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
                item_guid_generator,
                entry_id,
                quest_log_item_id,
                quantity,
            )
            .await;
        for quest_id in changed_non_bound_quest_ids {
            self.save_represented_quest_status_like_cpp(quest_id).await;
        }
        self.save_represented_quest_statuses_completed_after_like_cpp(completion_evidence_start)
            .await;

        self.send_new_item_plan(&SendNewItemPlan {
            player_guid,
            item_guid: last_item_guid,
            item_entry: entry_id,
            item_instance: SendNewItemInstancePlan {
                item_id: entry_id,
                random_properties_seed: 0,
                random_properties_id: 0,
                modifications: Vec::<SendNewItemModifier>::new(),
            },
            slot: last_bag,
            slot_in_bag: if last_count_in_stack == quantity {
                i16::from(last_slot)
            } else {
                -1
            },
            quest_log_item_id,
            quantity,
            quantity_in_inventory,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            pushed: true,
            created: false,
            display_text: SendNewItemDisplayText::Normal,
            dungeon_encounter_id: 0,
            is_encounter_loot: false,
            delivery: SendNewItemDelivery::Direct,
        });
        Some(QuestSourceItemStoreOutcomeLikeCpp::StoredNewItem)
    }
}
