// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.creature_kill`, the kill hook port (#1444 H3).
//!
//! This crate never sees the public module API. The kill completion raises a
//! plain event through a provided [`crate::WorldEntitiesHostLikeCpp`] method
//! whose default does nothing; the World overrides it and dispatches to the
//! linked modules. It is raised once per `setDeathState(JUST_DIED)`, after
//! the canonical map lock that transition took was released.

use wow_core::ObjectGuid;

/// `player.creature_kill`: TrinityCore `Unit::Kill` -> `OnCreatureKill`
/// (`Unit.cpp:10755`); AzerothCore `OnPlayerCreatureKill` (`Unit.cpp:14298`,
/// `PlayerScript.h:254`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatureKillHookEventLikeCpp {
    /// The killing player: the session's own.
    pub guid: ObjectGuid,
    pub creature_guid: ObjectGuid,
    pub creature_entry: u32,
    pub creature_level: u8,
    pub map_id: u32,
}
