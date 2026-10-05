// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the Social handler contexts (#1263 F5).
//!
//! The domain crate owns the handlers and their context type; the session only
//! binds its own state into that context, so the borrowed inputs are read once
//! per invocation and no session reference crosses into the handler.

use wow_world_social::{InspectHandlerCxLikeCpp, SocialInspectHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl SocialInspectHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn inspect_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> InspectHandlerCxLikeCpp<'a> {
        let player_position = crate::session::hub_ref(self).player_position_like_cpp();
        let player_faction_template_id =
            crate::session::hub_ref(self).player_faction_template_id_like_cpp();
        let player_map_id = self.core.player_map_id_like_cpp();
        let registry = self.core.player_registry();
        let canonical_map_manager = self.core.canonical_map_manager.as_ref();
        let publication = self.core.packet_publication_access_like_cpp();
        InspectHandlerCxLikeCpp::new(
            registry,
            canonical_map_manager,
            player_map_id,
            player_position,
            player_faction_template_id,
            publication,
        )
    }
}
