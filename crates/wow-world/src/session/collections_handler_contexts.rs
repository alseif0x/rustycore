// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the collections handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub, so no session reference crosses into the handler.

use wow_world_application::{CollectionsHandlerCxLikeCpp, CollectionsHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CollectionsHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn collections_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CollectionsHandlerCxLikeCpp<'a> {
        let (inventory, hub) = crate::session::split_inventory_mut(self);
        CollectionsHandlerCxLikeCpp::new(hub, inventory)
    }
}
