// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the hotfix/DB2 handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session
//! lends its hub plus the two read-only catalog stores, so no session
//! reference crosses into the handler.

use wow_world_application::{DataServiceHandlerCxLikeCpp, DataServiceHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl DataServiceHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn data_service_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> DataServiceHandlerCxLikeCpp<'a> {
        DataServiceHandlerCxLikeCpp::new(
            crate::session::hub_mut(self),
            catalogs.hotfixes.as_ref(),
            catalogs.tact_keys.as_ref(),
        )
    }
}
