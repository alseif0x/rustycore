// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the trade handler context (#1263 F5).
//!
//! The application crate owns the bodies and their context; the session only
//! lends its hub, social state and inventory state, so no session reference
//! crosses into the handler.

use wow_world_application::{TradeHandlerCxLikeCpp, TradeHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl TradeHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn trade_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> TradeHandlerCxLikeCpp<'a> {
        let (social, inventory, spell_state, hub) = crate::session::split_trade_mut(self);
        TradeHandlerCxLikeCpp::new(hub, social, inventory, spell_state)
    }
}
