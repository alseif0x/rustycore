// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the reputation handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! splits the hub from the faction catalogs so both disjoint borrows reach the
//! context and no session reference crosses into the handler.

use wow_world_application::{ReputationHandlerCxLikeCpp, ReputationHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl ReputationHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn reputation_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> ReputationHandlerCxLikeCpp<'a> {
        self.build_reputation_handler_cx_like_cpp()
    }
}

impl WorldSession {
    pub(crate) fn build_reputation_handler_cx_like_cpp(
        &mut self,
    ) -> ReputationHandlerCxLikeCpp<'_> {
        // The previous in-tree handlers cloned both catalog handles per
        // invocation; the borrowed context preserves that exact behavior.
        let faction_store = self.catalogs.faction_store().cloned();
        let friendship_rep_reaction_store = self.catalogs.friendship_rep_reaction_store().cloned();
        ReputationHandlerCxLikeCpp::new(
            crate::session::hub_mut(self),
            faction_store,
            friendship_rep_reaction_store,
        )
    }
}
