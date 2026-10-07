// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character equipment aggregation and effective stat publication.
//!
//! This module owns the application-side projection from equipped item data and
//! Player-owned item modifiers into the pure `wow-data` stat calculation. The
//! resulting snapshot is published to the canonical Player; packet formatting
//! remains in the character handler adapters.

use super::*;
pub(crate) use crate::session::hub_support::RepresentedPlayerGearStatsLikeCpp;
use crate::session::hub_support::{
    SPELL_SCHOOL_MASK_ALL_LIKE_CPP, SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
    SPELL_SCHOOL_MASK_SPELL_LIKE_CPP,
};

impl WorldSession {
    pub(super) fn stats_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::CharacterStatsApplicationCxLikeCpp<'_> {
        let (inventory, hub) = crate::session::split_inventory_mut(self);
        wow_world_application::stats_application_cx_from_hub_like_cpp(hub, inventory)
    }

    pub(super) fn represented_player_gear_stats_like_cpp(
        &mut self,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        self.stats_application_cx_like_cpp()
            .represented_player_gear_stats_like_cpp()
    }

    /// C++ `Player::Create` -> `InitStatsForLevel`/`UpdateMaxHealth`: the
    /// creation body moved to the `wow-world-application` character owner
    /// (#1263 F5); the character scenarios still call this entry point.
    #[cfg(test)]
    pub(super) fn player_stat_system_projection_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        self.stats_application_cx_like_cpp()
            .player_stat_system_projection_like_cpp(race, class, level, gear)
    }

    pub(crate) fn apply_represented_shapeshift_base_attack_time_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_represented_shapeshift_base_attack_time_like_cpp(&mut hub)
    }
}
