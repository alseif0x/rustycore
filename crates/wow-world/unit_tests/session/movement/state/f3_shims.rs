// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_movement_flags_like_cpp(&self) -> MovementFlag {
        crate::session::hub_ref(self).player_movement_flags_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_dismiss_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleDismissMovementLikeCpp] {
        self.fixtures
            .movement
            .represented_vehicle_dismiss_movements_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_base_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleBaseMovementLikeCpp] {
        self.fixtures
            .movement
            .represented_vehicle_base_movements_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_movement_time_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).player_movement_time_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_movement_force_mod_magnitude_changes_like_cpp(&mut self, count: u8) {
        crate::session::hub_mut(self).set_movement_force_mod_magnitude_changes_like_cpp(count)
    }
    #[cfg(test)]
    pub(crate) fn set_movement_force_mod_magnitude_like_cpp(&mut self, magnitude: f32) {
        crate::session::hub_mut(self).set_movement_force_mod_magnitude_like_cpp(magnitude)
    }
}
