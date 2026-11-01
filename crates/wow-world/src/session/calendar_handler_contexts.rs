// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the calendar handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only splits
//! its social state from the hub, so no session reference crosses into the
//! handler.

use wow_world_social::{CalendarHandlerCxLikeCpp, CalendarHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CalendarHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn calendar_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CalendarHandlerCxLikeCpp<'a> {
        self.build_calendar_handler_cx_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn build_calendar_handler_cx_like_cpp(&mut self) -> CalendarHandlerCxLikeCpp<'_> {
        let (social, hub) = crate::session::split_social_mut(self);
        CalendarHandlerCxLikeCpp::new(hub, social)
    }
}
