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
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::vehicle::{EjectPassenger, RequestVehicleExit};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;

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
    Ok(())
}
