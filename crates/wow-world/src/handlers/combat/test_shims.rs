// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the combat handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::CombatHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_attack_stop(&mut self, pkt: WorldPacket) {
        CombatHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_attack_stop(pkt)
            .await;
    }

    pub fn handle_set_sheathed(&mut self, pkt: WorldPacket) {
        CombatHandlerCxLikeCpp::new(crate::session::hub_mut(self)).handle_set_sheathed(pkt);
    }
}
