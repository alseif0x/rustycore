// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the combat handlers moved to
//! `wow-world-entities` (#1263 F5; #1363 slice A3).

use wow_packet::WorldPacket;
use wow_world_entities::CombatHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_attack_stop(&mut self, pkt: WorldPacket) {
        CombatHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_attack_stop(pkt)
            .await;
    }
}
