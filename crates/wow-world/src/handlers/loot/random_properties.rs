// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Random item-property generation and stack compatibility for loot items.

use rand::{
    Rng,
    distributions::{Distribution, WeightedIndex},
};
use wow_constants::{InventoryType, ItemQuality};
use wow_data::{ItemRandomEnchantmentTemplateEntry, ItemRandomPropertyTemplateEntry};
use wow_entities::Item;
use wow_packet::packets::loot::LootEntry;

use crate::session::WorldSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LootStoreRandomProperties {
    pub(super) id: i32,
    pub(super) seed: i32,
}

pub(super) fn loot_store_data_can_stack_with_item(
    loot_entry: &LootEntry,
    random_properties: LootStoreRandomProperties,
    item: &Item,
) -> bool {
    let data = item.data();
    data.random_properties_id == random_properties.id
        && data.property_seed == random_properties.seed
        && u8::try_from(data.context).unwrap_or(0) == loot_entry.item_context
}

impl WorldSession {
    pub(super) fn generate_loot_store_random_properties_with_rng_like_cpp<R: Rng + ?Sized>(
        &self,
        item_id: u32,
        rng: &mut R,
    ) -> LootStoreRandomProperties {
        // C++ Player::StoreLootItem calls ItemEnchantmentMgr::GenerateRandomProperties(itemid).
        let random_select = self.item_template_random_select(item_id);
        let random_suffix = self.item_template_random_suffix_group_id(item_id);
        if random_select == 0 && random_suffix == 0 {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        if random_select != 0 {
            let Some(random_properties_id) =
                self.select_random_enchantment_from_group_like_cpp(u32::from(random_select), rng)
            else {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            };

            if self
                .item_random_properties_store()
                .and_then(|store| store.get(random_properties_id))
                .is_none()
            {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            }

            return LootStoreRandomProperties {
                id: i32::try_from(random_properties_id).unwrap_or(0),
                seed: 0,
            };
        }

        let Some(random_suffix_id) =
            self.select_random_enchantment_from_group_like_cpp(u32::from(random_suffix), rng)
        else {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        };

        if self
            .item_random_suffix_store()
            .and_then(|store| store.get(random_suffix_id))
            .is_none()
        {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        let seed = self
            .item_random_property_template(item_id)
            .map(|template| self.random_property_points_like_cpp(template))
            .unwrap_or(0);

        LootStoreRandomProperties {
            id: -i32::try_from(random_suffix_id).unwrap_or(0),
            seed,
        }
    }

    fn select_random_enchantment_from_group_like_cpp<R: Rng + ?Sized>(
        &self,
        group_id: u32,
        rng: &mut R,
    ) -> Option<u32> {
        let group = self
            .item_random_enchantment_template_store()
            .and_then(|store| store.group(group_id))?;
        select_weighted_random_enchantment_like_cpp(group, rng)
    }

    fn random_property_points_like_cpp(&self, template: ItemRandomPropertyTemplateEntry) -> i32 {
        let prop_index =
            match <InventoryType as num_traits::FromPrimitive>::from_i8(template.inventory_type) {
                Some(InventoryType::NonEquip)
                | Some(InventoryType::Bag)
                | Some(InventoryType::Tabard)
                | Some(InventoryType::Ammo)
                | Some(InventoryType::Quiver)
                | Some(InventoryType::Relic)
                | None => return 0,
                Some(InventoryType::Head)
                | Some(InventoryType::Body)
                | Some(InventoryType::Chest)
                | Some(InventoryType::Legs)
                | Some(InventoryType::Weapon2Hand)
                | Some(InventoryType::Robe) => 0,
                Some(InventoryType::Shoulders)
                | Some(InventoryType::Waist)
                | Some(InventoryType::Feet)
                | Some(InventoryType::Hands)
                | Some(InventoryType::Trinket) => 1,
                Some(InventoryType::Neck)
                | Some(InventoryType::Wrists)
                | Some(InventoryType::Finger)
                | Some(InventoryType::Shield)
                | Some(InventoryType::Cloak)
                | Some(InventoryType::Holdable) => 2,
                Some(InventoryType::Weapon)
                | Some(InventoryType::WeaponMainhand)
                | Some(InventoryType::WeaponOffhand) => 3,
                Some(InventoryType::Ranged)
                | Some(InventoryType::Thrown)
                | Some(InventoryType::RangedRight) => 4,
                _ => return 0,
            };

        let Some(points) = self
            .rand_prop_points_store()
            .and_then(|store| store.get(u32::from(template.item_level)))
        else {
            return 0;
        };

        match <ItemQuality as num_traits::FromPrimitive>::from_i8(template.quality) {
            Some(ItemQuality::Uncommon) => points.good[prop_index] as i32,
            Some(ItemQuality::Rare) | Some(ItemQuality::Heirloom) => {
                points.superior[prop_index] as i32
            }
            Some(ItemQuality::Epic)
            | Some(ItemQuality::Legendary)
            | Some(ItemQuality::Artifact) => points.epic[prop_index] as i32,
            _ => 0,
        }
    }
}

pub(super) fn select_weighted_random_enchantment_like_cpp<R: Rng + ?Sized>(
    group: &[ItemRandomEnchantmentTemplateEntry],
    rng: &mut R,
) -> Option<u32> {
    let valid_rows = group
        .iter()
        .filter(|row| (0.000001..=100.0).contains(&row.chance))
        .collect::<Vec<_>>();
    let weights = valid_rows.iter().map(|row| row.chance).collect::<Vec<_>>();
    let distribution = WeightedIndex::new(weights).ok()?;
    Some(valid_rows[distribution.sample(rng)].enchantment_id)
}
