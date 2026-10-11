// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `stats_queries` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
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
