// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group invariant tests for [`super`].
//!
//! Moved verbatim from `wow_network::group_registry` by issue #137. Following
//! the extraction convention of issue #214, the only textual change is
//! dedenting by one level, which lets rustfmt collapse some argument lists and
//! drop their trailing commas.

#![cfg(test)]

use wow_data::DifficultyStore;

use super::*;

/// The loaded-difficulty port is satisfied by the real DB2 store, so these
/// invariants keep exercising C++'s exact validation instead of a hand-written
/// stand-in. `wow-data` is a development-only edge here.
impl GroupDifficultyValidatorLikeCpp for DifficultyStore {
    fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_dungeon_difficulty_id_like_cpp(self, difficulty)
    }

    fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_raid_difficulty_id_like_cpp(self, difficulty)
    }

    fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        DifficultyStore::check_loaded_legacy_raid_difficulty_id_like_cpp(self, difficulty)
    }
}

mod core;
mod instances;
mod invites;
mod markers_loot;
mod members_raid;
mod persistence;
mod ready_check;
