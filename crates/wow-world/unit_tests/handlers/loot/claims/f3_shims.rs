// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::handlers::loot) fn record_represented_gameobject_chest_release_metadata_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        crate::session::cx_loot(self)
            .record_represented_gameobject_chest_release_metadata_like_cpp(gameobject_guid, source)
    }
}
