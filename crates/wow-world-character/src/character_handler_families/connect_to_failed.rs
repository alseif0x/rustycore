// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character handler family: failed instance connect (`HandleConnectToFailed`).

use tracing::{info, warn};
use wow_packet::packets::auth::{ConnectToFailed, ConnectToSerial};
use wow_packet::packets::character::{CharacterLoginFailed, LoginFailureReasonLikeCpp};

use crate::character_handlers::{CharacterHandlerCxLikeCpp, ConnectToFailedStepLikeCpp};

impl CharacterHandlerCxLikeCpp<'_> {
    /// Handle ConnectToFailed — client couldn't connect to instance port.
    ///
    /// Retry with the next serial, or fall back to direct login if all retries
    /// are exhausted.
    ///
    /// C++ `WorldSocket::HandleConnectToFailed` retries `SendConnectToInstance`
    /// with the next serial and aborts the login with
    /// `LoginFailureReason::NoWorld` once the last attempt failed. The pending
    /// redirect entry lives on the session manager and the retry send on the
    /// session's instance link, so the thunk drops the entry where
    /// [`ConnectToFailedStepLikeCpp`] names it and resumes here for the retry
    /// decision.
    pub async fn handle_connect_to_failed(
        &mut self,
        pkt: ConnectToFailed,
    ) -> ConnectToFailedStepLikeCpp {
        warn!(
            "ConnectToFailed (serial={:?}) from account {}",
            pkt.serial,
            self.hub.shared().core.account_id
        );

        // Clean up the pending entry from SessionManager
        ConnectToFailedStepLikeCpp::DropPendingSessionManagerEntry { serial: pkt.serial }
    }

    /// C++ `WorldSocket::HandleConnectToFailed` tail: the pending session
    /// manager entry is gone, so the represented instance link is cleared and
    /// the next serial is selected.
    pub async fn handle_connect_to_failed_after_pending_entry_like_cpp(
        &mut self,
        serial: ConnectToSerial,
    ) -> ConnectToFailedStepLikeCpp {
        self.hub.core.set_instance_link_rx(None);

        // Try next serial
        if let Some(next_serial) = serial.next() {
            info!("Retrying ConnectTo with serial {:?}", next_serial);
            ConnectToFailedStepLikeCpp::SendConnectTo {
                serial: next_serial,
            }
        } else {
            warn!(
                "All ConnectTo retries exhausted for account {}, aborting login like C++",
                self.hub.shared().core.account_id
            );
            ConnectToFailedStepLikeCpp::AbortLogin
        }
    }

    /// C++ `AbortLogin` tail: the host cleared `m_playerLoading` and the
    /// character login claim, so the `LoginFailureReason::NoWorld` result may be
    /// published on the represented ConnectTo serial state.
    pub fn publish_connect_to_failed_abort_like_cpp(&mut self) {
        self.hub.core.set_connect_to_key(None);
        self.hub.core.set_connect_to_serial(None);
        self.hub.core.send_packet(&CharacterLoginFailed {
            code: LoginFailureReasonLikeCpp::NoWorld,
        });
    }
}
