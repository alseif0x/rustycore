// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest reward selection, item/currency granting and required-item removal.

use super::*;

impl WorldSession {
    pub(super) async fn store_quest_source_item_like_cpp(
        &mut self,
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
            .apply_quest_source_item_bound_objective_preflight_like_cpp(
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
        let Some(allocated_new_item_guids) =
            self.allocate_item_instance_guids_like_cpp(new_item_count)
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
            self.update_inventory_item_object_like_cpp(update.item_guid, |item| {
                item.set_count(update.new_count);
                if let Some(bonding) = source_item_bonding {
                    item.set_bonding(bonding);
                    if update.should_bind {
                        item.bind_if_stored(is_bag_pos(update.pos));
                    }
                }
            });
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
            .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
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

    async fn store_quest_reward_item_like_cpp(
        &mut self,
        entry_id: u32,
        quantity: u32,
        dest: &[ItemPosCount],
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if dest.is_empty() {
            return false;
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

        let item_bonding = self
            .item_storage_template(entry_id)
            .map(|template| template.bonding);
        let mut existing_updates = Vec::new();
        let mut new_stacks = Vec::new();
        let mut persistence_existing_stacks = Vec::new();
        let mut persistence_new_stacks = Vec::new();
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
        let Some(allocated_new_item_guids) =
            self.allocate_item_instance_guids_like_cpp(new_item_count)
        else {
            warn!(
                account = self.account_id,
                entry_id,
                count = new_item_count,
                "RewardQuest: process-wide item GUID allocator is unavailable"
            );
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
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
                        "RewardQuest: missing runtime item object for reward item stack"
                    );
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                let new_count = existing_item.count().saturating_add(dest.count);
                let existing_flags = existing_item.item_flags_bits();
                let should_bind = item_bonding.is_some_and(|bonding| {
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
                        "RewardQuest: represented reward item destination references missing bag"
                    );
                    self.send_equip_error(InventoryResult::WrongBagType, None, None, 0, 0);
                    return false;
                };

                let Some((db_guid, item_guid)) = allocated_new_item_guids.next() else {
                    warn!(
                        account = self.account_id,
                        entry_id,
                        "RewardQuest: preallocated item GUID count did not match store plan"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                };
                let max_durability = self.item_template_max_durability(entry_id);
                let should_bind = item_bonding.is_some_and(|bonding| {
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
                        "RewardQuest: reward item StoreNewItem transaction failed");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    warn!(account = self.account_id, entry_id, error = %reason,
                        "RewardQuest: reward item StoreNewItem commit outcome is unknown");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            }
        }

        for update in &existing_updates {
            self.update_inventory_item_object_like_cpp(update.item_guid, |item| {
                item.set_count(update.new_count);
                if let Some(bonding) = item_bonding {
                    item.set_bonding(bonding);
                    if update.should_bind {
                        item.bind_if_stored(is_bag_pos(update.pos));
                    }
                }
            });
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
                ItemContext::QuestReward,
                stack.slot,
            );
            if stack.bag != u8::from(wow_entities::INVENTORY_SLOT_BAG_0) {
                item_object.set_container_guid_and_slot(stack.contained_in, stack.bag);
            }
            if let Some(bonding) = item_bonding {
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
                    context: ItemContext::QuestReward as u8,
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

        let Some(inventory_item_counts) = self.represented_inventory_item_counts_like_cpp() else {
            return false;
        };
        let quantity_in_inventory = inventory_item_counts.get(&entry_id).copied().unwrap_or(0);
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
            quest_log_item_id: 0,
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
        true
    }

    async fn store_fixed_quest_reward_items_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }

            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item(*item_id, *count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(*item_id, *count, &dest)
                .await
            {
                return false;
            }
        }

        true
    }

    async fn store_chosen_quest_reward_item_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP || choice.item_id == 0
        {
            return true;
        }

        if self
            .item_store()
            .is_none_or(|store| store.get(choice.item_id).is_none())
        {
            return true;
        }

        for ((item_id, count), item_type) in quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
        {
            if *item_id == 0
                || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                || *item_id != choice.item_id
            {
                continue;
            }

            let (result, dest, _) = self
                .plan_store_new_direct_inventory_item(*item_id, *count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
            if !self
                .store_quest_reward_item_like_cpp(*item_id, *count, &dest)
                .await
            {
                return false;
            }
        }

        true
    }

    async fn store_quest_package_reward_entry_like_cpp(
        &mut self,
        entry: &QuestPackageItemEntry,
    ) -> bool {
        let Ok(item_id) = u32::try_from(entry.item_id) else {
            self.send_quest_package_reward_inventory_error_like_cpp(
                InventoryResult::ItemNotFound,
                0,
            );
            return false;
        };

        let (result, dest, _) = self
            .plan_store_new_direct_inventory_item(item_id, entry.item_quantity)
            .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));
        if result != InventoryResult::Ok {
            self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
            return false;
        }

        self.store_quest_reward_item_like_cpp(item_id, entry.item_quantity, &dest)
            .await
    }

    async fn store_quest_package_reward_items_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if quest.quest_package_id == 0
            || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
            || choice.item_id == 0
        {
            return true;
        }

        // C++ gates `RewardQuestPackage` behind a non-null selected reward item template.
        if self
            .item_store()
            .is_none_or(|store| store.get(choice.item_id).is_none())
        {
            return true;
        }

        let Some(store) = &self.quest_package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return true;
        };

        let primary_entries = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();
        let fallback_entries = store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut has_filtered_quest_package_reward = false;
        for entry in primary_entries {
            if !self.represented_can_select_quest_package_item_like_cpp(&entry) {
                continue;
            }

            has_filtered_quest_package_reward = true;
            if !self.store_quest_package_reward_entry_like_cpp(&entry).await {
                return false;
            }
        }

        if !has_filtered_quest_package_reward {
            for entry in fallback_entries {
                if !self.store_quest_package_reward_entry_like_cpp(&entry).await {
                    return false;
                }
            }
        }

        true
    }

    fn quest_reward_currency_gain_source_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
    ) -> CurrencyGainSourceLikeCpp {
        if (quest.flags_ex & QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP) != 0 {
            if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
                return CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps;
            }

            return CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps;
        }

        if quest.is_daily_like_cpp() {
            CurrencyGainSourceLikeCpp::DailyQuestReward
        } else if quest.is_weekly_like_cpp() {
            CurrencyGainSourceLikeCpp::WeeklyQuestReward
        } else if (quest.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
            CurrencyGainSourceLikeCpp::WorldQuestReward
        } else {
            CurrencyGainSourceLikeCpp::QuestReward
        }
    }

    async fn grant_quest_reward_currency_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> bool {
        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return false;
        };
        let delta = match self.add_currency_quest_reward_like_cpp(currency_id, amount, gain_source)
        {
            Ok(delta) => delta,
            Err(()) => {
                self.set_player_currencies_like_cpp(currency_snapshot);
                return false;
            }
        };

        if let Some(player_guid) = self.player_guid() {
            if let Err(outcome) = self
                .persist_standalone_player_currency_save_like_cpp(
                    player_guid.counter() as u64,
                    currency_snapshot,
                )
                .await
            {
                warn!(
                    account = self.account_id,
                    currency_id,
                    ?outcome,
                    "ChooseReward: quest reward currency save failed"
                );
                return false;
            }
        }

        if let Some(delta) = delta {
            let (Some(quantity), Some(amount)) = (
                i32::try_from(delta.quantity).ok(),
                i32::try_from(delta.amount).ok(),
            ) else {
                return true;
            };
            let mut packet = SetCurrency {
                type_id: delta.currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                tracked_quantity: None,
                max_quantity: delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                total_earned: delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                suppress_chat_log: delta.suppress_chat_log,
                quantity_change: Some(amount),
                quantity_gain_source: Some(gain_source as i32),
                quantity_lost_source: None,
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            };
            packet.suppress_chat_log = delta.suppress_chat_log;
            self.send_packet(&packet);
        }

        true
    }

    async fn grant_quest_reward_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let gain_source = Self::quest_reward_currency_gain_source_like_cpp(quest);

        if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            && choice.item_id != 0
            && self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id))
        {
            for ((currency_id, count), item_type) in quest
                .reward_choice_items
                .iter()
                .zip(quest.reward_choice_item_types.iter())
            {
                if *currency_id == 0
                    || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
                    || *currency_id != choice.item_id
                {
                    continue;
                }

                if !self
                    .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                    .await
                {
                    return false;
                }
            }
        }

        for (currency_id, count) in quest
            .reward_currencies
            .iter()
            .zip(quest.reward_currency_amounts.iter())
        {
            if *currency_id == 0 || *count == 0 {
                continue;
            }

            if !self
                .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                .await
            {
                return false;
            }
        }

        true
    }

    fn represented_direct_inventory_count_like_cpp(&self, item_entry: u32) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp()?
                .values()
                .filter(|item| item.entry_id == item_entry)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                        .filter(|item| !item.is_in_trade())
                        .map(|item| item.count())
                })
                .fold(0u32, u32::saturating_add),
        )
    }

    fn plan_quest_destroy_item_count_direct_like_cpp(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let effective_count = if count == u32::MAX {
            self.represented_direct_inventory_count_like_cpp(item_entry)?
        } else {
            count
        };

        if effective_count == 0 {
            return Some(Vec::new());
        }

        self.plan_destroy_item_count_direct_inventory(item_entry, effective_count)
    }

    async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let map_id = self.player_map_id_like_cpp();
        let mut item_changes = Vec::new();
        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return false;
        };
        let mut currency_losses = Vec::new();

        for objective in &quest.objectives {
            match objective.obj_type {
                QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL => {
                    let Ok(item_entry) = u32::try_from(objective.object_id) else {
                        return false;
                    };
                    let count = if (quest.flags & QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP) != 0 {
                        u32::MAX
                    } else {
                        u32::try_from(objective.amount).unwrap_or(u32::MAX)
                    };
                    let Some(mut changes) =
                        self.plan_quest_destroy_item_count_direct_like_cpp(item_entry, count)
                    else {
                        return false;
                    };
                    item_changes.append(&mut changes);
                }
                QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL => {
                    let (Ok(currency_id), Ok(amount)) = (
                        u32::try_from(objective.object_id),
                        u32::try_from(objective.amount),
                    ) else {
                        return false;
                    };
                    let Some(before) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    if !self.remove_currency(currency_id, amount) {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    }
                    let Some(after) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    let removed = before.saturating_sub(after);
                    if removed > 0 {
                        currency_losses.push((currency_id, after, removed));
                    }
                }
                _ => {}
            }
        }

        if (quest.flags_ex & QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP) == 0 {
            for (item_entry, count) in quest.item_drop.iter().zip(quest.item_drop_quantity.iter()) {
                if *item_entry == 0 {
                    continue;
                }
                let count = if *count == 0 { u32::MAX } else { *count };
                let Some(mut changes) =
                    self.plan_quest_destroy_item_count_direct_like_cpp(*item_entry, count)
                else {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    return false;
                };
                item_changes.append(&mut changes);
            }
        }

        if let Some(port) = self.player_inventory_persistence_port_like_cpp() {
            let Some(mut currencies) = self.player_currencies_like_cpp() else {
                self.set_player_currencies_like_cpp(currency_snapshot);
                return false;
            };
            let currency_save = self
                .plan_player_currency_save_like_cpp(player_guid.counter() as u64, &mut currencies);
            if !self.set_player_currencies_like_cpp(currencies) {
                return false;
            }
            let items = item_changes
                .iter()
                .map(|change| match *change {
                    ExtendedCostItemTurninChange::Update {
                        db_guid, new_count, ..
                    } => wow_persistence::QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid: db_guid,
                        new_count,
                    },
                    ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                        wow_persistence::QuestTurnInItemPersistenceLikeCpp::Delete {
                            item_guid: db_guid,
                        }
                    }
                })
                .collect();
            let outcome = port
                .persist_inventory_mutation_like_cpp(
                    wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(
                        wow_persistence::QuestTurnInPersistenceLikeCpp {
                            owner_guid: player_guid.counter() as u64,
                            items,
                            currency_save,
                        },
                    ),
                )
                .await;
            match outcome {
                wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    warn!(
                        account = self.account_id,
                        quest_id = quest.id,
                        error = %reason,
                        "ChooseReward: quest objective item/currency removal save failed"
                    );
                    return false;
                }
                wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    warn!(
                    account = self.account_id,
                    quest_id = quest.id,
                        error = %reason,
                        "ChooseReward: quest objective item/currency removal commit outcome is unknown"
                    );
                    return false;
                }
            }
        }

        self.apply_item_turnin_changes(player_guid, map_id, &item_changes);
        for (currency_id, quantity, removed) in currency_losses {
            let (Some(quantity), Some(removed)) =
                (i32::try_from(quantity).ok(), i32::try_from(removed).ok())
            else {
                continue;
            };
            self.send_packet(&SetCurrency {
                type_id: currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-removed),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            });
        }

        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    fn apply_represented_quest_reward_skill_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        if quest.reward_skill_line_id != 0 {
            self.represented_quest_reward_skill_updates_like_cpp
                .push((quest.reward_skill_line_id, quest.reward_skill_points));
        }
    }

    fn record_represented_quest_reward_spell_casts_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        {
            let caster_selection_unrepresented =
                (quest.flags & QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP) == 0;
            if quest.reward_spell > 0 {
                self.represented_quest_reward_spell_casts_like_cpp.push(
                    RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id: quest.reward_spell,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    },
                );
                return;
            }

            let display_spells = quest.reward_display_spell;
            for (index, spell_id) in display_spells.into_iter().enumerate() {
                if spell_id == 0 {
                    continue;
                }
                self.represented_quest_reward_spell_casts_like_cpp.push(
                    RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell {
                            index: index as u8,
                        },
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    },
                );
            }
        }
        #[cfg(not(test))]
        let _ = quest;
    }

    fn apply_represented_quest_title_and_talent_rewards_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        if quest.reward_title_id != 0 {
            self.represented_quest_reward_titles_like_cpp.push(
                RepresentedQuestRewardTitleLikeCpp {
                    quest_id: quest.id,
                    title_id: quest.reward_title_id,
                    char_title_lookup_unrepresented: true,
                    set_title_runtime_unrepresented: true,
                },
            );
        }
        if quest.reward_skill_points != 0 {
            self.represented_quest_reward_talent_points_like_cpp.push(
                RepresentedQuestRewardTalentPointsLikeCpp {
                    quest_id: quest.id,
                    points: quest.reward_skill_points,
                    init_talent_for_level_unrepresented: true,
                },
            );
        }
    }

    fn record_represented_quest_reward_mail_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        {
            if quest.reward_mail_template_id == 0 {
                return;
            }

            self.represented_quest_reward_mails_like_cpp
                .push(RepresentedQuestRewardMailLikeCpp {
                    quest_id: quest.id,
                    mail_template_id: quest.reward_mail_template_id,
                    delay_secs: quest.reward_mail_delay_secs,
                    sender_entry: (quest.reward_mail_sender_entry != 0)
                        .then_some(quest.reward_mail_sender_entry),
                    quest_giver_guid: (quest.reward_mail_sender_entry == 0)
                        .then_some(quest_giver_guid),
                    mail_template_lookup_unrepresented: true,
                    mail_draft_runtime_unrepresented: true,
                    character_db_transaction_unrepresented: true,
                });
        }
        #[cfg(not(test))]
        let _ = (quest, quest_giver_guid);
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let source = if quest.is_daily_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest
        } else if quest.is_weekly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest
        } else if quest.is_monthly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest
        } else if quest.is_repeatable() {
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest
        } else {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest
        };
        let gain_source = match source {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest => {
                ReputationGainSourceLikeCpp::Quest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest => {
                ReputationGainSourceLikeCpp::DailyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest => {
                ReputationGainSourceLikeCpp::WeeklyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest => {
                ReputationGainSourceLikeCpp::MonthlyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest => {
                ReputationGainSourceLikeCpp::RepeatableQuest
            }
        };
        let faction_store = self.faction_store().map(Arc::clone);
        let quest_faction_reward_store = self.quest_faction_reward_store.as_ref().map(Arc::clone);
        let reputation_reward_rate_store = self.reputation_reward_rate_store().map(Arc::clone);
        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);

        for slot in 0..wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT {
            let faction_id = quest.reward_faction_ids[slot];
            if faction_id == 0 {
                continue;
            }
            let faction_entry = match faction_store.as_deref() {
                Some(store) => match store.get(faction_id).cloned() {
                    Some(entry) => Some(entry),
                    None => continue,
                },
                None => None,
            };
            let faction_lookup_missing = faction_entry.is_none();

            let reward_faction_override = quest.reward_faction_overrides[slot];
            let (base_reputation_before_gain, no_quest_bonus, quest_faction_reward_lookup) =
                if reward_faction_override != 0 {
                    (reward_faction_override / 100, true, false)
                } else if let Some(store) = quest_faction_reward_store.as_deref() {
                    let row = if quest.reward_faction_values[slot] < 0 {
                        2
                    } else {
                        1
                    };
                    let field = quest.reward_faction_values[slot].unsigned_abs() as usize;
                    let rep = store
                        .get(row)
                        .and_then(|entry| entry.difficulty.get(field).copied())
                        .map(i32::from)
                        .unwrap_or(0);
                    (rep, false, false)
                } else {
                    (0, false, true)
                };

            if base_reputation_before_gain == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let quest_level_for_gain =
                player_quest_level_like_cpp(quest, self.player_level_like_cpp()).max(0) as u32;
            let reputation_rates = self.reputation_rates_like_cpp();
            let Some(percent_before_reward_rate) = self
                .reputation_gain_percent_before_reward_rate_like_cpp(
                    gain_source,
                    quest_level_for_gain,
                    base_reputation_before_gain,
                    faction_id,
                    no_quest_bonus,
                )
            else {
                continue;
            };
            let reputation_after_low_level_rate_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                percent_before_reward_rate,
            );
            if reputation_after_low_level_rate_like_cpp == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let (
                reputation_after_reward_rate_like_cpp,
                percent_after_reward_rate_like_cpp,
                reputation_reward_rate_lookup,
            ) = if reputation_reward_rate_store.is_some() {
                if let Some(rate) =
                    self.reputation_reward_rate_for_source_like_cpp(gain_source, faction_id)
                {
                    if rate <= 0.0 {
                        continue;
                    }
                    let percent = percent_before_reward_rate * rate;
                    (
                        calculate_pct_i32_f32_like_cpp(base_reputation_before_gain, percent),
                        percent,
                        false,
                    )
                } else {
                    (
                        reputation_after_low_level_rate_like_cpp,
                        percent_before_reward_rate,
                        false,
                    )
                }
            } else {
                (
                    reputation_after_low_level_rate_like_cpp,
                    percent_before_reward_rate,
                    true,
                )
            };
            let reputation_after_recruit_a_friend_bonus_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                self.apply_recruit_a_friend_reputation_bonus_like_cpp(
                    gain_source,
                    percent_after_reward_rate_like_cpp,
                ),
            );
            if reputation_after_recruit_a_friend_bonus_like_cpp == 0 && !quest_faction_reward_lookup
            {
                continue;
            }

            let current_rank_for_cap = if quest.reward_faction_cap_in[slot] != 0
                && reputation_after_recruit_a_friend_bonus_like_cpp > 0
            {
                self.canonical_player_reputation_standing_like_cpp(faction_id)
                    .map(reputation_rank_from_standing_like_cpp)
            } else {
                None
            };
            if current_rank_for_cap.is_some_and(|current_rank| {
                i32::from(current_rank) >= quest.reward_faction_cap_in[slot]
            }) {
                continue;
            }

            let no_spillover = (quest.reward_faction_flags & (1u32 << slot)) != 0;
            let modify_reputation_runtime_unrepresented =
                if let (Some(faction_entry), Some(faction_store)) =
                    (faction_entry.as_ref(), faction_store.as_deref())
                {
                    let options = crate::reputation::mgr::SetReputationOptionsLikeCpp {
                        incremental: true,
                        spillover_only: false,
                        no_spillover,
                        reputation_gain_rate: reputation_rates.gain,
                        paragon_reward_quest_status_none_like_cpp: true,
                        renown_current_level_like_cpp: 0,
                        renown_currency_increased_cap_quantity_like_cpp: 0,
                        player_race: self.player_race_like_cpp(),
                        player_class: self.player_class_like_cpp(),
                    };
                    let db_spillover_template = reputation_spillover_template_store
                        .as_deref()
                        .and_then(|store| store.get(faction_id));
                    let mutation = self.mutate_reputation_mgr_like_cpp(|mgr| {
                        let outcome = mgr.set_reputation_like_cpp(
                            faction_entry,
                            reputation_after_recruit_a_friend_bonus_like_cpp,
                            options,
                            faction_store,
                            db_spillover_template,
                            friendship_rep_reaction_store.as_deref(),
                            paragon_reputation_store.as_deref(),
                            currency_types_store.as_deref(),
                        );
                        let packet = outcome.send_state_rep_list_id.map(|rep_list_id| {
                            mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id))
                        });
                        (outcome, packet)
                    });
                    let owner_unavailable = mutation.is_none();
                    if let Some((_outcome, Some(packet))) = mutation {
                        self.send_packet(&packet);
                    }
                    owner_unavailable
                } else {
                    true
                };

            #[cfg(test)]
            {
                self.represented_quest_reward_reputations_like_cpp.push(
                    RepresentedQuestRewardReputationLikeCpp {
                        quest_id: quest.id,
                        slot: slot as u8,
                        faction_id,
                        reward_faction_value: quest.reward_faction_values[slot],
                        reward_faction_override,
                        reward_faction_cap_in: quest.reward_faction_cap_in[slot],
                        base_reputation_before_gain,
                        reputation_after_low_level_rate_like_cpp,
                        reputation_after_reward_rate_like_cpp,
                        no_quest_bonus,
                        no_spillover,
                        source,
                        faction_store_lookup_unrepresented: faction_lookup_missing,
                        quest_faction_reward_store_lookup_unrepresented:
                            quest_faction_reward_lookup,
                        reputation_reward_rate_lookup_unrepresented: reputation_reward_rate_lookup,
                        gray_level_script_hook_unrepresented: true,
                        reputation_rank_cap_check_unrepresented: quest.reward_faction_cap_in[slot]
                            != 0
                            && reputation_after_recruit_a_friend_bonus_like_cpp > 0
                            && current_rank_for_cap.is_none(),
                        calculate_reputation_gain_unrepresented: true,
                        modify_reputation_runtime_unrepresented,
                    },
                );
            }
        }
    }

    async fn apply_quest_reward_lockout_status_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let now = GameTime::now().as_secs() as i64;
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let mut save_daily = false;
        let mut save_weekly = false;
        let mut save_monthly = false;
        let mut save_seasonal = false;

        if quest.is_daily_like_cpp() || quest.is_df_quest_like_cpp() {
            save_daily = true;
        } else if quest.is_weekly_like_cpp() {
            save_weekly = true;
        } else if quest.is_monthly_like_cpp() {
            save_monthly = true;
        } else if quest.is_seasonal_like_cpp() {
            save_seasonal = true;
        }

        if self
            .mutate_player_quest_gameplay_like_cpp(|state| {
                if save_daily {
                    state.last_daily_quest_time_secs = now;
                    if quest.is_df_quest_like_cpp() {
                        state.df_quest_ids.insert(quest.id);
                    } else {
                        state.daily_quest_ids.insert(quest.id);
                    }
                } else if save_weekly {
                    state.weekly_quest_ids.insert(quest.id);
                } else if save_monthly {
                    state.monthly_quest_ids.insert(quest.id);
                } else if save_seasonal {
                    state
                        .seasonal_quests
                        .entry(quest.event_id_for_quest_like_cpp())
                        .or_default()
                        .insert(quest.id, now.max(0) as u64);
                    state.seasonal_quest_changed = true;
                }
            })
            .is_none()
        {
            return;
        }

        let Some(port) = self.player_quest_persistence_port_like_cpp() else {
            return;
        };

        let owner_guid = player_guid.counter() as u64;
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let request = if save_daily {
            let mut quest_ids = recurrence
                .daily_quest_ids
                .iter()
                .copied()
                .collect::<Vec<_>>();
            quest_ids.extend(recurrence.df_quest_ids.iter().copied());
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Daily {
                owner_guid,
                completed_time: recurrence.last_daily_quest_time_secs,
                quest_ids,
            }
        } else if save_weekly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Weekly {
                owner_guid,
                quest_ids: recurrence.weekly_quest_ids.iter().copied().collect(),
            }
        } else if save_monthly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Monthly {
                owner_guid,
                quest_ids: recurrence.monthly_quest_ids.iter().copied().collect(),
            }
        } else if save_seasonal {
            let completions = recurrence
                .seasonal_quests
                .iter()
                .flat_map(|(event_id, quests)| {
                    quests.iter().filter_map(|(quest_id, completed_time)| {
                        Some(
                            wow_persistence::PlayerQuestSeasonalCompletionPersistenceLikeCpp {
                                quest_id: *quest_id,
                                event_id: *event_id,
                                completed_time: i64::try_from(*completed_time).ok()?,
                            },
                        )
                    })
                })
                .collect();
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Seasonal {
                owner_guid,
                completions,
            }
        } else {
            return;
        };

        match port.persist_lockout_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => warn!(
                account = self.account_id,
                quest_id = quest.id,
                error = %reason,
                "ChooseReward: represented reward lockout status save failed"
            ),
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                account = self.account_id,
                quest_id = quest.id,
                error = %reason,
                "ChooseReward: represented reward lockout commit outcome is unknown"
            ),
        }
    }

    pub(super) fn read_quest_choice_item_like_cpp(
        pkt: &mut wow_packet::WorldPacket,
    ) -> Result<QuestChoiceItemLikeCpp, wow_packet::PacketError> {
        // C++ `QuestChoiceItem` starts with `ResetBitPos(); ReadBits(2)`, then
        // an `Item::ItemInstance`, then signed `Quantity`.
        pkt.reset_bits();
        let loot_item_type = pkt.read_bits(2)? as u8;

        let item_id = pkt.read_int32()? as u32;
        let _random_properties_seed = pkt.read_int32()?;
        let _random_properties_id = pkt.read_int32()?;

        let has_item_bonus = pkt.read_bit()?;
        pkt.reset_bits();

        let item_mod_count = pkt.read_bits(6)?;
        pkt.reset_bits();
        for _ in 0..item_mod_count {
            let _value = pkt.read_int32()?;
            let _modifier_type = pkt.read_uint8()?;
        }

        if has_item_bonus {
            let _context = pkt.read_uint8()?;
            let bonus_count = pkt.read_uint32()?;
            for _ in 0..bonus_count {
                let _bonus_id = pkt.read_uint32()?;
            }
        }

        let quantity = pkt.read_int32()?;

        Ok(QuestChoiceItemLikeCpp {
            loot_item_type,
            item_id,
            quantity,
        })
    }

    pub(super) fn represented_reward_choice_matches_loaded_type_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
            .any(|((item_id, _quantity), item_type)| {
                *item_id != 0 && *item_id == choice.item_id && *item_type == choice.loot_item_type
            })
    }

    pub(super) fn represented_reward_choice_template_exists_like_cpp(
        &self,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        match choice.loot_item_type {
            QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP => self
                .item_store()
                .is_some_and(|store| store.get(choice.item_id).is_some()),
            QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP => self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id)),
            _ => false,
        }
    }

    fn represented_can_select_quest_package_item_like_cpp(
        &self,
        quest_package_item: &QuestPackageItemEntry,
    ) -> bool {
        let Ok(item_id) = u32::try_from(quest_package_item.item_id) else {
            return false;
        };
        if self
            .item_store()
            .is_none_or(|store| store.get(item_id).is_none())
        {
            return false;
        }

        let Some(sparse) = self
            .item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
        else {
            return false;
        };

        let player_team = crate::session::player_team_for_race_cpp(self.player_race_like_cpp());
        if ((sparse.flags[1] & ItemFlags2::FactionAlliance as u32) != 0
            && player_team != wow_constants::unit::Team::Alliance)
            || ((sparse.flags[1] & ItemFlags2::FactionHorde as u32) != 0
                && player_team != wow_constants::unit::Team::Horde)
        {
            return false;
        }

        match quest_package_item.display_type {
            QUEST_PACKAGE_FILTER_EVERYONE_LIKE_CPP => true,
            QUEST_PACKAGE_FILTER_CLASS_LIKE_CPP => false,
            QUEST_PACKAGE_FILTER_LOOT_SPECIALIZATION_LIKE_CPP => false,
            _ => false,
        }
    }

    pub(super) fn represented_quest_package_choice_matches_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
            || quest.quest_package_id == 0
        {
            return false;
        }

        let Some(store) = &self.quest_package_item_store else {
            return false;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return false;
        };

        let primary_valid = store
            .quest_package_items_like_cpp(quest.quest_package_id)
            .filter(|entry| entry.item_id == choice_item_id)
            .any(|entry| self.represented_can_select_quest_package_item_like_cpp(entry));
        if primary_valid {
            return true;
        }

        store
            .quest_package_items_fallback_like_cpp(quest.quest_package_id)
            .any(|entry| entry.item_id == choice_item_id)
    }

    fn send_quest_failed_like_cpp(&self, quest_id: u32, reason: InventoryResult) {
        if quest_id == 0 {
            return;
        }

        self.send_packet(&QuestGiverQuestFailed {
            quest_id,
            reason: reason as u32,
        });
    }

    fn represented_quest_reward_inventory_plan_result_like_cpp(
        &self,
        item_id: u32,
        count: u32,
    ) -> InventoryResult {
        self.plan_store_new_direct_inventory_item(item_id, count)
            .map(|(result, _, _)| result)
            .unwrap_or(InventoryResult::ItemNotFound)
    }

    fn send_quest_package_reward_inventory_error_like_cpp(
        &self,
        result: InventoryResult,
        item_id: u32,
    ) {
        let limit_category = self
            .item_storage_template(item_id)
            .map(|template| u32::from(template.item_limit_category))
            .unwrap_or(0);
        self.send_equip_error(result, None, None, 0, limit_category);
    }

    pub(super) fn represented_can_reward_quest_inventory_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        // C++ `Player::CanRewardQuest(quest, rewardType, rewardId, true)`.
        if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP {
            for ((item_id, count), item_type) in quest
                .reward_choice_items
                .iter()
                .zip(quest.reward_choice_item_types.iter())
            {
                if *item_id == 0
                    || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                    || *item_id != choice.item_id
                {
                    continue;
                }

                let result =
                    self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
                if result != InventoryResult::Ok {
                    self.send_quest_failed_like_cpp(quest.id, result);
                    return false;
                }
            }
        }

        for (item_id, count) in quest.reward_items.iter().zip(quest.reward_amounts.iter()) {
            if *item_id == 0 {
                continue;
            }

            let result =
                self.represented_quest_reward_inventory_plan_result_like_cpp(*item_id, *count);
            if result != InventoryResult::Ok {
                self.send_quest_failed_like_cpp(quest.id, result);
                return false;
            }
        }

        if quest.quest_package_id == 0
            || choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
        {
            return true;
        }

        let Some(store) = &self.quest_package_item_store else {
            return true;
        };
        let Ok(choice_item_id) = i32::try_from(choice.item_id) else {
            return true;
        };

        let mut has_filtered_quest_package_reward = false;
        for entry in store.quest_package_items_like_cpp(quest.quest_package_id) {
            if entry.item_id != choice_item_id
                || !self.represented_can_select_quest_package_item_like_cpp(entry)
            {
                continue;
            }

            has_filtered_quest_package_reward = true;
            let Ok(item_id) = u32::try_from(entry.item_id) else {
                self.send_quest_package_reward_inventory_error_like_cpp(
                    InventoryResult::ItemNotFound,
                    0,
                );
                return false;
            };
            let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                item_id,
                entry.item_quantity,
            );
            if result != InventoryResult::Ok {
                self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                return false;
            }
        }

        if !has_filtered_quest_package_reward {
            for entry in store.quest_package_items_fallback_like_cpp(quest.quest_package_id) {
                if entry.item_id != choice_item_id {
                    continue;
                }

                let Ok(item_id) = u32::try_from(entry.item_id) else {
                    self.send_quest_package_reward_inventory_error_like_cpp(
                        InventoryResult::ItemNotFound,
                        0,
                    );
                    return false;
                };
                let result = self.represented_quest_reward_inventory_plan_result_like_cpp(
                    item_id,
                    entry.item_quantity,
                );
                if result != InventoryResult::Ok {
                    self.send_quest_package_reward_inventory_error_like_cpp(result, item_id);
                    return false;
                }
            }
        }

        true
    }

    pub(super) async fn reward_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let quest_id = quest.id;
        let choice_item_id = choice.item_id;
        self.set_represented_can_delay_teleport_like_cpp(true);

        macro_rules! reward_abort {
            () => {{
                self.set_represented_can_delay_teleport_like_cpp(false);
                return false;
            }};
        }

        if !self
            .remove_quest_required_items_and_currencies_like_cpp(quest)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented quest objective/item-drop removal failed before reward mutation"
            );
            reward_abort!();
        }

        self.remove_represented_timed_quest_like_cpp(quest_id);

        if !self.store_fixed_quest_reward_items_like_cpp(quest).await {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented fixed reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_chosen_quest_reward_item_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented chosen reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_quest_package_reward_items_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest package item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .grant_quest_reward_currencies_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest reward currency grant failed before reward mutation"
            );
            reward_abort!();
        }

        self.apply_represented_quest_reward_skill_like_cpp(quest);

        let money = quest.reward_money_difficulty;
        if money > 0 {
            match self
                .mutate_and_persist_player_gold_exclusive_like_cpp(|old_money| {
                    crate::session::loot_money_durable_outcome_like_cpp(old_money, money as u64).0
                })
                .await
            {
                Some((old_money, new_money)) => {
                    if old_money != new_money {
                        self.enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                    }
                }
                None => {
                    // Boundary: the represented reward path persists item and
                    // currency grants before reaching money and does not yet
                    // own C++ `Player::RewardQuest` as one durable transaction.
                    // Aborting here would leave the quest retryable after those
                    // grants and permit duplicates. Preserve the existing
                    // completion behavior; an ambiguous money COMMIT has
                    // already quarantined/kicked the session in the shared
                    // helper. Atomic quest reward persistence is separate debt.
                    warn!(
                        account = self.account_id,
                        quest_id,
                        money,
                        "Quest reward money was not durably established; preserving non-atomic represented reward completion to avoid duplicate retry"
                    );
                }
            }
        }

        self.apply_represented_quest_title_and_talent_rewards_like_cpp(quest);
        self.record_represented_quest_reward_mail_like_cpp(quest, quest_giver_guid);
        self.apply_quest_reward_lockout_status_like_cpp(quest).await;

        let xp = self.quest_xp_reward_like_cpp(quest);
        let rewarded_slot = self.find_quest_slot_like_cpp(quest_id);

        self.invalidate_player_quest_status_authority_like_cpp();
        if self
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.statuses.remove(&quest_id);
                if !quest.is_repeatable() {
                    state.rewarded_quest_ids.insert(quest_id);
                }
            })
            .is_none()
        {
            return false;
        }
        if !quest.is_repeatable() {
            self.save_quest_to_db(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
                .await;
        } else {
            self.delete_quest_from_db(quest_id).await;
        }
        self.sync_player_registry_state_like_cpp();
        if let Some(slot) = rewarded_slot {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }

        info!(
            account = self.account_id,
            quest_id,
            xp,
            gold = money,
            repeatable = quest.is_repeatable(),
            "Quest rewarded"
        );

        let game_event_outcome = self
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await;
        debug!(
            account = self.account_id,
            quest_id,
            outcome = ?game_event_outcome,
            "Represented C++ GameEventMgr::HandleQuestComplete notification after quest reward"
        );

        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp,
            money,
            skill_line_id: quest.reward_skill_line_id,
            skill_points: quest.reward_skill_points,
            use_quest_reward_currency: false,
        });

        self.send_packet(&QuestUpdateComplete { quest_id });

        self.record_represented_quest_reward_reputation_like_cpp(quest);
        self.record_represented_quest_reward_spell_casts_like_cpp(quest);

        if xp > 0 {
            // C++ `Player::RewardQuest` calls `GiveXP(XP, nullptr)`: quest XP
            // does not consume rested XP. RAF remains mutually exclusive with
            // rested XP and may still apply inside `GiveXP`.
            self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
        }

        self.set_represented_can_delay_teleport_like_cpp(false);

        true
    }
}
