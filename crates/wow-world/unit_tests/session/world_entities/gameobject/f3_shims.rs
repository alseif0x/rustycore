// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn canonical_gameobject_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        self.core.canonical_gameobject_access_like_cpp(guid)
    }
    pub(crate) fn apply_represented_gameobject_cooldown_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        cooldown_secs: u32,
    ) -> bool {
        self.world_entities
            .apply_represented_gameobject_cooldown_like_cpp(gameobject_guid, cooldown_secs)
    }
}
