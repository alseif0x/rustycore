// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_mount_vehicle_kit_like_cpp<R>(
        &mut self,
        update: impl FnOnce(&mut Option<Vehicle>) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_player_mount_vehicle_kit_like_cpp(update)
    }
    #[cfg(test)]
    pub(crate) fn active_player_transport_server_time_like_cpp(&self) -> i32 {
        crate::session::hub_ref(self).active_player_transport_server_time_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_active_player_transport_server_time_like_cpp(&mut self, value: i32) {
        crate::session::hub_mut(self).set_active_player_transport_server_time_like_cpp(value)
    }
    #[cfg(test)]
    pub(crate) fn represented_taxi_benchmark_mode_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_taxi_benchmark_mode_like_cpp()
    }
    #[cfg(test)]
    pub(in crate::session) fn replace_player_taxi_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerTaxiState,
    ) -> bool {
        crate::session::hub_mut(self).replace_player_taxi_state_like_cpp(state)
    }
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_taxi_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTaxiState) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_player_taxi_state_like_cpp(f)
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_destinations_like_cpp(&mut self, destinations: Vec<u32>) {
        crate::session::hub_mut(self).set_taxi_destinations_like_cpp(destinations)
    }
    #[cfg(test)]
    pub(crate) fn taxi_destinations_like_cpp(&self) -> Vec<u32> {
        crate::session::hub_ref(self).taxi_destinations_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_flight_state_like_cpp(
        &mut self,
        current_node: RepresentedTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
    ) {
        crate::session::hub_mut(self)
            .set_taxi_flight_state_like_cpp(current_node, node_after_teleport)
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_cleanup_state_like_cpp(&mut self, unit_flags: UnitFlags, mounted: bool) {
        crate::session::hub_mut(self).set_taxi_cleanup_state_like_cpp(unit_flags, mounted)
    }
    #[cfg(test)]
    pub(crate) fn taxi_unit_flags_like_cpp(&self) -> UnitFlags {
        crate::session::hub_ref(self).taxi_unit_flags_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn taxi_mounted_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).taxi_mounted_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_on_transport_like_cpp(&mut self, on_transport: bool) {
        self.fixtures
            .vehicles
            .set_player_on_transport_like_cpp(on_transport)
    }
    pub(in crate::session) fn player_transport_state_like_cpp(
        &self,
    ) -> Option<Option<wow_entities::PlayerTransportState>> {
        crate::session::hub_ref(self).player_transport_state_like_cpp()
    }
}
