// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Vehicle handlers that only need the hub and the registry-sync seam.
//!
//! C++ source of truth: `src/server/game/Handlers/VehicleHandler.cpp` for
//! `HandleEjectPassenger` and `HandleRequestVehicleExit`. The represented
//! vehicle seat state lives in the Core hub, so the World session only builds
//! the borrowed context and the registry-state publication stays a bounded
//! seam (#1263 F5). The seat-change, dismiss and ride-interact bodies stay in
//! the World shell while they need its movement sanitization and spell-click
//! planning helpers.

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::vehicle::{
    EjectPassenger, RequestVehicleExit, RequestVehicleNextSeat, RequestVehiclePrevSeat,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;

// ── Pure handler-action vocabulary (moved from the World shell) ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleHandlerAction {
    Noop,
    Reject,
    ValidateMovementAndExitVehicle,
    ChangeSeat { seat_id: i8, next: bool },
    ValidateMovementAndChangeSeat { next: bool },
    HandleSpellClick { vehicle: ObjectGuid, seat_id: i8 },
    EnterVehicle { vehicle: ObjectGuid },
    ExitVehicle,
}

pub fn move_dismiss_vehicle_action_like_cpp(charmed_vehicle: ObjectGuid) -> VehicleHandlerAction {
    if charmed_vehicle.is_empty() {
        VehicleHandlerAction::Noop
    } else {
        VehicleHandlerAction::ValidateMovementAndExitVehicle
    }
}

pub fn request_adjacent_vehicle_seat_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    next: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    VehicleHandlerAction::ChangeSeat { seat_id: -1, next }
}

pub fn move_change_vehicle_seats_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    vehicle_base_guid: ObjectGuid,
    status_guid: ObjectGuid,
    dst_vehicle: ObjectGuid,
    dst_seat_index: u8,
    dst_vehicle_exists_with_empty_seat: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    if vehicle_base_guid != status_guid {
        return VehicleHandlerAction::Noop;
    }
    if dst_vehicle.is_empty() {
        return VehicleHandlerAction::ValidateMovementAndChangeSeat {
            next: dst_seat_index != u8::MAX,
        };
    }
    if dst_vehicle_exists_with_empty_seat {
        return VehicleHandlerAction::HandleSpellClick {
            vehicle: dst_vehicle,
            seat_id: dst_seat_index as i8,
        };
    }
    VehicleHandlerAction::Noop
}

pub fn request_vehicle_switch_seat_action_like_cpp(
    has_vehicle_base: bool,
    can_switch_from_current_seat: bool,
    vehicle_base_guid: ObjectGuid,
    requested_vehicle: ObjectGuid,
    seat_index: u8,
    requested_vehicle_exists_with_empty_seat: bool,
) -> VehicleHandlerAction {
    if !has_vehicle_base {
        return VehicleHandlerAction::Noop;
    }
    if !can_switch_from_current_seat {
        return VehicleHandlerAction::Reject;
    }
    if vehicle_base_guid == requested_vehicle {
        return VehicleHandlerAction::ChangeSeat {
            seat_id: seat_index as i8,
            next: true,
        };
    }
    if requested_vehicle_exists_with_empty_seat {
        return VehicleHandlerAction::HandleSpellClick {
            vehicle: requested_vehicle,
            seat_id: seat_index as i8,
        };
    }
    VehicleHandlerAction::Noop
}

pub fn ride_vehicle_interact_action_like_cpp(
    vehicle: ObjectGuid,
    target_is_player_with_vehicle_kit: bool,
    target_is_raid_member: bool,
    target_is_within_interaction_distance: bool,
    map_exists: bool,
    map_is_battle_arena: bool,
) -> VehicleHandlerAction {
    if !target_is_player_with_vehicle_kit
        || !target_is_raid_member
        || !target_is_within_interaction_distance
        || !map_exists
        || map_is_battle_arena
    {
        return VehicleHandlerAction::Noop;
    }
    VehicleHandlerAction::EnterVehicle { vehicle }
}

pub fn eject_passenger_action_like_cpp(
    player_has_vehicle_kit: bool,
    passenger: ObjectGuid,
    passenger_found: bool,
    passenger_on_same_vehicle: bool,
    passenger_seat_is_ejectable: bool,
) -> VehicleHandlerAction {
    if !player_has_vehicle_kit
        || !passenger.is_unit()
        || !passenger_found
        || !passenger_on_same_vehicle
    {
        return VehicleHandlerAction::Reject;
    }
    if passenger_seat_is_ejectable {
        VehicleHandlerAction::ExitVehicle
    } else {
        VehicleHandlerAction::Reject
    }
}

pub fn request_vehicle_exit_action_like_cpp(
    has_vehicle: bool,
    current_seat_can_enter_or_exit: bool,
) -> VehicleHandlerAction {
    if !has_vehicle {
        return VehicleHandlerAction::Noop;
    }
    if current_seat_can_enter_or_exit {
        VehicleHandlerAction::ExitVehicle
    } else {
        VehicleHandlerAction::Reject
    }
}

/// Borrowed inputs of one vehicle handler invocation.
pub struct VehicleHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> VehicleHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    /// CMSG_EJECT_PASSENGER — eject one passenger from the ridden vehicle.
    pub async fn handle_eject_passenger(&mut self, packet: EjectPassenger) {
        self.hub
            .represented_eject_passenger_like_cpp(packet.passenger);
    }

    /// CMSG_REQUEST_VEHICLE_EXIT — leave the current vehicle seat.
    ///
    /// Returns whether the registry state must be re-published by the host.
    pub async fn handle_request_vehicle_exit(&mut self, _packet: RequestVehicleExit) -> bool {
        self.represented_request_vehicle_exit_like_cpp()
    }

    /// CMSG_REQUEST_VEHICLE_PREV_SEAT.
    pub async fn handle_request_vehicle_prev_seat(
        &mut self,
        _packet: RequestVehiclePrevSeat,
    ) -> bool {
        self.represented_request_adjacent_vehicle_seat_like_cpp(false)
    }

    /// CMSG_REQUEST_VEHICLE_NEXT_SEAT.
    pub async fn handle_request_vehicle_next_seat(
        &mut self,
        _packet: RequestVehicleNextSeat,
    ) -> bool {
        self.represented_request_adjacent_vehicle_seat_like_cpp(true)
    }

    /// C++ `WorldSession::HandleRequestVehiclePrevSeat`/`NextSeat` transition.
    pub fn represented_request_adjacent_vehicle_seat_like_cpp(&mut self, next: bool) -> bool {
        let has_vehicle_base = self
            .hub
            .shared()
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some();
        let can_switch_from_current_seat = self
            .hub
            .shared()
            .represented_current_vehicle_seat_can_switch_from_like_cpp();
        match request_adjacent_vehicle_seat_action_like_cpp(
            has_vehicle_base,
            can_switch_from_current_seat,
            next,
        ) {
            VehicleHandlerAction::ChangeSeat { seat_id, next } => {
                #[cfg(any(test, feature = "test-fixtures"))]
                self.hub
                    .fixtures
                    .vehicles
                    .represented_vehicle_seat_change_requests_like_cpp
                    .push(
                        wow_world_core::session::RepresentedVehicleSeatChangeRequestLikeCpp {
                            seat_id,
                            next,
                        },
                    );
                #[cfg(not(any(test, feature = "test-fixtures")))]
                let _ = (seat_id, next);
                true
            }
            _ => false,
        }
    }

    /// C++ `WorldSession::HandleRequestVehicleExit`.
    pub fn represented_request_vehicle_exit_like_cpp(&mut self) -> bool {
        let Some(seat_flags) = self
            .hub
            .shared()
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
        else {
            return false;
        };
        if !wow_data::vehicle_seat_flags_can_enter_or_exit_like_cpp(seat_flags) {
            return false;
        }

        if !self.hub.set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        true
    }
}

/// Builds a vehicle handler context from a host's hub.
pub trait VehicleHandlerHostLikeCpp<C> {
    fn vehicle_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> VehicleHandlerCxLikeCpp<'a>;

    /// Re-publishes the registry state after a vehicle seat change; the World
    /// session still owns the registry-sync providers.
    fn sync_player_registry_state_after_vehicle_change_like_cpp(&mut self);
}

fn handle_eject_passenger_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VehicleHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match EjectPassenger::read(&mut pkt) {
            Ok(packet) => {
                session
                    .vehicle_handler_cx_like_cpp(catalogs)
                    .handle_eject_passenger(packet)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read EjectPassenger: {e}"),
        }
    })
}

fn handle_request_vehicle_exit_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VehicleHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match RequestVehicleExit::read(&mut pkt) {
            Ok(packet) => {
                let changed = session
                    .vehicle_handler_cx_like_cpp(catalogs)
                    .handle_request_vehicle_exit(packet)
                    .await;
                if changed {
                    session.sync_player_registry_state_after_vehicle_change_like_cpp();
                }
            }
            Err(e) => tracing::warn!("Failed to read RequestVehicleExit: {e}"),
        }
    })
}

fn handle_request_vehicle_prev_seat_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VehicleHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match RequestVehiclePrevSeat::read(&mut pkt) {
            Ok(packet) => {
                session
                    .vehicle_handler_cx_like_cpp(catalogs)
                    .handle_request_vehicle_prev_seat(packet)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read RequestVehiclePrevSeat: {e}"),
        }
    })
}

fn handle_request_vehicle_next_seat_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: VehicleHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match RequestVehicleNextSeat::read(&mut pkt) {
            Ok(packet) => {
                session
                    .vehicle_handler_cx_like_cpp(catalogs)
                    .handle_request_vehicle_next_seat(packet)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read RequestVehicleNextSeat: {e}"),
        }
    })
}

/// Registers the vehicle handlers on the packet registry.
pub fn register_vehicle_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: VehicleHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::EjectPassenger,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_eject_passenger",
        handler: handle_eject_passenger_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleExit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_exit",
        handler: handle_request_vehicle_exit_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehiclePrevSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_prev_seat",
        handler: handle_request_vehicle_prev_seat_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestVehicleNextSeat,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_vehicle_next_seat",
        handler: handle_request_vehicle_next_seat_thunk::<S, C>,
    })?;
    Ok(())
}
