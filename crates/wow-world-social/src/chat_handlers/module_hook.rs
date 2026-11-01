// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `chat.message` for [`ChatHandlerCxLikeCpp`](super::ChatHandlerCxLikeCpp)
//! (#1444 H4).

use super::*;
use crate::{ChatHookVerdictLikeCpp, ChatMessageHookEventLikeCpp};

/// C++ `WorldSession::HandleChatMessage` refuses a client text above 511
/// bytes (`ChatHandler.cpp:228`); a replaced text gets the same limit.
const MAX_CHAT_TEXT_BYTES_LIKE_CPP: usize = 511;

impl ChatHandlerCxLikeCpp<'_> {
    /// Ask the linked modules about a message that passed every C++ check and
    /// is about to be sent, where AzerothCore calls `OnPlayerCanUseChat`.
    ///
    /// Returns the text to send, or `None` when the message is dropped. A
    /// replaced text goes through the client-text rules again (length,
    /// `ValidateMessage`, emptiness and hyperlink shape, `ChatHandler.cpp:228-242`);
    /// one that fails them drops the message rather than fall back to the
    /// text a module wanted replaced. With no chat module this returns the
    /// text untouched, without a copy.
    pub(super) fn module_chat_text_like_cpp(
        &self,
        msg_type: ChatMsg,
        language: u32,
        text: String,
        receiver: Option<ObjectGuid>,
    ) -> Option<String> {
        if !self.module_hooks.chat_hooks_wanted_like_cpp() {
            return Some(text);
        }
        let Some(guid) = self.hub.shared().core.player_guid() else {
            return Some(text);
        };
        let event = ChatMessageHookEventLikeCpp {
            guid,
            msg_type,
            language,
            text,
            receiver,
        };
        match self.module_hooks.chat_message_like_cpp(&event) {
            ChatHookVerdictLikeCpp::Allow => Some(event.text),
            ChatHookVerdictLikeCpp::Deny => {
                debug!(
                    account = self.hub.shared().core.account_id,
                    ty = ?msg_type,
                    "Chat message dropped by a linked module"
                );
                None
            }
            ChatHookVerdictLikeCpp::Replace(mut replaced) => {
                let valid = replaced.len() <= MAX_CHAT_TEXT_BYTES_LIKE_CPP
                    && !replaced.is_empty()
                    && validate_message_like_cpp(
                        &mut replaced,
                        self.policy.fake_message_preventing,
                    )
                    && !replaced.is_empty()
                    && check_all_links_shape_like_cpp(&replaced);
                if valid {
                    Some(replaced)
                } else {
                    warn!(
                        account = self.hub.shared().core.account_id,
                        ty = ?msg_type,
                        "A linked module replaced a chat text with one the client rules refuse; message dropped"
                    );
                    None
                }
            }
        }
    }
}
