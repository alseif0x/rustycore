// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! WorldSession compatibility entrypoints for application-owned stat updates.

use super::*;

impl WorldSession {
    /// Recalculate all stats from base + gear and send a VALUES update.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    pub(crate) fn send_stat_update(&mut self) -> bool {
        self.stats_application_cx_like_cpp()
            .send_stat_update_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character/stats_update/test_only.rs"]
mod test_only;
