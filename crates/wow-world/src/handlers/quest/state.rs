// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest log slots and the accept/complete/remove status lifecycle.

use super::*;

impl WorldSession {
    pub(crate) fn quest_log_create_entries_like_cpp(&self) -> Vec<(u32, u32, i64, [u16; 24])> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_quest::quest_log_create_entries_like_cpp(
            &owner,
            &self.quest_state,
            &self.catalogs,
            cfg!(test),
        )
    }

    pub(crate) fn send_represented_quest_log_slot_update_like_cpp(&mut self, slot: u8) {
        let owner = self.core.quest_objective_access_like_cpp();
        let publication = self.core.packet_publication_access_like_cpp();
        wow_world_quest::send_represented_quest_log_slot_update_like_cpp(
            &owner,
            &self.quest_state,
            &self.catalogs,
            &publication,
            slot,
            cfg!(test),
        );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/quest/state/test_only.rs"]
mod test_only;
