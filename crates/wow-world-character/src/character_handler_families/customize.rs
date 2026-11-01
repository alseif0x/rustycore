// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: character customization (`HandleCharCustomizeOpcode`).

use std::sync::Arc;

use tracing::{info, warn};
use wow_constants::character::{AT_LOGIN_CUSTOMIZE_LIKE_CPP, RESPONSE_SUCCESS_LIKE_CPP};
use wow_core::ObjectGuid;
use wow_packet::packets::character::{CharCustomize, CharCustomizeFailure, CharCustomizeSuccess};
use wow_persistence::CharacterAdministrationPersistencePortLikeCpp;

use crate::character_handlers::{
    CHAR_CREATE_ERROR_LIKE_CPP, CHAR_CREATE_NAME_IN_USE_LIKE_CPP, CharacterHandlerCxLikeCpp,
};

impl CharacterHandlerCxLikeCpp<'_> {
    fn send_char_customize_failure_like_cpp(&self, result: u8, guid: ObjectGuid) {
        self.hub
            .core
            .send_packet(&CharCustomizeFailure { result, guid });
    }

    fn send_char_customize_success_like_cpp(&self, request: &CharCustomize) {
        self.hub.core.send_packet(&CharCustomizeSuccess {
            guid: request.guid,
            sex_id: request.sex_id,
            customizations: request.customizations.clone(),
            name: request.name.clone(),
        });
    }

    /// Handle CMSG_CHAR_CUSTOMIZE.
    ///
    /// C++ `HandleCharCustomizeOpcode` refuses a character the account does not
    /// own, loads the at-login candidate, requires `AT_LOGIN_CUSTOMIZE`,
    /// validates the new name, clears the flag and commits the customization.
    /// The character-administration port belongs to the World session's
    /// lifecycle state, so the thunk lends it.
    pub async fn handle_char_customize(
        &mut self,
        port: Option<Arc<dyn CharacterAdministrationPersistencePortLikeCpp>>,
        request: CharCustomize,
    ) {
        if !self.hub.shared().core.is_legit_character(&request.guid) {
            warn!(
                "Account {} tried to customize non-owned character {:?}",
                self.hub.shared().core.account_id,
                request.guid
            );
            self.hub.core.kick(
                "WorldSession::HandleCharCustomize Trying to customise character of another account",
            );
            return;
        }

        let port = match port {
            Some(port) => port,
            None => {
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        let candidate = match port
            .load_customize_candidate_like_cpp(request.guid.counter() as u64)
            .await
        {
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(candidate) => {
                candidate
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::NotFound => {
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
            wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Character customize info query failed: {reason}");
                self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
                return;
            }
        };

        let old_name = candidate.old_name;
        let mut at_login_flags = candidate.at_login_flags;
        if (at_login_flags & AT_LOGIN_CUSTOMIZE_LIKE_CPP) == 0 {
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        let name_result =
            wow_entities::represented_character_rename_name_result_like_cpp(&request.name);
        if name_result != RESPONSE_SUCCESS_LIKE_CPP {
            self.send_char_customize_failure_like_cpp(name_result, request.guid);
            return;
        }

        if request.name != old_name {
            match port.find_character_name_like_cpp(&request.name).await {
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Loaded(()) => {
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_NAME_IN_USE_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Character customize name query failed: {reason}");
                    self.send_char_customize_failure_like_cpp(
                        CHAR_CREATE_ERROR_LIKE_CPP,
                        request.guid,
                    );
                    return;
                }
                wow_persistence::CharacterAdministrationLoadOutcomeLikeCpp::NotFound => {}
            }
        }

        at_login_flags &= !AT_LOGIN_CUSTOMIZE_LIKE_CPP;

        let customizations = request
            .customizations
            .iter()
            .map(
                |choice| wow_persistence::CharacterCustomizationPersistenceLikeCpp {
                    option_id: choice.option_id,
                    choice_id: choice.choice_id,
                },
            )
            .collect();
        if let wow_persistence::CharacterAdministrationMutationOutcomeLikeCpp::Failed { reason } =
            port.commit_customize_like_cpp(
                request.guid.counter() as u64,
                &request.name,
                at_login_flags,
                customizations,
            )
            .await
        {
            warn!("Character customize transaction failed: {reason}");
            self.send_char_customize_failure_like_cpp(CHAR_CREATE_ERROR_LIKE_CPP, request.guid);
            return;
        }

        info!(
            "Account {} customized character {:?} from {} to {}",
            self.hub.shared().core.account_id,
            request.guid,
            old_name,
            request.name
        );
        self.send_char_customize_success_like_cpp(&request);
    }
}
