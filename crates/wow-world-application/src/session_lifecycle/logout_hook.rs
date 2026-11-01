// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The `player.logout` hook event (#1444 H1).

use wow_core::ObjectGuid;
use wow_world_lifecycle::FinalizationMode;

/// One character leaving the world, raised by the finalization ledger.
///
/// TrinityCore `sScriptMgr->OnPlayerLogout(_player)` (`WorldSession.cpp:655`);
/// AzerothCore `PlayerScript::OnPlayerLogout` (`PlayerScript.h:338`, called
/// from `WorldSession.cpp:828`). Both run after `SaveToDB` and the social and
/// group cleanup, and before `RemovePlayerFromMap`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerLogoutHookEventLikeCpp {
    pub guid: ObjectGuid,
    pub account_id: u32,
    /// Which `LogoutPlayer` caller this is: back to character selection, the
    /// timed logout, or a disconnect.
    pub mode: FinalizationMode,
}

#[cfg(test)]
#[path = "../../unit_tests/session_lifecycle/logout_hook.rs"]
mod tests;
