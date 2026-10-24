// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Player::TeleportTo`: the gates, the same-map near teleport, the far
//! detach with the writer fence, and the delayed teleport after the update
//! (#1363 slice 11a; moved from the World `session/movement/transfer.rs`).

use tracing::{info, warn};
use wow_core::Position;
use wow_world_core::session::SessionState;
use wow_world_core::session::movement_protocol::{
    TELE_TO_SEAMLESS_LIKE_CPP, TeleportToOptionsLikeCpp,
};

use super::far_transfer::{
    clear_active_player_transport_server_time_override_for_far_teleport_like_cpp,
    initiate_far_teleport_after_delay_like_cpp,
    maybe_leave_represented_battleground_on_far_teleport_like_cpp,
};
use super::{
    CLASS_DEATH_KNIGHT_LIKE_CPP, DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP,
    DEATH_KNIGHT_START_MAP_LIKE_CPP, PlayerTeleportHostLikeCpp, TELE_REVIVE_AT_TELEPORT_LIKE_CPP,
    TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP, TELE_TO_SPELL_LIKE_CPP,
    TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP, TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP,
    TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
};

pub fn pending_teleport_save_destination_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &H,
) -> Option<(u16, Position)> {
    if let Some((map_id, position)) = host.pending_teleport_like_cpp() {
        return Some((u16::try_from(map_id).unwrap_or(u16::MAX), position));
    }
    if let Some(teleport) = host.hub_ref().player_teleport_state_snapshot_like_cpp()
        && teleport.near_pending
    {
        return teleport.near_destination;
    }
    None
}
pub async fn teleport_to_with_options<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    new_map: u32,
    new_pos: wow_core::Position,
    options: TeleportToOptionsLikeCpp,
) {
    if options & TELE_REVIVE_AT_TELEPORT_LIKE_CPP != 0
        && host.hub_ref().resolved_player_is_alive_like_cpp() == Some(false)
    {
        // The ordinary teleport capability has no resurrection catalogs.
        // Preserve its fail-closed behavior without imposing corpse operations
        // on callers that issue an ordinary area-trigger or recovery teleport.
        return;
    }
    teleport_to_inner(host, new_map, new_pos, options, None).await;
}

pub async fn teleport_to_with_options_and_resurrection<H>(
    host: &mut H,
    new_map: u32,
    new_pos: wow_core::Position,
    options: TeleportToOptionsLikeCpp,
    catalogs: crate::ResurrectionOperationCatalogs<'_>,
) where
    H: PlayerTeleportHostLikeCpp,
{
    teleport_to_inner(host, new_map, new_pos, options, Some(catalogs)).await;
}

async fn teleport_to_inner<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    new_map: u32,
    new_pos: wow_core::Position,
    mut options: TeleportToOptionsLikeCpp,
    resurrection_catalogs: Option<crate::ResurrectionOperationCatalogs<'_>>,
) {
    if host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_some_and(|state| state.recovery == wow_entities::PlayerTransferRecovery::Terminal)
    {
        return;
    }
    // C++ Player.cpp:1239 validates the catalog map and all four coordinates
    // before movement, combat, pet, ownership or teleport state is changed.
    if host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .is_none_or(|maps| maps.get(new_map).is_none())
        || !new_pos.is_valid_map_coord_like_cpp()
    {
        warn!(
            "Invalid map or coordinates for teleport to {} from account {}",
            new_map,
            host.core().account_id
        );
        return;
    }

    if host.is_map_disabled_for_player_like_cpp(new_map) {
        warn!(
            account = host.core().account_id,
            map_id = new_map,
            "Teleport blocked by C++ DisableMgr map gate"
        );
        host.hub_ref()
            .send_transfer_aborted_like_cpp(new_map, TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP);
        return;
    }

    if let Some(target_map) = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(new_map).copied())
        && target_map.is_battleground_or_arena()
        && !host.hub_ref().player_in_represented_battleground_like_cpp()
    {
        warn!(
            account = host.core().account_id,
            map_id = new_map,
            "Teleport silently blocked by C++ battleground assignment gate"
        );
        return;
    }

    if let Some(target_map) = host
        .hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(new_map).copied())
        && host.core().expansion < target_map.expansion_like_cpp()
    {
        warn!(
            account = host.core().account_id,
            map_id = new_map,
            session_expansion = host.core().expansion,
            required_expansion = target_map.expansion_like_cpp(),
            "Teleport blocked by C++ client expansion gate"
        );
        host.hub_ref().send_transfer_aborted_with_params_like_cpp(
            new_map,
            TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP,
            target_map.expansion_like_cpp(),
            0,
        );
        return;
    }

    let Some(current_pos) = host.hub_ref().player_position_like_cpp() else {
        warn!(
            "Cannot teleport account {}: no current position",
            host.core().account_id
        );
        return;
    };

    exit_represented_vehicle_for_teleport_like_cpp(host);
    host.hub_mut().reset_teleport_movement_state_like_cpp();

    if host.hub_ref().player_class_like_cpp() == CLASS_DEATH_KNIGHT_LIKE_CPP
        && host.core().player_map_id_like_cpp() == DEATH_KNIGHT_START_MAP_LIKE_CPP
        && u32::from(host.core().player_map_id_like_cpp()) != new_map
        && host.hub_ref().player_is_game_master_like_cpp() != Some(true)
        && !host
            .known_spells_like_cpp()
            .contains(&DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP)
    {
        host.hub_ref().send_transfer_aborted_with_params_like_cpp(
            new_map,
            TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
            1,
            0,
        );
        return;
    }

    // Approved recovery exception: a detached Player needs AddPlayerToMap,
    // even when returning to its old map. A near ACK only relocates it.
    let active_map = host.core().current_canonical_player_map_key_like_cpp();
    let same_map_near_teleport = active_map.is_some_and(|key| key.map_id == new_map);
    if same_map_near_teleport {
        if !host
            .hub_mut()
            .set_represented_far_teleport_pending_like_cpp(false)
        {
            return;
        }
        initiate_same_map_near_teleport_like_cpp(
            host,
            new_map,
            new_pos,
            options,
            resurrection_catalogs,
        )
        .await;
        return;
    }

    if let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
        host.player_cannot_enter_target_map_like_cpp(new_map)
    {
        host.hub_ref().send_transfer_aborted_with_params_like_cpp(
            new_map,
            transfer_abort,
            arg,
            map_difficulty_x_condition_id,
        );
        return;
    }

    options = host
        .hub_ref()
        .teleport_options_after_seamless_gate_like_cpp(new_map, options);
    if active_map.is_none() {
        options &= !TELE_TO_SEAMLESS_LIKE_CPP;
    }

    if !same_map_near_teleport {
        let Some(can_delay) = host
            .hub_ref()
            .player_teleport_state_snapshot_like_cpp()
            .map(|state| state.can_delay)
        else {
            return;
        };
        if !host
            .hub_mut()
            .update_player_teleport_state_like_cpp(|state| {
                state.has_delayed = can_delay;
                state.near_pending = false;
                state.near_destination = None;
                state.near_destination_zone_area = None;
                if can_delay {
                    state.far_pending = true;
                    state.delayed = Some((new_map, new_pos, options));
                }
            })
        {
            return;
        }
        if can_delay {
            return;
        }

        host.hub_mut().set_selection_guid_like_cpp(None);
        crate::combat_stop_like_cpp(host);
        crate::reset_contested_pvp_like_cpp(host);
        maybe_leave_represented_battleground_on_far_teleport_like_cpp(host, new_map);
        host.hub_mut()
            .unsummon_represented_pet_temporary_if_any_like_cpp();
        let _ = host
            .hub_ref()
            .remove_all_dynamic_objects_for_current_player_like_cpp();
        let _ = host
            .hub_ref()
            .remove_all_area_triggers_for_current_player_like_cpp();
        if options & TELE_TO_SPELL_LIKE_CPP == 0 {
            let _ = host.interrupt_non_melee_spells_for_far_teleport_like_cpp();
        }
        let _ = host.remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp();
    }

    info!(
        account = host.core().account_id,
        old_map = host.core().player_map_id_like_cpp(),
        new_map = new_map,
        old_pos = format!(
            "({:.2}, {:.2}, {:.2})",
            current_pos.x, current_pos.y, current_pos.z
        ),
        new_pos = format!("({:.2}, {:.2}, {:.2})", new_pos.x, new_pos.y, new_pos.z),
        "Player teleporting to new map"
    );

    use wow_packet::packets::misc::{SuspendToken, TransferPending};

    // 1. SMSG_TRANSFER_PENDING — tell client to start loading screen
    if !host.player_logout_like_cpp() && options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
        let transfer_pending = TransferPending {
            map_id: new_map,
            old_map_position: current_pos,
            ship: None,
            transfer_spell_id: None,
        };
        host.core().send_packet_realm(&transfer_pending);
        clear_active_player_transport_server_time_override_for_far_teleport_like_cpp(host);
    }

    let _ = host
        .hub_mut()
        .remove_current_player_from_canonical_current_map_like_cpp();
    // #1263 C2: the far-transfer detach point. The canonical Player is now
    // detached (residence `Detached`), so the canonical key resolution
    // below is expected to be absent and the runtime map key falls
    // back. Every guard taken here is released before the event is emitted.
    {
        let old_map_id = host.core().player_map_id_like_cpp();
        let canonical_key = host.core().current_canonical_player_map_key_like_cpp();
        let (resolved_map_id, resolved_instance_id) =
            host.core().current_runtime_map_key_like_cpp();
        let _destination_map_id = u16::try_from(new_map).unwrap_or(u16::MAX);
        host.core().emit_runtime_location_capture_like_cpp(
            wow_world_core::session::RuntimeLocationCaptureLikeCpp {
                capture_id: "C2",
                root: "WorldSession::teleport_to_with_options::far_transfer_detach",
                phase: "detach",
                player_guid_counter: host.core().player_guid.map(|guid| guid.counter() as u32),
                session_state: wow_world_core::session::session_state_name_like_cpp(
                    host.core().state,
                ),
                requested_map_id: old_map_id,
                requested_instance_id: Some(resolved_instance_id),
                canonical_key_present: canonical_key.is_some(),
                resolved_map_id,
                resolved_instance_id,
                detail: "far_teleport_detached_before_worldport_response",
            },
        );
    }

    // 2. Store pending destination — completed in handle_world_port_response
    if !host.set_pending_teleport_like_cpp(Some((new_map, new_pos))) {
        return;
    }
    *host.active_area_trigger_mut_like_cpp() = None;

    // Retain native completion authority before an interruptible writer wait.
    if !host
        .hub_mut()
        .set_represented_far_teleport_pending_like_cpp(true)
    {
        return;
    }
    host.core_mut().state = SessionState::Transfer;

    // 3. SMSG_SUSPEND_TOKEN — pause movement processing on client. C++
    // Player::TeleportTo sets SequenceIndex = m_movementCounter WITHOUT incrementing
    // (Player.cpp:1466), and HandleMoveWorldportAck's ResumeToken uses the SAME counter
    // (the before-add reset happens only after ResumeToken). The client pairs suspend and
    // resume by this index, so they MUST match — a hardcoded 1 here vs the real counter in
    // ResumeToken left the client stuck on the loading screen. #NEXT.R8.ENTITIES.1229.
    if !host.player_logout_like_cpp() {
        if options & TELE_TO_SEAMLESS_LIKE_CPP == 0
            && !host
                .core()
                .wait_for_realm_send_before_instance_update_like_cpp()
                .await
        {
            host.core_mut()
                .kick("TransferPending writer fence failed; retain native destination");
            return;
        }
        let Some(suspend_seq) = host.hub_ref().movement_counter_like_cpp() else {
            return;
        };
        host.send_packet(&SuspendToken {
            sequence_index: suspend_seq,
            reason: if options & TELE_TO_SEAMLESS_LIKE_CPP != 0 {
                2
            } else {
                1
            },
        });
    }

    info!(
        account = host.core().account_id,
        "Teleport initiated: map {} → {} dest ({:.2}, {:.2}, {:.2}); awaiting WorldPortResponse",
        host.core().player_map_id_like_cpp(),
        new_map,
        new_pos.x,
        new_pos.y,
        new_pos.z
    );
}
fn exit_represented_vehicle_for_teleport_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
) -> bool {
    if host
        .hub_ref()
        .player_vehicle_seat_state_like_cpp()
        .and_then(|(flags, _)| flags)
        .is_none()
    {
        return false;
    }

    if !host
        .hub_mut()
        .set_player_vehicle_seat_state_like_cpp(None, None)
    {
        return false;
    }
    host.sync_player_registry_state_like_cpp();
    true
}
async fn initiate_same_map_near_teleport_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    destination: wow_core::Position,
    options: TeleportToOptionsLikeCpp,
    resurrection_catalogs: Option<crate::ResurrectionOperationCatalogs<'_>>,
) {
    let Some(can_delay) = host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .map(|state| state.can_delay)
    else {
        return;
    };
    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.has_delayed = can_delay;
        })
    {
        return;
    }
    if can_delay {
        let map_id_u16 = u16::try_from(map_id).unwrap_or(host.core().player_map_id_like_cpp());
        let _ = host
            .hub_mut()
            .update_player_teleport_state_like_cpp(|state| {
                state.near_pending = true;
                state.near_destination = Some((map_id_u16, destination));
                state.near_destination_zone_area = None;
                state.delayed = Some((map_id, destination, options));
            });
        return;
    }

    host.hub_mut()
        .unsummon_represented_pet_for_same_map_teleport_if_out_of_range_like_cpp(
            destination,
            options,
        );

    if host.hub_ref().resolved_player_is_alive_like_cpp() == Some(false)
        && options & TELE_REVIVE_AT_TELEPORT_LIKE_CPP != 0
    {
        let Some(catalogs) = resurrection_catalogs else {
            return;
        };
        if !host
            .revive_for_teleport(catalogs, map_id, destination)
            .await
        {
            return;
        }
    }

    if options & TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP == 0 {
        crate::combat_stop_like_cpp(host);
    }

    let map_id = u16::try_from(map_id).unwrap_or(host.core().player_map_id_like_cpp());
    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.far_destination = None;
            state.near_pending = true;
            state.near_destination = Some((map_id, destination));
            state.near_destination_zone_area = None;
        })
    {
        return;
    }
    if let Some(current_pos) = host.hub_ref().player_position_like_cpp() {
        host.hub_mut()
            .set_fall_information_like_cpp(0, current_pos.z);
    }

    if !host.player_logout_like_cpp() {
        let Some(sequence_index) = host.hub_mut().next_movement_counter_like_cpp() else {
            return;
        };
        if let Some(mover_guid) = host.player_guid() {
            host.hub_ref()
                .send_same_map_move_update_teleport_to_visible_set_like_cpp(mover_guid);
            host.send_packet(&wow_packet::packets::movement::MoveTeleport {
                mover_guid,
                position: destination,
                facing: destination.orientation,
                sequence_index,
                preload_world: 0,
                transport_guid: None,
            });
        }
    }

    info!(
        account = host.core().account_id,
        map_id,
        new_pos = format!(
            "({:.2}, {:.2}, {:.2})",
            destination.x, destination.y, destination.z
        ),
        "Player same-map near teleport initiated; awaiting MoveTeleportAck"
    );
}
pub async fn process_represented_delayed_teleport_after_update_like_cpp<
    H: PlayerTeleportHostLikeCpp,
>(
    host: &mut H,
    catalogs: crate::ResurrectionOperationCatalogs<'_>,
) -> bool {
    let Some(teleport) = host.hub_ref().player_teleport_state_snapshot_like_cpp() else {
        return false;
    };
    if !teleport.has_delayed {
        return false;
    }
    // C++ Player::Update only executes a delayed teleport for a living player
    // (`Player.cpp:1151-1155`); a ghost must keep its pending intent.
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return false;
    }

    let Some((map_id, destination, options)) = teleport.delayed else {
        let _ = host
            .hub_mut()
            .update_player_teleport_state_like_cpp(|state| {
                state.has_delayed = false;
            });
        return false;
    };

    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.delayed = None;
            state.can_delay = false;
            state.has_delayed = false;
        })
    {
        return false;
    }
    if host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .is_some_and(|key| key.map_id == map_id)
    {
        initiate_same_map_near_teleport_like_cpp(
            host,
            map_id,
            destination,
            options,
            Some(catalogs),
        )
        .await;
    } else {
        initiate_far_teleport_after_delay_like_cpp(host, map_id, destination, options).await;
    }
    true
}
