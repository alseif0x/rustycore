// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the client-state handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! hands over the mutable hub view, so no session reference crosses into the
//! handler.

use wow_world_application::{ClientStateHandlerCxLikeCpp, ClientStateHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl ClientStateHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn client_state_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> ClientStateHandlerCxLikeCpp<'a> {
        ClientStateHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }
}
