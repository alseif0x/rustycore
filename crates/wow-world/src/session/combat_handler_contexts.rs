// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the combat handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends `CMSG_ATTACKSWING`,
//! whose body and shell-owned admission seam stay on `WorldSession`.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
use wow_world_application::{CombatHandlerCxLikeCpp, CombatHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CombatHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn combat_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CombatHandlerCxLikeCpp<'a> {
        CombatHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn handle_attack_swing<'a>(&'a mut self, pkt: WorldPacket) -> HandlerFuture<'a, ()> {
        Box::pin(async move { WorldSession::handle_attack_swing(self, pkt).await })
    }
}
