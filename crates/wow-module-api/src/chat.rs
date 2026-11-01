// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `chat.*` hook family (source API 2, #1444 H4): a pre-decision.
//!
//! Modelled on AzerothCore `PlayerScript::OnPlayerCanUseChat`
//! (`PlayerScript.h:657-709`), which may veto a message or change its text,
//! and on TrinityCore `PlayerScript::OnChat` (`ScriptMgr.h:745-753`), which
//! may change it. The owner asks after its own validation and the sender's
//! level and silence checks, right before the message is sent; a replaced
//! text is validated again with the client-text rules before it is used.

use wow_core::ObjectGuid;

use crate::decision::ChatDecision;

/// Which chat the message was written to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ChatKind {
    Say,
    Yell,
    /// A free-text emote (`CMSG_CHAT_MESSAGE_EMOTE`).
    Emote,
    Party,
    Raid,
    RaidWarning,
    InstanceChat,
    Whisper,
}

/// `chat.message`: a player is about to send a chat message.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ChatMessageSnapshot {
    pub guid: ObjectGuid,
    pub kind: ChatKind,
    /// The client language id (`LANG_*`).
    pub language: u32,
    /// The text so far: a module earlier in `ModuleId` order may already
    /// have replaced it.
    pub text: String,
    /// The receiver of a whisper; `None` for every other kind.
    pub receiver: Option<ObjectGuid>,
}

impl ChatMessageSnapshot {
    #[must_use]
    pub fn new(
        guid: ObjectGuid,
        kind: ChatKind,
        language: u32,
        text: impl Into<String>,
        receiver: Option<ObjectGuid>,
    ) -> Self {
        Self {
            guid,
            kind,
            language,
            text: text.into(),
            receiver,
        }
    }
}

/// The `chat.*` family.
pub trait ChatModule: Send + Sync {
    /// AzerothCore `PlayerScript::OnPlayerCanUseChat`: return
    /// [`ChatDecision::Deny`] to drop the message or
    /// [`ChatDecision::ReplaceText`] to send another text instead.
    fn on_chat_message(&self, _snapshot: &ChatMessageSnapshot) -> ChatDecision {
        ChatDecision::Allow
    }
}

/// What one `chat.message` dispatch decided, composed over every module.
#[derive(Debug)]
#[must_use]
pub struct ChatOutcome {
    /// `Allow`, `Deny`, or `ReplaceText` with the final text.
    pub decision: ChatDecision,
    /// The module that denied the message, if one did.
    pub denied_by: Option<crate::ModuleId>,
    /// Modules a panic disabled during this dispatch. A panicking module's
    /// decision is discarded; the message goes on with the others'.
    pub disabled: Vec<crate::ModuleId>,
}
