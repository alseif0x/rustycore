// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented effects that act on the session player (#1363 slice 9c), from
//! the World `session/spell_effects/effects_player.rs`.

use super::SpellEffectHostLikeCpp;
use tracing::info;

/// Ask nearby sessions to run the same visibility diff after this player enters.
///
/// C++ adds the new Player to the map before `UpdateVisibilityForPlayer`, so
/// other players discover it through their normal `m_clientGUIDs` path. Queue
/// the existing full visibility command instead of sending a raw CREATE that
/// cannot update the receiver's client-visible GUID set.
pub fn notify_other_players_visibility_changed_like_cpp<H: SpellEffectHostLikeCpp>(host: &H) {
    let Some(guid) = host.player_guid() else {
        return;
    };
    let Some(registry) = &host.core().player_registry else {
        return;
    };
    let Some(pos) = host.hub_ref().player_position_like_cpp() else {
        return;
    };
    let map_id = host.core().player_map_id_like_cpp();
    let instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let visibility_range = host
        .hub_ref()
        .config
        .player_map_visibility_range_like_cpp(map_id);
    let target_combat_reach = host.represented_visibility_source_combat_reach_like_cpp();
    let mut broadcast_count = 0;

    for recipient in registry.runtime_recipients() {
        if recipient.guid == guid
            || !recipient.is_in_world
            || recipient.map_id != map_id
            || recipient.instance_id != instance_id
        {
            continue;
        }
        if !wow_core::visibility_distance_allows_like_cpp(
            &recipient.position,
            recipient.combat_reach,
            &pos,
            target_combat_reach,
            visibility_range,
        ) {
            continue;
        }

        match registry.request_current_visibility_refresh(
            recipient.registration,
            map_id,
            instance_id,
        ) {
            Ok(()) => {
                broadcast_count += 1;
            }
            Err(_) => {}
        }
    }

    if broadcast_count > 0 {
        info!(
            "Queued visibility refresh for {:?} on {} nearby players on map {}",
            guid, broadcast_count, map_id
        );
    }
}

pub fn player_current_map_instanceable_like_cpp<H: SpellEffectHostLikeCpp>(host: &H) -> bool {
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    host.hub_ref()
        .catalogs
        .map_store()
        .and_then(|store| store.get(map_id))
        .is_some_and(|entry| entry.instance_type != wow_data::map::MAP_COMMON)
}

pub fn add_honor_xp_to_current_player_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    xp: i32,
) -> Option<wow_entities::PlayerValuesUpdate> {
    let xp = u32::try_from(xp).ok()?;
    let player_level = host.hub_ref().player_level_like_cpp();
    host.core()
        .mutate_canonical_player_like_cpp(|player| {
            player
                .add_honor_xp_like_cpp(xp, player_level)
                .then(|| player.values_update(true))
        })
        .flatten()
}
