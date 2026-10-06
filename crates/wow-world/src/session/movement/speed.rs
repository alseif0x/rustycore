//! Represented movement speed and its published changes.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn handle_force_speed_change_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
        speed: f32,
    ) -> bool {
        let Some(move_type) = crate::session::movement_speed_ack_move_type_like_cpp(opcode) else {
            crate::session::hub_ref(self).trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.UnknownMoveType",
                Some(opcode),
                "kick",
            );
            crate::session::hub_mut(self).record_movement_speed_ack_event_like_cpp(
                MovementSpeedAckEventLikeCpp {
                    opcode,
                    move_type: None,
                    ack_speed: speed,
                    expected_speed: None,
                    remaining_forced_changes: None,
                    action: MovementSpeedAckActionLikeCpp::Kicked,
                },
            );
            return false;
        };

        if !self.record_validated_movement_ack_like_cpp(opcode, ack, Some(speed)) {
            crate::session::hub_ref(self).trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.InvalidMovementAck",
                Some(opcode),
                "kick",
            );
            crate::session::hub_mut(self).record_movement_speed_ack_event_like_cpp(
                MovementSpeedAckEventLikeCpp {
                    opcode,
                    move_type: Some(move_type),
                    ack_speed: speed,
                    expected_speed: None,
                    remaining_forced_changes: None,
                    action: MovementSpeedAckActionLikeCpp::Kicked,
                },
            );
            return false;
        }

        let Some(mut remaining_forced_changes) =
            crate::session::hub_ref(self).resolved_forced_speed_changes_like_cpp(move_type)
        else {
            return false;
        };
        if remaining_forced_changes > 0 {
            let Some(remaining) =
                crate::session::hub_mut(self).consume_forced_speed_change_like_cpp(move_type)
            else {
                return false;
            };
            remaining_forced_changes = remaining;
            if remaining_forced_changes > 0 {
                {
                    let a0 = MovementSpeedAckEventLikeCpp {
                        opcode,
                        move_type: Some(move_type),
                        ack_speed: speed,
                        expected_speed: crate::session::hub_ref(self)
                            .resolved_player_movement_speed_like_cpp(move_type),
                        remaining_forced_changes: Some(remaining_forced_changes),
                        action: MovementSpeedAckActionLikeCpp::SkippedPending,
                    };
                    crate::session::hub_mut(self).record_movement_speed_ack_event_like_cpp(a0)
                };
                return true;
            }
        }

        let Some(expected_speed) =
            crate::session::hub_ref(self).resolved_player_movement_speed_like_cpp(move_type)
        else {
            return false;
        };
        let Some(player_on_transport) =
            crate::session::hub_ref(self).player_on_transport_state_like_cpp()
        else {
            return false;
        };
        let action = if !player_on_transport && (expected_speed - speed).abs() > 0.01 {
            if expected_speed > speed {
                // C++ calls SetSpeedRate(GetSpeedRate()) to force the client back to the server value.
                crate::session::hub_ref(self).trace_anticheat_violation_like_cpp(
                    "HandleForceSpeedChangeAck.ClientSpeedLower",
                    Some(opcode),
                    "correct",
                );
                MovementSpeedAckActionLikeCpp::Corrected
            } else {
                crate::session::hub_ref(self).trace_anticheat_violation_like_cpp(
                    "HandleForceSpeedChangeAck.ClientSpeedHigher",
                    Some(opcode),
                    "kick",
                );
                self.kick("WorldSession::HandleForceSpeedChangeAck Incorrect speed");
                MovementSpeedAckActionLikeCpp::Kicked
            }
        } else {
            MovementSpeedAckActionLikeCpp::Accepted
        };

        crate::session::hub_mut(self).record_movement_speed_ack_event_like_cpp(
            MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: Some(move_type),
                ack_speed: speed,
                expected_speed: Some(expected_speed),
                remaining_forced_changes: Some(remaining_forced_changes),
                action,
            },
        );
        !matches!(action, MovementSpeedAckActionLikeCpp::Kicked)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/speed/f3_shims.rs"]
mod f3_shims;
