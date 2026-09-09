//! Represented equipment: equip and unequip over the canonical Player.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    /// C++ `DB2Manager::GetItemChildEquipment(parentItemId)`.
    pub(crate) fn item_child_equipment_for_parent_like_cpp(
        &self,
        parent_item_id: u32,
    ) -> Option<&ItemChildEquipmentEntry> {
        self.item_child_equipment_store
            .as_ref()?
            .values()
            .find(|entry| entry.parent_item_id == parent_item_id)
    }
    pub(in crate::session) fn represented_equip_spell_fits_shapeshift_like_cpp(
        &self,
        spell_id: u32,
    ) -> bool {
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return true;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };

        let Some(form_id) = self.represented_shapeshift_form_like_cpp() else {
            return false;
        };
        spell_store
            .check_shapeshift_like_cpp(spell_id, form_id, |form| {
                self.spell_catalogs
                    .spell_shapeshift_form_store
                    .as_ref()
                    .and_then(|store| store.get(form))
            })
            .unwrap_or(SpellCastResult::Success)
            == SpellCastResult::Success
    }
    pub(crate) fn apply_initial_equipped_item_equip_auras_like_cpp(&mut self) -> Option<usize> {
        let mut equipped: Vec<_> = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| (item.slot(), item.object().guid()))
            .collect();
        equipped.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(
            equipped
                .into_iter()
                .map(|(_slot, item_guid)| self.apply_initial_item_equip_auras_like_cpp(item_guid))
                .sum(),
        )
    }
    pub(in crate::session) fn apply_initial_item_equip_auras_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> usize {
        let Some(_player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(item_effect_store) = self.item_effect_store.as_ref().cloned() else {
            return 0;
        };
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .filter(|item| item.container_guid().is_empty() && item.slot() < INVENTORY_SLOT_BAG_END)
            .map(|item| item.object().entry())
        else {
            return 0;
        };
        if !self.initial_loaded_item_mods_can_apply_like_cpp(item_guid) {
            return 0;
        }

        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let mut applied = 0usize;
        if self
            .item_stats_store
            .as_ref()
            .and_then(|store| store.sparse_template(item_entry))
            .is_some_and(|template| template.item_flags().contains(ItemFlags::LEGACY))
        {
            return 0;
        }

        let effects: Vec<_> = item_effect_store
            .item_effects_for_item_id_like_cpp(item_entry)
            .into_iter()
            .cloned()
            .collect();
        for effect in effects {
            if effect.trigger_type != 1 {
                continue;
            }
            if effect.spell_id <= 0 {
                continue;
            }
            if effect.chr_specialization_id != 0
                && Some(u32::from(effect.chr_specialization_id)) != primary_spec
            {
                continue;
            }
            if !self.represented_equip_spell_fits_shapeshift_like_cpp(effect.spell_id as u32) {
                continue;
            }

            let Some(spell_info) = self
                .spell_store()
                .and_then(|store| store.get(effect.spell_id))
                .cloned()
            else {
                continue;
            };
            let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 {
                continue;
            }
            if self
                .apply_aura_with_effect_mask_like_cpp(
                    effect.spell_id,
                    item_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            {
                applied += 1;
            }
        }

        applied
    }
    pub(in crate::session) fn inventory_equip_capabilities_like_cpp(&self) -> Option<(bool, bool)> {
        if let Some(capabilities) = self.with_owned_player_like_cpp(|player| {
            (
                player.unit().can_dual_wield_like_cpp(),
                player.can_titan_grip(),
            )
        }) {
            return Some(capabilities);
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some((false, false));
        }

        None
    }
    /// C++ `Player::CanUnequipItem` for any represented top-level or bag position.
    pub(crate) fn can_unequip_inventory_item_at_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        swap: bool,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        let pos = make_item_pos(bag, slot);
        if !is_equipment_packed_pos(pos) && !is_bag_pos(pos) {
            return InventoryResult::Ok;
        }

        let Some(player) = self.direct_inventory_player_snapshot() else {
            return InventoryResult::Ok;
        };
        let is_charmed = self
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().subsystems().control.is_charmed()
            })
            .unwrap_or(false);
        let is_in_progress_arena = self
            .player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.represented_status == Some(3))
            && self
                .map_store()
                .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
                .is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

        let Some(is_in_combat) = self.resolved_in_combat_like_cpp() else {
            return InventoryResult::CantDoThatRightNow;
        };

        player.can_unequip_item(CanUnequipItemArgs {
            pos,
            source_item,
            proto,
            swap,
            source_is_not_empty_bag,
            is_charmed,
            is_in_combat,
            is_in_progress_arena,
        })
    }
    #[cfg(test)]
    pub(crate) fn creature_equipment_store_like_cpp(
        &self,
    ) -> Option<&Arc<CreatureEquipmentStoreLikeCpp>> {
        self.creature_equipment_store_like_cpp.as_ref()
    }
    pub fn send_equip_error(
        &self,
        result: InventoryResult,
        item1: Option<ObjectGuid>,
        item2: Option<ObjectGuid>,
        required_level: u32,
        limit_category: u32,
    ) {
        let mut packet = InventoryChangeFailure::new(
            result,
            item1.unwrap_or(ObjectGuid::EMPTY),
            item2.unwrap_or(ObjectGuid::EMPTY),
        );

        if result != InventoryResult::Ok {
            packet.container_b_slot = 0;
            match result {
                InventoryResult::CantEquipLevelI | InventoryResult::PurchaseLevelTooLow => {
                    packet.level = required_level;
                }
                InventoryResult::ItemMaxLimitCategoryCountExceededIs
                | InventoryResult::ItemMaxLimitCategorySocketedExceededIs
                | InventoryResult::ItemMaxLimitCategoryEquippedExceededIs => {
                    packet.limit_category = limit_category;
                }
                _ => {}
            }
        }

        // C++ `Opcodes.cpp` registers `SMSG_INVENTORY_CHANGE_FAILURE` on
        // `CONNECTION_TYPE_REALM`, including errors raised by instance-routed
        // inventory requests after `ConnectTo`.
        self.send_packet_realm(&packet);
    }
    pub(crate) fn record_represented_avg_equipped_item_level_update_like_cpp(&mut self) {
        #[cfg(test)]
        {
            let Some(avg_equipped_item_level) = self.represented_avg_equipped_item_level_like_cpp()
            else {
                return;
            };
            self.represented_avg_equipped_item_level_updates_like_cpp
                .push(avg_equipped_item_level);
        }
    }
    pub(in crate::session) fn represented_can_equip_unique_item_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: &Item,
        except_slot: u8,
    ) -> InventoryResult {
        let Some(player) = self.direct_inventory_player_snapshot() else {
            return InventoryResult::ItemNotFound;
        };
        let Some(proto) = self.item_storage_template(entry_id) else {
            return InventoryResult::ItemNotFound;
        };

        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return InventoryResult::ItemNotFound;
        };
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return InventoryResult::ItemNotFound;
        };
        let mut equipped_templates = Vec::new();
        let mut equipped_items_with_templates = Vec::new();
        let mut equipped_gems = Vec::new();
        for (&slot, inventory_item) in &inventory_items {
            if slot >= EQUIPMENT_SLOT_END {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let Some(template) = self.item_storage_template(inventory_item.entry_id) else {
                continue;
            };
            for gem in &item.data().gems {
                let Ok(gem_entry) = u32::try_from(gem.item_id) else {
                    continue;
                };
                let Some(gem_template) = self.item_storage_template(gem_entry) else {
                    continue;
                };
                equipped_gems.push(EquippedGemRef::new(
                    slot,
                    gem_entry,
                    gem_template.item_limit_category,
                ));
            }
            equipped_templates.push((slot, item, template));
        }

        for (slot, item, template) in &equipped_templates {
            equipped_items_with_templates.push(ItemStorageRef::new(
                INVENTORY_SLOT_BAG_0,
                *slot,
                *item,
                Some(template),
            ));
        }

        let mut socketed_gem_templates = Vec::new();
        for gem in &runtime_item.data().gems {
            let Ok(gem_entry) = u32::try_from(gem.item_id) else {
                continue;
            };
            let Some(gem_template) = self.item_storage_template(gem_entry) else {
                continue;
            };
            let source_limit_category_count = if gem_template.item_limit_category == 0 {
                1
            } else {
                runtime_item
                    .data()
                    .gems
                    .iter()
                    .filter_map(|source_gem| u32::try_from(source_gem.item_id).ok())
                    .filter_map(|source_gem_entry| self.item_storage_template(source_gem_entry))
                    .filter(|source_gem_template| {
                        source_gem_template.item_limit_category == gem_template.item_limit_category
                    })
                    .count() as u32
            };
            let unique_equippable = gem_template.flags.contains(ItemFlags::UNIQUE_EQUIPPABLE);
            let limit_category =
                self.item_limit_category_template_like_cpp(gem_template.item_limit_category);
            socketed_gem_templates.push((
                gem_template,
                unique_equippable,
                limit_category,
                source_limit_category_count,
            ));
        }
        let mut socketed_gems = Vec::new();
        for (gem_template, unique_equippable, limit_category, source_limit_category_count) in
            &socketed_gem_templates
        {
            socketed_gems.push(SocketedGemUniqueRef::new(
                Some(gem_template),
                *unique_equippable,
                limit_category.as_ref(),
                *source_limit_category_count,
            ));
        }

        let unique_equippable = proto.flags.contains(ItemFlags::UNIQUE_EQUIPPABLE);
        let limit_category = self.item_limit_category_template_like_cpp(proto.item_limit_category);
        player.can_equip_unique_item(CanEquipUniqueItemArgs {
            source_item: Some(runtime_item),
            proto: Some(&proto),
            except_slot,
            limit_count: 1,
            unique_equippable,
            limit_category: limit_category.as_ref(),
            equipped_items: &equipped_items_with_templates,
            equipped_gems: &equipped_gems,
            socketed_gems: &socketed_gems,
        })
    }
    pub(in crate::session) fn represented_avg_total_item_level_can_equip_item_like_cpp(
        &self,
        entry_id: u32,
        runtime_item: &Item,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) -> InventoryResult {
        let inventory_item = InventoryItem {
            guid: runtime_item.object().guid(),
            entry_id,
            db_guid: runtime_item.object().guid().counter() as u64,
            inventory_type: self.item_template_inventory_type(entry_id),
        };
        self.can_equip_inventory_item_like_cpp(
            &inventory_item,
            runtime_item,
            NULL_SLOT,
            true,
            false,
            false,
            can_dual_wield,
            can_titan_grip,
        )
        .result
    }
    /// C++ `Player::CanEquipItem` for a represented runtime item.
    pub(crate) fn plan_equip_existing_inventory_item_like_cpp(
        &self,
        source_bag: u8,
        source_slot: u8,
        requested_slot: u8,
        swap: bool,
    ) -> Option<(InventoryResult, u16)> {
        let inventory_item = self.get_inventory_item_by_pos(source_bag, source_slot)?;
        let runtime_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid)?;
        let (can_dual_wield, can_titan_grip) = self.inventory_equip_capabilities_like_cpp()?;
        let is_in_combat = self.resolved_in_combat_like_cpp()?;
        let outcome = self.can_equip_inventory_item_like_cpp(
            &inventory_item,
            &runtime_item,
            requested_slot,
            swap,
            true,
            is_in_combat,
            can_dual_wield,
            can_titan_grip,
        );
        Some((outcome.result, outcome.dest))
    }
    #[allow(clippy::too_many_arguments)]
    fn can_equip_inventory_item_like_cpp(
        &self,
        inventory_item: &InventoryItem,
        runtime_item: &Item,
        requested_slot: u8,
        swap: bool,
        not_loading: bool,
        is_in_combat: bool,
        can_dual_wield: bool,
        can_titan_grip: bool,
    ) -> CanEquipItemOutcome {
        let Some(mut player) = self.direct_inventory_player_snapshot() else {
            return CanEquipItemOutcome {
                result: InventoryResult::ItemNotFound,
                dest: 0,
                unique_ignore_slot: None,
            };
        };
        let entry_id = inventory_item.entry_id;
        let Some(proto) = self.item_storage_template(entry_id) else {
            return CanEquipItemOutcome {
                result: InventoryResult::ItemNotFound,
                dest: 0,
                unique_ignore_slot: None,
            };
        };

        player
            .unit_mut()
            .set_can_dual_wield_like_cpp(can_dual_wield);
        player.set_can_titan_grip(can_titan_grip, 0);

        let Some(item_objects) = self.resolved_inventory_item_objects_like_cpp() else {
            return CanEquipItemOutcome {
                result: InventoryResult::ItemNotFound,
                dest: 0,
                unique_ignore_slot: None,
            };
        };
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return CanEquipItemOutcome {
                result: InventoryResult::ItemNotFound,
                dest: 0,
                unique_ignore_slot: None,
            };
        };
        let mut template_cache = HashMap::new();
        for item in item_objects.values() {
            let item_entry = item.object().entry();
            if let std::collections::hash_map::Entry::Vacant(entry) =
                template_cache.entry(item_entry)
            {
                if let Some(template) = self.item_storage_template(item_entry) {
                    entry.insert(template);
                }
            }
        }

        let mut represented_bag_slots_by_guid = HashMap::new();
        for (&slot, inventory_item) in &inventory_items {
            if Self::is_buyback_slot(slot) {
                continue;
            }
            if is_represented_bag_slot(slot) && item_objects.contains_key(&inventory_item.guid) {
                represented_bag_slots_by_guid.insert(inventory_item.guid, slot);
            }
        }

        let mut storage_rows = Vec::new();
        let mut equipped_items = Vec::new();
        for (&slot, inventory_item) in &inventory_items {
            if Self::is_buyback_slot(slot) {
                continue;
            }
            let Some(item) = item_objects.get(&inventory_item.guid) else {
                continue;
            };
            let Some(template) = template_cache.get(&inventory_item.entry_id) else {
                continue;
            };
            storage_rows.push((INVENTORY_SLOT_BAG_0, slot, item, template));
            if slot < EQUIPMENT_SLOT_END {
                equipped_items.push(ItemSlotRef::new(INVENTORY_SLOT_BAG_0, slot, item));
            }
        }
        for item in item_objects.values() {
            if item.is_in_trade() {
                continue;
            }
            let container_guid = item.container_guid();
            if container_guid.is_empty() {
                continue;
            }
            let Some(&bag_slot) = represented_bag_slots_by_guid.get(&container_guid) else {
                continue;
            };
            let item_entry = item.object().entry();
            let Some(template) = template_cache.get(&item_entry) else {
                continue;
            };
            storage_rows.push((bag_slot, item.slot(), item, template));
        }
        let stored_items: Vec<_> = storage_rows
            .iter()
            .map(|(bag, slot, item, template)| {
                ItemStorageRef::new(*bag, *slot, *item, Some(template))
            })
            .collect();

        let mainhand_template = self
            .resolved_inventory_item_like_cpp(EQUIPMENT_SLOT_MAINHAND)
            .and_then(|item| self.item_storage_template(item.entry_id));
        let is_two_hand_used = player.is_two_hand_used_template(mainhand_template.as_ref());
        let can_use_result = self.can_use_inventory_item_represented_with_loading_like_cpp(
            inventory_item,
            Some(runtime_item),
            not_loading,
        );
        let proto_always_allow_dual_wield = self
            .item_template_flags3(entry_id)
            .is_some_and(|flags| (flags & ItemFlags3::AlwaysAllowDualWield as u32) != 0);
        let limit_category = self.item_limit_category_template_like_cpp(proto.item_limit_category);
        let (is_stunned, is_charmed) = self
            .canonical_player_snapshot_like_cpp(|player| {
                (
                    player.unit().has_unit_state(UnitState::STUNNED.bits()),
                    player.unit().subsystems().control.is_charmed(),
                )
            })
            .unwrap_or((false, false));
        let is_in_progress_arena = self
            .player_battleground_state_snapshot_like_cpp()
            .is_some_and(|state| state.represented_status == Some(3))
            && self
                .map_store()
                .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
                .is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

        let offhand_item =
            self.get_inventory_item_by_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_OFFHAND);
        let offhand_runtime = offhand_item
            .as_ref()
            .and_then(|item| item_objects.get(&item.guid));
        let offhand_proto = offhand_item
            .as_ref()
            .and_then(|item| template_cache.get(&item.entry_id));
        let offhand_can_unequip_result = self.can_unequip_inventory_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_OFFHAND,
            false,
            offhand_runtime,
            offhand_proto,
            offhand_item
                .as_ref()
                .is_some_and(|item| self.direct_item_contains_items(item.guid)),
        );
        let offhand_can_store_result = offhand_item
            .as_ref()
            .and_then(|_| {
                self.plan_store_existing_inventory_item_like_cpp(
                    INVENTORY_SLOT_BAG_0,
                    EQUIPMENT_SLOT_OFFHAND,
                )
            })
            .map_or(InventoryResult::Ok, |(result, _, _)| result);

        let make_args = |can_equip_unique_result| CanEquipItemArgs {
            slot: requested_slot,
            proto: Some(&proto),
            source_item: Some(runtime_item),
            source_bop_trade_allowed_for_player: false,
            swap,
            not_loading,
            is_stunned,
            is_charmed,
            is_in_combat,
            is_in_progress_arena,
            weapon_change_timer_active: false,
            current_generic_spell_allows_equip: None,
            current_channeled_spell_allows_equip: None,
            heirloom_required_level_failed: false,
            can_use_result,
            can_equip_unique_result,
            can_dual_wield,
            can_titan_grip,
            is_two_hand_used,
            proto_always_allow_dual_wield,
            has_required_profession_skill: false,
            profession_slot: None,
            offhand_can_unequip_result,
            offhand_can_store_result,
            limit_category: limit_category.as_ref(),
            equipped_items: &equipped_items,
            stored_items: &stored_items,
        };

        // C++ computes the unique-equip ignore slot from the selected equip
        // destination. Run the already-ported selector once with a neutral
        // unique result, then repeat with the exact CanEquipUniqueItem result.
        let initial = player.can_equip_item(make_args(InventoryResult::Ok));
        if initial.result != InventoryResult::Ok {
            return initial;
        }
        let unique_result = self.represented_can_equip_unique_item_like_cpp(
            entry_id,
            runtime_item,
            initial.unique_ignore_slot.unwrap_or(NULL_SLOT),
        );
        player.can_equip_item(make_args(unique_result))
    }
    pub(crate) fn represented_avg_equipped_item_level_like_cpp(&self) -> Option<f32> {
        let (_, can_titan_grip) = self.inventory_equip_capabilities_like_cpp()?;
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let mut total_item_level = 0u32;
        for slot in 0..EQUIPMENT_SLOT_END {
            let Some(inventory_item) = inventory_items.get(&slot) else {
                continue;
            };
            let runtime_item = self.resolved_inventory_item_object_like_cpp(inventory_item.guid);
            let Some(item_level) = self
                .represented_item_level_like_cpp(inventory_item.entry_id, runtime_item.as_ref())
            else {
                continue;
            };
            total_item_level = total_item_level.saturating_add(item_level);
            let is_mainhand_two_hand = slot == EQUIPMENT_SLOT_MAINHAND
                && !can_titan_grip
                && self
                    .item_storage_template(inventory_item.entry_id)
                    .is_some_and(|item_template| {
                        item_template.inventory_type == InventoryType::Weapon2Hand
                    });
            if is_mainhand_two_hand {
                total_item_level = total_item_level.saturating_add(item_level);
            }
        }

        Some(total_item_level as f32 / 16.0)
    }
    #[cfg(test)]
    pub(crate) fn represented_avg_equipped_item_level_updates_like_cpp(&self) -> &[f32] {
        &self.represented_avg_equipped_item_level_updates_like_cpp
    }
}
