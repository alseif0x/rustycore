// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the support handler context (#1263 F5).
//!
//! The lifecycle crate owns the handlers and their context; the session only
//! splits its lifecycle state from the hub and lends the driver's support
//! policy, so no session reference crosses into the handler.

use wow_world_lifecycle::{SupportHandlerCxLikeCpp, SupportHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl SupportHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn support_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> SupportHandlerCxLikeCpp<'a> {
        let (lifecycle, hub) = crate::session::split_lifecycle_mut(self);
        SupportHandlerCxLikeCpp::new(lifecycle, hub, catalogs.support_feature_policy.as_ref())
    }
}
