// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the chat handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only splits
//! its social state from the hub and lends the process chat policy, so no
//! session reference crosses into the handler.

use wow_world_social::{ChatHandlerCxLikeCpp, ChatHandlerHostLikeCpp};

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

impl ChatHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn chat_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
    ) -> ChatHandlerCxLikeCpp<'a> {
        let (social, hub) = crate::session::split_social_mut(self);
        ChatHandlerCxLikeCpp::new(hub, social, catalogs.chat_policy.as_ref())
    }
}
