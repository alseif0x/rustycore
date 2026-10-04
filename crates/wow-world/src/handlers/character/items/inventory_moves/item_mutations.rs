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
        self.inventory_equip_cx_like_cpp()
            .execute_inventory_equip_to_empty_raw_like_cpp(source_bag, source_slot, destination)
            .await;
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
        let Some(reason) = ({
            let (s, h) = crate::session::split_inventory_ref(self);
            s.represented_auto_unequip_offhand_reason_like_cpp(h, false)
        }) else {
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
            self.catalogs.item_effect_count_like_cpp(source.entry_id),
        );
        let destination_mutable = item_storage_mutable_persistence_like_cpp(
            destination.db_guid,
            &destination_object,
            destination_count,
            destination_object.item_flags_bits(),
            destination_enchantments,
            self.catalogs
                .item_effect_count_like_cpp(destination.entry_id),
        );
        let Some(inventory_port) = self.lifecycle.player_inventory_persistence_port_like_cpp()
        else {
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
            self.core.player_map_id_like_cpp(),
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
                self.core.player_map_id_like_cpp(),
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
                self.core.player_map_id_like_cpp(),
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
