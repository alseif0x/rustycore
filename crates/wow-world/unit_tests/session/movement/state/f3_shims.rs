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

impl WorldSession {
    pub(crate) fn remove_currency(&mut self, currency_id: u32, amount: u32) -> bool {
        let access = self.core.owned_player_currency_access_like_cpp();
        self.inventory
            .remove_currency_with_access_like_cpp(&access, currency_id, amount)
    }
    pub(crate) fn remove_legit_character(&mut self, guid: &ObjectGuid) {
        crate::session::hub_mut(self).remove_legit_character(guid)
    }
    pub(crate) fn set_represented_mover_fixed_position_vehicle_like_cpp(&mut self, fixed: bool) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_mover_fixed_position_vehicle_like_cpp(fixed);
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .movement
                .represented_mover_fixed_position_vehicle_like_cpp = fixed;
        }
    }
    pub(in crate::session) fn resolved_movement_force_mod_magnitude_like_cpp(&self) -> Option<f32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.unit().movement_force_mod_magnitude_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.movement_force_mod_magnitude_like_cpp);
        }
        canonical
    }
}
