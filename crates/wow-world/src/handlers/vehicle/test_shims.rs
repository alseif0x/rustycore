// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the vehicle handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_world_application::VehicleHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn vehicle_test_cx_like_cpp(&mut self) -> VehicleHandlerCxLikeCpp<'_> {
        VehicleHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    pub async fn handle_eject_passenger(
        &mut self,
        packet: wow_packet::packets::vehicle::EjectPassenger,
    ) {
        self.vehicle_test_cx_like_cpp()
            .handle_eject_passenger(packet)
            .await;
    }

    pub async fn handle_request_vehicle_prev_seat(
        &mut self,
        packet: wow_packet::packets::vehicle::RequestVehiclePrevSeat,
    ) {
        self.vehicle_test_cx_like_cpp()
            .handle_request_vehicle_prev_seat(packet)
            .await;
    }

    pub async fn handle_request_vehicle_next_seat(
        &mut self,
        packet: wow_packet::packets::vehicle::RequestVehicleNextSeat,
    ) {
        self.vehicle_test_cx_like_cpp()
            .handle_request_vehicle_next_seat(packet)
            .await;
    }

    pub fn represented_request_adjacent_vehicle_seat_like_cpp(&mut self, next: bool) -> bool {
        self.vehicle_test_cx_like_cpp()
            .represented_request_adjacent_vehicle_seat_like_cpp(next)
    }

    pub async fn handle_request_vehicle_exit(
        &mut self,
        packet: wow_packet::packets::vehicle::RequestVehicleExit,
    ) {
        let changed = self
            .vehicle_test_cx_like_cpp()
            .handle_request_vehicle_exit(packet)
            .await;
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub fn represented_request_vehicle_exit_like_cpp(&mut self) -> bool {
        let changed = self
            .vehicle_test_cx_like_cpp()
            .represented_request_vehicle_exit_like_cpp();
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
        changed
    }
}
