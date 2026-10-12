// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character undelete cooldown status.

use crate::character_handlers::CharacterHandlerCxLikeCpp;

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_GET_UNDELETE_CHARACTER_COOLDOWN_STATUS.
    ///
    /// The client sends this when it wants to know if character undelete is
    /// available. We always respond with "no cooldown" (undelete available).
    pub async fn handle_get_undelete_cooldown_status(&mut self) {
        self.publication_like_cpp()
            .send_packet(&wow_packet::packets::misc::UndeleteCooldownStatusResponse::no_cooldown());
    }
}
