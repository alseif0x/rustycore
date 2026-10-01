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
    pub(crate) fn reconcile_player_transport_membership_like_cpp(
        &self,
        player_guid: ObjectGuid,
        requested_transport_guid: Option<ObjectGuid>,
    ) -> MovementTransportMembershipLikeCpp {
        crate::session::hub_ref(self)
            .reconcile_player_transport_membership_like_cpp(player_guid, requested_transport_guid)
    }
    pub(crate) fn set_player_movement_time_like_cpp(&mut self, time: u32) {
        crate::session::hub_mut(self).set_player_movement_time_like_cpp(time)
    }
    pub(crate) fn adjust_client_movement_time_like_cpp(&self, time: u32) -> u32 {
        crate::session::hub_ref(self).adjust_client_movement_time_like_cpp(time)
    }
    pub(crate) fn set_player_movement_flags_like_cpp(&mut self, flags: MovementFlag) {
        crate::session::hub_mut(self).set_player_movement_flags_like_cpp(flags)
    }
    pub(crate) fn set_player_position_like_cpp(&mut self, position: wow_core::Position) {
        crate::session::hub_mut(self).set_player_position_like_cpp(position)
    }
    pub(crate) fn player_position_like_cpp(&self) -> Option<wow_core::Position> {
        crate::session::hub_ref(self).player_position_like_cpp()
    }
    pub(crate) fn set_player_map_position_like_cpp(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
    ) {
        crate::session::hub_mut(self).set_player_map_position_like_cpp(map_id, position)
    }
    pub(crate) fn player_moved_unit_guid_like_cpp(&self) -> Option<ObjectGuid> {
        crate::session::hub_ref(self).player_moved_unit_guid_like_cpp()
    }
    pub(crate) fn remove_current_player_from_canonical_current_map_like_cpp(&mut self) -> bool {
        crate::session::hub_mut(self).remove_current_player_from_canonical_current_map_like_cpp()
    }
}
