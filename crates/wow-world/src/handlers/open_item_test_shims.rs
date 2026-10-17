// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the item-container bodies moved to
//! `wow_world_loot_ops` (#1363 slice 4c): one-call method forwards with this
//! session as the host, so no scenario call site changes. The item lock-id
//! read the moved `CMSG_OPEN_ITEM` body inlined keeps its scenario here.

#![cfg(test)]

use wow_packet::WorldPacket;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_open_item(&mut self, pkt: WorldPacket) {
        wow_world_loot_ops::handle_open_item(self, pkt).await
    }
    pub(crate) fn item_template_lock_id(&self, item_id: u32) -> Option<u16> {
        self.catalogs.item_template_lock_id(item_id)
    }
}
