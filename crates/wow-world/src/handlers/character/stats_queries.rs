// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only stat projections and level-up deltas.

use super::*;

impl WorldSession {
    pub(crate) fn level_up_stat_deltas_like_cpp(&self, new_level: u8) -> Option<(i32, [i32; 5])> {
        let store = self.player_stats()?;
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let old = store.get(race, class, self.player_level_like_cpp())?;
        let new = store.get(race, class, new_level)?;
        let old_stats = old.primary_stats_like_cpp();
        let new_stats = new.primary_stats_like_cpp();
        Some((
            i32::try_from(new.base_mana)
                .unwrap_or(i32::MAX)
                .saturating_sub(i32::try_from(old.base_mana).unwrap_or(i32::MAX)),
            std::array::from_fn(|index| {
                i32::from(new_stats[index]).saturating_sub(i32::from(old_stats[index]))
            }),
        ))
    }
}
