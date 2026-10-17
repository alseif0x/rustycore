// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the vehicle handlers moved to
//! `wow-world-entities` (#1263 F5; #1363 slice A1).
#![cfg(test)]
use wow_world_entities::VehicleHandlerCxLikeCpp;

use wow_core::ObjectGuid;
use wow_packet::packets::movement::MovementInfo;
use wow_packet::packets::vehicle::{
    MoveChangeVehicleSeats, MoveDismissVehicle, RequestVehicleSwitchSeat, RideVehicleInteract,
};

use crate::session::WorldSession;

impl WorldSession {
    fn vehicle_test_cx_like_cpp(&mut self) -> VehicleHandlerCxLikeCpp<'_> {
        VehicleHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    pub(crate) async fn handle_request_vehicle_prev_seat(
        &mut self,
        packet: wow_packet::packets::vehicle::RequestVehiclePrevSeat,
    ) {
        self.vehicle_test_cx_like_cpp()
            .handle_request_vehicle_prev_seat(packet)
            .await;
    }

    pub(crate) async fn handle_request_vehicle_next_seat(
        &mut self,
        packet: wow_packet::packets::vehicle::RequestVehicleNextSeat,
    ) {
        self.vehicle_test_cx_like_cpp()
            .handle_request_vehicle_next_seat(packet)
            .await;
    }

    pub(crate) fn represented_request_adjacent_vehicle_seat_like_cpp(
        &mut self,
        next: bool,
    ) -> bool {
        self.vehicle_test_cx_like_cpp()
            .represented_request_adjacent_vehicle_seat_like_cpp(next)
    }

    pub(crate) fn represented_request_vehicle_exit_like_cpp(&mut self) -> bool {
        let changed = self
            .vehicle_test_cx_like_cpp()
            .represented_request_vehicle_exit_like_cpp();
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
        changed
    }
}

/// #1363 slice 4b moved these bodies to `vehicle_session` (`wow_world_entities`);
/// the session-hotspot scenarios keep their method syntax through these shims.
impl WorldSession {
    pub(crate) async fn handle_move_dismiss_vehicle(&mut self, packet: MoveDismissVehicle) {
        wow_world_entities::handle_move_dismiss_vehicle(self, packet).await
    }

    pub(crate) async fn handle_move_change_vehicle_seats(
        &mut self,
        packet: MoveChangeVehicleSeats,
    ) {
        wow_world_entities::handle_move_change_vehicle_seats(self, packet).await
    }

    pub(crate) async fn handle_request_vehicle_switch_seat(
        &mut self,
        packet: RequestVehicleSwitchSeat,
    ) {
        wow_world_entities::handle_request_vehicle_switch_seat(self, packet).await
    }

    pub(crate) async fn handle_ride_vehicle_interact(&mut self, packet: RideVehicleInteract) {
        wow_world_entities::handle_ride_vehicle_interact(self, packet).await
    }

    pub(crate) fn represented_move_dismiss_vehicle_like_cpp(
        &mut self,
        status: &mut MovementInfo,
    ) -> bool {
        wow_world_entities::represented_move_dismiss_vehicle_like_cpp(self, status)
    }

    pub(crate) fn represented_move_change_vehicle_seats_like_cpp(
        &mut self,
        status: &mut MovementInfo,
        dst_vehicle: ObjectGuid,
        dst_seat_index: u8,
    ) -> bool {
        wow_world_entities::represented_move_change_vehicle_seats_like_cpp(
            self,
            status,
            dst_vehicle,
            dst_seat_index,
        )
    }

    pub(crate) fn represented_request_vehicle_switch_seat_like_cpp(
        &mut self,
        requested_vehicle: ObjectGuid,
        seat_index: u8,
    ) -> bool {
        wow_world_entities::represented_request_vehicle_switch_seat_like_cpp(
            self,
            requested_vehicle,
            seat_index,
        )
    }

    pub(crate) fn represented_ride_vehicle_interact_like_cpp(
        &mut self,
        vehicle: ObjectGuid,
    ) -> bool {
        wow_world_entities::represented_ride_vehicle_interact_like_cpp(self, vehicle)
    }
}
