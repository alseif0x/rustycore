// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn apply_represented_player_instance_reset_result_like_cpp(
        &mut self,
        map_id: u32,
        result: GroupInstanceResetResultLikeCpp,
        method: GroupInstanceResetMethodLikeCpp,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.apply_represented_player_instance_reset_result_like_cpp(
            &mut hub, map_id, result, method,
        )
    }
}
