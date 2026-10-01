// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::handlers::loot) fn represented_notify_loot_item_removed_from_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        authority: Option<&OwnedLootAuthority>,
        snapshot: &wow_loot::OwnedLootSnapshot,
        loot_list_id: u8,
    ) {
        let (state, mut hub) = crate::session::split_loot_mut(self);
        state.represented_notify_loot_item_removed_from_snapshot_like_cpp(
            &mut hub,
            owner_guid,
            authority,
            snapshot,
            loot_list_id,
        )
    }
}
