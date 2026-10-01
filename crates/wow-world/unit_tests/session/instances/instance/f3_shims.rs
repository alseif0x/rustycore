// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn check_instance_count_at_like_cpp(
        &mut self,
        instance_id: u32,
        now_secs: u64,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.check_instance_count_at_like_cpp(&mut hub, instance_id, now_secs)
    }
    #[cfg(test)]
    pub(crate) fn represented_player_recent_instance_id_like_cpp(&self, map_id: u32) -> u32 {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_player_recent_instance_id_like_cpp(hub, map_id)
    }
    pub(crate) fn forget_represented_player_recent_instance_like_cpp(
        &mut self,
        map_id: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.forget_represented_player_recent_instance_like_cpp(&mut hub, map_id)
    }
}
