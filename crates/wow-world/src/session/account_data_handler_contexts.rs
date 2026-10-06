// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the account-data handler context (#1263 F5).
//!
//! The lifecycle crate owns the handlers and their context; the session only
//! splits its lifecycle state from the hub so both disjoint borrows reach the
//! context and no session reference crosses into the handler.

use wow_world_lifecycle::{AccountDataHandlerCxLikeCpp, AccountDataHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl AccountDataHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn account_data_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> AccountDataHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        AccountDataHandlerCxLikeCpp::new(lifecycle, hub)
    }
}
