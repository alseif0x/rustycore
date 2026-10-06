// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the gameobject handler context (#1263 F5).
//!
//! The application crate owns the handlers and their context; the session only
//! splits its interaction and world-entity state from the hub.

use wow_world_application::{GameObjectHandlerCxLikeCpp, GameObjectHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl GameObjectHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn gameobject_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> GameObjectHandlerCxLikeCpp<'a> {
        let (interaction, world_entities, hub) =
            crate::session::split_interaction_world_entities_mut(self);
        GameObjectHandlerCxLikeCpp::new(hub, interaction, world_entities)
    }
}
