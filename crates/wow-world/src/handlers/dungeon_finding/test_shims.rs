// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the dungeon-finder handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::DungeonFindingHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    fn dungeon_finding_test_cx_like_cpp(&mut self) -> DungeonFindingHandlerCxLikeCpp<'_> {
        DungeonFindingHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    pub async fn handle_df_get_join_status(&mut self, pkt: WorldPacket) {
        self.dungeon_finding_test_cx_like_cpp()
            .handle_df_get_join_status(pkt)
            .await;
    }

    pub async fn handle_request_conquest_formula_constants(&mut self, pkt: WorldPacket) {
        self.dungeon_finding_test_cx_like_cpp()
            .handle_request_conquest_formula_constants(pkt)
            .await;
    }

    pub async fn handle_request_lfg_list_blacklist(&mut self, pkt: WorldPacket) {
        self.dungeon_finding_test_cx_like_cpp()
            .handle_request_lfg_list_blacklist(pkt)
            .await;
    }

    pub async fn handle_lfg_list_get_status(&mut self, pkt: WorldPacket) {
        self.dungeon_finding_test_cx_like_cpp()
            .handle_lfg_list_get_status(pkt)
            .await;
    }
}
