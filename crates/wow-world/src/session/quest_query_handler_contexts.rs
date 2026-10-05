// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the quest query handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! lends its hub and the read-only quest store.

use wow_world_application::{QuestQueryHandlerCxLikeCpp, QuestQueryHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl QuestQueryHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn quest_query_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> QuestQueryHandlerCxLikeCpp<'a> {
        let quest_store = self.catalogs.quests.store.clone();
        QuestQueryHandlerCxLikeCpp::new(crate::session::hub_mut(self), quest_store)
    }
}
