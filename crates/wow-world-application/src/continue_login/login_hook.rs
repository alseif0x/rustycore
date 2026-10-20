// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `player.login` hook event (#1445 H0).
//!
//! A hook port is a provided host-trait method with an empty default body that
//! takes a plain domain event. This crate never sees the public module API:
//! the World adapter converts the event into a `wow-module-api` snapshot,
//! dispatches it and applies the validated effects through the canonical
//! owners. With no adapter, as in every test host, the port is a no-op.

use wow_core::ObjectGuid;
use wow_world_core::session::HubRef;

/// The facts of one completed login, read after the login finished.
///
/// TrinityCore `sScriptMgr->OnPlayerLogin(pCurrChar, firstLogin)`
/// (`CharacterHandler.cpp:1390`); AzerothCore `PlayerScript::OnPlayerLogin`
/// (`PlayerScript.h:332`) and `OnPlayerFirstLogin` (`PlayerScript.h:481`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerLoginHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub level: u8,
    pub map_id: u16,
    /// C++ `firstLogin`: the character had `AT_LOGIN_FIRST` when it entered.
    pub first_login: bool,
}

impl PlayerLoginHookEventLikeCpp {
    /// Read the event from the session hub. `None` without a selected player,
    /// in which case C++ never reaches its call either.
    pub fn from_hub_like_cpp(hub: HubRef<'_>, first_login: bool) -> Option<Self> {
        let guid = hub.core.player_guid()?;
        Some(Self {
            guid,
            name: hub.player_name_like_cpp().unwrap_or_default(),
            race: hub.player_race_like_cpp(),
            class: hub.player_class_like_cpp(),
            level: hub.player_level_like_cpp(),
            map_id: hub.core.player_map_id_like_cpp(),
            first_login,
        })
    }
}

#[cfg(test)]
#[path = "../../unit_tests/continue_login/login_hook.rs"]
mod tests;
