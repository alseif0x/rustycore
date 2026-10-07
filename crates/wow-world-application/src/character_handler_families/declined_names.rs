// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: declined names (`HandleSetPlayerDeclinedNames`).

use tracing::warn;
use wow_packet::packets::character::{
    DECLINED_NAMES_RESULT_ERROR_LIKE_CPP, SetPlayerDeclinedNames, SetPlayerDeclinedNamesResult,
};
use wow_packet::{ClientPacket, WorldPacket};

use crate::character_handlers::CharacterHandlerCxLikeCpp;

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_SET_PLAYER_DECLINED_NAMES.
    ///
    /// C++ resolves the target character through `sCharacterCache`, requires a
    /// Cyrillic base name, normalizes all five declined forms, validates them
    /// with `ObjectMgr::CheckDeclinedNames`, then replaces the
    /// `character_declinedname` row and returns success. Rust does not yet
    /// carry that character-cache / locale-validation runtime through this
    /// session path, so this bounded seam preserves the parse/dispatch and the
    /// C++ error-result branch instead of fabricating persisted declined names.
    pub async fn handle_set_player_declined_names(&mut self, mut pkt: WorldPacket) {
        let request = match SetPlayerDeclinedNames::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad SetPlayerDeclinedNames: {error}");
                return;
            }
        };

        self.publication_like_cpp()
            .send_packet(&SetPlayerDeclinedNamesResult {
                player: request.player,
                result_code: DECLINED_NAMES_RESULT_ERROR_LIKE_CPP,
            });
    }
}
