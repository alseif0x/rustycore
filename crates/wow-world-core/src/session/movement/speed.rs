//! Hub operations for represented movement speed.

use crate::session::movement_protocol::{
    MovementSpeedAckEventLikeCpp, PLAYER_BASE_MOVE_SPEED_LIKE_CPP, UnitMoveTypeLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::MovementState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn movement_speed_ack_events_like_cpp(&self) -> &[MovementSpeedAckEventLikeCpp] {
        &self.movement_speed_ack_events_like_cpp
    }
}
impl crate::session::HubMut<'_> {
    pub fn recompute_represented_forward_speed_rates_like_cpp(&mut self) {
        self.recompute_represented_run_speed_rate_like_cpp();
        self.recompute_represented_swim_speed_rate_like_cpp();
        self.recompute_represented_flight_speed_rate_like_cpp();
    }

    pub fn record_movement_speed_ack_event_like_cpp(
        &mut self,
        event: MovementSpeedAckEventLikeCpp,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .movement
            .movement_speed_ack_events_like_cpp
            .push(event);
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = event;
    }

    pub fn consume_forced_speed_change_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.consume_forced_speed_change_like_cpp(index)
            })
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let count = &mut self.fixtures.movement.forced_speed_changes_like_cpp[index];
            if *count > 0 {
                *count = count.saturating_sub(1);
            }
            return Some(*count);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_forced_speed_changes_like_cpp(&mut self, move_type: UnitMoveTypeLikeCpp, count: u8) {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_forced_speed_changes_like_cpp(index, count)
            })
            .unwrap_or(false);
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.movement.forced_speed_changes_like_cpp[index] = count;
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_movement_speed_rate_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let _ = self.set_player_movement_speed_rate_like_cpp_inner(move_type, rate.max(0.01));
    }
}
impl crate::session::HubRef<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_movement_speed_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> f32 {
        self.resolved_player_movement_speed_like_cpp(move_type)
            .expect("test Player movement-speed owner must resolve")
    }

    pub fn resolved_forced_speed_changes_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.forced_speed_changes_like_cpp(index))
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.forced_speed_changes_like_cpp[index]);
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn forced_speed_changes_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> u8 {
        self.resolved_forced_speed_changes_like_cpp(move_type)
            .expect("test Player forced-speed owner must resolve")
    }
}
impl crate::session::HubMut<'_> {
    fn set_player_movement_speed_rate_like_cpp_inner(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) -> bool {
        let (_presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.set_player_movement_speed_rate_like_cpp_inner(move_type, rate)
    }

    pub fn recompute_represented_run_speed_rate_like_cpp(&mut self) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.recompute_represented_run_speed_rate_like_cpp(&presentation);
    }

    pub fn recompute_represented_flight_speed_rate_like_cpp(&mut self) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.recompute_represented_flight_speed_rate_like_cpp(&presentation);
    }

    pub fn recompute_represented_swim_speed_rate_like_cpp(&mut self) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.recompute_represented_swim_speed_rate_like_cpp(&presentation);
    }

    pub fn recompute_represented_backward_speed_rates_like_cpp(&mut self) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.recompute_represented_backward_speed_rates_like_cpp(&presentation);
    }

    pub fn recompute_represented_mounted_speed_rates_like_cpp(&mut self) {
        self.recompute_represented_run_speed_rate_like_cpp();
        self.recompute_represented_flight_speed_rate_like_cpp();
    }

    pub fn set_player_movement_speed_rate_and_notify_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let (presentation, mut control) = self.aura_removal_mount_accesses_like_cpp();
        control.set_player_movement_speed_rate_and_notify_like_cpp(&presentation, move_type, rate);
    }

}
impl crate::session::HubRef<'_> {
    pub fn resolved_player_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().speed_rate_at_like_cpp(index))
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.movement_speed_rates_like_cpp[index]);
        }
        canonical
    }

    pub fn resolved_player_movement_speed_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        Some(
            PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()]
                * self.resolved_player_movement_speed_rate_like_cpp(move_type)?,
        )
    }





}
