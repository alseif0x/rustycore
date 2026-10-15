// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ completes worldport native effects before logout/save (WorldSession.cpp:544).
//! Rust can suspend in visibility or world-state reads after clearing the far flag.
//! Retain only remaining native phases on the canonical Player; never replay the ACK.
//! (#1363 slice 13a; moved from the World `session/lifecycle/transfer_completion.rs`.)
use wow_core::Position;
use wow_entities::PlayerWorldportPostAddPhaseLikeCpp as Phase;

use super::WorldportHostLikeCpp;
use super::recovery::terminate_worldport_recovery_like_cpp;

pub fn begin_worldport_post_add_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    position: Position,
) -> bool {
    let (state, mut hub) = host.split_lifecycle_mut();
    state.begin_worldport_post_add_like_cpp(&mut hub, map_id, position)
}

fn advance_worldport_post_add_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
    phase: Phase,
) -> bool {
    let (state, mut hub) = host.split_lifecycle_mut();
    state.advance_worldport_post_add_like_cpp(&mut hub, phase)
}

/// Finish represented native phases, not client initialization or a durable save.
/// False forbids ordinary disconnect cleanup: the owner/completion is unavailable.
/// No packet, query or await is needed; terrain reads remain outside map guards.
pub fn finish_worldport_native_before_disconnect_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
) -> bool {
    if host.player_guid().is_none() {
        return true;
    }
    let Some(mut state) = host.hub_ref().player_teleport_state_snapshot_like_cpp() else {
        return false;
    };
    if state.far_pending && state.recovery != wow_entities::PlayerTransferRecovery::Terminal {
        if !finish_pending_far_entry_before_disconnect_like_cpp(host) {
            return false;
        }
        let Some(updated) = host.hub_ref().player_teleport_state_snapshot_like_cpp() else {
            return false;
        };
        state = updated;
        if state.far_pending && state.recovery != wow_entities::PlayerTransferRecovery::Terminal {
            return false;
        }
    }
    let Some(progress) = state.post_add else {
        return true;
    };
    if !host
        .core()
        .with_owned_player_like_cpp(|player| {
            player.unit().world().map_id() == progress.map_id
                && player.unit().world().position() == progress.position
        })
        .unwrap_or(false)
    {
        return false;
    }
    if progress.phase == Phase::BeforeZone {
        if !crate::apply_post_add_zone_from_terrain_like_cpp(
            host,
            progress.map_id as i32,
            &progress.position,
        ) || !advance_worldport_post_add_like_cpp(host, Phase::ZoneApplied)
        {
            return false;
        }
    }
    if progress.phase < Phase::ScalingApplied {
        if host
            .update_represented_item_level_area_based_scaling_with_publication_like_cpp(false)
            .is_none()
            || !advance_worldport_post_add_like_cpp(host, Phase::ScalingApplied)
        {
            return false;
        }
    }
    wow_world_lifecycle::resummon_pet_temporary_unsummoned_like_cpp(host);
    wow_world_entities::process_represented_delayed_resurrection_after_teleport_like_cpp(host);
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.teleport_state_mut_like_cpp().post_add = None;
        })
        .is_some()
}

/// C++ LogoutPlayer completes a pending far transfer without waiting for a client ACK.
/// Preserve the existing bounded homebind/terminal-source contract, without packets.
fn finish_pending_far_entry_before_disconnect_like_cpp<H: WorldportHostLikeCpp>(
    host: &mut H,
) -> bool {
    use wow_entities::PlayerTransferRecovery;
    let Some(state) = host.hub_ref().player_teleport_state_snapshot_like_cpp() else {
        return false;
    };
    if state.can_delay || state.post_add.is_some() {
        return false;
    }
    let Some(mut destination) = state.far_destination else {
        return false;
    };
    if !wow_world_instances::try_attach_worldport_destination_with_publication_like_cpp(
        host,
        destination.0,
        destination.1,
        false,
    ) {
        let homebind = wow_world_spell::represented_homebind_like_cpp(host)
            .filter(|home| (home.map_id, home.position) != destination);
        if state.recovery != PlayerTransferRecovery::None || homebind.is_none() {
            terminate_worldport_recovery_like_cpp(host);
            return true;
        }
        let homebind = homebind.expect("checked homebind");
        destination = (homebind.map_id, homebind.position);
        if !host
            .hub_mut()
            .update_player_teleport_state_like_cpp(|state| {
                state.recovery = PlayerTransferRecovery::Homebind;
                state.far_destination = Some(destination);
            })
        {
            return false;
        }
        if !wow_world_instances::try_attach_worldport_destination_with_publication_like_cpp(
            host,
            destination.0,
            destination.1,
            false,
        ) {
            terminate_worldport_recovery_like_cpp(host);
            return true;
        }
    }
    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| {
            state.far_pending = false;
            state.far_destination = None;
        })
    {
        return false;
    }
    host.hub_mut().reset_movement_counter_like_cpp();
    host.hub_ref().update_registry_position();
    begin_worldport_post_add_like_cpp(host, destination.0, destination.1)
}
