// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::handlers::loot) fn apply_represented_gameobject_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        selected_pool_looted: bool,
        whole_object_fully_looted: bool,
    ) {
        self.loot_release_cx_like_cpp()
            .apply_represented_gameobject_loot_release_like_cpp(
                guid,
                player_guid,
                selected_pool_looted,
                whole_object_fully_looted,
                None,
            )
    }
}
