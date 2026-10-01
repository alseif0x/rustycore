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
    pub(in crate::session) fn set_player_movement_speed_rate_and_notify_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        crate::session::hub_mut(self)
            .set_player_movement_speed_rate_and_notify_like_cpp(move_type, rate)
    }
    #[cfg(test)]
    pub(crate) fn player_movement_speed_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> f32 {
        crate::session::hub_ref(self).player_movement_speed_like_cpp(move_type)
    }
    #[cfg(test)]
    pub(crate) fn set_forced_speed_changes_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        count: u8,
    ) {
        crate::session::hub_mut(self).set_forced_speed_changes_like_cpp(move_type, count)
    }
    #[cfg(test)]
    pub(crate) fn forced_speed_changes_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> u8 {
        crate::session::hub_ref(self).forced_speed_changes_like_cpp(move_type)
    }
    #[cfg(test)]
    pub(crate) fn set_player_movement_speed_rate_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        crate::session::hub_mut(self).set_player_movement_speed_rate_like_cpp(move_type, rate)
    }
    #[cfg(test)]
    pub(crate) fn movement_speed_ack_events_like_cpp(&self) -> &[MovementSpeedAckEventLikeCpp] {
        self.fixtures.movement.movement_speed_ack_events_like_cpp()
    }
    pub(in crate::session) fn resolved_forced_speed_changes_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        crate::session::hub_ref(self).resolved_forced_speed_changes_like_cpp(move_type)
    }
}
