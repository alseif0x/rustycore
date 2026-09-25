// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory packet adapters for swap, equip, storage, and item modification.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub async fn handle_swap_inv_item(&mut self, swap: SwapInvItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_swap_inv_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            swap,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_auto_equip_item(&mut self, equip: AutoEquipItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_auto_equip_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            equip,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_auto_equip_item_slot(&mut self, equip: AutoEquipItemSlot) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_auto_equip_item_slot_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            equip,
        )
        .await;
    }

    #[cfg(test)]
    pub async fn handle_swap_item(&mut self, swap: wow_packet::packets::item::SwapItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_swap_item_with_generator_like_cpp(generators.item.as_ref(), &catalogs, swap)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_auto_store_bag_item(
        &mut self,
        store: wow_packet::packets::item::AutoStoreBagItem,
    ) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_auto_store_bag_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            store,
        )
        .await;
    }

    /// Handle CMSG_SWAP_INV_ITEM: drag-and-drop item between two inventory slots.
    pub async fn handle_swap_inv_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        swap: SwapInvItem,
    ) {
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
            item_guid_generator,
            creature_spawn_catalogs,
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.src_slot),
            wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, swap.dst_slot),
        )
        .await;
    }

    /// Handle CMSG_AUTO_EQUIP_ITEM: right-click to auto-equip/unequip an item.
    pub async fn handle_auto_equip_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        equip: AutoEquipItem,
    ) {
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
            self.execute_inventory_swap_positions_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                source_pos,
                destination,
            )
            .await;
            return;
        };
        let displaced_is_child = self
            .resolved_inventory_item_object_like_cpp(displaced.guid)
            .is_some_and(|item| item.has_item_flag(ItemFieldFlags::CHILD));
        if displaced_is_child {
            self.execute_inventory_swap_positions_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                source_pos,
                destination,
            )
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
                self.execute_inventory_swap_positions_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    source_pos,
                    destination,
                )
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
            self.execute_inventory_swap_positions_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                source_pos,
                destination,
            )
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
            item_guid_generator,
            creature_spawn_catalogs,
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
        self.execute_inventory_equip_to_empty_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            equip.pack_slot,
            equip.slot,
            destination,
        )
        .await;
    }

    /// Handle CMSG_AUTO_EQUIP_ITEM_SLOT.
    ///
    /// C++ treats this as an explicit GUID + destination equipment-slot swap:
    /// it requires exactly one `InvUpdate` source position, verifies that the
    /// GUID still lives at that source position, rejects src==dst, then calls
    /// `Player::SwapItem` with the packed source position.
    pub async fn handle_auto_equip_item_slot_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        equip: AutoEquipItemSlot,
    ) {
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
        self.execute_inventory_swap_positions_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            source,
            destination,
        )
        .await;
    }

    /// Handle CMSG_SWAP_ITEM: C++ container-aware swap between two positions.
    /// C++ ref: `WorldSession::HandleSwapItem`
    /// (`Handlers/ItemHandler.cpp:130-173`).
    pub async fn handle_swap_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        swap: wow_packet::packets::item::SwapItem,
    ) {
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
        self.execute_inventory_swap_positions_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            source,
            destination,
        )
        .await;
    }

    /// Handle CMSG_AUTO_STORE_BAG_ITEM: right-click to store item in bag/backpack.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreBagItemOpcode`
    /// (`Handlers/ItemHandler.cpp:699-743`).
    pub async fn handle_auto_store_bag_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
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

        let runtime_item = self.resolved_inventory_item_object_like_cpp(source.guid);
        let proto = self.item_storage_template(source.entry_id);
        let source_pos = wow_entities::make_item_pos(store.container_slot_a, store.slot_a);
        if is_equipment_pos(store.container_slot_a, store.slot_a)
            || wow_entities::is_bag_pos(source_pos)
        {
            let result = self.can_unequip_inventory_item_at_like_cpp(
                store.container_slot_a,
                store.slot_a,
                !wow_entities::is_bag_pos(source_pos),
                runtime_item.as_ref(),
                proto.as_ref(),
                self.direct_item_contains_items(source.guid),
            );
            if result != InventoryResult::Ok {
                self.send_equip_error(result, Some(source.guid), None, 0, 0);
                return;
            }
        }

        self.execute_inventory_storage_move_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
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
        let Some(runtime_item) = self.resolved_inventory_item_object_like_cpp(item.guid) else {
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
        let _ = self.apply_inventory_item_object_updates_like_cpp(
            item.guid,
            &[wow_entities::ItemObjectUpdateLikeCpp::ClearEnchantment(
                EnchantmentSlot::EnhancementTemporary,
            )],
        );
    }
}
