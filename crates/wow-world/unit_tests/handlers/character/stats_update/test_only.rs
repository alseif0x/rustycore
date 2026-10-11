// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `stats_update` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn player_stat_changes_like_cpp(
        &mut self,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.stats_application_cx_like_cpp()
            .player_stat_changes_like_cpp()
    }
    pub(crate) fn player_stat_changes_with_represented_item_bonuses_like_cpp(
        &mut self,
        include_represented_item_bonuses: bool,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.stats_application_cx_like_cpp()
            .player_stat_changes_with_represented_item_bonuses_like_cpp(
                include_represented_item_bonuses,
            )
    }
    /// C++ `Player::GiveLevel` stat refill and publication entrypoint.
    pub(crate) fn send_level_up_stat_update_like_cpp(&mut self) {
        self.stats_application_cx_like_cpp()
            .send_level_up_stat_update_like_cpp();
    }
}
