// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented player-state spell effects (#1363 slice 12b2), moved from
//! the World `session/spell_effects/{effect_apply,effects_player,effects_progress}.rs`:
//! C++ `Spell::EffectStuck`, `Spell::EffectSelfResurrect` and
//! `Spell::EffectProficiency` (`SpellEffects.cpp`). The former World
//! `#[cfg(test)]` fixture steps run only for the World test session, which
//! alone returns its fixtures from `test_fixtures_like_cpp`.

use std::time::Instant;

use wow_constants::{ItemClass, PowerType};
use wow_packet::packets::misc::SetProficiency;

use super::SpellExecutionHostLikeCpp;
use crate::effect_session::represented_homebind_like_cpp;

/// C++ `Spell::EffectStuck` (`SpellEffects.cpp:3265-3303`).
pub async fn apply_stuck_effect_like_cpp<H: SpellExecutionHostLikeCpp>(host: &mut H) {
    const HEARTHSTONE_SPELL_ID_LIKE_CPP: i32 = 8690;

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if !host
        .hub_ref()
        .config
        .represented_cast_unstuck_enabled_like_cpp
    {
        return;
    }
    if host.hub_ref().resolved_is_in_taxi_flight_like_cpp() != Some(false) {
        return;
    }

    match host.hub_ref().resolved_player_is_alive_like_cpp() {
        None => return,
        Some(false) => {
            let Some(resurrection) = crate::player_resurrection_state_snapshot_like_cpp(host)
            else {
                return;
            };
            if resurrection.death_timer_remaining_ms == 0 {
                host.set_player_ghost_flag_like_cpp(true);
                #[cfg(any(test, feature = "test-fixtures"))]
                if let Some(fixtures) = host.test_fixtures_like_cpp() {
                    fixtures.combat.represented_repop_at_graveyard_count = fixtures
                        .combat
                        .represented_repop_at_graveyard_count
                        .saturating_add(1);
                }
            }
            return;
        }
        Some(true) => {}
    }

    let Some(hearthstone_last_cast) = ({
        let (state, hub) = host.split_spell_state_ref();
        state.spell_last_cast_time_like_cpp(hub, HEARTHSTONE_SPELL_ID_LIKE_CPP)
    }) else {
        return;
    };
    if hearthstone_last_cast.is_some() {
        host.set_player_alive_like_cpp(false);
        let values_update = host.core().mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_health(0);
            player.values_update(true)
        });
        if let Some(values_update) = values_update {
            host.core()
                .send_player_values_update_like_cpp(&values_update);
        } else {
            host.core()
                .send_player_health_values_update_like_cpp(player_guid, 0);
        }
        return;
    }

    if let Some(homebind) = represented_homebind_like_cpp(host) {
        host.teleport_to(homebind.map_id, homebind.position).await;
    }

    let Some(hearthstone_cooldown_ms) = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_store
        .as_deref()
        .and_then(|store| store.get(HEARTHSTONE_SPELL_ID_LIKE_CPP))
        .map(|spell_info| spell_info.recovery_time_ms.max(spell_info.cooldown_ms))
    else {
        return;
    };
    if host
        .mutate_cast_execution_like_cpp(|state| {
            state
                .last_cast_time_per_spell
                .insert(HEARTHSTONE_SPELL_ID_LIKE_CPP, Instant::now());
        })
        .is_none()
    {
        return;
    }
    {
        let (s, mut h) = host.split_spell_state_mut();
        s.record_cast_character_spell_cooldown_like_cpp(
            &mut h,
            HEARTHSTONE_SPELL_ID_LIKE_CPP,
            hearthstone_cooldown_ms,
        )
    };
    host.send_packet(&wow_packet::packets::spell::CooldownEvent {
        spell_id: HEARTHSTONE_SPELL_ID_LIKE_CPP,
        is_pet: false,
    });
}

pub async fn apply_self_resurrect_effect_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    caster_guid: wow_core::ObjectGuid,
    damage: i32,
    misc_value: i32,
) -> Result<(), &'static str> {
    if !caster_guid.is_player() {
        return Ok(());
    }
    let player_guid = host
        .player_guid()
        .ok_or("Canonical Player identity is unavailable for self-resurrection")?;
    if player_guid != caster_guid {
        return Err("Resolved self-resurrection caster has no canonical Player owner");
    }
    let player_state = host
        .core()
        .with_owned_player_like_cpp(|player| {
            (player.guid() == caster_guid).then(|| {
                (
                    player.unit().world().object().is_in_world(),
                    player.unit().is_alive(),
                    player.unit().data().max_health,
                    player.get_max_power(PowerType::Mana),
                )
            })
        })
        .flatten()
        .ok_or("Canonical Player state is unavailable for self-resurrection")?;
    if !player_state.0 || player_state.1 {
        return Ok(());
    }

    // Source computes both locals before ResurrectPlayer mutates Player state.
    let (health, mana) =
        self_resurrection_values_like_cpp(damage, misc_value, player_state.2, player_state.3)?;
    host.self_resurrect_player_like_cpp(item_guid_generator, creature_spawn_catalogs, health, mana)
        .await
}

fn self_resurrection_values_like_cpp(
    damage: i32,
    misc_value: i32,
    max_health: u64,
    max_mana: i32,
) -> Result<(u32, u32), &'static str> {
    if damage < 0 {
        let health = damage
            .checked_neg()
            .ok_or("Self-resurrection flat health is outside Source's defined domain")?
            as u32;
        return Ok((health, misc_value as u32));
    }

    let health = calculate_pct_u64_like_cpp(max_health, damage)? as u32;
    let mana = if max_mana > 0 {
        calculate_pct_i32_like_cpp(max_mana, damage)? as u32
    } else {
        0
    };
    Ok((health, mana))
}

fn calculate_pct_u64_like_cpp(base: u64, pct: i32) -> Result<u64, &'static str> {
    let value = (base as f32) * (pct as f32) / 100.0_f32;
    if !value.is_finite() || value < 0.0 || value >= u64::MAX as f32 {
        return Err("Self-resurrection health percentage is outside Source's defined float domain");
    }
    Ok(value as u64)
}

fn calculate_pct_i32_like_cpp(base: i32, pct: i32) -> Result<i32, &'static str> {
    let value = (base as f32) * (pct as f32) / 100.0_f32;
    if !value.is_finite() || value < 0.0 || value >= 2_147_483_648.0_f32 {
        return Err("Self-resurrection mana percentage is outside Source's defined float domain");
    }
    Ok(value as i32)
}

#[cfg(test)]
#[path = "../../unit_tests/player_effects_self_resurrection.rs"]
mod self_resurrection_values_tests;

pub fn apply_proficiency_effect_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
) -> Result<(), &'static str> {
    let _ = host.player_guid().ok_or("No player GUID")?;
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    let Some(equipped) = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_equipped_items_store
        .as_ref()
        .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
        .cloned()
    else {
        return Ok(());
    };

    let sub_class_mask = u32::try_from(equipped.equipped_item_subclass).unwrap_or(0);
    if sub_class_mask == 0 {
        return Ok(());
    }

    let update = if equipped.equipped_item_class == ItemClass::Weapon as i8 {
        let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
            player
                .add_weapon_proficiency_like_cpp(sub_class_mask)
                .map(|mask| (ItemClass::Weapon as u8, mask))
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = canonical.or_else(|| {
            (world_test && host.core().player_handle_like_cpp.is_none()).then(|| {
                host.mutate_player_persistent_capability_state_like_cpp(|state| {
                    if state.weapon_proficiency & sub_class_mask == 0 {
                        state.weapon_proficiency |= sub_class_mask;
                        Some((ItemClass::Weapon as u8, state.weapon_proficiency))
                    } else {
                        None
                    }
                })
                .expect("handle-less fixture capability owner")
            })
        });
        canonical
    } else if equipped.equipped_item_class == ItemClass::Armor as i8 {
        let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
            player
                .add_armor_proficiency_like_cpp(sub_class_mask)
                .map(|mask| (ItemClass::Armor as u8, mask))
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = canonical.or_else(|| {
            (world_test && host.core().player_handle_like_cpp.is_none()).then(|| {
                host.mutate_player_persistent_capability_state_like_cpp(|state| {
                    if state.armor_proficiency & sub_class_mask == 0 {
                        state.armor_proficiency |= sub_class_mask;
                        Some((ItemClass::Armor as u8, state.armor_proficiency))
                    } else {
                        None
                    }
                })
                .expect("handle-less fixture capability owner")
            })
        });
        canonical
    } else {
        Some(None)
    };
    let Some(update) = update else {
        return Err("No canonical Player persistent-capability owner");
    };
    if let Some((proficiency_class, proficiency_mask)) = update {
        host.send_packet(&SetProficiency {
            proficiency_mask,
            proficiency_class,
        });
    }

    Ok(())
}
