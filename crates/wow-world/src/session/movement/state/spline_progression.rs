//! Spline completion and taxi-movement progression operations.

use super::*;

impl WorldSession {
    pub(in crate::session) fn record_move_spline_done_taxi_event_like_cpp(
        &mut self,
        spline_id: i32,
        action: MoveSplineDoneTaxiActionLikeCpp,
        destination_node_id: Option<u32>,
        teleport_map_id: Option<u16>,
        teleport_position: Option<wow_core::Position>,
        honorless_target_cast: bool,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        #[cfg(test)]
        self.fixtures
            .teleport
            .move_spline_done_taxi_events_like_cpp
            .push(MoveSplineDoneTaxiEventLikeCpp {
                spline_id,
                action,
                destination_node_id,
                teleport_map_id,
                teleport_position,
                honorless_target_cast,
            });
        #[cfg(not(test))]
        let _ = (
            spline_id,
            destination_node_id,
            teleport_map_id,
            teleport_position,
            honorless_target_cast,
        );
        action
    }

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
