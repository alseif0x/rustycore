// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the battle.net handler context (#1263 F5).
//!
//! The lifecycle crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.

use wow_world_lifecycle::{BattlenetHandlerCxLikeCpp, BattlenetHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl BattlenetHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn battlenet_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> BattlenetHandlerCxLikeCpp<'a> {
        BattlenetHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }
}
