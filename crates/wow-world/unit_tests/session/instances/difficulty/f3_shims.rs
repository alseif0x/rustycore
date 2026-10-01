// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_dungeon_difficulty_id_like_cpp(&self) -> u32 {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_dungeon_difficulty_id_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn set_represented_dungeon_difficulty_id_for_test_like_cpp(
        &mut self,
        difficulty_id: u32,
    ) {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.set_represented_dungeon_difficulty_id_for_test_like_cpp(&mut hub, difficulty_id)
    }
}
