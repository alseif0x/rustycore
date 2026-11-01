// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Hook port of the chat handlers (#1444 H4).
//!
//! This crate never sees the public module API. The chat handlers run inside
//! a borrowed context ([`crate::ChatHandlerCxLikeCpp`]) with no host to call,
//! so the World lends the context a [`ChatModuleHookPortLikeCpp`] when it
//! builds it, as it does for `QuestRewardCx`. The World adapter
//! (`wow-world` `session/module_hooks.rs`) converts the event, asks the
//! linked modules and returns their composed verdict; the handler applies it.

use wow_core::ObjectGuid;
use wow_packet::packets::chat::ChatMsg;

/// `chat.message`: AzerothCore `PlayerScript::OnPlayerCanUseChat`
/// (`PlayerScript.h:657-709`), TrinityCore `PlayerScript::OnChat`
/// (`ScriptMgr.h:745-753`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessageHookEventLikeCpp {
    pub guid: ObjectGuid,
    /// The requested chat type: `Say`, `Yell`, `Emote`, `Party`, `Raid`,
    /// `RaidWarning`, `InstanceChat` or `Whisper`.
    pub msg_type: ChatMsg,
    pub language: u32,
    pub text: String,
    /// The whisper receiver.
    pub receiver: Option<ObjectGuid>,
}

/// What the linked modules decided about one chat message.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ChatHookVerdictLikeCpp {
    /// Send the message unchanged.
    #[default]
    Allow,
    /// Drop it: nothing is sent, as when C++ `OnPlayerCanUseChat` returns
    /// false.
    Deny,
    /// Send this text instead, once it passes the client-text rules again.
    Replace(String),
}

/// The hook port a chat handler context raises `chat.message` through.
pub trait ChatModuleHookPortLikeCpp: Sync {
    /// Whether any linked module wants `chat.*`. Asked before an event is
    /// built, so a message nobody inspects costs no copy.
    fn chat_hooks_wanted_like_cpp(&self) -> bool {
        false
    }

    fn chat_message_like_cpp(
        &self,
        _event: &ChatMessageHookEventLikeCpp,
    ) -> ChatHookVerdictLikeCpp {
        ChatHookVerdictLikeCpp::Allow
    }
}
