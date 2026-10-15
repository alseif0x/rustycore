// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bounded, Player-owned homebind recovery. No retry counter or owner mirror in Session.
//! The terminal source-save policy is an explicitly approved legacy departure.
//! (#1363 slice 13a; moved from the World `session/lifecycle/transfer_recovery.rs`.)
use wow_entities::PlayerTransferRecovery;

use wow_world_entities::PlayerTeleportHostLikeCpp;

use super::WorldportHostLikeCpp;

pub(super) fn recovery_worldport_ack_ready_like_cpp<H: WorldportHostLikeCpp>(host: &H) -> bool {
    let (state, hub) = host.split_lifecycle_ref();
    state.recovery_worldport_ack_ready_like_cpp(hub)
}

pub async fn recover_rejected_worldport_like_cpp<H: WorldportHostLikeCpp>(host: &mut H) {
    let Some(state) = host
        .core()
        .with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp())
    else {
        host.core_mut()
            .kick("worldport recovery has no Player owner");
        return;
    };
    if state.recovery != PlayerTransferRecovery::None {
        terminate_worldport_recovery_like_cpp(host);
        return;
    }
    // A delayed Player update is not a failed recovery attempt. The normal
    // Session driver resets can_delay before admitting an ACK.
    if state.can_delay {
        return;
    }
    let Some(homebind) = wow_world_spell::represented_homebind_like_cpp(host) else {
        terminate_worldport_recovery_like_cpp(host);
        return;
    };
    let destination = (homebind.map_id, homebind.position);
    if PlayerTeleportHostLikeCpp::pending_teleport_like_cpp(host) == Some(destination) {
        // Already rejected this exact destination; do not replay it forever.
        terminate_worldport_recovery_like_cpp(host);
        return;
    }
    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.recovery = PlayerTransferRecovery::Homebind;
        })
    {
        host.core_mut()
            .kick("worldport recovery lost its Player owner");
        return;
    }
    wow_world_entities::teleport_to_with_options(
        host,
        homebind.map_id,
        homebind.position,
        wow_world_entities::TELE_TO_NONE_LIKE_CPP,
    )
    .await;
    if PlayerTeleportHostLikeCpp::pending_teleport_like_cpp(host) != Some(destination) {
        terminate_worldport_recovery_like_cpp(host);
    }
}

pub(super) fn terminate_worldport_recovery_like_cpp<H: WorldportHostLikeCpp>(host: &mut H) {
    let (state, mut hub) = host.split_lifecycle_mut();
    state.terminate_worldport_recovery_like_cpp(&mut hub)
}
