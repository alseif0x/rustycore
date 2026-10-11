// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn current_canonical_player_map_key_like_cpp(&self) -> Option<wow_map::MapKey> {
        self.core.current_canonical_player_map_key_like_cpp()
    }
}

impl WorldSession {
    pub(in crate::session) fn canonical_map_has_seer_like_object_like_cpp(
        &self,
        target: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.canonical_map_has_seer_like_object_like_cpp(hub, target)
    }
}
