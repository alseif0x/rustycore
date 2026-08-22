// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spells, auras, cooldowns and talents.

use super::*;

impl Player {
    pub const fn titan_grip_penalty_spell_id(&self) -> u32 {
        self.titan_grip_penalty_spell_id
    }
}
