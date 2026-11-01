// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_out_of_bounds_like_cpp(&self) -> bool {
        self.fixtures.movement.player_out_of_bounds_like_cpp()
    }
    pub(in crate::session) fn set_represented_can_swim_to_fly_transition_like_cpp(
        &mut self,
        enable: bool,
    ) -> bool {
        crate::session::hub_mut(self).set_represented_can_swim_to_fly_transition_like_cpp(enable)
    }
}
