// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! WorldSession compatibility entrypoints for application-owned stat updates.

use super::*;

impl WorldSession {
    pub(super) fn player_stat_changes_like_cpp(
        &mut self,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.stats_application_cx_like_cpp()
            .player_stat_changes_like_cpp()
    }

    pub(super) fn player_stat_changes_with_represented_item_bonuses_like_cpp(
        &mut self,
        include_represented_item_bonuses: bool,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.stats_application_cx_like_cpp()
            .player_stat_changes_with_represented_item_bonuses_like_cpp(
                include_represented_item_bonuses,
            )
    }

    /// Recalculate all stats from base + gear and send a VALUES update.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    pub(crate) fn send_stat_update(&mut self) -> bool {
        self.stats_application_cx_like_cpp()
            .send_stat_update_like_cpp()
    }

    /// C++ `HandleModTotalPercentStat` application entrypoint.
    pub(crate) fn send_total_stat_percentage_update_like_cpp(
        &mut self,
        preserve_health_pct: bool,
    ) {
        self.stats_application_cx_like_cpp()
            .send_total_stat_percentage_update_like_cpp(preserve_health_pct);
    }

    /// C++ `Player::GiveLevel` stat refill and publication entrypoint.
    pub(crate) fn send_level_up_stat_update_like_cpp(&mut self) {
        self.stats_application_cx_like_cpp()
            .send_level_up_stat_update_like_cpp();
    }

    pub(super) fn send_login_stat_update_with_represented_item_bonuses_like_cpp(&mut self) {
        self.stats_application_cx_like_cpp()
            .send_login_stat_update_with_represented_item_bonuses_like_cpp();
    }
}
