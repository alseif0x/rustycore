// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented far world transfer after a delayed teleport: the interruptible
//! writer fence, the battleground leave and the transport server-time reset
//! (#1363 slice 11a; moved from the World `session/movement/far_transfer.rs`).
//! The ordering established by #585 and #586 is unchanged.

use wow_world_core::session::movement_protocol::{
    TELE_TO_SEAMLESS_LIKE_CPP, TeleportToOptionsLikeCpp,
};
use wow_world_core::session::{
    PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP, SessionState,
};

use super::{PlayerTeleportHostLikeCpp, TELE_TO_SPELL_LIKE_CPP};

pub(super) async fn initiate_far_teleport_after_delay_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    destination: wow_core::Position,
    options: TeleportToOptionsLikeCpp,
) {
    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.near_pending = false;
            state.near_destination = None;
            state.near_destination_zone_area = None;
        })
    {
        return;
    }
    let options = if host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .is_none()
    {
        options & !TELE_TO_SEAMLESS_LIKE_CPP
    } else {
        options
    };
    host.hub_mut().set_selection_guid_like_cpp(None);
    crate::combat_stop_like_cpp(host);
    crate::reset_contested_pvp_like_cpp(host);
    maybe_leave_represented_battleground_on_far_teleport_like_cpp(host, map_id);
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

    let Some(current_pos) = host.hub_ref().player_position_like_cpp() else {
        return;
    };

    use wow_packet::packets::misc::{SuspendToken, TransferPending};

    if !host.player_logout_like_cpp() && options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
        let transfer_pending = TransferPending {
            map_id,
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

    if !host.set_pending_teleport_like_cpp(Some((map_id, destination))) {
        return;
    }
    *host.active_area_trigger_mut_like_cpp() = None;
    if !host
        .hub_mut()
        .set_represented_far_teleport_pending_like_cpp(true)
    {
        return;
    }
    host.core_mut().state = SessionState::Transfer;

    if !host.player_logout_like_cpp() {
        if options & TELE_TO_SEAMLESS_LIKE_CPP == 0
            && !host
                .core()
                .wait_for_realm_send_before_instance_update_like_cpp()
                .await
        {
            host.core_mut()
                .kick("Delayed TransferPending writer fence failed; retain native destination");
            return;
        }
        // C++ SuspendToken.SequenceIndex = m_movementCounter (Player.cpp:1466); must match
        // the ResumeToken sent later so the client resumes. #NEXT.R8.ENTITIES.1229.
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
}
pub(super) fn maybe_leave_represented_battleground_on_far_teleport_like_cpp<
    H: PlayerTeleportHostLikeCpp,
>(
    host: &mut H,
    new_map: u32,
) {
    if host
        .hub_ref()
        .player_battleground_state_snapshot_like_cpp()
        .and_then(|state| state.battleground_map_id_like_cpp())
        .is_some_and(|bg_map_id| bg_map_id != new_map)
    {
        request_represented_battleground_leave_like_cpp(host);
    }
}
pub(super) fn clear_active_player_transport_server_time_override_for_far_teleport_like_cpp<
    H: PlayerTeleportHostLikeCpp,
>(
    host: &mut H,
) {
    let _ = host
        .hub_mut()
        .mutate_active_player_update_state_like_cpp(|state| {
            state.active_local_flags &= !PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP;
            state.active_transport_server_time = 0;
        });
    host.sync_current_player_session_visibility_detection_like_cpp();
}

// Moved from the World `session/quest/state.rs` (#1363 slice 12c).
#[cfg_attr(not(any(test, feature = "test-fixtures")), allow(unused_variables))]
fn request_represented_battleground_leave_like_cpp<H: PlayerTeleportHostLikeCpp>(host: &mut H) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .battleground
            .represented_battleground_leave_requests_like_cpp = fixtures
            .battleground
            .represented_battleground_leave_requests_like_cpp
            .saturating_add(1);
    }
}
