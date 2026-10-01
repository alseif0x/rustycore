// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[allow(dead_code)]
    pub(crate) fn tick_represented_gameobject_door_or_button_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> bool {
        self.world_entities
            .tick_represented_gameobject_door_or_button_like_cpp(gameobject_guid)
    }
}
