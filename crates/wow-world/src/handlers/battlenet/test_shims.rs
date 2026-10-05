// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the battle.net handlers moved to
//! `wow-world-lifecycle` (#1263 F5).

use wow_packet::packets::battlenet::{BattlenetRequest, ChangeRealmTicket};
use wow_world_lifecycle::BattlenetHandlerCxLikeCpp;

use crate::session::WorldSession;

impl WorldSession {
    pub async fn handle_battlenet_request(&mut self, req: BattlenetRequest) {
        BattlenetHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_battlenet_request(req)
            .await;
    }

    pub async fn handle_change_realm_ticket(&mut self, ticket: ChangeRealmTicket) {
        BattlenetHandlerCxLikeCpp::new(crate::session::hub_mut(self))
            .handle_change_realm_ticket(ticket)
            .await;
    }
}
