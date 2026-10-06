// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Social command handlers that remain in the World session.
//!
//! The contact and social-list packet family (AddFriend, AddIgnore, DelFriend,
//! DelIgnore, SendContactList, SetContactNotes, SocialContractRequest,
//! AcceptSocialContract, AccountNotificationAcknowledged) moved to
//! `wow-world-social` in #1263 F5; this module keeps the represented trade and
//! duel command handlers the session mailbox drives.

use crate::session::WorldSession;
use crate::session::mailbox::{
    CancelRepresentedTradeLikeCppCommand, SendRepresentedDuelCountdownLikeCppCommand,
    SendRepresentedTradeStatusLikeCppCommand, UnacceptRepresentedTradeLikeCppCommand,
};

#[cfg(test)]
use wow_packet::packets::social::{
    AcceptSocialContract, AccountNotificationAcknowledged, DelIgnore,
};

#[cfg(test)]
mod test_shims;

// ── handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    pub(crate) fn handle_cancel_represented_trade_command_like_cpp(
        &mut self,
        command: CancelRepresentedTradeLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        self.social
            .record_represented_trade_cancel_like_cpp(command.status);
        if !self.clear_represented_active_trade_partner_like_cpp() {
            return;
        }
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_trade_status_command_like_cpp(
        &mut self,
        command: SendRepresentedTradeStatusLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_unaccept_represented_trade_command_like_cpp(
        &mut self,
        command: UnacceptRepresentedTradeLikeCppCommand,
    ) {
        if !matches!(
            self.resolved_represented_active_trade_partner_like_cpp(),
            Some(Some(_))
        ) {
            return;
        }

        if !self.set_represented_trade_accepted_like_cpp_for_command(false) {
            return;
        }
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_duel_countdown_command_like_cpp(
        &mut self,
        command: SendRepresentedDuelCountdownLikeCppCommand,
    ) {
        self.send_raw_packet(&command.packet_bytes);
    }
}

#[cfg(test)]
#[path = "../../unit_tests/handlers/social/tests/mod.rs"]
mod tests;
