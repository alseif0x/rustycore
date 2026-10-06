// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the eight C++ `BattlePetHandler.cpp` handlers
//! moved to `wow-world-application` (#1263 F5). Every entry point dispatches
//! through the registered production thunk, so the release context built by
//! [`super::battle_pet_host`] is exercised by each caller.

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
        .expect("registered battle-pet handler");
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_battle_pet_request_journal(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetRequestJournal, pkt).await;
    }

    pub async fn handle_battle_pet_request_journal_lock(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetRequestJournalLock, pkt).await;
    }

    pub async fn handle_battle_pet_clear_fanfare(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetClearFanfare, pkt).await;
    }

    pub async fn handle_battle_pet_set_flags(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetSetFlags, pkt).await;
    }

    pub async fn handle_battle_pet_set_battle_slot(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetSetBattleSlot, pkt).await;
    }

    pub async fn handle_battle_pet_summon(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetSummon, pkt).await;
    }

    pub async fn handle_battle_pet_update_notify(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::BattlePetUpdateNotify, pkt).await;
    }

    pub async fn handle_query_battle_pet_name(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::QueryBattlePetName, pkt).await;
    }
}
