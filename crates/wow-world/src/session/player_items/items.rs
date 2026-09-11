//! Remaining represented item operations owned by the inventory responsibility.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn set_vendor_buy_item_test_override_like_cpp(
        &mut self,
        item: VendorBuyItemTestOverrideLikeCpp,
    ) {
        self.vendor_buy_item_test_override_like_cpp = Some(item);
    }
    #[cfg(test)]
    pub(crate) fn vendor_buy_item_test_override_like_cpp(
        &self,
    ) -> Option<VendorBuyItemTestOverrideLikeCpp> {
        self.vendor_buy_item_test_override_like_cpp
    }
    pub(in crate::session) fn represented_player_has_quest_for_loot_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        self.represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(item_id)
            || self
                .item_template_addon_quest_log_item_id_like_cpp(item_id)
                .is_some_and(|quest_log_item_id| {
                    quest_log_item_id != 0
                        && self
                            .represented_current_player_has_incomplete_quest_objective_for_object_id_like_cpp(
                            i32::try_from(quest_log_item_id).unwrap_or(i32::MAX),
                        )
                })
            || self.represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(item_id)
    }
    fn represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.represented_current_player_has_incomplete_quest_objective_for_object_id_like_cpp(
            item_object_id,
        )
    }
    fn represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        let Some(quest_store) = self.quests.store.as_ref() else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        quests.statuses.values().any(|status| {
            if status.status != crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }
            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };
            quest
                .item_drop
                .iter()
                .enumerate()
                .any(|(index, drop_item_id)| {
                    if *drop_item_id != item_id {
                        return false;
                    }
                    let Some(template) = self.item_storage_template(item_id) else {
                        return false;
                    };
                    let quantity = quest.item_drop_quantity[index];
                    let mut max_allowed_count = if quantity != 0 {
                        quantity
                    } else {
                        template.max_stack_size
                    };
                    if template.max_count > 0 {
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }
                    self.represented_inventory_item_counts_like_cpp()
                        .is_some_and(|counts| {
                            counts.get(&item_id).copied().unwrap_or(0) < max_allowed_count
                        })
                })
        })
    }
    /// Install the process-wide C++
    /// `sObjectMgr->GetGenerator<HighGuid::Item>()` mirror.
    #[cfg(test)]
    pub fn set_item_guid_generator_like_cpp(&mut self, generator: Arc<ObjectGuidGenerator>) {
        assert_eq!(
            generator.high_guid(),
            HighGuid::Item,
            "item GUID allocator must use HighGuid::Item"
        );
        self.item_guid_generator_like_cpp = Some(generator);
    }
    #[cfg(test)]
    pub(crate) fn item_guid_generator_like_cpp_for_bridge(
        &self,
    ) -> Option<Arc<ObjectGuidGenerator>> {
        self.item_guid_generator_like_cpp.clone()
    }
    /// Bounded C++ `CollectionMgr::OnItemAdded`.
    pub(crate) fn on_item_added_to_collection_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Vec<wow_entities::PlayerValuesUpdate> {
        let item_id = item.object().entry();
        let mut updates = Vec::new();

        if self
            .heirloom_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
            && self.add_account_heirloom_like_cpp(item_id, 0)
            && let Some(update) = self.add_player_heirloom_dynamic_fields_like_cpp(item_id, 0)
        {
            updates.push(update);
        }

        if let Some(update) = self.add_item_appearance_for_runtime_item_like_cpp(item) {
            updates.push(update);
        }

        updates
    }
    pub(in crate::session) fn item_spec_class_mask_from_overrides_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        let overrides = self
            .items
            .spec_override_store
            .as_ref()?
            .overrides_for_item_like_cpp(item_id)?;
        let chr_specializations = self.chr.specialization_store.as_ref()?;

        let mut mask = 0_u32;
        for item_spec_override in overrides {
            if let Some(specialization) =
                chr_specializations.get(u32::from(item_spec_override.spec_id))
            {
                mask |= player_class_mask_for_transmog_like_cpp(specialization.class_id);
            }
        }

        Some(mask)
    }
    /// C++ `DB2Manager::GetItemDisplayId`.
    pub fn item_display_id(&self, item_id: u32, appearance_mod_id: u32) -> Option<u32> {
        let modified = self
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get_for_item(item_id, appearance_mod_id))?;
        let appearance_id = u32::try_from(modified.item_appearance_id).ok()?;
        self.items
            .appearance_store
            .as_ref()
            .and_then(|store| store.item_display_info_id(appearance_id))
    }
    pub(in crate::session) fn represented_top_level_item_mod_targets_like_cpp(
        &self,
    ) -> Option<Vec<(u8, ObjectGuid)>> {
        let mut targets = self
            .resolved_inventory_item_objects_like_cpp()?
            .values()
            .filter(|item| {
                item.container_guid().is_empty()
                    && item.slot() < INVENTORY_SLOT_BAG_END
                    && !item.is_broken()
            })
            .map(|item| (item.slot(), item.object().guid()))
            .collect::<Vec<_>>();
        targets.sort_by_key(|(slot, guid)| (*slot, guid.counter()));
        Some(targets)
    }
    pub(crate) fn record_represented_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> bool {
        !self
            .record_represented_items_set_item_events_like_cpp(item_guid, apply)
            .is_empty()
    }
    pub(in crate::session) fn record_represented_items_set_item_events_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        apply: bool,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(item_entry) = self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .map(|item| item.object().entry())
        else {
            return Vec::new();
        };
        let Some(item_set) = self.item_set_for_item_id_like_cpp(item_entry).cloned() else {
            return Vec::new();
        };

        let events = if apply {
            self.record_represented_add_items_set_item_like_cpp(item_guid, &item_set)
        } else {
            self.record_represented_remove_items_set_item_like_cpp(item_guid, &item_set)
        };
        #[cfg(test)]
        self.represented_item_set_spell_events_like_cpp
            .extend(events.iter().copied());
        events
    }
    fn record_represented_add_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        item_set: &wow_data::ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        if item_set.required_skill != 0 {
            let Some(skill_value) =
                self.resolved_player_skill_value_like_cpp(item_set.required_skill as u16)
            else {
                return Vec::new();
            };
            if skill_value < item_set.required_skill_rank {
                return Vec::new();
            }
        }
        if item_set.set_flags & ITEM_SET_FLAG_LEGACY_INACTIVE_LIKE_CPP != 0 {
            return Vec::new();
        }
        if self.represented_heirloom_item_set_bonus_over_level_cap_like_cpp(item_guid) {
            return Vec::new();
        }

        let mut events = Vec::new();
        let Some(equipped_count_after) =
            self.mutate_player_item_modifier_runtime_like_cpp(|state| {
                let effect = state
                    .item_set_effects
                    .entry(item_set.id)
                    .or_insert_with(|| RepresentedItemSetEffectLikeCpp {
                        item_set_id: item_set.id,
                        ..Default::default()
                    });
                effect.equipped_items.insert(item_guid);
                effect.equipped_items.len()
            })
        else {
            return Vec::new();
        };

        let primary_spec = self.represented_primary_specialization_id_like_cpp();
        let spells: Vec<_> = self
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) > equipped_count_after {
                continue;
            }
            if !self.represented_item_set_spell_exists_like_cpp(item_set_spell.spell_id) {
                continue;
            }
            let inserted = self
                .mutate_player_item_modifier_runtime_like_cpp(|state| {
                    state
                        .item_set_effects
                        .get_mut(&item_set.id)
                        .is_some_and(|effect| effect.set_bonuses.insert(item_set_spell.id))
                })
                .unwrap_or(false);
            if !inserted {
                continue;
            }
            if item_set_spell.chr_spec_id != 0
                && Some(u32::from(item_set_spell.chr_spec_id)) != primary_spec
            {
                continue;
            }
            events.push(RepresentedItemSetSpellEventLikeCpp {
                item_set_id: item_set.id,
                spell_entry_id: item_set_spell.id,
                spell_id: item_set_spell.spell_id,
                threshold: item_set_spell.threshold,
                apply: true,
            });
        }

        events
    }
    fn record_represented_remove_items_set_item_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        item_set: &wow_data::ItemSetEntry,
    ) -> Vec<RepresentedItemSetSpellEventLikeCpp> {
        let Some(equipped_count_after) = self
            .mutate_player_item_modifier_runtime_like_cpp(|state| {
                let effect = state.item_set_effects.get_mut(&item_set.id)?;
                effect.equipped_items.remove(&item_guid);
                Some(effect.equipped_items.len())
            })
            .flatten()
        else {
            return Vec::new();
        };
        let mut events = Vec::new();

        let spells: Vec<_> = self
            .item_set_spells_like_cpp(item_set.id)
            .into_iter()
            .cloned()
            .collect();
        for item_set_spell in spells {
            if usize::from(item_set_spell.threshold) <= equipped_count_after {
                continue;
            }
            let removed = self
                .mutate_player_item_modifier_runtime_like_cpp(|state| {
                    state
                        .item_set_effects
                        .get_mut(&item_set.id)
                        .is_some_and(|effect| effect.set_bonuses.remove(&item_set_spell.id))
                })
                .unwrap_or(false);
            if !removed {
                continue;
            }
            events.push(RepresentedItemSetSpellEventLikeCpp {
                item_set_id: item_set.id,
                spell_entry_id: item_set_spell.id,
                spell_id: item_set_spell.spell_id,
                threshold: item_set_spell.threshold,
                apply: false,
            });
        }

        let _ = self.mutate_player_item_modifier_runtime_like_cpp(|state| {
            if state
                .item_set_effects
                .get(&item_set.id)
                .is_some_and(|effect| effect.equipped_items.is_empty())
            {
                state.item_set_effects.remove(&item_set.id);
            }
        });

        events
    }
    pub(crate) fn item_drop_rate_like_cpp(&self, item_id: u32) -> f32 {
        let quality = self
            .item_template_quality(item_id)
            .and_then(<ItemQuality as num_traits::FromPrimitive>::from_i8);
        match quality {
            Some(ItemQuality::Poor) => self.loot_drop_rates.item_poor,
            Some(ItemQuality::Normal) => self.loot_drop_rates.item_normal,
            Some(ItemQuality::Uncommon) => self.loot_drop_rates.item_uncommon,
            Some(ItemQuality::Rare) => self.loot_drop_rates.item_rare,
            Some(ItemQuality::Epic) => self.loot_drop_rates.item_epic,
            Some(ItemQuality::Legendary) => self.loot_drop_rates.item_legendary,
            Some(ItemQuality::Artifact) => self.loot_drop_rates.item_artifact,
            _ => 1.0,
        }
    }
    pub(crate) fn item_effect_count_like_cpp(&self, item_entry: u32) -> usize {
        self.items
            .effect_store
            .as_ref()
            .map(|store| {
                store
                    .item_effects_for_item_id_like_cpp(item_entry)
                    .len()
                    .min(MAX_ITEM_SPELLS)
            })
            .unwrap_or(0)
    }
    /// C++ `Item::IsBoundAccountWide` template-flag predicate.
    pub fn is_item_bound_account_wide(&self, item_id: u32) -> bool {
        self.item_template_flags(item_id)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    }
    pub(in crate::session) fn item_shield_block_value_like_cpp(&self, item_id: u32) -> Option<i16> {
        let basic = self.items.store.as_ref()?.get(item_id)?;
        if basic.class_id != ItemClass::Armor as u8
            || basic.subclass_id != ItemSubClassArmor::Shield as u8
        {
            return None;
        }

        let template = self.item_random_property_template(item_id)?;
        let item_level = u32::from(template.item_level);
        let quality = u32::try_from(template.quality).ok()?;
        self.shield_block_regular_game_table
            .as_ref()?
            .shield_block_for_quality_like_cpp(item_level, quality)
            .filter(|value| *value != 0)
    }
    pub(crate) fn insert_buyback_item_like_cpp(
        &mut self,
        slot: u8,
        item: InventoryItem,
    ) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.store_buyback_item_in_slot_like_cpp(slot, item)
        })
        .flatten()
    }
    pub(crate) fn remove_buyback_item_like_cpp(&mut self, slot: u8) -> Option<InventoryItem> {
        self.mutate_player_inventory_runtime_like_cpp(|inventory| {
            inventory.remove_buyback_item_from_slot_like_cpp(slot)
        })
        .flatten()
    }
    /// Remove a fully-looted runtime item after its DB rows were deleted.
    pub(crate) fn remove_fully_looted_runtime_item(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        if bag == INVENTORY_SLOT_BAG_0
            && self
                .resolved_inventory_item_like_cpp(slot)
                .is_some_and(|item| item.guid == item_guid)
        {
            self.remove_inventory_item_like_cpp(slot);
        }
        self.remove_inventory_item_object(item_guid);
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn represented_has_item_count_like_cpp(&self, item_entry: u32, count: u32) -> bool {
        if count == 0 {
            return true;
        }

        self.represented_inventory_item_counts_like_cpp()
            .is_some_and(|counts| counts.get(&item_entry).copied().unwrap_or(0) >= count)
    }
    pub(crate) fn can_destroy_direct_item_like_cpp(
        &self,
        slot: u8,
        source_item: Option<&Item>,
        proto: Option<&ItemStorageTemplate>,
        source_is_not_empty_bag: bool,
    ) -> InventoryResult {
        self.can_unequip_inventory_item_at_like_cpp(
            INVENTORY_SLOT_BAG_0,
            slot,
            false,
            source_item,
            proto,
            source_is_not_empty_bag,
        )
    }
    pub(crate) fn direct_item_contains_items(&self, item_guid: ObjectGuid) -> bool {
        self.resolved_inventory_item_objects_like_cpp()
            .is_some_and(|items| {
                items
                    .values()
                    .any(|item| item.container_guid() == item_guid)
            })
    }
    pub(crate) fn has_active_non_item_loot_views_like_cpp(&self) -> bool {
        (!self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item())
            || self
                .active_loot_view_owners
                .iter()
                .any(|guid| !guid.is_item())
    }
    /// Allocate item database/object GUIDs from the process-wide generator.
    ///
    /// C++ initializes this generator once from `MAX(item_instance.guid) + 1`
    /// in `ObjectMgr::SetHighestGuids`, and every `Item::CreateItem` consumes
    /// the next value.  Rust sessions execute concurrently, so the shared
    /// `ObjectGuidGenerator` uses an atomic fetch-add.  Allocations are never
    /// returned after a later persistence failure, matching C++ Item creation.
    pub(crate) fn allocate_item_instance_guids_with_generator_like_cpp(
        &self,
        generator: &ObjectGuidGenerator,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        if count == 0 {
            return Some(Vec::new());
        }
        if generator.high_guid() != HighGuid::Item {
            return None;
        }

        let realm_id = self.realm_id();
        (0..count)
            .map(|_| {
                let counter = generator.generate();
                let db_guid = u64::try_from(counter).ok()?;
                Some((db_guid, ObjectGuid::create_item(realm_id, counter)))
            })
            .collect()
    }
    #[cfg(test)]
    pub(crate) fn allocate_item_instance_guids_like_cpp(
        &self,
        count: usize,
    ) -> Option<Vec<(u64, ObjectGuid)>> {
        let generator = self.item_guid_generator_like_cpp.as_deref()?;
        self.allocate_item_instance_guids_with_generator_like_cpp(generator, count)
    }
    pub(in crate::session) fn represented_has_item_fit_to_spell_requirements_like_cpp(
        &self,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        const SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP: u32 = 0x0010_0000;

        if equipped.equipped_item_class < 0 {
            return true;
        }

        match equipped.equipped_item_class {
            class if class == ItemClass::Weapon as i8 => {
                self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    EQUIPMENT_SLOT_MAINHAND,
                    equipped,
                ) || self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                    EQUIPMENT_SLOT_OFFHAND,
                    equipped,
                )
            }
            class if class == ItemClass::Armor as i8 => {
                if self
                    .spell_catalogs
                    .spell_store
                    .as_ref()
                    .is_some_and(|store| {
                        store.has_attribute8_like_cpp(
                            equipped.spell_id,
                            SPELL_ATTR8_REQUIRES_EQUIPPED_INV_TYPES_LIKE_CPP,
                        )
                    })
                {
                    [
                        EQUIPMENT_SLOT_HEAD,
                        EQUIPMENT_SLOT_SHOULDERS,
                        EQUIPMENT_SLOT_CHEST,
                        EQUIPMENT_SLOT_WAIST,
                        EQUIPMENT_SLOT_LEGS,
                        EQUIPMENT_SLOT_FEET,
                        EQUIPMENT_SLOT_WRISTS,
                        EQUIPMENT_SLOT_HANDS,
                    ]
                    .into_iter()
                    .all(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            slot, equipped,
                        )
                    })
                } else {
                    self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                        EQUIPMENT_SLOT_OFFHAND,
                        equipped,
                    ) || (EQUIPMENT_SLOT_HEAD..EQUIPMENT_SLOT_MAINHAND).any(|slot| {
                        self.represented_equipped_item_in_slot_fits_spell_requirements_like_cpp(
                            slot, equipped,
                        )
                    })
                }
            }
            _ => false,
        }
    }
    pub(in crate::session) fn represented_item_fits_spell_requirements_like_cpp(
        &self,
        item_id: u32,
        equipped: &SpellEquippedItemsEntry,
    ) -> bool {
        let Some(item) = self
            .items
            .store
            .as_ref()
            .and_then(|store| store.get(item_id))
        else {
            return false;
        };

        if equipped.equipped_item_class >= 0 {
            if equipped.equipped_item_class != item.class_id as i8 {
                return false;
            }

            if equipped.equipped_item_subclass != 0 {
                let subclass = u32::from(item.subclass_id);
                if subclass >= i32::BITS
                    || (equipped.equipped_item_subclass & (1_i32 << subclass)) == 0
                {
                    return false;
                }
            }
        }

        true
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_remove_item_like_cpp(
        &mut self,
        remove: RepresentedAuctionRemoveItemLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_remove_items_like_cpp.push(remove);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_remove_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionRemoveItemLikeCpp] {
        &self.represented_auction_remove_items_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_auction_sell_item_like_cpp(
        &mut self,
        sell: RepresentedAuctionSellItemLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_auction_sell_items_like_cpp.push(sell);
    }
    #[cfg(test)]
    pub(crate) fn represented_auction_sell_items_like_cpp(
        &self,
    ) -> &[RepresentedAuctionSellItemLikeCpp] {
        &self.represented_auction_sell_items_like_cpp
    }
    pub(in crate::session) fn record_represented_offhand_item_mod_remove_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
    ) -> bool {
        if self
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some_and(|item| item.is_broken())
        {
            return false;
        }

        self.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_OFFHAND, false) != 0
    }
    pub(crate) fn resolved_buyback_items_like_cpp(&self) -> Option<HashMap<u8, InventoryItem>> {
        self.resolved_player_inventory_runtime_like_cpp()
            .map(|inventory| inventory.buyback_items().clone())
    }
    #[cfg(test)]
    pub(crate) fn buyback_items_like_cpp(&self) -> &HashMap<u8, InventoryItem> {
        &self.buyback_items
    }
    #[cfg(test)]
    pub(crate) fn represented_item_mod_reapply_events_like_cpp(
        &self,
    ) -> &[RepresentedItemModsReapplyEventLikeCpp] {
        &self.represented_item_mod_reapply_events_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_trade_spell_cast_item_like_cpp(&self) -> Option<ObjectGuid> {
        self.player_trade_state_snapshot_like_cpp()
            .flatten()
            .and_then(|state| state.spell_cast_item_guid)
    }
    pub(in crate::session) fn uncage_cast_item_still_matches_like_cpp(
        &self,
        cast_item_entry: u32,
        modifiers: SpellCastBattlePetItemModifiersLikeCpp,
    ) -> Option<(u8, u8, InventoryItem)> {
        let (bag, slot, inventory_item) =
            self.get_inventory_item_by_guid_like_cpp(modifiers.source_item_guid)?;
        if inventory_item.entry_id != cast_item_entry {
            return None;
        }
        let item = self.resolved_inventory_item_object_like_cpp(modifiers.source_item_guid)?;
        (item.object().entry() == cast_item_entry
            && item.get_modifier(ItemModifier::BattlePetSpeciesId) == modifiers.species_id
            && item.get_modifier(ItemModifier::BattlePetBreedData) == modifiers.breed_data
            && item.get_modifier(ItemModifier::BattlePetLevel) == u32::from(modifiers.level)
            && item.get_modifier(ItemModifier::BattlePetDisplayId) == modifiers.display_id)
            .then_some((bag, slot, inventory_item))
    }
    pub(crate) async fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle persistence port is unavailable".to_owned(),
            };
        };
        port.load_uncage_item_state_like_cpp(wow_persistence::PlayerUncageItemStateRequestLikeCpp {
            player_guid: player_db_guid,
            item_guid: item_db_guid,
        })
        .await
    }
    pub(crate) fn use_represented_gameobject_item_forge_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: wow_entities::ItemForgeUseSource,
    ) -> bool {
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::ItemForgeUsed {
                gameobject_guid,
                player_guid,
                condition_id: source.condition_id,
                forge_type: source.forge_type,
            },
        );

        true
    }
}
