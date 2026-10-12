// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_quest_push_result_sender_mismatch_count_like_cpp(&self) -> u32 {
        self.quest_state
            .represented_quest_push_result_sender_mismatch_count_like_cpp()
    }
}
