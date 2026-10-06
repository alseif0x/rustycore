// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the vehicle handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub and runs the deferred registry-state publication.

use wow_world_application::{VehicleHandlerCxLikeCpp, VehicleHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl VehicleHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn vehicle_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> VehicleHandlerCxLikeCpp<'a> {
        VehicleHandlerCxLikeCpp::new(crate::session::hub_mut(self))
    }

    fn sync_player_registry_state_after_vehicle_change_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }
}
