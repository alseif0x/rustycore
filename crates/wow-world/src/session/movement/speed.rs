//! Represented movement speed and its published changes.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn movement_speed_ack_move_type_like_cpp(
        opcode: ClientOpcodes,
    ) -> Option<UnitMoveTypeLikeCpp> {
        match opcode {
            ClientOpcodes::MoveForceWalkSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Walk),
            ClientOpcodes::MoveForceRunSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Run),
            ClientOpcodes::MoveForceRunBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::RunBack),
            ClientOpcodes::MoveForceSwimSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Swim),
            ClientOpcodes::MoveForceSwimBackSpeedChangeAck => Some(UnitMoveTypeLikeCpp::SwimBack),
            ClientOpcodes::MoveForceTurnRateChangeAck => Some(UnitMoveTypeLikeCpp::TurnRate),
            ClientOpcodes::MoveForceFlightSpeedChangeAck => Some(UnitMoveTypeLikeCpp::Flight),
            ClientOpcodes::MoveForceFlightBackSpeedChangeAck => {
                Some(UnitMoveTypeLikeCpp::FlightBack)
            }
            ClientOpcodes::MoveForcePitchRateChangeAck => Some(UnitMoveTypeLikeCpp::PitchRate),
            _ => None,
        }
    }
    pub(in crate::session) fn resolved_player_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_like_cpp(|player| player.unit().speed_rate_at_like_cpp(index))
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.movement_speed_rates_like_cpp[index]);
        }
        canonical
    }
    fn set_player_movement_speed_rate_like_cpp_inner(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) -> bool {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_speed_rate_at_like_cpp(index, rate)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.movement_speed_rates_like_cpp[index] = rate;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    fn resolved_player_movement_speed_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        Some(
            PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()]
                * self.resolved_player_movement_speed_rate_like_cpp(move_type)?,
        )
    }
    #[cfg(test)]
    pub(crate) fn player_movement_speed_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> f32 {
        self.resolved_player_movement_speed_like_cpp(move_type)
            .expect("test Player movement-speed owner must resolve")
    }
    fn represented_run_speed_rate_like_cpp(&self) -> Option<f32> {
        let mounted = self.resolved_player_mounted_like_cpp()?;
        let (main_mod_effect, stack_effect, not_stack_effect) = if mounted {
            (
                RepresentedAuraEffectLikeCpp::MountedSpeed,
                RepresentedAuraEffectLikeCpp::MountedSpeedAlways,
                RepresentedAuraEffectLikeCpp::MountedSpeedNotStack,
            )
        } else {
            (
                RepresentedAuraEffectLikeCpp::Speed,
                RepresentedAuraEffectLikeCpp::SpeedAlways,
                RepresentedAuraEffectLikeCpp::SpeedNotStack,
            )
        };
        let main_mod = self.max_represented_aura_amount_like_cpp(main_mod_effect)?;
        let stack_bonus = self.total_represented_aura_amount_multiplier_like_cpp(stack_effect)?;
        let not_stack_bonus = 1.0
            + self
                .max_represented_aura_amount_like_cpp(not_stack_effect)?
                .max(0) as f32
                / 100.0;

        let speed = stack_bonus.max(not_stack_bonus) * (1.0 + main_mod.max(0) as f32 / 100.0);
        Some(self.apply_represented_forward_speed_adjustments_like_cpp(
            UnitMoveTypeLikeCpp::Run,
            speed,
        )?)
    }
    fn apply_represented_forward_speed_adjustments_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
        mut speed: f32,
    ) -> Option<f32> {
        let normal_speed_cap = self.max_represented_aura_amount_like_cpp(
            RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
        )? as f32
            / PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()];
        if normal_speed_cap > 0.0 && speed > normal_speed_cap {
            speed = normal_speed_cap;
        }
        if move_type == UnitMoveTypeLikeCpp::Run {
            let minimum_speed_rate = self.max_represented_aura_amount_like_cpp(
                RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
            )? as f32
                / PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()];
            if speed < minimum_speed_rate {
                speed = minimum_speed_rate;
            }
        }
        let slow = self.max_negative_represented_aura_amount_like_cpp(
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
        )?;
        if slow < 0 {
            speed *= 1.0 + slow as f32 / 100.0;
        }
        let minimum_speed = self
            .max_represented_aura_amount_like_cpp(RepresentedAuraEffectLikeCpp::MinimumSpeed)?
            as f32
            / 100.0;
        if speed < minimum_speed {
            speed = minimum_speed;
        }
        Some(speed)
    }
    fn represented_flight_speed_rate_like_cpp(&self) -> Option<f32> {
        let mounted = self.resolved_player_mounted_like_cpp()?;
        let (flight_mod, flight_always) = if mounted {
            (
                self.max_represented_aura_amount_like_cpp(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                )?,
                self.total_represented_aura_amount_multiplier_like_cpp(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways,
                )?,
            )
        } else {
            (
                self.total_represented_aura_amount_like_cpp(
                    RepresentedAuraEffectLikeCpp::FlightSpeed,
                )? + self.total_represented_aura_amount_like_cpp(
                    RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
                )?,
                1.0,
            )
        };
        let flight_not_stack = 1.0
            + self
                .max_represented_aura_amount_like_cpp(
                    RepresentedAuraEffectLikeCpp::FlightSpeedNotStack,
                )?
                .max(0) as f32
                / 100.0;

        let speed = flight_always.max(flight_not_stack) * (1.0 + flight_mod.max(0) as f32 / 100.0);
        Some(self.apply_represented_forward_speed_adjustments_like_cpp(
            UnitMoveTypeLikeCpp::Flight,
            speed,
        )?)
    }
    fn represented_swim_speed_rate_like_cpp(&self) -> Option<f32> {
        let speed = 1.0
            + self
                .max_represented_aura_amount_like_cpp(RepresentedAuraEffectLikeCpp::SwimSpeed)?
                .max(0) as f32
                / 100.0;
        self.apply_represented_forward_speed_adjustments_like_cpp(UnitMoveTypeLikeCpp::Swim, speed)
    }
    fn represented_backward_speed_rate_like_cpp(&self) -> Option<f32> {
        let mut speed = 1.0;
        let slow = self.max_negative_represented_aura_amount_like_cpp(
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
        )?;
        if slow < 0 {
            speed *= 1.0 + slow as f32 / 100.0;
        }
        let minimum_speed = self
            .max_represented_aura_amount_like_cpp(RepresentedAuraEffectLikeCpp::MinimumSpeed)?
            as f32
            / 100.0;
        if speed < minimum_speed {
            speed = minimum_speed;
        }
        Some(speed)
    }
    pub(in crate::session) fn recompute_represented_run_speed_rate_like_cpp(&mut self) {
        let Some(speed) = self.represented_run_speed_rate_like_cpp() else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, speed);
    }
    pub(in crate::session) fn recompute_represented_flight_speed_rate_like_cpp(&mut self) {
        let Some(speed) = self.represented_flight_speed_rate_like_cpp() else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Flight, speed);
    }
    pub(in crate::session) fn recompute_represented_swim_speed_rate_like_cpp(&mut self) {
        let Some(speed) = self.represented_swim_speed_rate_like_cpp() else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Swim, speed);
    }
    pub(in crate::session) fn recompute_represented_backward_speed_rates_like_cpp(&mut self) {
        let Some(speed) = self.represented_backward_speed_rate_like_cpp() else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            UnitMoveTypeLikeCpp::RunBack,
            speed,
        );
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            UnitMoveTypeLikeCpp::SwimBack,
            speed,
        );
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            UnitMoveTypeLikeCpp::FlightBack,
            speed,
        );
    }
    pub(in crate::session) fn recompute_represented_mounted_speed_rates_like_cpp(&mut self) {
        self.recompute_represented_run_speed_rate_like_cpp();
        self.recompute_represented_flight_speed_rate_like_cpp();
    }
    pub(in crate::session) fn recompute_represented_forward_speed_rates_like_cpp(&mut self) {
        self.recompute_represented_run_speed_rate_like_cpp();
        self.recompute_represented_swim_speed_rate_like_cpp();
        self.recompute_represented_flight_speed_rate_like_cpp();
    }
    fn player_movement_speed_opcodes_like_cpp(
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<(ServerOpcodes, ServerOpcodes)> {
        match move_type {
            UnitMoveTypeLikeCpp::Run => Some((
                ServerOpcodes::MoveSetRunSpeed,
                ServerOpcodes::MoveUpdateRunSpeed,
            )),
            UnitMoveTypeLikeCpp::Flight => Some((
                ServerOpcodes::MoveSetFlightSpeed,
                ServerOpcodes::MoveUpdateFlightSpeed,
            )),
            UnitMoveTypeLikeCpp::Swim => Some((
                ServerOpcodes::MoveSetSwimSpeed,
                ServerOpcodes::MoveUpdateSwimSpeed,
            )),
            UnitMoveTypeLikeCpp::RunBack => Some((
                ServerOpcodes::MoveSetRunBackSpeed,
                ServerOpcodes::MoveUpdateRunBackSpeed,
            )),
            UnitMoveTypeLikeCpp::SwimBack => Some((
                ServerOpcodes::MoveSetSwimBackSpeed,
                ServerOpcodes::MoveUpdateSwimBackSpeed,
            )),
            UnitMoveTypeLikeCpp::FlightBack => Some((
                ServerOpcodes::MoveSetFlightBackSpeed,
                ServerOpcodes::MoveUpdateFlightBackSpeed,
            )),
            _ => None,
        }
    }
    pub(in crate::session) fn set_player_movement_speed_rate_and_notify_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let rate = rate.max(0.01);
        let Some(current_rate) = self.resolved_player_movement_speed_rate_like_cpp(move_type)
        else {
            return;
        };
        if current_rate == rate {
            return;
        }
        if !self.set_player_movement_speed_rate_like_cpp_inner(move_type, rate) {
            return;
        }
        self.propagate_represented_player_speed_to_pet_like_cpp(move_type, rate);

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some((set_opcode, update_opcode)) =
            Self::player_movement_speed_opcodes_like_cpp(move_type)
        else {
            return;
        };

        if self
            .increment_forced_speed_changes_like_cpp(move_type)
            .is_none()
        {
            return;
        }

        let Some(speed) = self.resolved_player_movement_speed_like_cpp(move_type) else {
            return;
        };
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
            return;
        };

        let self_packet = wow_packet::packets::movement::MoveSetSpeed {
            opcode: set_opcode,
            mover_guid: player_guid,
            sequence_index,
            speed,
        }
        .to_bytes();
        if self.send_tx().send(self_packet).is_err() {
            warn!("Send channel closed for account {}", self.account_id);
        }

        let Some(status) = self.current_player_movement_info_like_cpp(player_guid) else {
            return;
        };
        self.broadcast_to_movement_set_like_cpp(
            wow_packet::packets::movement::MoveUpdateSpeed {
                opcode: update_opcode,
                status,
                speed,
            }
            .to_bytes(),
            false,
        );
    }
    pub(crate) fn handle_force_speed_change_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
        speed: f32,
    ) -> bool {
        let Some(move_type) = Self::movement_speed_ack_move_type_like_cpp(opcode) else {
            self.trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.UnknownMoveType",
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
        };

        if !self.record_validated_movement_ack_like_cpp(opcode, ack, Some(speed)) {
            self.trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.InvalidMovementAck",
                Some(opcode),
                "kick",
            );
            self.record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: Some(move_type),
                ack_speed: speed,
                expected_speed: None,
                remaining_forced_changes: None,
                action: MovementSpeedAckActionLikeCpp::Kicked,
            });
            return false;
        }

        let Some(mut remaining_forced_changes) =
            self.resolved_forced_speed_changes_like_cpp(move_type)
        else {
            return false;
        };
        if remaining_forced_changes > 0 {
            let Some(remaining) = self.consume_forced_speed_change_like_cpp(move_type) else {
                return false;
            };
            remaining_forced_changes = remaining;
            if remaining_forced_changes > 0 {
                self.record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                    opcode,
                    move_type: Some(move_type),
                    ack_speed: speed,
                    expected_speed: self.resolved_player_movement_speed_like_cpp(move_type),
                    remaining_forced_changes: Some(remaining_forced_changes),
                    action: MovementSpeedAckActionLikeCpp::SkippedPending,
                });
                return true;
            }
        }

        let Some(expected_speed) = self.resolved_player_movement_speed_like_cpp(move_type) else {
            return false;
        };
        let Some(player_on_transport) = self.player_on_transport_state_like_cpp() else {
            return false;
        };
        let action = if !player_on_transport && (expected_speed - speed).abs() > 0.01 {
            if expected_speed > speed {
                // C++ calls SetSpeedRate(GetSpeedRate()) to force the client back to the server value.
                self.trace_anticheat_violation_like_cpp(
                    "HandleForceSpeedChangeAck.ClientSpeedLower",
                    Some(opcode),
                    "correct",
                );
                MovementSpeedAckActionLikeCpp::Corrected
            } else {
                self.trace_anticheat_violation_like_cpp(
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

        self.record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
            opcode,
            move_type: Some(move_type),
            ack_speed: speed,
            expected_speed: Some(expected_speed),
            remaining_forced_changes: Some(remaining_forced_changes),
            action,
        });
        !matches!(action, MovementSpeedAckActionLikeCpp::Kicked)
    }
    pub(in crate::session) fn record_movement_speed_ack_event_like_cpp(
        &mut self,
        event: MovementSpeedAckEventLikeCpp,
    ) {
        #[cfg(test)]
        self.movement_speed_ack_events_like_cpp.push(event);
        #[cfg(not(test))]
        let _ = event;
    }
    pub(in crate::session) fn resolved_forced_speed_changes_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_like_cpp(|player| player.forced_speed_changes_like_cpp(index))
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.forced_speed_changes_like_cpp[index]);
        }
        canonical
    }
    fn increment_forced_speed_changes_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.increment_forced_speed_changes_like_cpp(index)
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let count = &mut self.forced_speed_changes_like_cpp[index];
            *count = count.saturating_add(1);
            return Some(*count);
        }
        canonical
    }
    fn consume_forced_speed_change_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.consume_forced_speed_change_like_cpp(index)
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let count = &mut self.forced_speed_changes_like_cpp[index];
            if *count > 0 {
                *count = count.saturating_sub(1);
            }
            return Some(*count);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn set_forced_speed_changes_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        count: u8,
    ) {
        let index = move_type.index();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_forced_speed_changes_like_cpp(index, count)
            })
            .unwrap_or(false);
        if canonical || self.player_handle_like_cpp.is_none() {
            self.forced_speed_changes_like_cpp[index] = count;
        }
    }
    #[cfg(test)]
    pub(crate) fn forced_speed_changes_like_cpp(&self, move_type: UnitMoveTypeLikeCpp) -> u8 {
        self.resolved_forced_speed_changes_like_cpp(move_type)
            .expect("test Player forced-speed owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn set_player_movement_speed_rate_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let _ = self.set_player_movement_speed_rate_like_cpp_inner(move_type, rate.max(0.01));
    }
    #[cfg(test)]
    pub(crate) fn movement_speed_ack_events_like_cpp(&self) -> &[MovementSpeedAckEventLikeCpp] {
        &self.movement_speed_ack_events_like_cpp
    }
}
