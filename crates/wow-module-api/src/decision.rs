// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pre-decision results.
//!
//! A confirmed-result notification observes what already happened and cannot
//! veto it. A pre-decision hook runs at a named point *before* the canonical
//! transition and may allow, deny or modify it. Source API 2 declares the
//! decision shape here; the first hook that uses it is `chat.message` (#1444
//! H4), modelled on AzerothCore `PlayerScript::OnPlayerCanUseChat`
//! (`PlayerScript.h:657-709`).

/// What a module decided about a chat message.
///
/// Decisions compose in `ModuleId` order: the first `Deny` wins and stops the
/// message; each `ReplaceText` feeds the next module; the owner re-validates a
/// replaced text with the same rules as client text before using it.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ChatDecision {
    /// Leave the message as it is.
    #[default]
    Allow,
    /// Drop the message. Nothing is broadcast.
    Deny,
    /// Continue with this text instead.
    ReplaceText(String),
}
