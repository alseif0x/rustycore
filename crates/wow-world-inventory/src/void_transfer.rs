// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

use wow_constants::{EnchantmentSlot, ItemModifier};
use wow_entities::{INVENTORY_SLOT_BAG_0, PlayerInventoryItem as InventoryItem};
use wow_world_core::session::HubRef;
use crate::{EffectiveVoidStorageRandomPropertiesLikeCpp, PlannedVoidDestroyedInventoryItemLikeCpp};

impl crate::InventoryState {
    pub fn plan_void_storage_destroyed_items_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        slot: u8,
        inventory_item: InventoryItem,
        cleared_mainhand_enchantments: Vec<wow_constants::EnchantmentSlot>,
    ) -> Option<Vec<PlannedVoidDestroyedInventoryItemLikeCpp>> {
        let mut destroyed_items = self
            .represented_inventory_descendants_postorder_like_cpp(hub, inventory_item.guid)?
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
}

impl crate::InventoryState {
    /// Resolve the state installed by C++ `Item::SetItemRandomProperties`.
    pub fn effective_void_storage_random_properties_like_cpp(
        &self,
        hub: HubRef<'_>,
        random_properties_id: i32,
        random_properties_seed: i32,
    ) -> EffectiveVoidStorageRandomPropertiesLikeCpp {
        let mut result = EffectiveVoidStorageRandomPropertiesLikeCpp::default();
        if random_properties_id > 0 {
            let Some(entry) = hub
                .catalogs
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
            let Some(entry) = hub
                .catalogs
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

    pub fn void_storage_merged_item_write_like_cpp(
        &self,
        hub: HubRef<'_>,
        inventory_item: &InventoryItem,
        item: &wow_entities::Item,
        enchantments: &str,
    ) -> wow_persistence::VoidStorageMergedInventoryItemWriteLikeCpp {
        let data = item.data();
        let mut charges = String::new();
        for charge in data.spell_charges.iter().take(
            hub.catalogs
                .item_effect_count_like_cpp(item.object().entry()),
        ) {
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
}

impl crate::InventoryState {
    pub fn void_storage_withdrawal_container_db_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        planned_container_db_guids: &HashMap<u8, u64>,
    ) -> Option<u64> {
        planned_container_db_guids
            .get(&bag)
            .copied()
            .or_else(|| self.inventory_container_db_guid_like_cpp(hub, bag))
    }

    pub fn void_storage_withdrawal_container_item_guid_like_cpp(
        &self,
        hub: HubRef<'_>,
        bag: u8,
        planned_container_item_guids: &HashMap<u8, wow_core::ObjectGuid>,
    ) -> Option<wow_core::ObjectGuid> {
        if bag == INVENTORY_SLOT_BAG_0 {
            return hub.core.player_guid();
        }

        planned_container_item_guids.get(&bag).copied().or_else(|| {
            self.resolved_inventory_item_like_cpp(hub, bag)
                .map(|item| item.guid)
        })
    }
}
