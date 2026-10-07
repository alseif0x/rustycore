// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the movement handler context (#1263 F5).
//!
//! The application crate owns the C++ `MovementHandler.cpp` bodies and their
//! context; the session only lends its hub, so no session reference crosses
//! into the handler.

use wow_world_application::{MovementHandlerCxLikeCpp, MovementHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl MovementHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn movement_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> MovementHandlerCxLikeCpp<'a> {
        MovementHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }
}
