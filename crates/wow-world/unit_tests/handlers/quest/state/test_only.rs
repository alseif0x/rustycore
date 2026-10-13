// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `state` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn get_quest_slot_quest_id_like_cpp(&self, slot: u8) -> Option<u32> {
        let owner = self.core.quest_objective_access_like_cpp();
        wow_world_quest::get_quest_slot_quest_id_like_cpp(
            &owner,
            &self.quest_state,
            slot,
            cfg!(test),
        )
    }
}
