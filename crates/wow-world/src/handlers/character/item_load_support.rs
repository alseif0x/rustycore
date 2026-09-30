// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-login item restoration helpers.

use super::*;

pub(in crate::handlers::character) fn loaded_item_random_properties_like_cpp(
    random_properties_id: i32,
    random_properties_seed: i32,
    item_random_properties_store: Option<&wow_data::ItemRandomPropertiesStore>,
    item_random_suffix_store: Option<&wow_data::ItemRandomSuffixStore>,
) -> Option<LoadedItemRandomPropertiesLikeCpp> {
    if random_properties_id > 0 {
        item_random_properties_store?
            .get(random_properties_id as u32)
            .map(|_| LoadedItemRandomPropertiesLikeCpp {
                id: random_properties_id,
                seed: 0,
            })
    } else if random_properties_id < 0 {
        item_random_suffix_store?
            .get(random_properties_id.unsigned_abs())
            .map(|_| LoadedItemRandomPropertiesLikeCpp {
                id: random_properties_id,
                seed: random_properties_seed,
            })
    } else {
        None
    }
}

pub(in crate::handlers::character) fn apply_loaded_item_instance_fields_like_cpp(
    item: &mut wow_entities::Item,
    enchantments: &[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT],
    random_properties: Option<LoadedItemRandomPropertiesLikeCpp>,
) {
    if let Some(random_properties) = random_properties {
        item.set_random_properties_id(random_properties.id);
        item.set_property_seed(random_properties.seed);
    }

    for (slot_index, enchantment) in enchantments.iter().enumerate() {
        let Some(slot) = <EnchantmentSlot as num_traits::FromPrimitive>::from_usize(slot_index)
        else {
            continue;
        };
        item.set_enchantment(
            slot,
            enchantment.id,
            enchantment.duration,
            enchantment.charges,
        );
    }
}

pub(in crate::handlers::character) fn loaded_item_spell_charges_like_cpp(
    charges: &str,
    effect_count: usize,
) -> [i32; wow_entities::MAX_ITEM_SPELLS] {
    let mut values = [0; wow_entities::MAX_ITEM_SPELLS];
    for (target, token) in values
        .iter_mut()
        .take(effect_count.min(wow_entities::MAX_ITEM_SPELLS))
        .zip(charges.split_whitespace())
    {
        *target = token.parse::<i32>().unwrap_or(0);
    }
    values
}

pub(in crate::handlers::character) fn apply_loaded_item_storage_mutable_fields_like_cpp(
    item: &mut wow_entities::Item,
    stored_expiration: u32,
    template_expiration: u32,
    charges: &str,
    effect_count: usize,
) -> bool {
    let expiration_needs_save = (template_expiration == 0) != (stored_expiration == 0);
    item.set_expiration(if expiration_needs_save {
        template_expiration
    } else {
        stored_expiration
    });
    for (index, charge) in loaded_item_spell_charges_like_cpp(charges, effect_count)
        .into_iter()
        .enumerate()
    {
        item.set_spell_charges(index, charge);
    }
    expiration_needs_save
}

pub(in crate::handlers::character) fn loaded_item_slot_applies_equipped_enchantments_like_cpp(
    slot: u8,
) -> bool {
    slot < INVENTORY_SLOT_BAG_END
}

pub(in crate::handlers::character) fn loaded_socketed_gems_like_cpp(
    fields: [(i32, String, u8); 3],
) -> Vec<SocketedGem> {
    let Some(last_populated_socket) = fields.iter().rposition(|(item_id, _, _)| *item_id > 0)
    else {
        return Vec::new();
    };
    fields
        .into_iter()
        .take(last_populated_socket + 1)
        .map(|(item_id, bonuses, context)| {
            if item_id <= 0 {
                return SocketedGem::default();
            }
            SocketedGem {
                item_id,
                context,
                bonus_list_ids: bonuses
                    .split_whitespace()
                    .filter_map(|bonus| bonus.parse::<u16>().ok())
                    .take(16)
                    .collect(),
            }
        })
        .collect()
}

pub(in crate::handlers::character) fn loaded_socketed_gem_create_updates_like_cpp(
    gems: &[SocketedGem],
) -> Vec<wow_packet::packets::update::SocketedGemValuesUpdate> {
    gems.iter()
        .map(|gem| {
            let mut bonus_list_ids = [0; 16];
            for (target, source) in bonus_list_ids.iter_mut().zip(&gem.bonus_list_ids) {
                *target = *source;
            }
            wow_packet::packets::update::SocketedGemValuesUpdate {
                socketed_gem_mask: 0x000F_FFFF,
                item_id: gem.item_id,
                context: gem.context,
                bonus_list_ids,
            }
        })
        .collect()
}

pub(in crate::handlers::character) fn loaded_item_effective_enchantments_like_cpp(
    loaded_enchantments: Option<&[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]>,
    random_properties_id: i32,
    item_random_properties_store: Option<&wow_data::ItemRandomPropertiesStore>,
    item_random_suffix_store: Option<&wow_data::ItemRandomSuffixStore>,
) -> [ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT] {
    let mut values = [ItemEnchantmentValuesUpdate::default(); wow_entities::MAX_ENCHANTMENT_SLOT];

    if random_properties_id > 0 {
        if let Some(entry) =
            item_random_properties_store.and_then(|store| store.get(random_properties_id as u32))
        {
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                values[EnchantmentSlot::Property2 as usize + offset].id =
                    i32::from(*enchantment_id);
            }
        }
    } else if random_properties_id < 0 {
        if let Some(entry) = item_random_suffix_store
            .and_then(|store| store.get(random_properties_id.unsigned_abs()))
        {
            for (offset, enchantment_id) in entry.enchantments.iter().take(3).enumerate() {
                values[EnchantmentSlot::Property0 as usize + offset].id =
                    i32::from(*enchantment_id);
            }
        }
    }

    // C++ Item::LoadFromDB synthesizes random-property slots first, then a
    // correctly sized persisted enchantment array overwrites every slot. A
    // valid all-zero array is therefore authoritative; only a missing or
    // malformed array keeps the synthesized fallback.
    if let Some(loaded_enchantments) = loaded_enchantments {
        values = *loaded_enchantments;
    }

    values
}

pub(in crate::handlers::character) fn loaded_item_enchantments_like_cpp(
    enchantments: &str,
) -> Option<[ItemEnchantmentValuesUpdate; wow_entities::MAX_ENCHANTMENT_SLOT]> {
    let mut values = [ItemEnchantmentValuesUpdate::default(); wow_entities::MAX_ENCHANTMENT_SLOT];
    let tokens: Vec<&str> = enchantments.split_whitespace().collect();
    if tokens.len() != wow_entities::MAX_ENCHANTMENT_SLOT * ITEM_ENCHANTMENT_DB_FIELDS {
        return None;
    }

    for slot_index in 0..wow_entities::MAX_ENCHANTMENT_SLOT {
        let base = slot_index * ITEM_ENCHANTMENT_DB_FIELDS;
        values[slot_index] = ItemEnchantmentValuesUpdate {
            item_enchantment_mask: 0,
            id: tokens[base].parse::<i32>().unwrap_or(0),
            duration: tokens[base + 1].parse::<u32>().unwrap_or(0),
            charges: tokens[base + 2].parse::<i16>().unwrap_or(0),
            field_a: 0,
            field_b: 0,
        };
    }

    Some(values)
}
