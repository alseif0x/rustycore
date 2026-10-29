// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
// C++ anchors: src/server/game/Entities/Item/Item.cpp:383-437, 585-694;
// src/server/game/Entities/Player/Player.cpp:18411-18470.

use std::collections::HashSet;

use wow_constants::{ItemFieldFlags, ItemModifier};
use wow_core::ObjectGuid;
use wow_entities::Item;
use wow_persistence::{PlayerInventoryItemLoadPayload, PlayerInventoryItemLoadRowLikeCpp};

pub(super) fn row_save_inputs_known_like_cpp(
    row: &PlayerInventoryItemLoadRowLikeCpp,
    item_context_is_representable: bool,
    item_enchantments_are_representable: bool,
    item_gem_ids_are_representable: bool,
    item_template_is_known: bool,
    catalogs: &wow_world_core::session::SessionCatalogs,
) -> bool {
    row.complete_payload.is_some()
        && item_context_is_representable
        && item_enchantments_are_representable
        && item_gem_ids_are_representable
        && item_template_is_known
        && catalogs.items.effect_store.is_some()
        && match row.random_properties_id.cmp(&0) {
            std::cmp::Ordering::Greater => catalogs.items.random_properties_store.is_some(),
            std::cmp::Ordering::Less => catalogs.items.random_suffix_store.is_some(),
            std::cmp::Ordering::Equal => true,
        }
}

pub(super) fn effective_enchantment_catalog_is_known_like_cpp(
    enchantments: &[wow_packet::packets::update::ItemEnchantmentValuesUpdate;
         wow_entities::MAX_ENCHANTMENT_SLOT],
    catalogs: &wow_world_core::session::SessionCatalogs,
) -> bool {
    !enchantments.iter().any(|enchantment| enchantment.id != 0)
        || catalogs
            .spell_catalogs
            .spell_item_enchantment_store
            .is_some()
}

pub(super) fn register_tradeable_item_like_cpp<H: crate::CharacterLoginHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    item: &Item,
    loading_owner_is_current: bool,
) -> bool {
    if !loading_owner_is_current {
        return false;
    }
    let (_, mut hub) = host.split_inventory_mut();
    hub.core
        .mutate_canonical_player_like_cpp(|player| {
            if player.guid() != player_guid {
                return false;
            }
            player.add_tradeable_item(item);
            true
        })
        .unwrap_or(false)
}

pub(super) fn hydrate_complete_payload_like_cpp(
    item: &mut Item,
    payload: Option<PlayerInventoryItemLoadPayload>,
    original_flags: u32,
    realm_id: u16,
) -> Option<Option<String>> {
    let payload = payload?;
    let creator_guid = if payload.creator_guid == 0 {
        ObjectGuid::EMPTY
    } else if ItemFieldFlags::from_bits_retain(original_flags).contains(ItemFieldFlags::CHILD) {
        ObjectGuid::create_item(realm_id, payload.creator_guid as i64)
    } else {
        ObjectGuid::create_player(realm_id, payload.creator_guid as i64)
    };
    let gift_creator_guid = if payload.gift_creator_guid == 0 {
        ObjectGuid::EMPTY
    } else {
        ObjectGuid::create_player(realm_id, payload.gift_creator_guid as i64)
    };
    item.set_creator(creator_guid);
    item.set_gift_creator(gift_creator_guid);
    item.set_text(payload.text.unwrap_or_default());
    item.set_modifier(
        ItemModifier::BattlePetSpeciesId,
        payload.battle_pet_species_id,
    );
    item.set_modifier(
        ItemModifier::BattlePetBreedData,
        payload.battle_pet_breed_data,
    );
    item.set_modifier(
        ItemModifier::BattlePetLevel,
        u32::from(payload.battle_pet_level),
    );
    item.set_modifier(
        ItemModifier::BattlePetDisplayId,
        payload.battle_pet_display_id,
    );

    const APPEARANCE_MODIFIERS: [ItemModifier; 6] = [
        ItemModifier::TransmogAppearanceAllSpecs,
        ItemModifier::TransmogAppearanceSpec1,
        ItemModifier::TransmogAppearanceSpec2,
        ItemModifier::TransmogAppearanceSpec3,
        ItemModifier::TransmogAppearanceSpec4,
        ItemModifier::TransmogAppearanceSpec5,
    ];
    const ILLUSION_MODIFIERS: [ItemModifier; 6] = [
        ItemModifier::EnchantIllusionAllSpecs,
        ItemModifier::EnchantIllusionSpec1,
        ItemModifier::EnchantIllusionSpec2,
        ItemModifier::EnchantIllusionSpec3,
        ItemModifier::EnchantIllusionSpec4,
        ItemModifier::EnchantIllusionSpec5,
    ];
    const SECONDARY_APPEARANCE_MODIFIERS: [ItemModifier; 6] = [
        ItemModifier::TransmogSecondaryAppearanceAllSpecs,
        ItemModifier::TransmogSecondaryAppearanceSpec1,
        ItemModifier::TransmogSecondaryAppearanceSpec2,
        ItemModifier::TransmogSecondaryAppearanceSpec3,
        ItemModifier::TransmogSecondaryAppearanceSpec4,
        ItemModifier::TransmogSecondaryAppearanceSpec5,
    ];
    for (modifier, value) in APPEARANCE_MODIFIERS
        .into_iter()
        .zip(payload.transmog_appearance)
        .chain(
            ILLUSION_MODIFIERS
                .into_iter()
                .zip(payload.transmog_enchant_illusion),
        )
        .chain(
            SECONDARY_APPEARANCE_MODIFIERS
                .into_iter()
                .zip(payload.transmog_secondary_appearance),
        )
    {
        item.set_modifier(modifier, value);
    }

    Some(payload.soulbound_trade_allowed_players)
}

pub(super) fn apply_bop_trade_metadata_like_cpp(
    item: &mut Item,
    allowed_players: Option<Option<String>>,
    original_flags: u32,
    template_max_stack_size: Option<u32>,
    realm_id: u16,
    register_tradeable: impl FnOnce(&Item) -> bool,
) -> bool {
    let original_flags = ItemFieldFlags::from_bits_retain(original_flags);
    if original_flags.contains(ItemFieldFlags::REFUNDABLE)
        || !original_flags.contains(ItemFieldFlags::BOP_TRADEABLE)
    {
        return true;
    }

    let Some(allowed_players) = allowed_players else {
        return true;
    };
    let Some(allowed_players) = allowed_players else {
        // Player.cpp's missing joined row only clears this field flag.  No new
        // database repair is represented for the existing allowlist state.
        item.remove_item_flag(ItemFieldFlags::BOP_TRADEABLE);
        return true;
    };
    let Some(template_max_stack_size) = template_max_stack_size else {
        return false;
    };

    let allowed_players = allowed_players
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(|token| ObjectGuid::create_player(realm_id, token.parse::<u64>().unwrap_or(0) as i64))
        .collect::<HashSet<_>>();
    if allowed_players.len() > 1 && template_max_stack_size == 1 && item.is_soul_bound() {
        item.set_soulbound_tradeable(allowed_players);
        register_tradeable(item)
    } else {
        item.clear_soulbound_tradeable();
        true
    }
}
