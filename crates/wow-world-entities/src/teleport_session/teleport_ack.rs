// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `WorldSession::HandleMoveTeleportAck`'s near-teleport state machine and
//! the resurrection delayed across a teleport (#1363 slice 11a; moved from the
//! World `session/movement/transfer.rs`).

use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
    PlayerResurrectionRequestLikeCpp,
};
use wow_world_core::session::movement_protocol::MoveTeleportAckActionLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::movement_protocol::MoveTeleportAckEventLikeCpp;

use super::PlayerTeleportHostLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub fn schedule_represented_resurrection_after_teleport(
    host: &mut impl PlayerTeleportHostLikeCpp,
    request: PlayerResurrectionRequestLikeCpp,
) -> bool {
    let continuation = PlayerResurrectionContinuation {
        operation: PlayerResurrectionOperation::RequestData(request),
        phase: PlayerResurrectionPhase::AwaitingTeleport,
    };
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let state = player.resurrection_state_mut_like_cpp();
            if state.delayed_after_teleport.is_some() {
                return false;
            }
            state.delayed_after_teleport = Some(continuation);
            true
        })
        .unwrap_or(false);
    if canonical || host.core().player_handle_like_cpp.is_none() {
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures
                .combat
                .represented_delayed_resurrection_after_teleport_like_cpp = Some(continuation);
        }
    }
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    canonical || world_test && host.core().player_handle_like_cpp.is_none()
}

pub async fn process_represented_delayed_resurrection_after_teleport_like_cpp<
    H: PlayerTeleportHostLikeCpp + crate::CorpseHostLikeCpp,
>(
    host: &mut H,
    catalogs: crate::ResurrectionOperationCatalogs<'_>,
) -> bool {
    crate::resurrection_request::process_after_teleport(host, catalogs).await
}
pub fn handle_move_teleport_ack_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    mover_guid: ObjectGuid,
    ack_index: i32,
    move_time: i32,
) -> MoveTeleportAckActionLikeCpp {
    let accepted = host
        .hub_mut()
        .record_move_teleport_ack_like_cpp(mover_guid, ack_index, move_time);
    let Some(teleport) = host.hub_ref().player_teleport_state_snapshot_like_cpp() else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };
    if !teleport.near_pending {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::NotBeingTeleportedNear,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    }

    if !accepted {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::WrongMover,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    }

    let Some((map_id, destination)) = teleport.near_destination else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingDestination,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };

    let Some(world_local_before) = host.hub_ref().player_world_local_state_like_cpp() else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };
    let Some(player_guid) = host.player_guid() else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };
    let Some(pvp_enabled_before) = host.hub_ref().player_is_pvp_like_cpp(player_guid) else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };
    let Some(in_pvp_before) = host.hub_ref().player_has_in_pvp_flag_like_cpp(player_guid) else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    };

    if !host
        .hub_mut()
        .update_player_teleport_state_like_cpp(|state| state.near_pending = false)
    {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            Some(map_id),
            Some(destination),
            None,
            None,
            None,
            false,
            false,
            false,
            false,
        );
    }
    let old_zone = world_local_before.zone_id_like_cpp();
    host.hub_mut()
        .set_player_map_position_like_cpp(map_id, destination);
    host.hub_ref().update_registry_position();
    host.hub_mut()
        .set_fall_information_like_cpp(0, destination.z);

    let (new_zone, new_area) = teleport
        .near_destination_zone_area
        .unwrap_or(world_local_before.zone_area_like_cpp());
    host.update_zone_represented_like_cpp(new_zone, new_area);

    let zone_changed = old_zone != new_zone;
    let Some(world_local_after) = host.hub_ref().player_world_local_state_like_cpp() else {
        return record_move_teleport_ack_event_like_cpp(
            host,
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
            Some(map_id),
            Some(destination),
            Some(old_zone),
            Some(new_zone),
            Some(new_area),
            false,
            false,
            false,
            false,
        );
    };
    let honorless_target_cast = zone_changed && world_local_after.is_pvp_hostile_like_cpp();
    let pvp_disabled = zone_changed
        && !world_local_after.is_pvp_hostile_like_cpp()
        && pvp_enabled_before
        && !in_pvp_before;
    if pvp_disabled {
        host.hub_mut().update_player_pvp_like_cpp(false, true);
    }

    host.resummon_pet_temporary_unsummoned_like_cpp();
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.movement.delayed_operations_processed_like_cpp = fixtures
            .movement
            .delayed_operations_processed_like_cpp
            .saturating_add(1);
    }

    record_move_teleport_ack_event_like_cpp(
        host,
        mover_guid,
        ack_index,
        move_time,
        MoveTeleportAckActionLikeCpp::Accepted,
        Some(map_id),
        Some(destination),
        Some(old_zone),
        Some(new_zone),
        Some(new_area),
        honorless_target_cast,
        pvp_disabled,
        true,
        true,
    )
}
#[allow(clippy::too_many_arguments)]
fn record_move_teleport_ack_event_like_cpp<H: PlayerTeleportHostLikeCpp>(
    host: &mut H,
    mover_guid: ObjectGuid,
    ack_index: i32,
    move_time: i32,
    action: MoveTeleportAckActionLikeCpp,
    destination_map_id: Option<u16>,
    destination_position: Option<wow_core::Position>,
    old_zone_id: Option<u32>,
    new_zone_id: Option<u32>,
    new_area_id: Option<u32>,
    honorless_target_cast: bool,
    pvp_disabled: bool,
    pet_resummon_requested: bool,
    delayed_operations_processed: bool,
) -> MoveTeleportAckActionLikeCpp {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .teleport
            .move_teleport_ack_events_like_cpp
            .push(MoveTeleportAckEventLikeCpp {
                mover_guid,
                ack_index,
                move_time,
                action,
                destination_map_id,
                destination_position,
                old_zone_id,
                new_zone_id,
                new_area_id,
                honorless_target_cast,
                pvp_disabled,
                pet_resummon_requested,
                delayed_operations_processed,
            });
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (
        host,
        mover_guid,
        ack_index,
        move_time,
        destination_map_id,
        destination_position,
        old_zone_id,
        new_zone_id,
        new_area_id,
        honorless_target_cast,
        pvp_disabled,
        pet_resummon_requested,
        delayed_operations_processed,
    );
    action
}
