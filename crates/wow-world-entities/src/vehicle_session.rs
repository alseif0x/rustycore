// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vehicle seat, dismiss and ride-interact bodies (#1363 slice 4b; moved from
//! the World `handlers/vehicle.rs`, `session/movement/state.rs`,
//! `session/quest/state.rs`, `session/taxi/operations.rs` and
//! `session/spell_state/spell_click.rs`).
//!
//! C++ refs: `WorldSession::HandleMoveDismissVehicle`,
//! `HandleMoveChangeVehicleSeats`, `HandleRequestVehicleSwitchSeat` and
//! `HandleRideVehicleInteract` (`Handlers/VehicleHandler.cpp`). They live
//! beside the vehicle registrar because their branch decisions
//! (`VehicleHandlerAction`) and the spell-click plan
//! (`RepresentedSpellClickPlanLikeCpp`) are application types. The former
//! `#[cfg(test)]` fixture steps run through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`.

use crate::PlayerMovementHostLikeCpp;
use wow_core::ObjectGuid;
use wow_packet::packets::vehicle::{
    MoveChangeVehicleSeats, MoveDismissVehicle, RequestVehicleSwitchSeat, RideVehicleInteract,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp,
    RepresentedVehicleEnterRequestLikeCpp, RepresentedVehicleSeatChangeRequestLikeCpp,
    RepresentedVehicleSeatSpellClickRequestLikeCpp,
};

use crate::spell_click_values::RepresentedSpellClickPlanLikeCpp;
use crate::vehicle_handlers::{
    VehicleHandlerAction, move_change_vehicle_seats_action_like_cpp,
    request_vehicle_switch_seat_action_like_cpp, ride_vehicle_interact_action_like_cpp,
};

/// The World operations the moved vehicle bodies still call; each is the
/// same-named `WorldSession` operation.
pub trait VehicleSessionHostLikeCpp: PlayerMovementHostLikeCpp {
    fn represented_handle_spell_click_plan_with_seat_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        seat_id: Option<i8>,
    ) -> RepresentedSpellClickPlanLikeCpp;
    fn represented_player_is_same_raid_with_like_cpp(
        &self,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) -> bool;
}

/// C++ `HandleMoveDismissVehicle`.
///
/// Full `Player::ExitVehicle` requires live charm/passenger ownership. Until that runtime
/// exists, this handler mirrors the C++ early-return shape through represented charm state,
/// then copies the validated movement status before represented exit cleanup.
pub async fn handle_move_dismiss_vehicle<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    mut packet: MoveDismissVehicle,
) {
    represented_move_dismiss_vehicle_like_cpp(host, &mut packet.status);
}

/// C++ `HandleMoveChangeVehicleSeats`.
pub async fn handle_move_change_vehicle_seats<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    mut packet: MoveChangeVehicleSeats,
) {
    represented_move_change_vehicle_seats_like_cpp(
        host,
        &mut packet.status,
        packet.dst_vehicle,
        packet.dst_seat_index,
    );
}

/// C++ `HandleRequestVehicleSwitchSeat`.
pub async fn handle_request_vehicle_switch_seat<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    packet: RequestVehicleSwitchSeat,
) {
    represented_request_vehicle_switch_seat_like_cpp(host, packet.vehicle, packet.seat_index);
}

/// C++ `HandleRideVehicleInteract`.
pub async fn handle_ride_vehicle_interact<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    packet: RideVehicleInteract,
) {
    represented_ride_vehicle_interact_like_cpp(host, packet.vehicle);
}

pub fn represented_move_dismiss_vehicle_like_cpp<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    status: &mut wow_packet::packets::movement::MovementInfo,
) -> bool {
    let vehicle_guid = host.core().represented_player_charmed_guid_like_cpp();
    if vehicle_guid.is_empty()
        || host
            .hub_ref()
            .player_vehicle_seat_state_like_cpp()
            .is_none()
    {
        return false;
    }

    crate::sanitize_movement_info_represented_like_cpp(host, status);
    host.hub_mut()
        .set_player_movement_time_like_cpp(status.time);
    host.hub_mut()
        .set_player_movement_flags_like_cpp(status.flags);
    host.hub_mut().set_player_position_like_cpp(status.position);
    host.hub_ref().update_registry_position();
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .movement
            .represented_vehicle_dismiss_movements_like_cpp
            .push(RepresentedVehicleDismissMovementLikeCpp {
                vehicle_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            });
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

pub fn represented_move_change_vehicle_seats_like_cpp<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    status: &mut wow_packet::packets::movement::MovementInfo,
    dst_vehicle: ObjectGuid,
    dst_seat_index: u8,
) -> bool {
    let Some(vehicle_base_guid) = host
        .hub_ref()
        .represented_vehicle_base_guid_for_switch_like_cpp()
    else {
        return false;
    };
    if !host
        .hub_ref()
        .represented_current_vehicle_seat_can_switch_from_like_cpp()
    {
        return false;
    }

    crate::sanitize_movement_info_represented_like_cpp(host, status);
    if status.guid != vehicle_base_guid {
        return false;
    }

    let dst_seat_id = dst_seat_index as i8;
    let dst_vehicle_exists_with_empty_seat = !dst_vehicle.is_empty()
        && represented_vehicle_seat_spell_click_plan_available_like_cpp(
            host,
            dst_vehicle,
            dst_seat_id,
        );
    let action = move_change_vehicle_seats_action_like_cpp(
        true,
        true,
        vehicle_base_guid,
        status.guid,
        dst_vehicle,
        dst_seat_index,
        dst_vehicle_exists_with_empty_seat,
    );

    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .movement
            .represented_vehicle_base_movements_like_cpp
            .push(RepresentedVehicleBaseMovementLikeCpp {
                vehicle_guid: vehicle_base_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            });
    }
    let _ = record_represented_vehicle_seat_action_like_cpp(host, action);
    true
}

pub fn represented_request_vehicle_switch_seat_like_cpp<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    requested_vehicle: ObjectGuid,
    seat_index: u8,
) -> bool {
    let Some(vehicle_base_guid) = host
        .hub_ref()
        .represented_vehicle_base_guid_for_switch_like_cpp()
    else {
        return false;
    };
    let seat_id = seat_index as i8;
    let requested_vehicle_exists_with_empty_seat = vehicle_base_guid != requested_vehicle
        && represented_vehicle_seat_spell_click_plan_available_like_cpp(
            host,
            requested_vehicle,
            seat_id,
        );
    let action = request_vehicle_switch_seat_action_like_cpp(
        true,
        host.hub_ref()
            .represented_current_vehicle_seat_can_switch_from_like_cpp(),
        vehicle_base_guid,
        requested_vehicle,
        seat_index,
        requested_vehicle_exists_with_empty_seat,
    );
    record_represented_vehicle_seat_action_like_cpp(host, action)
}

pub(crate) fn record_represented_vehicle_seat_action_like_cpp<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    action: VehicleHandlerAction,
) -> bool {
    match action {
        VehicleHandlerAction::ChangeSeat { seat_id, next } => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .vehicles
                    .represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id, next });
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = (seat_id, next);
            true
        }
        VehicleHandlerAction::ValidateMovementAndChangeSeat { next } => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .vehicles
                    .represented_vehicle_seat_change_requests_like_cpp
                    .push(RepresentedVehicleSeatChangeRequestLikeCpp { seat_id: -1, next });
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = next;
            true
        }
        VehicleHandlerAction::HandleSpellClick { vehicle, seat_id } => {
            let plan =
                host.represented_handle_spell_click_plan_with_seat_like_cpp(vehicle, Some(seat_id));
            if plan.casts.is_empty() {
                return false;
            }
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .vehicles
                    .represented_vehicle_seat_spell_click_requests_like_cpp
                    .push(RepresentedVehicleSeatSpellClickRequestLikeCpp {
                        vehicle_guid: vehicle,
                        seat_id,
                        planned_casts: plan.casts.len(),
                        exact_context_unrepresented: plan.exact_context_unrepresented,
                    });
            }
            true
        }
        _ => false,
    }
}

pub fn represented_ride_vehicle_interact_like_cpp<H: VehicleSessionHostLikeCpp>(
    host: &mut H,
    vehicle_guid: ObjectGuid,
) -> bool {
    const INTERACTION_DISTANCE_LIKE_CPP: f32 = 5.0;

    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let Some(player_position) = host.hub_ref().player_position_like_cpp() else {
        return false;
    };
    let Some(registry) = host.core().player_registry() else {
        return false;
    };
    let Some(target) = registry.vehicle_interaction_snapshot(vehicle_guid) else {
        return false;
    };

    let target_is_player_with_vehicle_kit = vehicle_guid.is_player() && target.has_vehicle_kit;
    let target_is_raid_member =
        host.represented_player_is_same_raid_with_like_cpp(player_guid, vehicle_guid);
    let current_map_id = host.core().player_map_id_like_cpp();
    let current_instance_id = host
        .core()
        .current_canonical_player_map_key_like_cpp()
        .map(|key| key.instance_id)
        .unwrap_or(0);
    let target_is_within_interaction_distance = target.map_id == current_map_id
        && target.instance_id == current_instance_id
        && target
            .position
            .is_within_dist(&player_position, INTERACTION_DISTANCE_LIKE_CPP);
    let current_map_entry = host
        .hub_ref()
        .catalogs
        .map_store()
        .and_then(|store| store.get(u32::from(current_map_id)));
    let map_exists = current_map_entry.is_some();
    let map_is_battle_arena =
        current_map_entry.is_some_and(|entry| entry.instance_type == wow_data::map::MAP_ARENA);

    let action = ride_vehicle_interact_action_like_cpp(
        vehicle_guid,
        target_is_player_with_vehicle_kit,
        target_is_raid_member,
        target_is_within_interaction_distance,
        map_exists,
        map_is_battle_arena,
    );
    match action {
        VehicleHandlerAction::EnterVehicle { vehicle } => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .vehicles
                    .represented_vehicle_enter_requests_like_cpp
                    .push(RepresentedVehicleEnterRequestLikeCpp {
                        vehicle_guid: vehicle,
                    });
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let _ = vehicle;
            true
        }
        _ => false,
    }
}

pub(crate) fn represented_vehicle_seat_spell_click_plan_available_like_cpp<
    H: VehicleSessionHostLikeCpp,
>(
    host: &H,
    vehicle_guid: ObjectGuid,
    seat_id: i8,
) -> bool {
    !host
        .represented_handle_spell_click_plan_with_seat_like_cpp(vehicle_guid, Some(seat_id))
        .casts
        .is_empty()
}
