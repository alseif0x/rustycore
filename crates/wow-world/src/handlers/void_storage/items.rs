//! Items operations of void_storage.
//!
//! Divided out of the single inherent impl under #707; every method keeps
//! its name, signature and body.

use super::*;

impl WorldSession {
    /// C++ passes packet `uint32 DstSlot` to helpers taking `uint8`, so the
    /// language conversion truncates before the 160-slot range check.
    pub(super) fn void_storage_swap_destination_slot_like_cpp(dst_slot: u32) -> u8 {
        dst_slot as u8
    }

    pub(super) fn void_storage_item_write_like_cpp(
        item: &RepresentedVoidStorageItemLikeCpp,
    ) -> wow_persistence::VoidStorageItemWriteLikeCpp {
        wow_persistence::VoidStorageItemWriteLikeCpp {
            item_id: item.item_id,
            item_entry: item.item_entry,
            creator_guid: item.creator_guid.counter() as u64,
            fixed_scaling_level: item.fixed_scaling_level,
            random_properties_id: item.random_properties_id,
            random_properties_seed: item.random_properties_seed,
            context: item.context,
        }
    }

    /// Resolve the state installed by C++ `Item::SetItemRandomProperties`.
    pub(super) fn effective_void_storage_random_properties_like_cpp(
        &self,
        random_properties_id: i32,
        random_properties_seed: i32,
    ) -> EffectiveVoidStorageRandomPropertiesLikeCpp {
        let mut result = EffectiveVoidStorageRandomPropertiesLikeCpp::default();
        if random_properties_id > 0 {
            let Some(entry) = self
                .item_random_properties_store()
                .and_then(|store| store.get(random_properties_id as u32))
            else {
                return result;
            };
            result.id = random_properties_id;
            // C++ only installs PropertySeed for a suffix; positive random
            // properties keep the newly created item's zero seed.
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                result.enchantment_ids[EnchantmentSlot::Property2 as usize + offset] =
                    i32::from(*enchantment_id);
            }
        } else if random_properties_id < 0 {
            let Some(entry) = self
                .item_random_suffix_store()
                .and_then(|store| store.get(random_properties_id.unsigned_abs()))
            else {
                return result;
            };
            result.id = random_properties_id;
            result.seed = random_properties_seed;
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                result.enchantment_ids[EnchantmentSlot::Property0 as usize + offset] =
                    i32::from(*enchantment_id);
            }
        }
        result
    }

    pub(super) fn void_storage_enchantments_db_string_like_cpp(
        enchantment_ids: &[i32; wow_entities::MAX_ENCHANTMENT_SLOT],
    ) -> String {
        enchantment_ids
            .iter()
            .flat_map(|id| [id.to_string(), "0".to_string(), "0".to_string()])
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn overwrite_void_storage_random_property_enchantments_like_cpp(
        enchantments: &str,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) -> String {
        let mut fields = enchantments
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if fields.len() != wow_entities::MAX_ENCHANTMENT_SLOT * 3 {
            fields = vec!["0".to_string(); wow_entities::MAX_ENCHANTMENT_SLOT * 3];
        }
        let slots = if random_properties.id > 0 {
            EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
        } else if random_properties.id < 0 {
            EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
        } else {
            return fields.join(" ");
        };
        for slot in slots {
            let base = slot * 3;
            fields[base] = random_properties.enchantment_ids[slot].to_string();
            fields[base + 1] = "0".to_string();
            fields[base + 2] = "0".to_string();
        }
        fields.join(" ")
    }

    pub(super) fn void_storage_merged_item_write_like_cpp(
        &self,
        inventory_item: &InventoryItem,
        item: &wow_entities::Item,
        enchantments: &str,
    ) -> wow_persistence::VoidStorageMergedInventoryItemWriteLikeCpp {
        let data = item.data();
        let mut charges = String::new();
        for charge in data
            .spell_charges
            .iter()
            .take(self.item_effect_count_like_cpp(item.object().entry()))
        {
            charges.push_str(&charge.to_string());
            charges.push(' ');
        }

        wow_persistence::VoidStorageMergedInventoryItemWriteLikeCpp {
            item_db_guid: inventory_item.db_guid,
            item_entry: item.object().entry(),
            owner_guid: data.owner.counter() as u64,
            creator_guid: data.creator.counter() as u64,
            gift_creator_guid: data.gift_creator.counter() as u64,
            count: item.count(),
            expiration: data.expiration,
            charges,
            dynamic_flags: data.dynamic_flags,
            enchantments: enchantments.to_owned(),
            durability: data.durability,
            create_played_time: data.create_played_time,
            text: item.text().to_owned(),
            battle_pet_species_id: item.get_modifier(ItemModifier::BattlePetSpeciesId),
            battle_pet_breed_data: item.get_modifier(ItemModifier::BattlePetBreedData),
            battle_pet_level: item.get_modifier(ItemModifier::BattlePetLevel),
            battle_pet_display_id: item.get_modifier(ItemModifier::BattlePetDisplayId),
            random_properties_id: data.random_properties_id,
            property_seed: data.property_seed,
            context: data.context,
        }
    }

    pub(super) fn apply_effective_void_storage_random_properties_like_cpp(
        item: &mut wow_entities::Item,
        random_properties: &EffectiveVoidStorageRandomPropertiesLikeCpp,
    ) {
        if random_properties.id == 0 {
            return;
        }
        item.set_random_properties_id(random_properties.id);
        item.set_property_seed(random_properties.seed);
        let slots = if random_properties.id > 0 {
            EnchantmentSlot::Property2 as usize..=EnchantmentSlot::Property4 as usize
        } else {
            EnchantmentSlot::Property0 as usize..=EnchantmentSlot::Property2 as usize
        };
        for slot_index in slots {
            if let Some(slot) = EnchantmentSlot::from_usize(slot_index) {
                item.set_enchantment(slot, random_properties.enchantment_ids[slot_index], 0, 0);
            }
        }
    }

    pub(super) fn plan_void_storage_destroyed_items_like_cpp(
        &self,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
    ) -> Option<Vec<PlannedVoidDestroyedInventoryItemLikeCpp>> {
        let mut destroyed_items = self
            .represented_inventory_descendants_postorder_like_cpp(inventory_item.guid)?
            .into_iter()
            .map(
                |(bag, slot, inventory_item)| PlannedVoidDestroyedInventoryItemLikeCpp {
                    bag,
                    slot,
                    inventory_item,
                    cleared_mainhand_enchantments: Vec::new(),
                },
            )
            .collect::<Vec<_>>();
        destroyed_items.push(PlannedVoidDestroyedInventoryItemLikeCpp {
            bag,
            slot,
            inventory_item,
            cleared_mainhand_enchantments,
        });
        Some(destroyed_items)
    }

    pub(super) fn clear_item_publication_changes_like_cpp(item: &mut wow_entities::Item) {
        item.clear_item_data_changes();
        item.object_mut().clear_update_mask(false);
    }

    pub(super) fn apply_committed_void_storage_destroyed_items_like_cpp(
        &mut self,
        destroyed_items: &[PlannedVoidDestroyedInventoryItemLikeCpp],
    ) -> Option<(Vec<wow_core::ObjectGuid>, Vec<u32>)> {
        let mut destroyed_guids = Vec::with_capacity(destroyed_items.len());
        let mut changed_quest_ids = Vec::new();
        for destroyed in destroyed_items {
            let _ = self.apply_inventory_item_remove_side_effects_like_cpp(
                destroyed.bag,
                destroyed.slot,
                destroyed.inventory_item.guid,
                &destroyed.cleared_mainhand_enchantments,
            );
            let removed = self.apply_committed_inventory_item_removal_like_cpp(
                destroyed.bag,
                destroyed.slot,
                destroyed.inventory_item.guid,
            );
            debug_assert!(removed);
            destroyed_guids.push(destroyed.inventory_item.guid);
            // C++ recursive DestroyItem runs ItemRemovedQuestCheck after each
            // child/parent removal, preserving intermediate objective updates.
            changed_quest_ids
                .extend(self.apply_quest_item_removed_like_cpp(destroyed.inventory_item.entry_id)?);
        }
        changed_quest_ids.sort_unstable();
        changed_quest_ids.dedup();
        Some((destroyed_guids, changed_quest_ids))
    }
}
