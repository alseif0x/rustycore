// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical item-move application and publication operations.

use super::*;

impl WorldSession {
    pub(crate) async fn execute_inventory_equip_to_empty_raw_like_cpp(
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
        let Some(runtime_item) = self.resolved_inventory_item_object_like_cpp(source.guid) else {
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
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            source.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::ReplaceAllItemFlags(
                ItemFieldFlags::from_bits_retain(planned_item.item_flags_bits()),
            )],
        );
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
        self.sync_player_registry_state_like_cpp();
    }

    /// C++ `Player::AutoUnequipOffhandIfNeed` after an equipment move. The
    /// normal two-hand equip path has already proved `CanStoreItem`, so use the
    /// same persisted storage executor rather than the older runtime-only
    /// represented helper.
    pub(crate) async fn execute_inventory_auto_unequip_offhand_if_need_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
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
            item_guid_generator,
            creature_spawn_catalogs,
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

    pub(crate) async fn execute_inventory_stack_merge_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
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
        let Some(source_object) = self.resolved_inventory_item_object_like_cpp(source.guid) else {
            return;
        };
        let Some(destination_object) =
            self.resolved_inventory_item_object_like_cpp(destination.guid)
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

        let _ = self.apply_inventory_item_object_updates_like_cpp(
            destination.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::SetCount(
                destination_count,
            )],
        );
        self.send_packet(&UpdateObject::item_stack_count_update(
            destination.guid,
            self.player_map_id_like_cpp(),
            destination_count,
        ));
        if source_count > 0 {
            let mut updates = vec![wow_entities::ItemObjectUpdateLikeCpp::SetCount(
                source_count,
            )];
            updates.extend(
                source_cleared
                    .iter()
                    .copied()
                    .map(wow_entities::ItemObjectUpdateLikeCpp::ClearEnchantment),
            );
            let _ = self.apply_inventory_item_object_updates_like_cpp(source.guid, &updates);
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
        self.sync_player_registry_state_like_cpp();
        if is_equipment_pos(destination_bag, destination_slot) {
            self.execute_inventory_auto_unequip_offhand_if_need_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
            )
            .await;
        }
    }
}
