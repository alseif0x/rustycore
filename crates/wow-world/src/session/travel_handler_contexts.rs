// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the travel handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session
//! splits its lifecycle state from the hub, so no session reference crosses
//! into the handler.

use wow_world_application::{TravelHandlerCxLikeCpp, TravelHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl TravelHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn travel_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> TravelHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        TravelHandlerCxLikeCpp::new(hub, lifecycle)
    }
}
