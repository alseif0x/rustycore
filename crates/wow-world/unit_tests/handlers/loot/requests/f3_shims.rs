// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn close_retired_active_loot_windows_like_cpp(&mut self, player_guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.close_retired_active_loot_windows_like_cpp(&mut hub, player_guid)
    }
}
