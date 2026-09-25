//! Spline completion and taxi-movement progression operations.

use super::*;

impl WorldSession {
    pub(crate) fn apply_move_time_skipped_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        time_skipped: u32,
    ) -> bool {
        // C++ validates against the active `m_unitMovedByMe`, so a controlled
        // Creature/Pet is a valid mover too (MovementHandler.cpp:721-739).
        // Keep the owner-specific write on the mover rather than silently
        // advancing the Player clock for every ACK.
        let adjusted_time = if self.player_moved_unit_guid_like_cpp() != Some(mover_guid) {
            None
        } else if self.player_guid() == Some(mover_guid) {
            self.resolved_player_movement_time_like_cpp()
                .map(|time| time.wrapping_add(time_skipped))
                .inspect(|adjusted_time| self.set_player_movement_time_like_cpp(*adjusted_time))
        } else {
            self.mutate_world_creature(mover_guid, |creature| {
                let adjusted_time = creature
                    .creature
                    .unit()
                    .movement_time_like_cpp()
                    .wrapping_add(time_skipped);
                creature
                    .creature
                    .unit_mut()
                    .set_movement_time_like_cpp(adjusted_time);
                adjusted_time
            })
        };
        let accepted = adjusted_time.is_some();

        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveTimeSkipped,
            mover_guid,
            ack_index: None,
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time,
            speed: None,
            time_skipped: Some(time_skipped),
            spline_id: None,
            accepted,
        });
        accepted
    }

    pub(crate) fn record_move_spline_done_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        spline_id: i32,
    ) -> bool {
        let accepted = self.validate_and_sanitize_movement_ack_status_represented_like_cpp(status);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveSplineDone,
            mover_guid: status.guid,
            ack_index: None,
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: None,
            speed: None,
            time_skipped: None,
            spline_id: Some(spline_id),
            accepted,
        });
        accepted
    }

    pub(crate) fn handle_move_spline_done_taxi_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        spline_id: i32,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        if !self.record_move_spline_done_like_cpp(status, spline_id) {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::InvalidMovement,
                None,
                None,
                None,
                false,
            );
        }

        let Some(taxi_state) = self.player_taxi_state_snapshot_like_cpp() else {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        };
        let current_destination = taxi_state.taxi_destination_like_cpp();
        if let Some(destination_node_id) = current_destination {
            let Some(flight) = taxi_state.flight_like_cpp() else {
                return self.record_move_spline_done_taxi_event_like_cpp(
                    spline_id,
                    MoveSplineDoneTaxiActionLikeCpp::InProgressNoFlightGenerator,
                    Some(destination_node_id),
                    None,
                    None,
                    false,
                );
            };

            let destination_map_id = self
                .taxi_node_map_ids_like_cpp
                .get(&destination_node_id)
                .copied();
            let should_teleport = destination_map_id
                .map(|map_id| map_id != self.player_map_id_like_cpp())
                .unwrap_or(false)
                || flight.current_node.teleport_flag;

            if should_teleport {
                if let (Some(map_id), Some(node)) = (destination_map_id, flight.node_after_teleport)
                {
                    if self
                        .advance_player_taxi_flight_after_teleport_like_cpp()
                        .is_none()
                    {
                        return self.record_move_spline_done_taxi_event_like_cpp(
                            spline_id,
                            MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                            Some(destination_node_id),
                            None,
                            None,
                            false,
                        );
                    }
                    self.set_player_map_position_like_cpp(map_id, node.position);
                    return self.record_move_spline_done_taxi_event_like_cpp(
                        spline_id,
                        MoveSplineDoneTaxiActionLikeCpp::TeleportRequested,
                        Some(destination_node_id),
                        Some(map_id),
                        Some(node.position),
                        false,
                    );
                }
            }

            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::InProgressNoTeleport,
                Some(destination_node_id),
                None,
                None,
                false,
            );
        }

        if taxi_state.destinations_like_cpp().len() != 1 {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        }

        if !self.cleanup_player_after_taxi_flight_like_cpp() {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        }
        let current_z = self
            .player_position_like_cpp()
            .map(|position| position.z)
            .unwrap_or(status.position.z);
        self.set_fall_information_like_cpp(0, current_z);
        let honorless_target_cast = self
            .player_world_local_state_like_cpp()
            .is_some_and(|state| state.is_pvp_hostile_like_cpp());

        self.record_move_spline_done_taxi_event_like_cpp(
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::FinalCleanup,
            None,
            None,
            None,
            honorless_target_cast,
        )
    }

    fn record_move_spline_done_taxi_event_like_cpp(
        &mut self,
        spline_id: i32,
        action: MoveSplineDoneTaxiActionLikeCpp,
        destination_node_id: Option<u32>,
        teleport_map_id: Option<u16>,
        teleport_position: Option<wow_core::Position>,
        honorless_target_cast: bool,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        #[cfg(test)]
        self.move_spline_done_taxi_events_like_cpp
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
        &self.move_spline_done_taxi_events_like_cpp
    }
}
