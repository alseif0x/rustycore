// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: player login (`HandlePlayerLoginOpcode`).

use tracing::warn;
use wow_core::ObjectGuid;
use wow_packet::packets::character::{
    CharacterLoginFailed, LoginFailureReasonLikeCpp, PlayerLogin,
};

use crate::character_handlers::{CharacterHandlerCxLikeCpp, PlayerLoginStepLikeCpp};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle CMSG_PLAYER_LOGIN — initiate ConnectTo flow.
    ///
    /// Instead of sending the login sequence directly, we send SMSG_CONNECT_TO
    /// to redirect the client to the instance port. The login sequence is sent
    /// after the client reconnects via `handle_continue_player_login`.
    ///
    /// C++ `HandlePlayerLoginOpcode` refuses a session that already has a
    /// character loading or active, refuses a character the account does not
    /// own, then hands the character to `SendConnectToInstance`. The represented
    /// `m_playerLoading` lives on the World session's lifecycle state, the live
    /// character claim on that state's login claim, and the ConnectTo send on
    /// the session's instance link, so the thunk lends `player_loading` and
    /// runs the claim and the send where [`PlayerLoginStepLikeCpp`] names them.
    pub async fn handle_player_login(
        &mut self,
        player_loading: Option<ObjectGuid>,
        pkt: PlayerLogin,
    ) -> PlayerLoginStepLikeCpp {
        if player_loading.is_some() || self.hub.shared().core.player_guid().is_some() {
            warn!(
                account = self.hub.shared().core.account_id,
                "Player tried to login while another character is loading or active"
            );
            self.hub
                .core
                .kick("WorldSession::HandlePlayerLoginOpcode Another client logging in");
            return PlayerLoginStepLikeCpp::Complete;
        }

        // Verify character ownership
        if !self.hub.shared().core.is_legit_character(&pkt.guid) {
            warn!(
                "Account {} tried to login with non-owned character {:?}",
                self.hub.shared().core.account_id,
                pkt.guid
            );
            return PlayerLoginStepLikeCpp::Complete;
        }

        // C++ exposes one live `Player*` per character GUID through
        // ObjectAccessor. Claim that ownership before ConnectTo/DB loading so
        // two sessions cannot become independent save authorities.
        PlayerLoginStepLikeCpp::ClaimCharacterLogin { guid: pkt.guid }
    }

    /// C++ `ObjectAccessor` live-character claim tail: the session refused the
    /// claim, so the duplicate-live-character login failure is published.
    pub fn publish_player_login_duplicate_character_like_cpp(&mut self, guid: ObjectGuid) {
        warn!(
            account = self.hub.shared().core.account_id,
            guid = ?guid,
            "Rejecting duplicate live-character login"
        );
        self.hub.core.send_packet(&CharacterLoginFailed {
            code: LoginFailureReasonLikeCpp::DuplicateCharacter,
        });
    }
}
