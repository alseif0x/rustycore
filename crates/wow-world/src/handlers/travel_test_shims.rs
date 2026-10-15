// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the travel bodies moved to
//! `wow_world_instances` (`CMSG_AREA_TRIGGER`) and `wow_world_entities`
//! (`CMSG_ACTIVATE_TAXI`) (#1363 slice 4d): one-call forwards with this
//! session as the host, so no scenario call site changes.

#![cfg(test)]

use wow_packet::WorldPacket;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_area_trigger(&mut self, pkt: WorldPacket) {
        let catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        wow_world_instances::handle_area_trigger_with_catalogs_like_cpp(self, &catalogs, pkt).await;
    }

    pub(crate) async fn handle_activate_taxi(&mut self, pkt: WorldPacket) {
        wow_world_entities::handle_activate_taxi(self, pkt).await
    }
}
