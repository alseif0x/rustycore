// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the gameobject interaction handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::GameObjectHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn gameobject_test_cx_like_cpp(&mut self) -> GameObjectHandlerCxLikeCpp<'_> {
        let (interaction, world_entities, hub) =
            crate::session::split_interaction_world_entities_mut(self);
        GameObjectHandlerCxLikeCpp::new(hub, interaction, world_entities)
    }

    pub async fn handle_close_interaction(&mut self, pkt: WorldPacket) {
        self.gameobject_test_cx_like_cpp()
            .handle_close_interaction(pkt)
            .await;
    }

    pub async fn handle_game_obj_report_use(&mut self, pkt: WorldPacket) {
        self.gameobject_test_cx_like_cpp()
            .handle_game_obj_report_use(pkt)
            .await;
    }
}
