// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character rename (`HandleCharRenameOpcode`).

use std::sync::Arc;

use tracing::warn;
use wow_constants::character::RESPONSE_SUCCESS_LIKE_CPP;
use wow_core::ObjectGuid;
use wow_packet::packets::character::{CharacterRenameRequest, CharacterRenameResult};
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;

use crate::character_handlers::{
    CHAR_CREATE_ERROR_LIKE_CPP, CharRenameStepLikeCpp, CharacterHandlerCxLikeCpp,
};

impl CharacterHandlerCxLikeCpp<'_> {
    fn send_character_rename_like_cpp(
        &self,
        result: u8,
        guid: ObjectGuid,
        new_name: impl Into<String>,
    ) {
        let name = new_name.into();
        self.hub.core.send_packet(&CharacterRenameResult {
            result,
            name,
            guid: (result == RESPONSE_SUCCESS_LIKE_CPP).then_some(guid),
        });
    }

    /// Handle CMSG_CHARACTER_RENAME_REQUEST.
    ///
    /// C++ `HandleCharRenameOpcode` refuses a character the account does not
    /// own, validates the new name, resolves the character-administration port
    /// and submits the read for a commit admitted only in its ready callback.
    /// The port and the callback rail belong to the World session's lifecycle
    /// state, so the thunk lends the port and submits where
    /// [`CharRenameStepLikeCpp`] names it.
    pub async fn handle_character_rename_request(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharacterRenameRequest,
    ) -> CharRenameStepLikeCpp {
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to rename non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.kick(
                "WorldSession::HandleCharRenameOpcode rename character from a different account",
            );
            return CharRenameStepLikeCpp::Complete;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&pkt.new_name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_character_rename_like_cpp(name_result, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        }

        let Some(port) = port else {
            self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, pkt.guid, pkt.new_name);
            return CharRenameStepLikeCpp::Complete;
        };

        CharRenameStepLikeCpp::SubmitCharacterRename {
            port,
            guid: pkt.guid,
            new_name: pkt.new_name,
        }
    }

    /// C++ `HandleCharRenameOpcode` tail: the session's rename callback rail
    /// refused the read, so the `CHAR_CREATE_ERROR` result is published.
    pub fn publish_character_rename_refusal_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_name: String,
    ) {
        self.send_character_rename_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, guid, new_name);
    }
}
