// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only stat projections and level-up deltas.

use super::*;

impl WorldSession {
    pub(crate) fn level_up_stat_deltas_like_cpp(
        &mut self,
        new_level: u8,
    ) -> Option<(i32, [i32; 5])> {
        self.stats_application_cx_like_cpp()
            .level_up_stat_deltas_like_cpp(new_level)
    }
}
