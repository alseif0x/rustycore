// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spirit-healer resurrection lifecycle.

use crate::{
    CorpseHostLikeCpp, CorpseLifecycleCatalogsLikeCpp, closest_graveyard_like_cpp,
    player_corpse_location_like_cpp, resurrect_player_like_cpp, spawn_corpse_bones_like_cpp,
};

/// C++ `WorldSession::SendSpiritResurrect` (`NPCHandler.cpp:347-370`).
pub async fn send_spirit_resurrect<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    resurrect_player_like_cpp(host, catalogs.resurrection(), 0.5, true).await;
    host.apply_spirit_healer_durability_loss();

    // C++ caches GetCorpseLocation before SpawnCorpseBones and asks the
    // graveyard store directly; this does not require the corpse object to be
    // present in the current map.
    let corpse_graveyard_id =
        if let Some((map_id, position)) = player_corpse_location_like_cpp(host) {
            closest_graveyard_like_cpp(host, catalogs.graveyards, map_id, position)
        } else {
            None
        };

    spawn_corpse_bones_like_cpp(host, catalogs.item_guid_generator, true).await;

    let Some(corpse_graveyard_id) = corpse_graveyard_id else {
        return;
    };
    let ghost_position = host.hub_ref().player_position_like_cpp();
    let ghost_graveyard_id = if let Some(position) = ghost_position {
        let map_id = u32::from(host.core().player_map_id_like_cpp());
        closest_graveyard_like_cpp(host, catalogs.graveyards, map_id, position)
    } else {
        None
    };
    if ghost_graveyard_id == Some(corpse_graveyard_id) {
        return;
    }

    let graveyard = host
        .hub_ref()
        .catalogs
        .world_safe_loc_store_like_cpp
        .as_ref()
        .and_then(|store| store.get(corpse_graveyard_id))
        .map(|entry| (entry.map_id, entry.position));
    if let Some((map_id, position)) = graveyard {
        host.teleport_to(map_id, position).await;
    }
}
