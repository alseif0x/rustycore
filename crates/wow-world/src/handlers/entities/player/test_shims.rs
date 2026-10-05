// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the player query handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::PlayerQueryHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_query_time(&mut self) {
        PlayerQueryHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_query_time()
            .await;
    }

    pub async fn handle_query_next_mail_time(&mut self) {
        PlayerQueryHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_query_next_mail_time()
            .await;
    }

    pub async fn handle_set_selection(&mut self, pkt: WorldPacket) {
        PlayerQueryHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_set_selection(pkt)
            .await;
    }
}
