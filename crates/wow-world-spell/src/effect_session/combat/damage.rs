// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented environmental damage, taunt and the spell-damage entry
//! (#1363 slice 9c), from the World
//! `session/spell_effects/effect_combat/damage_and_combat_application.rs`.

use super::*;
use wow_core::ObjectGuid;

pub const DAMAGE_FIRE_LIKE_CPP: u8 = 5;

pub async fn apply_effect_environmental_damage_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: &wow_data::SpellEffectInfo,
    target_guid: ObjectGuid,
    creature_spawns: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE {
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if target_guid != player_guid
        || host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
    {
        return;
    }
    if host
        .hub_ref()
        .resolved_player_damage_control_like_cpp()
        .is_none_or(|state| state.environmental_damage_immune)
    {
        return;
    }

    let damage = u32::try_from(effect.effect_base_points.max(0)).unwrap_or(0);
    let Ok(_) = crate::SpellClickHostLikeCpp::apply_native_player_damage_like_cpp(
        host,
        damage,
        wow_entities::PlayerDeathOperationContext::new(player_guid, false),
        creature_spawns,
        item_guid_generator,
    )
    .await
    else {
        return;
    };
    host.sync_player_registry_state_like_cpp();
    host.core().send_environmental_damage_log_like_cpp(
        player_guid,
        DAMAGE_FIRE_LIKE_CPP,
        damage,
        0,
        0,
    );
}

/// Helper: apply damage to target creature.
pub async fn apply_damage_with_generator_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    spell_id: Option<i32>,
    target_guid: ObjectGuid,
    damage_amount: u32,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    host.apply_damage_from_caster_like_cpp(
        item_guid_generator,
        spell_id,
        player_guid,
        target_guid,
        damage_amount,
        ObjectGuid::create_null(),
        0,
    )
    .await
}
