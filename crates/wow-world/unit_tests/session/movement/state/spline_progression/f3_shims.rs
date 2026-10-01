// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn apply_move_time_skipped_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        time_skipped: u32,
    ) -> bool {
        crate::session::hub_mut(self).apply_move_time_skipped_like_cpp(mover_guid, time_skipped)
    }
}
