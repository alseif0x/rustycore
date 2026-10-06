// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the loot handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::packets::loot::SetLootSpecialization;
use wow_world_application::LootHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn loot_test_cx_like_cpp(&mut self) -> LootHandlerCxLikeCpp<'_> {
        let (loot, hub) = crate::session::split_loot_mut(self);
        LootHandlerCxLikeCpp::new(hub, loot)
    }

    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        self.loot_test_cx_like_cpp()
            .handle_set_loot_specialization(packet)
            .await;
    }
}
