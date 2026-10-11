// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `xp_grants` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    /// Apply the visible XP grant phases. The handler's async wrapper persists
    /// the resulting projection after this operation returns.
    pub(crate) fn give_xp_runtime_like_cpp(
        &mut self,
        xp: u32,
        victim: wow_core::ObjectGuid,
        group_rate: f32,
    ) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            let (mut operation, mut fixtures) = self.quest_xp_gain_context_like_cpp();
            operation.give_xp_runtime_like_cpp(xp, victim, group_rate, &mut fixtures)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        self.quest_xp_gain_context_like_cpp()
            .give_xp_runtime_like_cpp(xp, victim, group_rate)
    }
}
