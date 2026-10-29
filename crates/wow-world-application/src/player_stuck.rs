// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Cross-domain C++ `Spell::EffectStuck` orchestration.

use wow_core::ObjectGuid;
use wow_entities::PlayerDeathOperationContext;
use wow_map::{MapKey, PlayerHandle};
use wow_world_entities::{
    CombatSessionHostLikeCpp, CorpseHostLikeCpp, CorpseLifecycleCatalogsLikeCpp,
    ResurrectionOperationCatalogs,
};
use wow_world_inventory::ItemSessionHostLikeCpp;
use wow_world_quest::QuestItemStorageHostLikeCpp;
use wow_world_spell::PlayerCastHostLikeCpp;

const HEARTHSTONE_SPELL_ID_LIKE_CPP: i32 = 8690;

#[derive(Clone, Copy)]
struct CurrentPlayerMapLikeCpp {
    handle: PlayerHandle,
    key: MapKey,
    residence_revision: u64,
    difficulty_id: u8,
}

fn current_player_map_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    caster_guid: ObjectGuid,
) -> Result<CurrentPlayerMapLikeCpp, &'static str> {
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("EffectStuck canonical Player handle is unavailable")?;
    if host.player_guid() != Some(handle.guid()) || caster_guid != handle.guid() {
        return Err("EffectStuck Player caster does not match the current Player incarnation");
    }

    let manager = host
        .core()
        .canonical_map_manager
        .as_ref()
        .ok_or("EffectStuck canonical MapManager is unavailable")?
        .clone();
    let manager = manager
        .lock()
        .map_err(|_| "EffectStuck canonical MapManager is unavailable")?;
    let (key, residence_revision) = manager
        .player_active_residence_revision_like_cpp(handle)
        .ok_or("EffectStuck Player has no active Map residence")?;
    let managed = manager
        .find_map(key.map_id, key.instance_id)
        .ok_or("EffectStuck Player Map is unavailable")?;
    let current = manager
        .with_player_like_cpp(handle, |player| {
            let world = player.unit().world();
            player.guid() == caster_guid
                && world.object().is_in_world()
                && world.has_current_map()
                && MapKey::new(world.map_id(), world.instance_id()) == key
        })
        .unwrap_or(false);
    if !current {
        return Err("EffectStuck canonical Player is not in its active Map");
    }
    Ok(CurrentPlayerMapLikeCpp {
        handle,
        key,
        residence_revision,
        difficulty_id: managed.difficulty(),
    })
}

fn owner_is_current_in_residence_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    owner: CurrentPlayerMapLikeCpp,
) -> bool {
    if host.core().player_handle_like_cpp != Some(owner.handle)
        || host.player_guid() != Some(owner.handle.guid())
    {
        return false;
    }
    let Some(manager) = host.core().canonical_map_manager.as_ref() else {
        return false;
    };
    let Ok(manager) = manager.lock() else {
        return false;
    };
    manager.player_active_residence_revision_like_cpp(owner.handle)
        == Some((owner.key, owner.residence_revision))
        && manager
            .with_player_like_cpp(owner.handle, |player| {
                let world = player.unit().world();
                player.guid() == owner.handle.guid()
                    && world.object().is_in_world()
                    && world.has_current_map()
                    && MapKey::new(world.map_id(), world.instance_id()) == owner.key
            })
            .unwrap_or(false)
}

fn player_incarnation_is_current_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    owner: CurrentPlayerMapLikeCpp,
) -> bool {
    host.core().player_handle_like_cpp == Some(owner.handle)
        && host.player_guid() == Some(owner.handle.guid())
        && host
            .core()
            .with_owned_player_like_cpp(|player| player.guid() == owner.handle.guid())
            .unwrap_or(false)
}

fn has_hearthstone_cooldown_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    difficulty_id: u8,
) -> Result<bool, &'static str> {
    let hub = host.hub_ref();
    let spell_store = hub
        .catalogs
        .spell_store()
        .ok_or("EffectStuck SpellInfo catalog is unavailable")?;
    let difficulties = hub
        .catalogs
        .difficulty_store()
        .ok_or("EffectStuck Difficulty catalog is unavailable")?;
    let selected_difficulty = wow_data::spell::resolve_spell_info_key_difficulty(
        spell_store,
        HEARTHSTONE_SPELL_ID_LIKE_CPP as u32,
        difficulty_id,
        Some(difficulties),
    )
    .map_err(|_| "EffectStuck Hearthstone SpellInfo difficulty is unavailable")?
    .ok_or("EffectStuck Hearthstone SpellInfo key is unavailable")?;
    if spell_store
        .spell_info_for_exact_key_like_cpp(
            HEARTHSTONE_SPELL_ID_LIKE_CPP,
            selected_difficulty,
            Some(difficulties),
        )
        .map_err(|_| "EffectStuck Hearthstone SpellInfo payload is unavailable")?
        .is_none()
    {
        return Err("EffectStuck Hearthstone SpellInfo payload is unavailable");
    }
    let serverside_spells = hub
        .catalogs
        .spell_catalogs
        .serverside_spell_store
        .as_deref()
        .ok_or("EffectStuck serverside spell catalog is unavailable")?;
    let class_options = hub.catalogs.spell_class_options_store().map(AsRef::as_ref);

    hub.core
        .with_owned_player_like_cpp(|player| {
            let history = &player.unit().subsystems().spells.history;
            if !history.cooldowns_loaded {
                return Err("EffectStuck native SpellHistory cooldown ledger is unavailable");
            }
            let ignores_cooldown = wow_world_spell::active_spell_cooldown_aura_affects_like_cpp(
                player,
                HEARTHSTONE_SPELL_ID_LIKE_CPP as u32,
                difficulty_id,
                spell_store,
                serverside_spells,
                class_options,
                Some(difficulties),
            )
            .map_err(|_| "EffectStuck active Player Aura metadata is unavailable")?;
            if ignores_cooldown {
                return Ok(false);
            }
            if history
                .cooldown(HEARTHSTONE_SPELL_ID_LIKE_CPP as u32)
                .is_some()
            {
                return Ok(true);
            }
            let category_id = spell_store
                .category_id_at_difficulty(
                    HEARTHSTONE_SPELL_ID_LIKE_CPP as u32,
                    difficulty_id,
                    Some(difficulties),
                )
                .map_err(|_| "EffectStuck Hearthstone category metadata is unavailable")?;
            Ok(history.has_cooldown_membership_like_cpp(
                HEARTHSTONE_SPELL_ID_LIKE_CPP as u32,
                category_id,
            ))
        })
        .ok_or("EffectStuck canonical Player SpellHistory is unavailable")?
}

async fn kill_self_like_cpp<H>(
    host: &mut H,
    owner: CurrentPlayerMapLikeCpp,
    creature_spawn_catalogs: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Result<(), &'static str>
where
    H: PlayerCastHostLikeCpp
        + QuestItemStorageHostLikeCpp
        + ItemSessionHostLikeCpp
        + CorpseHostLikeCpp
        + CombatSessionHostLikeCpp
        + crate::TrainerSessionHostLikeCpp
        + Send,
{
    let player_guid = owner.handle.guid();
    let (operation, pvp_death) = {
        let manager = host
            .core()
            .canonical_map_manager
            .as_ref()
            .ok_or("EffectStuck native death MapManager is unavailable")?
            .clone();
        let mut manager = manager
            .lock()
            .map_err(|_| "EffectStuck native death MapManager is unavailable")?;
        if manager.player_active_residence_revision_like_cpp(owner.handle)
            != Some((owner.key, owner.residence_revision))
        {
            return Err("EffectStuck Player residence changed before native KillSelf");
        }
        let (pvp_death, in_world) = {
            let map = manager
                .find_map(owner.key.map_id, owner.key.instance_id)
                .ok_or("EffectStuck native death Map disappeared")?
                .map();
            let player = map
                .get_typed_player(player_guid)
                .ok_or("EffectStuck native death Player left its Map")?;
            let control = &player.unit().subsystems().control;
            let pvp_death = match control
                .charmer_guid
                .or(control.owner_guid)
                .filter(|guid| guid.is_player())
            {
                Some(owner_guid) => map.get_typed_player(owner_guid).is_some(),
                None => true,
            };
            let in_world = manager
                .with_player_like_cpp(owner.handle, |current| {
                    let world = current.unit().world();
                    current.guid() == player_guid
                        && world.object().is_in_world()
                        && world.has_current_map()
                        && MapKey::new(world.map_id(), world.instance_id()) == owner.key
                })
                .unwrap_or(false);
            (pvp_death, in_world)
        };
        if !in_world {
            return Err("EffectStuck native KillSelf Player is not in its active Map");
        }
        let operation = manager
            .with_player_mut_like_cpp(owner.handle, |player| {
                if player.guid() != player_guid
                    || player.has_unresolved_native_death_operation()
                    || player.unit().data().health == 0
                    || !player.unit().is_alive()
                {
                    return None;
                }
                player.begin_native_death_operation(PlayerDeathOperationContext::new(
                    player_guid,
                    true,
                ))
            })
            .ok_or("EffectStuck native death Player incarnation is unavailable")?
            .ok_or("EffectStuck native KillSelf could not admit death")?;
        (operation, pvp_death)
    };

    crate::finish_native_player_death_like_cpp(
        host,
        operation,
        pvp_death,
        ResurrectionOperationCatalogs {
            item_guid_generator,
            creature_spawns: creature_spawn_catalogs,
            graveyards: None,
        },
    )
    .await?;
    if !owner_is_current_in_residence_like_cpp(host, owner) {
        return Err("EffectStuck native KillSelf owner or Map residence changed during death");
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.retire_completed_native_death_operation(operation)
        })
        .filter(|retired| *retired)
        .ok_or("EffectStuck native KillSelf completion fence could not retire")?;
    Ok(())
}

/// C++ `Spell::EffectStuck` (`SpellEffects.cpp:3263-3303`).
pub async fn apply_stuck_effect_like_cpp<H>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    caster_guid: ObjectGuid,
) -> Result<(), &'static str>
where
    H: PlayerCastHostLikeCpp
        + QuestItemStorageHostLikeCpp
        + ItemSessionHostLikeCpp
        + CorpseHostLikeCpp
        + CombatSessionHostLikeCpp
        + crate::TrainerSessionHostLikeCpp
        + Send,
{
    if !host
        .hub_ref()
        .config
        .represented_cast_unstuck_enabled_like_cpp
        || !caster_guid.is_player()
    {
        return Ok(());
    }

    let owner = current_player_map_like_cpp(host, caster_guid)?;
    let (in_flight, alive, death_timer, homebind) = host
        .core()
        .with_owned_player_like_cpp(|player| {
            (
                player.taxi_state_like_cpp().is_in_flight_like_cpp(),
                player.unit().is_alive(),
                player
                    .resurrection_state_like_cpp()
                    .death_timer_remaining_ms,
                player.gameplay_state().homebind,
            )
        })
        .ok_or("EffectStuck canonical Player state is unavailable")?;
    if in_flight {
        return Ok(());
    }
    if !alive {
        if death_timer != 0 {
            return Ok(());
        }
        let graveyards = host
            .hub_ref()
            .catalogs
            .graveyard_store()
            .cloned()
            .ok_or("EffectStuck graveyard catalog is unavailable")?;
        wow_world_entities::repop_at_graveyard_like_cpp(
            host,
            CorpseLifecycleCatalogsLikeCpp {
                item_guid_generator,
                creature_spawns: creature_spawn_catalogs,
                graveyards: graveyards.as_ref(),
            },
        )
        .await;
        if !player_incarnation_is_current_like_cpp(host, owner) {
            return Err("EffectStuck repop Player incarnation changed");
        }
        return Ok(());
    }

    if has_hearthstone_cooldown_like_cpp(host, owner.difficulty_id)? {
        return kill_self_like_cpp(host, owner, creature_spawn_catalogs, item_guid_generator).await;
    }

    let homebind = homebind.ok_or("EffectStuck Player homebind is unavailable")?;
    // C++ casts Hearthstone with TRIGGERED_FULL_MASK here. Spell::SendSpellCooldown
    // reaches SpellHistory::HandleCooldowns, which ignores that full mask
    // (SpellHistory.cpp:252-257); the temporary cast adds no Hearthstone cooldown.
    host.teleport_to_with_options_like_cpp(
        ResurrectionOperationCatalogs {
            item_guid_generator,
            creature_spawns: creature_spawn_catalogs,
            graveyards: None,
        },
        homebind.map_id,
        homebind.position,
        wow_world_entities::TELE_TO_SPELL_LIKE_CPP,
    )
    .await;
    if !player_incarnation_is_current_like_cpp(host, owner) {
        return Err("EffectStuck home teleport Player incarnation changed");
    }
    Ok(())
}
