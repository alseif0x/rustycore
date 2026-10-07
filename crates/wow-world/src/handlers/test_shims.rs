// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the C++ `NPCHandler.cpp` handlers moved to
//! `wow-world-application` (#1263 F5). Every entry point dispatches through the
//! registered production thunk, so the release context built by
//! [`super::npc_host`] is exercised by each caller. `TabardVendorActivate` has
//! no test caller and therefore no entry point here.

use wow_constants::ClientOpcodes;
use wow_packet::WorldPacket;

use crate::session::WorldSession;

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered NPC handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_spirit_healer_activate(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::SpiritHealerActivate, pkt).await;
    }

    pub async fn handle_request_stabled_pets(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::RequestStabledPets, pkt).await;
    }
}
