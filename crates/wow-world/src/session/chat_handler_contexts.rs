// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side construction of the chat handler context (#1263 F5).
//!
//! The social crate owns the handlers and their context; the session only splits
//! its social state from the hub and lends the process chat policy, so no
//! session reference crosses into the handler.
//!
//! `#1263 F5 remaining families`: the adapter also lends `CMSG_SEND_TEXT_EMOTE`,
//! whose body stays in the World shell and needs the session catalog view.

use wow_handler::HandlerFuture;
use wow_packet::WorldPacket;
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

    fn handle_text_emote_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a SessionHandlerCatalogsLikeCpp,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()> {
        Box::pin(async move {
            WorldSession::handle_text_emote_with_catalogs_like_cpp(
                self,
                catalogs.emotes_text.as_ref(),
                catalogs.emotes.as_ref(),
                catalogs.chat_policy.as_ref(),
                pkt,
            )
            .await
        })
    }
}
