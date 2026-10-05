// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the arena-team handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only
//! splits its social state from the hub.

use wow_world_social::{ArenaTeamHandlerCxLikeCpp, ArenaTeamHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl ArenaTeamHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn arena_team_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> ArenaTeamHandlerCxLikeCpp<'a> {
        let (social, hub) = crate::session::split_social_mut(self);
        ArenaTeamHandlerCxLikeCpp::new(hub, social)
    }
}
