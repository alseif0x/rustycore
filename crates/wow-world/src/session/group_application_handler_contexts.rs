// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the cross-domain group handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! splits its social and lifecycle state from the hub.

use wow_world_application::{GroupHandlerCxLikeCpp, GroupHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl GroupHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn group_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GroupHandlerCxLikeCpp<'a> {
        let (social, lifecycle, hub) = crate::session::split_social_lifecycle_mut(self);
        GroupHandlerCxLikeCpp::new(social, lifecycle, hub)
    }

    fn sync_player_registry_state_after_group_subgroup_like_cpp(&mut self) {
        self.sync_player_registry_state_like_cpp();
    }

    fn refresh_visible_gameobjects_or_spell_clicks_after_group_change_like_cpp(&mut self) {
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }
}
