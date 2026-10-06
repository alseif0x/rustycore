// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the loot handler contexts (#1263 F5).
//!
//! The application crate owns the handlers and their contexts; the host lives
//! next to the loot handlers so the session aggregate only exposes what the
//! release context already needs.

use wow_world_application::{LootHandlerCxLikeCpp, LootHandlerHostLikeCpp, LootReleaseCxLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl LootHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn loot_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> LootHandlerCxLikeCpp<'a> {
        let (loot, hub) = crate::session::split_loot_mut(self);
        LootHandlerCxLikeCpp::new(hub, loot)
    }

    fn loot_release_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> LootReleaseCxLikeCpp<'a> {
        self.loot_release_cx_like_cpp()
    }
}
