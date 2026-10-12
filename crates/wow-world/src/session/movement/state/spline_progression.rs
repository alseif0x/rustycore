//! Spline completion and taxi-movement progression operations.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn move_spline_done_taxi_events_like_cpp(
        &self,
    ) -> &[MoveSplineDoneTaxiEventLikeCpp] {
        &self.fixtures.teleport.move_spline_done_taxi_events_like_cpp
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/session/movement/state/spline_progression/f3_shims.rs"]
mod f3_shims;
