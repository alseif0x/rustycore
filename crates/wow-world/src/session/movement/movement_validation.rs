//! Movement acknowledgements and represented validation gates.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn remove_canonical_attacker_like_cpp(
        &mut self,
        victim: ObjectGuid,
        attacker: ObjectGuid,
    ) {
        if self
            .mutate_canonical_player_by_guid_like_cpp(victim, |victim| {
                victim.unit_mut().remove_attacker_like_cpp(attacker)
            })
            .is_some()
        {
            return;
        }
        let _ = self.mutate_canonical_creature_by_guid_like_cpp(victim, |victim| {
            victim.unit_mut().remove_attacker_like_cpp(attacker)
        });
    }
    pub(crate) fn validate_movement_ack_status_like_cpp(
        &self,
        status: &wow_packet::packets::movement::MovementInfo,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };

        status.guid == player_guid && status.position.is_valid_map_coord_like_cpp()
    }
    pub(crate) fn validate_and_sanitize_movement_ack_status_represented_like_cpp(
        &self,
        status: &mut wow_packet::packets::movement::MovementInfo,
    ) -> bool {
        if !self.validate_movement_ack_status_like_cpp(status) {
            return false;
        }

        self.sanitize_movement_info_flags_represented_like_cpp(status);
        true
    }
    pub(crate) fn record_movement_ack_event_like_cpp(&mut self, event: MovementAckEventLikeCpp) {
        #[cfg(test)]
        self.movement_ack_events_like_cpp.push(event);
        #[cfg(not(test))]
        let _ = event;
    }
    pub(crate) fn record_validated_movement_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
        speed: Option<f32>,
    ) -> bool {
        let accepted =
            self.validate_and_sanitize_movement_ack_status_represented_like_cpp(&mut ack.status);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode,
            mover_guid: ack.status.guid,
            ack_index: Some(ack.ack_index),
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: None,
            speed,
            time_skipped: None,
            spline_id: None,
            accepted,
        });
        accepted
    }
    pub(crate) fn apply_move_set_vehicle_rec_id_ack_like_cpp(
        &mut self,
        ack: &mut wow_packet::packets::movement::MovementAck,
    ) {
        self.sanitize_movement_info_represented_like_cpp(&mut ack.status);
    }
    pub(crate) fn record_apply_movement_force_ack_like_cpp(
        &mut self,
        ack: &mut wow_packet::packets::movement::MovementAck,
        force: &wow_packet::packets::movement::MovementForce,
    ) -> bool {
        if !self.validate_and_sanitize_movement_ack_status_represented_like_cpp(&mut ack.status) {
            self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
                opcode: ClientOpcodes::MoveApplyMovementForceAck,
                mover_guid: ack.status.guid,
                ack_index: Some(ack.ack_index),
                movement_force_id: Some(force.id),
                movement_force_type: Some(force.force_type.to_wire()),
                adjusted_time: None,
                speed: None,
                time_skipped: None,
                spline_id: None,
                accepted: false,
            });
            return false;
        }

        let adjusted_time = self.adjust_client_movement_time_like_cpp(ack.status.time);
        ack.status.time = adjusted_time;
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveApplyMovementForceAck,
            mover_guid: ack.status.guid,
            ack_index: Some(ack.ack_index),
            movement_force_id: Some(force.id),
            movement_force_type: Some(force.force_type.to_wire()),
            adjusted_time: Some(adjusted_time),
            speed: None,
            time_skipped: None,
            spline_id: None,
            accepted: true,
        });
        true
    }
    pub(crate) fn record_remove_movement_force_ack_like_cpp(
        &mut self,
        ack: &mut wow_packet::packets::movement::MovementAck,
        force_id: ObjectGuid,
    ) -> bool {
        if !self.validate_and_sanitize_movement_ack_status_represented_like_cpp(&mut ack.status) {
            self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
                opcode: ClientOpcodes::MoveRemoveMovementForceAck,
                mover_guid: ack.status.guid,
                ack_index: Some(ack.ack_index),
                movement_force_id: Some(force_id),
                movement_force_type: None,
                adjusted_time: None,
                speed: None,
                time_skipped: None,
                spline_id: None,
                accepted: false,
            });
            return false;
        }

        let adjusted_time = self.adjust_client_movement_time_like_cpp(ack.status.time);
        ack.status.time = adjusted_time;
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveRemoveMovementForceAck,
            mover_guid: ack.status.guid,
            ack_index: Some(ack.ack_index),
            movement_force_id: Some(force_id),
            movement_force_type: None,
            adjusted_time: Some(adjusted_time),
            speed: None,
            time_skipped: None,
            spline_id: None,
            accepted: true,
        });
        true
    }
    #[cfg(test)]
    pub(crate) fn movement_ack_events_like_cpp(&self) -> &[MovementAckEventLikeCpp] {
        &self.movement_ack_events_like_cpp
    }
    pub(crate) fn handle_movement_force_mod_magnitude_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
        speed: f32,
    ) -> bool {
        if !self.record_validated_movement_ack_like_cpp(opcode, ack, Some(speed)) {
            self.trace_anticheat_violation_like_cpp(
                "HandleMoveSetModMovementForceMagnitudeAck.InvalidMovementAck",
                Some(opcode),
                "kick",
            );
            self.record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: None,
                ack_speed: speed,
                expected_speed: None,
                remaining_forced_changes: None,
                action: MovementSpeedAckActionLikeCpp::Kicked,
            });
            return false;
        }

        let Some(mut remaining_forced_changes) =
            self.resolved_movement_force_mod_magnitude_changes_like_cpp()
        else {
            return false;
        };
        let Some(expected_magnitude) = self.resolved_movement_force_mod_magnitude_like_cpp() else {
            return false;
        };
        let mut action = MovementSpeedAckActionLikeCpp::Accepted;
        if remaining_forced_changes > 0 {
            let Some(remaining) = self.consume_movement_force_mod_magnitude_change_like_cpp()
            else {
                return false;
            };
            remaining_forced_changes = remaining;
            if remaining_forced_changes == 0 && (expected_magnitude - speed).abs() > 0.01 {
                self.trace_anticheat_violation_like_cpp(
                    "HandleMoveSetModMovementForceMagnitudeAck.IncorrectMagnitude",
                    Some(opcode),
                    "kick",
                );
                self.kick(
                    "WorldSession::HandleMoveSetModMovementForceMagnitudeAck Incorrect magnitude",
                );
                action = MovementSpeedAckActionLikeCpp::Kicked;
            }
        }

        self.record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
            opcode,
            move_type: None,
            ack_speed: speed,
            expected_speed: Some(expected_magnitude),
            remaining_forced_changes: Some(remaining_forced_changes),
            action,
        });
        !matches!(action, MovementSpeedAckActionLikeCpp::Kicked)
    }
}
