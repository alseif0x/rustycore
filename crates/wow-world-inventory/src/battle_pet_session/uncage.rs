// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The battle-pet spell effects (#1363 slice 7b): C++
//! `Spell::EffectCreateBattlePet` / `SPELL_EFFECT_UNCAGE_BATTLEPET` with its
//! `Player::DestroyItem` phase, and the grant-experience and grant-level
//! effects.

use tracing::warn;
use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_entities::{
    Item, PlayerInventoryItem as InventoryItem, SpellCastBattlePetItemModifiersLikeCpp,
    SpellCastMetadata,
};
use wow_world_core::battle_pet_account::BattlePetAddRequestKeyLikeCpp;

use super::durable::{
    battle_pet_grant_experience_durable_like_cpp, battle_pet_grant_level_durable_like_cpp,
    battle_pet_try_add_pet_durable_like_cpp,
};
use super::journal::{
    battle_pet_add_request_committed_like_cpp, battle_pet_has_max_pet_count_like_cpp,
    battle_pet_max_pet_level_like_cpp, battle_pet_species_entry_like_cpp,
};
use super::{
    BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP, BattlePetSessionHostLikeCpp,
    RepresentedBattlePetXpSourceLikeCpp,
};

fn uncage_cast_item_still_matches_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    cast_item_entry: u32,
    modifiers: SpellCastBattlePetItemModifiersLikeCpp,
) -> Option<(u8, u8, InventoryItem)> {
    let (state, hub) = host.split_inventory_ref();
    state.uncage_cast_item_still_matches_like_cpp(hub, cast_item_entry, modifiers)
}

fn get_inventory_item_by_guid_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    item_guid: ObjectGuid,
) -> Option<(u8, u8, InventoryItem)> {
    let (state, hub) = host.split_inventory_ref();
    state.get_inventory_item_by_guid_like_cpp(hub, item_guid)
}

fn resolved_inventory_item_object_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    guid: ObjectGuid,
) -> Option<Item> {
    let (state, hub) = host.split_inventory_ref();
    state.resolved_inventory_item_object_like_cpp(hub, guid)
}

/// C++ `Spell::EffectCreateBattlePet` / `SPELL_EFFECT_UNCAGE_BATTLEPET`.
/// The Login DB pet/receipt commit precedes the Character DB `DestroyItem`
/// phase so an interrupted cross-database operation can resume idempotently.
pub async fn apply_uncage_battle_pet_effect_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    cast_id: ObjectGuid,
    metadata: SpellCastMetadata,
) {
    let Some(modifiers) = metadata.cast_item_battle_pet_modifiers else {
        return;
    };

    let Some(request_key) = BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(
        modifiers.source_item_guid.to_raw_bytes(),
    ) else {
        warn!(
            account = host.core().account_id,
            species = modifiers.species_id,
            "Durable battle-pet uncage lacks a stable source-item identity"
        );
        return;
    };
    let request_already_committed =
        match battle_pet_add_request_committed_like_cpp(host, request_key).await {
            Ok(committed) => committed,
            Err(error) => {
                warn!(
                    account = host.core().account_id,
                    species = modifiers.species_id,
                    ?error,
                    "Failed to reconcile a durable battle-pet uncage request"
                );
                return;
            }
        };

    if request_already_committed {
        if metadata.cast_item_entry.is_some() {
            let _ =
                destroy_uncaged_battle_pet_item_durable_like_cpp(host, modifiers.source_item_guid)
                    .await;
        }
        return;
    }

    if let Some(cast_item_entry) = metadata.cast_item_entry
        && uncage_cast_item_still_matches_like_cpp(host, cast_item_entry, modifiers).is_none()
    {
        warn!(
            account = host.core().account_id,
            item_guid = modifiers.source_item_guid.counter(),
            cast_item_entry,
            "Battle-pet uncage source item disappeared or changed before effect execution"
        );
        return;
    }

    let Some(species_entry) = battle_pet_species_entry_like_cpp(host, modifiers.species_id) else {
        return;
    };

    let creature_id = u32::try_from(species_entry.creature_id).unwrap_or_default();
    let Some(max_pet_level) = battle_pet_max_pet_level_like_cpp(host) else {
        return;
    };
    if max_pet_level < modifiers.level {
        host.hub_mut().battle_pet_send_error_like_cpp(
            wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::TooHighLevelToUncage,
            creature_id,
        );
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: wow_packet::packets::spell::SpellCastVisual::default(),
            reason: SpellCastResult::CantAddBattlePet as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        return;
    }

    let Some(has_max_pet_count) =
        battle_pet_has_max_pet_count_like_cpp(host, modifiers.species_id, host.player_guid())
    else {
        return;
    };
    if has_max_pet_count {
        host.hub_mut().battle_pet_send_error_like_cpp(
            wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::CantHaveMorePetsOfType,
            creature_id,
        );
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: wow_packet::packets::spell::SpellCastVisual::default(),
            reason: SpellCastResult::CantAddBattlePet as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        return;
    }

    let breed = (modifiers.breed_data & 0x00FF_FFFF) as u16;
    let quality = ((modifiers.breed_data >> 24) & 0xFF) as u8;
    if let Err(error) = battle_pet_try_add_pet_durable_like_cpp(
        host,
        request_key.as_bytes(),
        modifiers.species_id,
        modifiers.display_id,
        breed,
        quality,
        modifiers.level,
    )
    .await
    {
        warn!(
            account = host.core().account_id,
            species = modifiers.species_id,
            ?error,
            "Durable battle-pet uncage add was rejected"
        );
        host.hub_mut().battle_pet_send_error_like_cpp(
            wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::CantHaveMorePetsOfType,
            creature_id,
        );
        host.send_packet(&wow_packet::packets::spell::CastFailed {
            cast_id,
            spell_id,
            visual: wow_packet::packets::spell::SpellCastVisual::default(),
            reason: SpellCastResult::CantAddBattlePet as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        return;
    }
    if let (Some(player_guid), Some(player_position)) = (
        host.player_guid(),
        host.hub_ref().player_position_like_cpp(),
    ) {
        host.send_packet(&wow_packet::packets::spell::PlaySpellVisual::self_target(
            player_guid,
            player_position,
            BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP,
        ));
    }
    if metadata.cast_item_entry.is_some() {
        let _ = destroy_uncaged_battle_pet_item_durable_like_cpp(host, modifiers.source_item_guid)
            .await;
    }
}

/// C++ `Spell::EffectGrantBattlePetExperience`.
pub async fn apply_grant_battle_pet_experience_effect_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    metadata: SpellCastMetadata,
    xp: u16,
) {
    let Some(pet_guid) = metadata.unit_target_battle_pet_companion_guid else {
        return;
    };

    let _ = battle_pet_grant_experience_durable_like_cpp(
        host,
        pet_guid,
        xp,
        RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
        1.0,
    )
    .await;
}

/// C++ `Spell::EffectGrantBattlePetLevel`.
pub async fn apply_grant_battle_pet_level_effect_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    metadata: SpellCastMetadata,
    granted_levels: u16,
) {
    let Some(pet_guid) = metadata.unit_target_battle_pet_companion_guid else {
        return;
    };

    let _ = battle_pet_grant_level_durable_like_cpp(host, pet_guid, granted_levels).await;
}

/// Finish C++ `Player::DestroyItem` after the Login DB pet/receipt commit.
///
/// The receipt makes this second database phase retryable: a reconnect may
/// observe the already-created pet and finish deleting the cage without
/// publishing another pet or visual.
pub async fn destroy_uncaged_battle_pet_item_durable_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    source_item_guid: ObjectGuid,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let player_db_guid = player_guid.counter() as u64;
    let item_db_guid = source_item_guid.counter() as u64;

    let (owner_guid, inventory_linked) = match host
        .uncage_item_state_like_cpp(player_db_guid, item_db_guid)
        .await
    {
        wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state) => {
            (state.owner_guid, state.inventory_linked)
        }
        wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                item_guid = item_db_guid,
                %reason,
                "Failed to inspect the uncaged battle-pet item"
            );
            return false;
        }
    };
    if owner_guid.is_some_and(|owner_guid| owner_guid != player_db_guid) {
        warn!(
            account = host.core().account_id,
            item_guid = item_db_guid,
            durable_owner = ?owner_guid,
            player_guid = player_db_guid,
            "Refusing to destroy an uncaged item owned by another character"
        );
        return false;
    }
    let Some((bag, slot, item)) = get_inventory_item_by_guid_like_cpp(host, source_item_guid)
    else {
        return owner_guid.is_none() && !inventory_linked;
    };
    let runtime_item = resolved_inventory_item_object_like_cpp(host, source_item_guid);
    host.destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
        bag,
        slot,
        item,
        runtime_item,
        owner_guid.map(|_| player_db_guid),
        "BattlePetUncage",
    )
    .await
}
