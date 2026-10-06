// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the character handler context (#1263 F5).
//!
//! The application crate owns the bodies and their context; the session only
//! lends its hub, inventory and world-entity state.

use wow_world_application::{CharacterHandlerCxLikeCpp, CharacterHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl CharacterHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn character_handler_cx_like_cpp<'a>(
        &'a mut self,
        _catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> CharacterHandlerCxLikeCpp<'a> {
        let (inventory, world_entities, hub) = crate::session::split_character_handler_mut(self);
        CharacterHandlerCxLikeCpp::new(hub, inventory, world_entities)
    }
}
