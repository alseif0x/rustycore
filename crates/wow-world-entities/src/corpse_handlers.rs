// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Corpse and resurrection handlers (#1363 slice 4a; moved from the World
//! `handlers/entities/corpse.rs`).
//!
//! C++ refs: `Handlers/MiscHandler.cpp` `HandleRequestCemeteryList`,
//! `HandleResurrectResponse`, `HandleRepopRequest` and `HandleReclaimCorpse`.
//! The registrations stay in `wow_world_application::corpse_handlers`.

use tracing::{info, warn};
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    ReclaimCorpse, RepopRequest, RequestCemeteryListResponse, ResurrectResponse,
};

use crate::world_entities_host::WorldEntitiesHostLikeCpp;

/// CMSG_REQUEST_CEMETERY_LIST — client asks for graveyards in zone.
/// C++ ref: `WorldSession::HandleRequestCemeteryList`.
pub async fn handle_request_cemetery_list_with_catalog_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    graveyard_store: &wow_data::GraveyardStore,
    _pkt: wow_packet::WorldPacket,
) {
    if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            state = ?host.state(),
            "RUST_CEMETERY_TRACE handler entry"
        );
    }
    let Some((zone_id, area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
        return;
    };
    if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            state = ?host.state(),
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            player = ?host.player_guid(),
            "RUST_CEMETERY_TRACE handler resolved zone_area"
        );
    }
    let Some(graveyards) = graveyard_store.graveyards_for_zone(zone_id) else {
        info!(
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            player = ?host.player_guid(),
            "No graveyards found in CMSG_REQUEST_CEMETERY_LIST"
        );
        return;
    };

    let mut cemetery_ids = Vec::new();
    for graveyard in graveyards {
        if cemetery_ids.len() >= 16 {
            break;
        }
        if host.graveyard_conditions_meet_like_cpp(&graveyard.conditions) {
            cemetery_ids.push(graveyard.safe_loc_id);
        }
    }

    if cemetery_ids.is_empty() {
        info!(
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            candidate_count = graveyards.len(),
            player = ?host.player_guid(),
            "No graveyards passed conditions in CMSG_REQUEST_CEMETERY_LIST"
        );
        return;
    }

    info!(
        zone = zone_id,
        area = area_id,
        map_id = host.core().player_map_id_like_cpp(),
        candidate_count = graveyards.len(),
        accepted_count = cemetery_ids.len(),
        cemetery_ids = ?cemetery_ids,
        player = ?host.player_guid(),
        "Sending C++ RequestCemeteryListResponse"
    );
    host.send_packet(&RequestCemeteryListResponse {
        is_gossip_triggered: false,
        cemetery_ids,
    });
}

/// CMSG_RESURRECT_RESPONSE — answer to a pending resurrection request.
/// C++ ref: `WorldSession::HandleResurrectResponse`.

pub async fn handle_resurrect_response<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let response = match ResurrectResponse::read(&mut pkt) {
        Ok(response) => response,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "ResurrectResponse parse failed: {error}"
            );
            return;
        }
    };

    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false) {
        return;
    }

    if response.response != 0 {
        host.clear_represented_resurrection_request_like_cpp();
        return;
    }

    let Some(request) =
        host.take_represented_resurrection_request_if_requested_by_like_cpp(response.resurrecter)
    else {
        return;
    };

    // C++ teleports to resurrection request location before applying the
    // resurrected state. InstanceScript combat-res charges, aura original
    // caster, and SpawnCorpseBones remain represented gaps.
    host.teleport_to(request.map_id, request.position).await;
    if host.pending_teleport_like_cpp().is_some() || host.hub_ref().near_teleport_pending_like_cpp()
    {
        host.schedule_represented_resurrection_after_teleport_like_cpp(request);
    } else {
        host.hub_mut()
            .apply_represented_resurrection_health_like_cpp(request.health);
    }
}

/// CMSG_REPOP_REQUEST — release spirit.
/// C++ ref: `WorldSession::HandleRepopRequest`.

pub async fn handle_repop_request<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let _request = match RepopRequest::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "RepopRequest parse failed: {error}"
            );
            return;
        }
    };

    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false)
        || host.hub_ref().player_has_ghost_flag_like_cpp()
    {
        return;
    }

    // C++ also blocks `SPELL_AURA_PREVENT_RESURRECTION`, handles JUST_DIED
    // promotion through KillPlayer, removes the pet, builds the corpse, and
    // teleports to the graveyard. Rust has only the represented death/ghost
    // seam here; full corpse/graveyard runtime remains open.
    host.set_player_alive_like_cpp(false);
    host.set_player_ghost_flag_like_cpp(true);
    // The World test build counts this repop in its fixture; the host owns that cfg(test) step.
    host.count_represented_repop_at_graveyard_for_test_like_cpp();
}

/// CMSG_RECLAIM_CORPSE — resurrect at corpse.
/// C++ ref: `WorldSession::HandleReclaimCorpse`.

pub async fn handle_reclaim_corpse<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let _request = match ReclaimCorpse::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "ReclaimCorpse parse failed: {error}"
            );
            return;
        }
    };

    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false) {
        return;
    }

    if !host.hub_ref().player_has_ghost_flag_like_cpp() {
        return;
    }

    // C++ checks arena, live corpse existence, reclaim delay, and distance
    // before `ResurrectPlayer(0.5f)` + `SpawnCorpseBones`. Those require the
    // full player-corpse runtime; this represented slice only clears the
    // ghost/dead state when the already-known C++ gates pass.
    host.set_player_ghost_flag_like_cpp(false);
    let restore_percent = if host.hub_ref().player_in_represented_battleground_like_cpp() {
        1.0
    } else {
        0.5
    };
    host.hub_mut()
        .apply_represented_resurrection_percent_like_cpp(restore_percent);
}
