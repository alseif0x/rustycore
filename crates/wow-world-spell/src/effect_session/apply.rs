// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented teleport-units and add-farsight effects (#1363 slice 9c), from
//! the World `session/spell_effects/effect_apply.rs`.

use super::SpellEffectHostLikeCpp;
use std::sync::Arc;
use tracing::debug;
use wow_core::ObjectGuid;
use wow_data::spell_duration_ms_like_cpp;
use wow_data::spell_effect_radius_like_cpp;
use wow_packet::packets::spell::SpellTargetData;

pub async fn apply_effect_teleport_units_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: &wow_data::SpellEffectInfo,
    target_guid: ObjectGuid,
    target_data: &SpellTargetData,
) {
    if !matches!(
        effect.effect,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS_OLD
            | wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS
    ) {
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };
    if target_guid != player_guid {
        // C++ also supports same-map creature NearTeleportTo here. Rust's
        // represented spell path only owns the player target today; do not
        // fabricate creature movement until Unit target selection is ported.
        return;
    }

    let Some(destination) = target_data.dst_location.as_ref().map(|dst| dst.position) else {
        debug!(
            account = host.core().account_id,
            effect_index = effect.effect_index,
            "Spell::EffectTeleportUnits represented no-op: missing destination"
        );
        return;
    };

    let target_map = target_data
        .map_id
        .and_then(|map_id| u32::try_from(map_id).ok())
        .unwrap_or_else(|| u32::from(host.core().player_map_id_like_cpp()));
    let mut destination = destination;
    if destination.orientation == 0.0 {
        if let Some(player_position) = host.hub_ref().player_position_like_cpp() {
            destination.orientation = player_position.orientation;
        }
    }

    host.teleport_to(target_map, destination).await;
}

/// Live bounded C++ `Spell::EffectAddFarsight` consumer.
///
/// Source-of-truth: canonical `wow_map::Map::map_objects`; this helper never
/// creates fallback/session mirror objects. C++ anchors:
/// `SpellEffects.cpp:2237-2261`, `SpellInfo.cpp:653-692`,
/// `SpellInfo.cpp:3894-3910`, `Spell.cpp:133-205`.
pub fn apply_effect_add_farsight_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    effect: &wow_data::SpellEffectInfo,
    target_data: &SpellTargetData,
    spell_visual_id: u32,
    cast_time_ms: u32,
) -> Option<wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp> {
    if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT {
        return None;
    }
    let dest = target_data.dst_location?.position;
    let player_guid = host.player_guid()?;
    let spell_id_u32 = u32::try_from(spell_id).ok()?;
    let spell_visual_id_i32 = i32::try_from(spell_visual_id).ok()?;
    let duration_index = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_misc_store
        .as_deref()
        .and_then(|store| store.get_by_spell_id(spell_id_u32))
        .map(|entry| u32::from(entry.duration_index))
        .unwrap_or(0);
    let duration_ms = spell_duration_ms_like_cpp(
        duration_index,
        host.hub_ref()
            .catalogs
            .spell_catalogs
            .spell_duration_store
            .as_deref(),
    );
    let radius = spell_effect_radius_like_cpp(
        effect.effect_radius_index_1,
        host.hub_ref()
            .catalogs
            .spell_catalogs
            .spell_radius_store
            .as_deref(),
    );
    let manager = Arc::clone(host.core().canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let mut instance_id = None;
    manager.do_for_all_maps_with_map_id(map_id, |managed| {
        if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
            instance_id = Some(managed.instance_id());
        }
    });
    let managed = manager.find_map_mut(map_id, instance_id.unwrap_or(0))?;
    Some(managed.map_mut().create_farsight_dynamic_object_like_cpp(
        player_guid,
        spell_id_u32,
        spell_visual_id_i32,
        dest,
        radius,
        duration_ms,
        u64::from(cast_time_ms),
        host.core().realm_id,
        player_guid.server_id(),
    ))
}
