// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player's own away-status transitions.
//!
//! C++ splits this deliberately: `Player::ToggleAFK` (Player.cpp:1201) and
//! `Player::ToggleDND` (Player.cpp:1213) only flip their flag, while the
//! auto-reply message is assigned by the chat handler
//! (ChatHandler.cpp:600..604 and :646..650). The split is kept here so the
//! session composes the same two steps instead of reaching into the Player's
//! gameplay state to write the message itself.

use crate::Player;

/// C++ `PLAYER_FLAGS_AFK`.
const PLAYER_FLAGS_AFK_LIKE_CPP: u32 = 0x0000_0002;
/// C++ `PLAYER_FLAGS_DND`.
const PLAYER_FLAGS_DND_LIKE_CPP: u32 = 0x0000_0004;

impl Player {
    /// C++ `Player::ToggleAFK` (Player.cpp:1201).
    ///
    /// Unimplemented participant: Classic also leaves a battleground when a
    /// non-GM player goes AFK outside an arena. That belongs to the
    /// battleground port, not to this transition.
    pub fn toggle_afk_like_cpp(&mut self) {
        if self.has_player_flag(PLAYER_FLAGS_AFK_LIKE_CPP) {
            self.remove_player_flag(PLAYER_FLAGS_AFK_LIKE_CPP);
        } else {
            self.set_player_flag(PLAYER_FLAGS_AFK_LIKE_CPP);
        }
    }

    /// C++ `Player::ToggleDND` (Player.cpp:1213).
    pub fn toggle_dnd_like_cpp(&mut self) {
        if self.has_player_flag(PLAYER_FLAGS_DND_LIKE_CPP) {
            self.remove_player_flag(PLAYER_FLAGS_DND_LIKE_CPP);
        } else {
            self.set_player_flag(PLAYER_FLAGS_DND_LIKE_CPP);
        }
    }

    /// C++ `sender->autoReplyMsg = ...` as the chat handler assigns it.
    pub fn set_auto_reply_message_like_cpp(&mut self, message: String) {
        self.gameplay_state_mut().social.auto_reply_msg_like_cpp = message;
    }

    pub fn is_afk_like_cpp(&self) -> bool {
        self.has_player_flag(PLAYER_FLAGS_AFK_LIKE_CPP)
    }

    pub fn is_dnd_like_cpp(&self) -> bool {
        self.has_player_flag(PLAYER_FLAGS_DND_LIKE_CPP)
    }
}
