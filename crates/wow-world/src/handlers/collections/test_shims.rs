// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the mount/toy collection handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_packet::WorldPacket;
use wow_world_application::CollectionsHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_mount_set_favorite(&mut self, pkt: WorldPacket) {
        CollectionsHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_mount_set_favorite(pkt)
            .await;
    }

    pub async fn handle_mount_special_anim(&mut self, pkt: WorldPacket) {
        CollectionsHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_mount_special_anim(pkt)
            .await;
    }

    pub async fn handle_mount_clear_fanfare(&mut self, pkt: WorldPacket) {
        CollectionsHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_mount_clear_fanfare(pkt)
            .await;
    }

    pub async fn handle_toy_clear_fanfare(&mut self, pkt: WorldPacket) {
        CollectionsHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_toy_clear_fanfare(pkt)
            .await;
    }
}
