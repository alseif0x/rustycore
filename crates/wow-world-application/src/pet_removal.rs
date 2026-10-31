// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-side orchestration of the death/release pet removal.

mod live;
mod save;

use wow_world_core::session::SessionCatalogs;
use wow_world_entities::ResurrectionOperationCatalogs;
use wow_world_inventory::VendorSessionHostLikeCpp;
use wow_world_lifecycle::PlayerMoneyCommitCancellationFenceLikeCpp;
use wow_world_quest::{QuestItemStorageHostLikeCpp, QuestSessionHostLikeCpp};
use wow_world_spell::PlayerCastHostLikeCpp;

use crate::TrainerSessionHostLikeCpp;

/// C++ Player.cpp:20839–20962: resolve GetPet before refunding reagents and
/// retain stale GUIDs when the current Map has no matching typed Pet.
///
/// Live removal currently admits quiescent pets with complete empty aura
/// input and no summon-spell or group continuation. Other live pets fail
/// before mutation; this is not complete RemovePet support.
pub async fn remove_pet_not_in_slot_like_cpp<H>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> Result<(), &'static str>
where
    H: QuestItemStorageHostLikeCpp
        + TrainerSessionHostLikeCpp
        + PlayerCastHostLikeCpp
        + VendorSessionHostLikeCpp,
{
    let owner = host
        .player_guid()
        .ok_or("pet removal Player is unavailable")?;
    let handle = host.core().player_handle_like_cpp;
    let (in_world, pet_guid, temporary_number, old_spell, battleground_instance_id) = host
        .core()
        .with_owned_player_like_cpp(|player| {
            let pets = player.pet_lifecycle_state_like_cpp();
            (
                player.unit().world().object().is_in_world(),
                player.gameplay_state().pet_guid,
                pets.temporary_unsummoned_pet_number,
                pets.old_pet_spell,
                player.battleground_instance_id_like_cpp(),
            )
        })
        .ok_or("pet removal canonical Player is unavailable")?;

    let live_pet = if in_world && let Some(pet_guid) = pet_guid.filter(|guid| guid.is_pet()) {
        let key = host
            .core()
            .current_canonical_player_map_key_like_cpp()
            .ok_or("pet removal Player Map is unavailable")?;
        let manager = host
            .core()
            .canonical_map_manager
            .as_ref()
            .ok_or("pet removal Map manager is unavailable")?;
        let manager = manager.lock().map_err(|_| "pet removal Map lock failed")?;
        let map = manager
            .find_map(key.map_id, key.instance_id)
            .ok_or("pet removal Player Map disappeared")?
            .map();
        map.get_typed_pet(pet_guid)
            .map(|pet| (pet_guid, pet.is_removed()))
    } else {
        None
    };
    if let Some((pet_guid, removed)) = live_pet {
        return if removed {
            Ok(())
        } else {
            live::remove_quiescent_pet(host, pet_guid).await
        };
    }

    let tracker = QuestSessionHostLikeCpp::lifecycle(host)
        .durable_loot_money_persistence_tracker_like_cpp()
        .clone();
    if tracker.is_indeterminate_like_cpp() {
        return Err("pet removal is fenced by unresolved persistence");
    }
    let mut fence = PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(tracker);
    if temporary_number != 0 && battleground_instance_id.is_none() {
        let difficulty = host.core().current_map_difficulty_id_like_cpp();
        let reagents = pet_spell_reagents(host.hub_ref().catalogs, old_spell, difficulty)?;
        for (entry, count) in reagents {
            crate::return_pet_reagent_like_cpp(
                host,
                &mut fence,
                catalogs.item_guid_generator,
                catalogs.creature_spawns,
                entry,
                count,
            )
            .await?;
        }
        if host.core().player_handle_like_cpp != handle || host.player_guid() != Some(owner) {
            return Err("pet removal Player changed during reagent return");
        }
        host.core()
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .pet_lifecycle_state_mut_like_cpp()
                    .temporary_unsummoned_pet_number = 0;
            })
            .ok_or("pet removal Player disappeared after reagent return")?;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player
                .pet_lifecycle_state_mut_like_cpp()
                .stable
                .current_pet_index = None;
        })
        .ok_or("pet removal stable owner disappeared")?;
    fence.disarm_like_cpp();
    Ok(())
}

fn pet_spell_reagents(
    catalogs: &SessionCatalogs,
    spell_id: u32,
    difficulty: u8,
) -> Result<Vec<(u32, u32)>, &'static str> {
    if spell_id == 0 {
        return Ok(Vec::new());
    }
    let spells = catalogs
        .spell_store()
        .ok_or("pet summon spell catalog is unavailable")?;
    let server = catalogs
        .spell_catalogs
        .serverside_spell_store
        .as_ref()
        .ok_or("pet summon server spell catalog is unavailable")?;
    let has_key = spells.contains_spell_info_any_difficulty_like_cpp(spell_id)
        || (0..=u8::MAX).any(|key| {
            server
                .get_serverside_spell_like_cpp(spell_id, u32::from(key))
                .is_some()
        });
    let source = wow_data::spell::resolve_spell_catalog_value(
        difficulty,
        catalogs.difficulty_store().map(std::convert::AsRef::as_ref),
        has_key,
        |key| match (
            spells.contains_spell_info_exact_like_cpp(spell_id, key),
            server
                .get_serverside_spell_like_cpp(spell_id, u32::from(key))
                .is_some(),
        ) {
            (true, true) => Some(Err("pet summon SpellInfo source collision")),
            (true, false) => Some(Ok(true)),
            (false, true) => Some(Ok(false)),
            (false, false) => None,
        },
    )
    .map_err(|_| "pet summon difficulty resolution failed")?
    .transpose()?;
    // SpellInfo.cpp:1337 and SpellInfo.h:394–395 leave server-spell reagents
    // empty. A missing SpellInfo also skips the refund in Player::RemovePet.
    if source != Some(true) {
        return Ok(Vec::new());
    }
    let costs = spells
        .effective_cast_costs_like_cpp()
        .ok_or("pet summon reagent catalog is unavailable")?;
    Ok(costs
        .reagent_for_spell_like_cpp(spell_id as i32)
        .map(|row| {
            row.reagent
                .into_iter()
                .zip(row.reagent_count)
                .filter_map(|(entry, count)| {
                    (entry > 0).then_some((entry as u32, i32::from(count) as u32))
                })
                .collect()
        })
        .unwrap_or_default())
}
