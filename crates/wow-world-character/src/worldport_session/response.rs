// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `CMSG_WORLD_PORT_RESPONSE` (#1363 slice 13a; moved from the World
//! `handlers/travel/travel.rs`).

use tracing::{info, warn};
use wow_world_core::session::SessionState;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_lifecycle::PlayerSaveOutcomeLikeCpp;

use wow_world_entities::PlayerTeleportHostLikeCpp;

use super::WorldportHostLikeCpp;
use super::completion::{
    begin_worldport_post_add_like_cpp, finish_worldport_native_before_disconnect_like_cpp,
};
use super::recovery::{recover_rejected_worldport_like_cpp, recovery_worldport_ack_ready_like_cpp};
use super::self_create::prepare_player_self_create_for_teleport_like_cpp;

/// C++ `Map::SendInitSelf` (Map.cpp:1826), invoked by `Map::AddPlayerToMap(initPlayer=true)`
/// on a non-seamless far teleport (HandleMoveWorldportAck -> AddPlayerToMap, Map.cpp:427-463).
/// Re-sends the player's OWN object (ActivePlayer create block) so the client finishes the
/// loading screen and enters the destination map. Sourced from session state; combat stats
/// are placeholders here (health from the live value, the rest defaulted) and corrected by
/// the `send_stat_update` that follows. Inventory item objects are not yet re-sent on
/// teleport, unlike Player.cpp:3586-3608 — an open #NEXT.R8.ENTITIES.1229 parity gap.
/// None means unavailable Player projection; Some reports channel acceptance only.
/// A rejected send must not suppress the remaining native worldport effects.
pub async fn send_player_self_create_for_teleport_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
    trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
) -> Option<bool> {
    let _ = trait_node_entries; // Compatibility until the caller's catalog contract is narrowed.
    let Some(player_pkt) = prepare_player_self_create_for_teleport_like_cpp(host) else {
        return None;
    };
    let sent = host.send_packet(&player_pkt);
    info!(
        account = host.core().account_id,
        map = player_pkt.map_id,
        accepted = sent,
        "[FAR_TELEPORT] prepared SendInitSelf (player ActivePlayer create) for destination map"
    );
    Some(sent)
}
/// CMSG_WORLD_PORT_RESPONSE — client confirms it has loaded the new map.
/// Admission anchor: C++ `WorldSession::HandleMoveWorldportAck` in MovementHandler.cpp.
/// Sent after SMSG_NEW_WORLD (which is emitted from handle_suspend_token_response).
/// We respond with SMSG_RESUME_TOKEN and replay the after-add init.

pub async fn handle_world_port_response_with_catalogs_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    trait_node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    _pkt: wow_packet::WorldPacket,
) {
    use wow_packet::packets::misc::ResumeToken;

    if host.state() == SessionState::Disconnecting || !recovery_worldport_ack_ready_like_cpp(host) {
        return;
    }
    if !host.hub_ref().represented_far_teleport_pending_like_cpp() {
        warn!(
            "WorldPortResponse from account {} but far teleport semaphore is not set",
            host.core().account_id
        );
        return;
    }
    let Some((new_map, new_pos)) = PlayerTeleportHostLikeCpp::pending_teleport_like_cpp(host)
    else {
        warn!(
            "WorldPortResponse from account {} but no pending teleport",
            host.core().account_id
        );
        return;
    };
    // C++ MovementHandler.cpp:90-134 does not continue successful entry
    // after failed admission/add. Retain the pending transfer until the
    // same Player is attached; recovery must not save destination coordinates
    // paired with the old map or publish a false LoggedIn transition.
    if !wow_world_instances::try_attach_worldport_destination_like_cpp(host, new_map, new_pos) {
        warn!(
            account = host.core().account_id,
            "WorldPortResponse could not attach its Player; transfer remains pending"
        );
        recover_rejected_worldport_like_cpp(host).await;
        return;
    }
    host.hub_mut()
        .set_represented_far_teleport_pending_like_cpp(false);
    if !host.set_pending_teleport_like_cpp(None) {
        return;
    }

    info!(
        account = host.core().account_id,
        "WorldPortResponse: completing teleport to map {} ({:.2}, {:.2}, {:.2})",
        new_map,
        new_pos.x,
        new_pos.y,
        new_pos.z
    );

    host.hub_ref().update_registry_position();

    // SMSG_NEW_WORLD was already sent from handle_suspend_token_response (C++ sends it in
    // HandleSuspendTokenResponse, BEFORE the client's worldport ack — MovementHandler.cpp:253);
    // it must NOT be resent here or the client never finishes loading. #NEXT.R8.ENTITIES.1229.

    // SMSG_RESUME_TOKEN — C++ HandleMoveWorldportAck sets SequenceIndex =
    // player->m_movementCounter (read here, before SendInitialPacketsBeforeAddToMap resets
    // it) and Reason = 1 for a non-seamless far teleport (MovementHandler.cpp:108-111).
    let Some(resume_seq) = host.hub_ref().movement_counter_like_cpp() else {
        host.core_mut()
            .kick("worldport lost its Player movement state");
        return;
    };
    host.send_packet(&ResumeToken {
        sequence_index: resume_seq,
        reason: 1,
    });
    info!(
        account = host.core().account_id,
        map = new_map,
        resume_seq,
        "[FAR_TELEPORT] worldport ack: sent ResumeToken(reason=1); NewWorld was sent at SuspendTokenResponse #NEXT.R8.ENTITIES.1229"
    );

    let Some(guid) = host.player_guid() else {
        host.core_mut().kick("worldport lost its Player identity");
        return;
    };
    let updateobject_trace_enabled = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();

    // Before-add control packets the client needs for the new map: C++
    // SendInitialPacketsBeforeAddToMap resets m_movementCounter (Player.cpp:23459) and
    // ends with SetMovedUnit -> SMSG_MOVE_SET_ACTIVE_MOVER, plus a fresh time sync. The
    // full before-add packet set (spells/factions/action bars/etc.) IS replayed by
    // C++ on non-seamless transfer. Rust still omits it: this is an open parity gap,
    // not proven client retention. Its DB-backed login helper needs separation.
    host.hub_mut().reset_movement_counter_like_cpp();
    host.send_packet(&wow_packet::packets::misc::MoveSetActiveMover { mover_guid: guid });
    host.core_mut().send_time_sync();

    // C++ Map::AddPlayerToMap(initPlayer=true) -> SendInitSelf (Map.cpp:446): re-send the
    // player's OWN object (ActivePlayer create block) for the destination map. Without it
    // the client loads to 100% but never enters the world. #NEXT.R8.ENTITIES.1229.
    let Some(self_create_accepted) =
        send_player_self_create_for_teleport_like_cpp(host, trait_node_entries).await
    else {
        host.core_mut()
            .kick("worldport self CREATE has incomplete Player state");
        return;
    };
    if !begin_worldport_post_add_like_cpp(host, new_map, new_pos) {
        host.core_mut()
            .kick("worldport post-add operation could not retain its Player");
        return;
    }

    // AddPlayerToMap-equivalent: refresh nearby world objects at the new position.
    wow_world_entities::send_nearby_creatures_with_catalogs_like_cpp(
        host,
        creature_spawn_catalogs,
        new_map as u16,
        &new_pos,
        0,
    )
    .await;
    wow_world_entities::send_nearby_gameobjects(host, new_map as u16, &new_pos, 0).await;
    info!(
        account = host.core().account_id,
        map = new_map,
        visible = host.core().client_visible_guids_like_cpp.len(),
        "[FAR_TELEPORT] replayed before-add (MoveSetActiveMover + TimeSync) + refreshed \
         nearby objects; now sending after-add init"
    );

    // SendInitialPacketsAfterAddToMap: post-add phase shift, InitWorldStates resolved for
    // the destination map, the PhasingHandler::OnMapChange phase shift, CUF profiles, auras.
    crate::send_initial_packets_after_add_to_map_with_catalogs_like_cpp(
        host,
        creature_spawn_catalogs,
        guid,
        &new_pos,
        new_map as i32,
        updateobject_trace_enabled,
    )
    .await;

    let Some((zone_id, area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
        return;
    };
    // MovementHandler.cpp:156-234 completes after-add initialization and
    // zone updates before pet recovery and ProcessDelayedOperations.
    // In particular, self CREATE must precede the delayed resurrection.
    if !finish_worldport_native_before_disconnect_like_cpp(host) {
        host.core_mut()
            .kick("worldport native completion unavailable");
        return;
    }

    // Player.cpp:1494-1503: delayed resurrection precedes DELAYED_SAVE_PLAYER.
    // Do not replay the ACK or promote an unknown COMMIT back to LoggedIn.
    if let Some(outcome) = host
        .resume_deferred_player_save_with_generator_like_cpp(item_guid_generator)
        .await
    {
        // A known rollback preserves the existing SaveToDB failure policy:
        // keep dirty intent for the next scheduled save, without a hot retry.
        if !matches!(
            outcome,
            PlayerSaveOutcomeLikeCpp::Applied | PlayerSaveOutcomeLikeCpp::Failed
        ) {
            host.core_mut()
                .kick("worldport deferred save did not complete; retain native intent");
            return;
        }
        if host.state() == SessionState::Disconnecting {
            return;
        }
    }

    // Preserve server effects above even when output closes. Client readiness,
    // however, must not be reported after a failed terminal publication.
    let final_stat_accepted = host.send_stat_update();
    if !self_create_accepted || !final_stat_accepted {
        host.core_mut()
            .kick("worldport self CREATE delivery or final stat publication unavailable");
        return;
    }
    info!(
        account = host.core().account_id,
        map = new_map,
        zone = zone_id,
        area = area_id,
        resume_seq,
        "[FAR_TELEPORT] final stat packet accepted by output channel"
    );

    // Back to LoggedIn — handler dispatch resumes.
    host.set_state(SessionState::LoggedIn);
}
