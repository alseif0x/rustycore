// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character deletion (`HandleCharDeleteOpcode`).

use std::sync::Arc;

use tracing::{info, warn};
use wow_packet::packets::character::{CharDelete, DeleteChar, response_codes};
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;

use crate::character_handlers::{CharDeleteStepLikeCpp, CharacterHandlerCxLikeCpp};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_CHAR_DELETE — delete a character.
    ///
    /// C++ `HandleCharDeleteOpcode` resolves the character-administration port,
    /// refuses a character the account does not own, deletes the row, refreshes
    /// the login-DB `realmcharacters` count and answers. The port and the
    /// refresh belong to the World session's lifecycle state, so the thunk
    /// lends the port and runs the refresh where [`CharDeleteStepLikeCpp`]
    /// names it.
    pub async fn handle_char_delete(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        pkt: CharDelete,
    ) -> CharDeleteStepLikeCpp {
        let port = match port {
            Some(port) => port,
            None => {
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                return CharDeleteStepLikeCpp::Complete;
            }
        };

        // Verify the character belongs to this account
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to delete non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            self.hub.core.send_packet(&DeleteChar {
                code: response_codes::CHAR_DELETE_FAILED,
            });
            return CharDeleteStepLikeCpp::Complete;
        }

        let account_id = self.hub.shared().core.account_id;
        match port
            .delete_owned_character_like_cpp(pkt.guid.counter() as u64, account_id)
            .await
        {
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Applied => {
                info!(
                    "Character {:?} deleted for account {}",
                    pkt.guid, account_id
                );
                self.hub.remove_legit_character(&pkt.guid);

                // Update realmcharacters count in login DB
                CharDeleteStepLikeCpp::RefreshRealmCharacters
            }
            wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to delete character: {reason}");
                self.hub.core.send_packet(&DeleteChar {
                    code: response_codes::CHAR_DELETE_FAILED,
                });
                CharDeleteStepLikeCpp::Complete
            }
        }
    }

    /// C++ `HandleCharDeleteOpcode` tail: the host refreshed the login-DB
    /// `realmcharacters` count, so the success result may be published.
    pub fn publish_char_delete_success_like_cpp(&mut self) {
        self.hub.core.send_packet(&DeleteChar {
            code: response_codes::CHAR_DELETE_SUCCESS,
        });
    }
}
