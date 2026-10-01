// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn load_completed_achievement_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = u32>,
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_completed_achievement_rows_like_cpp(&mut hub, rows)
    }
}
