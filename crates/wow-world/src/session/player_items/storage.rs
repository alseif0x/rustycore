//! Represented inventory storage: the complete store, remove and swap operations over the canonical Player item slots.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;
use crate::session_rules::CR_ARMOR_PENETRATION_LIKE_CPP;

impl WorldSession {
    pub(crate) fn move_represented_direct_inventory_item_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> bool {
        if src == dst {
            return true;
        }

        let src_item = self.resolved_inventory_item_like_cpp(src);
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let Some(src_item) = src_item else {
            return false;
        };

        self.insert_inventory_item_like_cpp(dst, src_item.clone());
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        self.update_inventory_item_object_like_cpp(src_item.guid, |item| {
            item.set_contained_in(player_guid);
            item.set_slot(dst);
        });

        if let Some(dst_item) = dst_item {
            self.insert_inventory_item_like_cpp(src, dst_item.clone());
            self.update_inventory_item_object_like_cpp(dst_item.guid, |item| {
                item.set_contained_in(player_guid);
                item.set_slot(src);
            });
        } else {
            self.remove_inventory_item_like_cpp(src);
        }

        true
    }
    pub(crate) fn move_represented_direct_inventory_item_with_item_mods_like_cpp(
        &mut self,
        src: u8,
        dst: u8,
    ) -> Option<bool> {
        if src == dst {
            return Some(false);
        }

        let src_item = self.resolved_inventory_item_like_cpp(src)?;
        let dst_item = self.resolved_inventory_item_like_cpp(dst);
        let mut item_mods_changed = false;

        if src < INVENTORY_SLOT_BAG_END {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                src,
                src_item.guid,
            );
        }

        if src < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, src, false);
            item_mods_changed = true;
        }

        if dst < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_direct_inventory_item_set_remove_like_cpp(
                INVENTORY_SLOT_BAG_0,
                dst,
                dst_item.guid,
            );
        }

        if dst < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, dst, false);
            item_mods_changed = true;
        }

        if !self.move_represented_direct_inventory_item_like_cpp(src, dst) {
            return None;
        }

        if dst < INVENTORY_SLOT_BAG_END {
            let _ = self.record_represented_items_set_item_like_cpp(src_item.guid, true);
        }

        if dst < INVENTORY_SLOT_BAG_END
            && self
                .resolved_inventory_item_object_like_cpp(src_item.guid)
                .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(src_item.guid, dst, true);
            item_mods_changed = true;
        }

        if src < INVENTORY_SLOT_BAG_END
            && let Some(dst_item) = dst_item.as_ref()
        {
            let _ = self.record_represented_items_set_item_like_cpp(dst_item.guid, true);
        }

        if src < INVENTORY_SLOT_BAG_END
            && dst_item.as_ref().is_some_and(|item| {
                self.resolved_inventory_item_object_like_cpp(item.guid)
                    .is_some_and(|item_object| !item_object.is_broken())
            })
        {
            let dst_item = dst_item.as_ref().expect("checked Some above");
            self.record_represented_item_mods_like_cpp(dst_item.guid, src, true);
            item_mods_changed = true;
        }

        Some(item_mods_changed)
    }
    pub(in crate::session) fn sync_canonical_direct_inventory_move_like_cpp(
        &mut self,
        src: u8,
        dst_bag: u8,
        dst_slot: u8,
        item_guid: ObjectGuid,
    ) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
            if dst_bag == INVENTORY_SLOT_BAG_0 {
                let _ = player.store_top_level_item(dst_slot, item_guid);
            } else {
                let _ = player.store_bag_item(dst_bag, dst_slot, item_guid);
            }
        });
    }
    pub(in crate::session) fn sync_canonical_direct_inventory_remove_like_cpp(&mut self, src: u8) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            let _ = player.remove_top_level_item(src);
        });
    }
    pub(crate) fn record_destroyed_inventory_item_mod_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, slot, false) != 0
    }
    pub(crate) fn record_direct_inventory_item_set_remove_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        self.record_represented_items_set_item_like_cpp(item_guid, false)
    }
    pub(in crate::session) fn represented_item_inventory_type_like_cpp(
        &self,
        item_entry: u32,
        item_guid: ObjectGuid,
    ) -> Option<InventoryType> {
        self.resolved_inventory_items_like_cpp()?
            .values()
            .find(|item| item.guid == item_guid)
            .and_then(|item| item.inventory_type)
            .and_then(<InventoryType as num_traits::FromPrimitive>::from_u8)
            .or_else(|| {
                self.items
                    .store
                    .as_ref()
                    .and_then(|store| store.get(item_entry))
                    .and_then(|record| {
                        <InventoryType as num_traits::FromPrimitive>::from_i8(record.inventory_type)
                    })
            })
    }
    /// Resolve C++ `ItemTemplate::GetInventoryType()` for equipment-slot mapping.
    pub fn item_template_inventory_type(&self, item_id: u32) -> Option<u8> {
        self.item_storage_template(item_id)
            .map(|template| template.inventory_type as u8)
            .filter(|&inventory_type| inventory_type != InventoryType::NonEquip as u8)
    }
    pub(crate) fn make_inventory_item_object(
        &self,
        item_guid: ObjectGuid,
        entry_id: u32,
        owner_guid: ObjectGuid,
        count: u32,
        durability: u32,
        context: ItemContext,
        slot: u8,
    ) -> Item {
        let max_durability = self.item_template_max_durability(entry_id).max(durability);
        let mut item = Item::new(i64::from(self.total_played_time));
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: entry_id,
            context,
            owner: Some(owner_guid),
            max_durability,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.set_count(count.max(1));
        item.set_durability(durability);
        item.set_slot(slot);
        item.set_container_guid(ObjectGuid::EMPTY);
        item
    }
    pub(crate) fn insert_inventory_item_object(&mut self, item: Item) -> Option<Item> {
        let item_guid = item.object().guid();
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.item_objects_mut().insert(item_guid, item)
        })
        .flatten()
    }
    pub(crate) fn update_inventory_item_object_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        update: impl FnOnce(&mut Item),
    ) -> bool {
        let mut update = Some(update);
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            let Some(item) = inventory.item_objects_mut().get_mut(&item_guid) else {
                return false;
            };
            if let Some(update) = update.take() {
                update(item);
            }
            true
        })
        .unwrap_or(false)
    }
    pub(crate) fn remove_inventory_item_object(&mut self, item_guid: ObjectGuid) -> Option<Item> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.item_objects_mut().remove(&item_guid)
        })
        .flatten()
    }
    pub(crate) fn clear_inventory_items_and_objects_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.inventory_items_mut().clear();
            inventory.item_objects_mut().clear();
        });
    }
    pub(crate) fn clear_all_inventory_runtime_like_cpp(&mut self) {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            *inventory = PlayerInventoryRuntime::default();
        });
        self.reset_represented_item_bonus_runtime_like_cpp();
    }
    pub(crate) fn insert_inventory_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.inventory_items_mut().insert(slot, item)
        })
        .flatten()
    }
    pub(crate) fn send_inventory_item_pending_values_update_like_cpp(&self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        if let Some(packet) = item_values_update_to_update_object(
            item_guid,
            self.player_map_id_like_cpp(),
            &item.values_update(),
        ) {
            self.send_packet(&packet);
        }
    }
    pub(crate) fn remove_inventory_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.inventory_items_mut().remove(&slot)
        })
        .flatten()
    }
    pub(crate) fn update_inventory_item_metadata_like_cpp(
        &mut self,
        slot: u8,
        item_guid: ObjectGuid,
        entry_id: u32,
        inventory_type: Option<u8>,
    ) -> bool {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            let Some(inventory_item) = inventory
                .inventory_items_mut()
                .get_mut(&slot)
                .filter(|inventory_item| inventory_item.guid == item_guid)
            else {
                return false;
            };
            inventory_item.entry_id = entry_id;
            inventory_item.inventory_type = inventory_type;
            true
        })
        .unwrap_or(false)
    }
    pub(crate) fn represented_inventory_item_counts_like_cpp(&self) -> Option<HashMap<u32, u32>> {
        let inventory_items = self.resolved_inventory_items_like_cpp()?;
        let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
        Some(
            inventory_items
                .values()
                .filter_map(|inventory_item| item_objects.get(&inventory_item.guid))
                .chain(item_objects.values().filter(|item| {
                    !item.container_guid().is_empty()
                        && item_objects.contains_key(&item.container_guid())
                }))
                .filter(|item| !item.is_in_trade())
                .fold(HashMap::new(), |mut counts, item| {
                    let entry_id = item.object().entry();
                    counts
                        .entry(entry_id)
                        .and_modify(|count| *count = count.saturating_add(item.count()))
                        .or_insert(item.count());
                    counts
                }),
        )
    }
    /// Resolve an inventory item by GUID following C++ `Player::GetItemByGuid`.
    ///
    /// C++ iterates direct inventory and represented bags. Rust still models
    /// nested bag contents through runtime `Item` objects, so this returns the
    /// effective `(bag, slot, item)` tuple needed by `DestroyItem`.
    pub(crate) fn get_inventory_item_by_guid_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> Option<(u8, u8, InventoryItem)> {
        if item_guid.is_empty() {
            return None;
        }

        if let Some((&slot, item)) = self
            .resolved_inventory_items_like_cpp()?
            .iter()
            .find(|(_, item)| item.guid == item_guid)
        {
            if (slot as usize) < PLAYER_SLOT_END && !crate::session_rules::is_buyback_slot(slot) {
                return Some((INVENTORY_SLOT_BAG_0, slot, item.clone()));
            }
        }

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid)?;
        if !runtime_item.is_in_bag() {
            return None;
        }

        let bag = runtime_item.bag_slot();
        let slot = runtime_item.slot();
        self.get_inventory_item_by_pos(bag, slot)
            .filter(|item| item.guid == item_guid)
            .map(|item| (bag, slot, item))
    }
    pub(crate) fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        self.can_use_inventory_item_represented_with_loading_like_cpp(item, runtime_item, true)
    }
    pub(in crate::session) fn direct_inventory_player_snapshot(&self) -> Option<Player> {
        if let Some(player) = self.with_owned_player_like_cpp(Clone::clone) {
            return Some(player);
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let player_guid = self.player_guid()?;
            let mut player = Player::new(None, false);
            player
                .unit_mut()
                .world_mut()
                .object_mut()
                .create(player_guid);
            player.set_inventory_slot_count(self.resolved_player_inventory_slot_count_like_cpp()?);
            player.set_bank_bag_slot_count(self.resolved_player_bank_bag_slot_count_like_cpp()?);

            let item_objects = self.resolved_inventory_item_objects_like_cpp()?;
            for (&slot, item) in &self.resolved_inventory_items_like_cpp()? {
                if (slot as usize) < PLAYER_SLOT_END && !crate::session_rules::is_buyback_slot(slot)
                {
                    let _ = player.store_top_level_item(slot, item.guid);
                    if is_represented_bag_slot(slot)
                        && item_objects.contains_key(&item.guid)
                        && let Some(template) = self.item_storage_template(item.entry_id)
                        && template.container_slots > 0
                    {
                        let _ =
                            player.register_bag_storage(slot, item.guid, template.container_slots);
                    }
                }
            }
            return Some(player);
        }

        None
    }
    pub(crate) fn remove_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };

        let Some(removed_enchantments) = self.mutate_canonical_player_like_cpp(|player| {
            let removed_enchantments = player.remove_enchantment_durations(&mut item);
            let _ = player.remove_item_durations(&item);
            removed_enchantments
        }) else {
            return;
        };

        if removed_enchantments.is_empty() {
            return;
        }

        let _ = self.update_inventory_item_object_like_cpp(item_guid, |stored_item| {
            for duration in removed_enchantments {
                stored_item.set_enchantment_duration(duration.slot, duration.left_duration_ms);
            }
        });
    }
    pub(crate) fn remove_inventory_tradeable_item_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.remove_tradeable_item(&item);
        });
    }
    pub(crate) fn add_inventory_item_duration_refs_like_cpp(&mut self, item_guid: ObjectGuid) {
        let Some(mut item) = self.resolved_inventory_item_object_like_cpp(item_guid) else {
            return;
        };
        let Some((owner_guid, item_update, enchantment_updates)) = self
            .mutate_canonical_player_like_cpp(|player| {
                let item_update = player.add_item_durations(&item);
                let enchantment_updates = player.add_enchantment_durations(&mut item);
                (player.guid(), item_update, enchantment_updates)
            })
        else {
            return;
        };

        self.insert_inventory_item_object(item);
        if let Some(update) = item_update {
            self.send_item_time_update_plan(&update);
        }
        self.send_item_enchant_time_update_plans(owner_guid, &enchantment_updates);
    }
    /// C++ `Player::ApplyItemObtainSpells(item, true)` after `_StoreItem`.
    pub(crate) async fn apply_inventory_item_obtain_spells_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        item_entry: u32,
    ) -> Vec<i32> {
        if self
            .item_template_flags(item_entry)
            .is_some_and(|flags| flags.contains(ItemFlags::LEGACY))
        {
            return Vec::new();
        }
        let spell_ids = self
            .items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .into_iter()
                    .filter(|effect| {
                        effect.trigger_type == ItemSpelltriggerType::OnPickup as i8
                            && effect.spell_id > 0
                    })
                    .map(|effect| effect.spell_id)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return Vec::new();
        };
        let mut applied = Vec::new();
        for spell_id in spell_ids {
            if visible_auras.values().any(|aura| aura.spell_id == spell_id) {
                continue;
            }
            // C++ `Player::ApplyItemObtainSpells` uses
            // `CastSpellExtraArgs().SetCastItem(item)`, whose default trigger
            // flags are TRIGGERED_NONE: not a triggered cast, and the global
            // cooldown applies.
            if self
                .execute_server_triggered_spell_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    spell_id,
                    player_guid,
                    SpellCastMetadata::default(),
                )
                .await
                .is_ok()
            {
                applied.push(spell_id);
            }
        }
        applied
    }
    #[cfg(test)]
    pub(crate) async fn apply_inventory_item_obtain_spells_like_cpp(
        &mut self,
        item_entry: u32,
    ) -> Vec<i32> {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.apply_inventory_item_obtain_spells_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            item_entry,
        )
        .await
    }
    pub(crate) fn apply_inventory_item_remove_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
        cleared_mainhand_enchantments: &[EnchantmentSlot],
    ) -> bool {
        self.remove_inventory_item_duration_refs_like_cpp(item_guid);
        self.remove_inventory_tradeable_item_like_cpp(item_guid);

        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        let _ = self.record_direct_inventory_item_set_remove_like_cpp(bag, slot, item_guid);
        let item_mods_changed =
            self.record_destroyed_inventory_item_mod_remove_like_cpp(bag, slot, item_guid);
        self.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.remove_item_flag2(ItemFieldFlags2::EQUIPPED);
            for enchantment_slot in cleared_mainhand_enchantments {
                item.clear_enchantment(*enchantment_slot);
            }
        });

        if slot < PROFESSION_SLOT_END {
            self.record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    /// C++ `StoreItem`/`BankItem`/`EquipItem` post-placement side effects for
    /// a runtime item whose persistence and position have already committed.
    pub(crate) fn apply_inventory_item_store_side_effects_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) -> bool {
        self.add_inventory_item_duration_refs_like_cpp(item_guid);
        if bag != INVENTORY_SLOT_BAG_0 || slot >= INVENTORY_SLOT_BAG_END {
            return false;
        }

        self.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        });
        let _ = self.record_represented_items_set_item_like_cpp(item_guid, true);
        let item_mods_changed = if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| !item.is_broken())
        {
            self.record_represented_item_mods_like_cpp(item_guid, slot, true) != 0
        } else {
            false
        };
        if slot < PROFESSION_SLOT_END {
            self.record_inventory_item_combat_stat_recalculations_like_cpp(slot);
        }
        item_mods_changed
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_inventory_item_combat_stat_recalculations_like_cpp(&mut self, slot: u8) {
        #[cfg(test)]
        {
            let attack = match slot {
                EQUIPMENT_SLOT_MAINHAND => Some(WeaponAttackType::BaseAttack),
                EQUIPMENT_SLOT_OFFHAND => Some(WeaponAttackType::OffAttack),
                _ => None,
            };
            if let Some(attack) = attack {
                self.represented_combat_stat_recalculations_like_cpp
                    .push(RepresentedCombatStatRecalculationLikeCpp::Expertise { attack });
                self.represented_combat_stat_recalculations_like_cpp.push(
                    RepresentedCombatStatRecalculationLikeCpp::Rating {
                        combat_rating: CR_ARMOR_PENETRATION_LIKE_CPP,
                    },
                );
            }
        }
    }
    #[cfg(test)]
    fn mirror_player_inventory_runtime_to_legacy_like_cpp(
        &mut self,
        inventory: &PlayerInventoryRuntime,
    ) {
        self.inventory_items = inventory.inventory_items().clone();
        self.buyback_items = inventory.buyback_items().clone();
        self.buyback_price = *inventory.buyback_price();
        self.buyback_timestamp = *inventory.buyback_timestamp();
        self.current_buyback_slot = inventory.current_buyback_slot();
        self.inventory_item_objects = inventory.item_objects().clone();
    }
    pub(crate) fn mutate_player_inventory_runtime_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let mut update = Some(update);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut inventory = PlayerInventoryRuntime::default();
            inventory
                .inventory_items_mut()
                .extend(self.inventory_items.clone());
            inventory
                .buyback_items_mut()
                .extend(self.buyback_items.clone());
            *inventory.buyback_price_mut() = self.buyback_price;
            *inventory.buyback_timestamp_mut() = self.buyback_timestamp;
            inventory.set_current_buyback_slot(self.current_buyback_slot);
            inventory
                .item_objects_mut()
                .extend(self.inventory_item_objects.clone());
            let result =
                update.take().expect("inventory mutation closure runs once")(&mut inventory);
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
            return Some(result);
        }
        let result = self.with_owned_player_mut_like_cpp(|player| {
            update.take().expect("inventory mutation closure runs once")(
                player.inventory_runtime_mut_like_cpp(),
            )
        });
        #[cfg(test)]
        if result.is_some()
            && let Some(inventory) = self
                .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            self.mirror_player_inventory_runtime_to_legacy_like_cpp(&inventory);
        }
        result
    }
    pub(in crate::session) fn resolved_player_inventory_runtime_like_cpp(
        &self,
    ) -> Option<PlayerInventoryRuntime> {
        if let Some(inventory) =
            self.with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
        {
            return Some(inventory);
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut inventory = PlayerInventoryRuntime::default();
            inventory
                .inventory_items_mut()
                .extend(self.inventory_items.clone());
            inventory
                .buyback_items_mut()
                .extend(self.buyback_items.clone());
            *inventory.buyback_price_mut() = self.buyback_price;
            *inventory.buyback_timestamp_mut() = self.buyback_timestamp;
            inventory.set_current_buyback_slot(self.current_buyback_slot);
            inventory
                .item_objects_mut()
                .extend(self.inventory_item_objects.clone());
            return Some(inventory);
        }
        None
    }
    pub(crate) fn resolved_inventory_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.inventory_items().clone())
    }
    pub(crate) fn resolved_inventory_item_objects_like_cpp(
        &self,
    ) -> Option<HashMap<ObjectGuid, Item>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.item_objects().clone())
    }
    pub(crate) fn resolved_inventory_item_like_cpp(&self, slot: u8) -> Option<InventoryItem> {
        self.resolved_player_inventory_runtime_like_cpp()?
            .inventory_items()
            .get(&slot)
            .cloned()
    }
    pub(crate) fn resolved_inventory_item_object_like_cpp(&self, guid: ObjectGuid) -> Option<Item> {
        self.resolved_player_inventory_runtime_like_cpp()?
            .item_objects()
            .get(&guid)
            .cloned()
    }
    #[cfg(test)]
    pub(crate) fn inventory_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.inventory_items
    }
    #[cfg(test)]
    pub(crate) fn inventory_item_objects_like_cpp(&self) -> &HashMap<ObjectGuid, Item> {
        &self.inventory_item_objects
    }
}
