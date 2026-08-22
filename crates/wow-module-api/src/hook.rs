// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `player.login` hook: its immutable snapshot and module trait.

use wow_core::ObjectGuid;

use crate::effect::ScopedEffects;

/// The immutable facts a login module may read.
///
/// C++ hands `PlayerScript::OnLogin` a live `Player*` and a `firstLogin` flag
/// (`ScriptMgr.h:764`). A trusted-but-external module gets a copy instead: it
/// can decide, but it cannot mutate the player, reach the session, or retain a
/// reference that outlives the hook.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerLoginSnapshot {
    pub guid: ObjectGuid,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub map_id: u16,
    /// C++ `firstLogin`: the character had `AT_LOGIN_FIRST` when it entered.
    pub first_login: bool,
}

/// A module that wants to observe completed logins.
pub trait PlayerLoginModule: Send + Sync {
    /// Mirrors C++ `PlayerScript::OnLogin`, invoked once per completed login.
    ///
    /// Effects are queued, never applied here: the owner validates the whole
    /// batch first, so returning an invalid effect changes nothing.
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>);
}
