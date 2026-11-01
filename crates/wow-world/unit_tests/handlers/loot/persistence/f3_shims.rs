// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn retire_committed_destroyed_item_loot_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.retire_committed_destroyed_item_loot_like_cpp(&mut hub, item_guid, player_guid)
    }
}
