//! Canonical PlayerTaxi snapshots, continuation and cleanup with existing fixture selection.

use super::*;

impl WorldSession {
    pub(crate) fn player_taxi_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerTaxiState> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.taxi_state_like_cpp().clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.gossip_handleless_fixture() {
            return Some(
                wow_entities::PlayerTaxiState::from_represented_parts_like_cpp(
                    self.taxi_destinations_like_cpp.clone(),
                    self.taxi_flight_state_like_cpp
                        .map(canonical_taxi_flight_state_like_cpp),
                    self.taxi_unit_flags_like_cpp.bits(),
                    self.taxi_mounted_like_cpp,
                ),
            );
        }
        canonical
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn replace_player_taxi_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerTaxiState,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_taxi_state_like_cpp(state.clone())
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.gossip_handleless_fixture() {
            self.taxi_destinations_like_cpp = state.destinations_like_cpp().to_vec();
            self.taxi_flight_state_like_cpp = state
                .flight_like_cpp()
                .map(represented_taxi_flight_state_like_cpp);
            self.taxi_unit_flags_like_cpp =
                UnitFlags::from_bits_retain(state.unit_flags_like_cpp());
            self.taxi_mounted_like_cpp = state.mounted_like_cpp();
            return true;
        }
        canonical
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn mutate_player_taxi_state_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerTaxiState) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.gossip_handleless_fixture() {
            let mut state = self.player_taxi_state_snapshot_like_cpp()?;
            let result = f(&mut state);
            return self
                .replace_player_taxi_state_like_cpp(state)
                .then_some(result);
        }
        // PlayerTaxi mutates the owning Player's route, not a Session copy.
        self.with_owned_player_mut_like_cpp(|player| f(&mut player.gameplay_state_mut().taxi))
    }
    pub(crate) fn advance_player_taxi_flight_after_teleport_like_cpp(
        &mut self,
    ) -> Option<wow_entities::PlayerTaxiFlightNodeLikeCpp> {
        let canonical = self.with_owned_player_mut_like_cpp(
            wow_entities::Player::advance_taxi_flight_after_teleport_like_cpp,
        );
        if let Some(result) = canonical {
            return result;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_taxi_state_like_cpp(|taxi| {
                    taxi.advance_taxi_flight_after_teleport_like_cpp()
                })
                .flatten();
        }
        None
    }
    pub(crate) fn cleanup_player_after_taxi_flight_like_cpp(&mut self) -> bool {
        let taxi_unit_flags = (UnitFlags::REMOVE_CLIENT_CONTROL | UnitFlags::ON_TAXI).bits();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.cleanup_after_taxi_flight_like_cpp(taxi_unit_flags)
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_taxi_state_like_cpp(|taxi| {
                    taxi.cleanup_after_taxi_flight_like_cpp(taxi_unit_flags);
                })
                .is_some();
        }
        false
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_destinations_like_cpp(&mut self, destinations: Vec<u32>) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.replace_destinations_like_cpp(destinations);
        });
    }
    #[cfg(test)]
    pub(crate) fn taxi_destinations_like_cpp(&self) -> Vec<u32> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.destinations_like_cpp().to_vec())
            .expect("test Player taxi owner must resolve")
    }
    pub(crate) fn resolved_is_in_taxi_flight_like_cpp(&self) -> Option<bool> {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.is_in_flight_like_cpp())
    }
    #[cfg(test)]
    pub(crate) fn is_in_taxi_flight_like_cpp(&self) -> bool {
        self.resolved_is_in_taxi_flight_like_cpp()
            .expect("test Player taxi owner must resolve")
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn set_taxi_flight_state_like_cpp(
        &mut self,
        current_node: RepresentedTaxiFlightNodeLikeCpp,
        node_after_teleport: Option<RepresentedTaxiFlightNodeLikeCpp>,
    ) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.begin_taxi_flight_like_cpp(
                canonical_taxi_flight_node_like_cpp(current_node),
                node_after_teleport.map(canonical_taxi_flight_node_like_cpp),
            );
        });
    }
    #[cfg(test)]
    pub(crate) fn set_taxi_cleanup_state_like_cpp(&mut self, unit_flags: UnitFlags, mounted: bool) {
        let _ = self.mutate_player_taxi_state_like_cpp(|taxi| {
            taxi.set_taxi_cleanup_state_like_cpp(unit_flags.bits(), mounted);
        });
    }
    #[cfg(test)]
    pub(crate) fn taxi_unit_flags_like_cpp(&self) -> UnitFlags {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| UnitFlags::from_bits_retain(taxi.unit_flags_like_cpp()))
            .expect("test Player taxi owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn taxi_mounted_like_cpp(&self) -> bool {
        self.player_taxi_state_snapshot_like_cpp()
            .map(|taxi| taxi.mounted_like_cpp())
            .expect("test Player taxi owner must resolve")
    }
}
