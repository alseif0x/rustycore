// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn resolved_player_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        crate::session::hub_ref(self).resolved_player_movement_speed_rate_like_cpp(move_type)
    }
    pub(in crate::session::movement::speed) fn set_player_movement_speed_rate_like_cpp_inner(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) -> bool {
        crate::session::hub_mut(self).set_player_movement_speed_rate_like_cpp_inner(move_type, rate)
    }
    pub(in crate::session) fn set_player_movement_speed_rate_and_notify_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        crate::session::hub_mut(self)
            .set_player_movement_speed_rate_and_notify_like_cpp(move_type, rate)
    }
}
