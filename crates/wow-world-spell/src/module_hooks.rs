// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.spell_cast`, the cast hook port (#1444 H3).
//!
//! This crate never sees the public module API. The spell execution raises a
//! plain event through a provided [`crate::SpellExecutionHostLikeCpp`] method
//! whose default does nothing; the World overrides it and dispatches to the
//! linked modules. #754: the session is the only writer of the player's spell
//! map, and no module effect can change spells or auras.

use wow_core::ObjectGuid;

/// `player.spell_cast`: TrinityCore `Spell::_cast` -> `OnPlayerSpellCast`
/// (`Spell.cpp:3671`); AzerothCore `OnPlayerSpellCast` (`Spell.cpp:3825`,
/// `PlayerScript.h:326`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpellCastHookEventLikeCpp {
    /// The casting player: the session's own.
    pub guid: ObjectGuid,
    pub spell_id: u32,
    pub cast_id: ObjectGuid,
    pub target_guid: ObjectGuid,
    /// The cast carries C++ `TRIGGERED_IGNORE_GCD`, as every
    /// `CastSpell(..., true)` (`TRIGGERED_FULL_MASK`) cast does; the only
    /// trigger flag the cast metadata represents.
    pub triggered: bool,
}
