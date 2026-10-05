// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the travel handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::TravelHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn travel_test_cx_like_cpp(&mut self) -> TravelHandlerCxLikeCpp<'_> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        TravelHandlerCxLikeCpp::new(hub, lifecycle)
    }

    pub async fn handle_suspend_token_response(&mut self, pkt: WorldPacket) {
        self.travel_test_cx_like_cpp()
            .handle_suspend_token_response(pkt)
            .await;
    }

    pub async fn handle_taxi_node_status_query(&mut self, pkt: WorldPacket) {
        self.travel_test_cx_like_cpp()
            .handle_taxi_node_status_query(pkt)
            .await;
    }

    pub async fn handle_update_area_trigger_visual(&mut self, pkt: WorldPacket) {
        self.travel_test_cx_like_cpp()
            .handle_update_area_trigger_visual(pkt)
            .await;
    }

    pub async fn handle_set_taxi_benchmark_mode(&mut self, pkt: WorldPacket) {
        let changed = self
            .travel_test_cx_like_cpp()
            .handle_set_taxi_benchmark_mode(pkt)
            .await;
        if changed {
            self.sync_player_registry_state_like_cpp();
        }
    }
}
