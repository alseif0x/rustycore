// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player difficulty preferences.
//!
//! C++ `Player` owns `m_dungeonDifficulty`, `m_raidDifficulty` and
//! `m_legacyRaidDifficulty`, and reads and writes each through its own
//! accessors: `GetDungeonDifficultyID` (`Player.h:1961`),
//! `GetRaidDifficultyID` (`:1962`), `GetLegacyRaidDifficultyID` (`:1963`),
//! `SetDungeonDifficultyID` (`:1964`), `SetRaidDifficultyID` (`:1965`) and
//! `SetLegacyRaidDifficultyID` (`:1966`).
//!
//! The per-kind setters live here under #773 so the three preferences are not
//! reached as `&mut u32` from another crate. The paired read and the load-time
//! replacement stay in `progression.rs` where they already were. Choosing which
//! preference a map or a group change applies to stays in `wow-world`, which
//! owns the map entry and the group difficulty kind.

use crate::Player;

impl Player {
    /// C++ `Player::GetDungeonDifficultyID` (`Player.h:1961`).
    #[must_use]
    pub fn dungeon_difficulty_id_like_cpp(&self) -> u32 {
        self.gameplay_state().dungeon_difficulty_id
    }

    /// C++ `Player::GetRaidDifficultyID` (`Player.h:1962`).
    #[must_use]
    pub fn raid_difficulty_id_like_cpp(&self) -> u32 {
        self.gameplay_state().raid_difficulty_id
    }

    /// C++ `Player::GetLegacyRaidDifficultyID` (`Player.h:1963`).
    #[must_use]
    pub fn legacy_raid_difficulty_id_like_cpp(&self) -> u32 {
        self.gameplay_state().legacy_raid_difficulty_id
    }

    /// C++ `Player::SetDungeonDifficultyID` (`Player.h:1964`).
    pub fn set_dungeon_difficulty_id_like_cpp(&mut self, difficulty_id: u32) {
        self.gameplay_state_mut().dungeon_difficulty_id = difficulty_id;
    }

    /// C++ `Player::SetRaidDifficultyID` (`Player.h:1965`).
    pub fn set_raid_difficulty_id_like_cpp(&mut self, difficulty_id: u32) {
        self.gameplay_state_mut().raid_difficulty_id = difficulty_id;
    }

    /// C++ `Player::SetLegacyRaidDifficultyID` (`Player.h:1966`).
    pub fn set_legacy_raid_difficulty_id_like_cpp(&mut self, difficulty_id: u32) {
        self.gameplay_state_mut().legacy_raid_difficulty_id = difficulty_id;
    }
}
