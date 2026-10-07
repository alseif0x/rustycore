// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the loot handlers moved to
//! `wow-world-application` (#1263 F5).

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;
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

    /// Dispatches through the registered production thunk so the release
    /// context built by the host trait is exercised by every caller.
    pub async fn handle_loot_release(&mut self, pkt: WorldPacket) {
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootRelease)
            .expect("registered loot release handler");
        let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        (entry.handler)(self, &catalogs, pkt).await;
    }

    /// Dispatches through the registered production thunk so the loot owner's
    /// host trait is exercised by every caller. The item-valuation catalogs come
    /// from the session's test stores, exactly as the pre-move test entry point
    /// built them.
    pub async fn handle_loot_unit(&mut self, pkt: WorldPacket) {
        let mut catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
        catalogs.item_valuation =
            std::sync::Arc::new(self.item_valuation_catalogs_for_test_like_cpp());
        let entry = crate::session::registry::registered_handler_entries_like_cpp()
            .find(|entry| entry.opcode == ClientOpcodes::LootUnit)
            .expect("registered loot unit handler");
        (entry.handler)(self, &catalogs, pkt).await;
    }
}
