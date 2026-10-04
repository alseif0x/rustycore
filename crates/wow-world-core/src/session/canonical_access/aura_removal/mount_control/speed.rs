// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{AuraMountControlAccessLikeCpp, PlayerAuraRemovalAccessLikeCpp};
use crate::session::movement_protocol::{UnitMoveTypeLikeCpp, PLAYER_BASE_MOVE_SPEED_LIKE_CPP};
use wow_entities::RepresentedAuraEffectLikeCpp;
use tracing::warn;
use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};
use std::time::Instant;
use wow_packet::ServerPacket;
use wow_core::ObjectGuid;
use wow_constants::{MovementFlag, ServerOpcodes};

impl AuraMountControlAccessLikeCpp<'_> {
    pub fn update_flight_flags_for_aura_like_cpp(
        &mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>, apply: bool,
    ) {
        let should_enable = if apply {
            true
        } else {
            let Some(has_fly) = presentation.visible_auras_snapshot_like_cpp()
                .map(|auras| auras.values().any(|aura| aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Fly)))
            else { return; };
            let Some(has_mounted_flight_speed) = presentation.visible_auras_snapshot_like_cpp()
                .map(|auras| auras.values().any(|aura| aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MountedFlightSpeed)))
            else { return; };
            has_fly || has_mounted_flight_speed
        };
        self.set_represented_can_swim_to_fly_transition_like_cpp(should_enable);
        let can_fly_changed = self.set_represented_can_fly_like_cpp(should_enable);
        if !should_enable && can_fly_changed {
            self.move_represented_player_fall_like_cpp();
        }
    }

    pub fn set_represented_can_fly_like_cpp(&mut self, enable: bool) -> bool {
        let Some(mut movement_flags) = self.resolved_player_movement_flags_like_cpp()
        else {
            return false;
        };
        let currently_enabled = movement_flags.contains(MovementFlag::CAN_FLY);
        if enable == currently_enabled {
            return false;
        }

        if enable {
            movement_flags.insert(MovementFlag::CAN_FLY);
            movement_flags.remove(MovementFlag::SWIMMING | MovementFlag::SPLINE_ELEVATION);
        } else {
            movement_flags.remove(MovementFlag::CAN_FLY | MovementFlag::MASK_MOVING_FLY);
            if let Some(position) = self.player_position_like_cpp() {
                self.set_fall_information_like_cpp(0, position.z);
            }
        }
        self.set_player_movement_flags_like_cpp(movement_flags);

        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveSetCanFly
        } else {
            ServerOpcodes::MoveUnsetCanFly
        });
        true
    }

    pub fn set_represented_can_swim_to_fly_transition_like_cpp(&mut self, enable: bool) -> bool {
        let canonical_changed = self.core.with_owned_player_mut_like_cpp(|player| {
            player.set_can_transition_between_swim_and_fly_like_cpp(enable)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let changed = canonical_changed.unwrap_or_else(|| {
            if self.core.player_handle_like_cpp.is_some()
                || *self.fixtures.can_swim_to_fly
                    == enable
            {
                return false;
            }
            *self.fixtures.can_swim_to_fly = enable;
            true
        });
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let Some(changed) = canonical_changed else {
            return false;
        };
        if !changed {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical_changed.is_some() {
            *self.fixtures.can_swim_to_fly = enable;
        }
        self.send_player_move_set_flag_like_cpp(if enable {
            ServerOpcodes::MoveEnableTransitionBetweenSwimAndFly
        } else {
            ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly
        });
        true
    }

    pub fn move_represented_player_fall_like_cpp(&mut self) -> bool {
        let Some(mut movement_flags) = self.resolved_player_movement_flags_like_cpp()
        else {
            return false;
        };
        if movement_flags.contains(MovementFlag::DISABLE_GRAVITY) {
            return false;
        }

        movement_flags.insert(MovementFlag::FALLING);
        self.set_player_movement_flags_like_cpp(movement_flags);
        if let Some(position) = self.player_position_like_cpp() {
            self.set_fall_information_like_cpp(0, position.z);
        }
        true
    }

    pub fn set_fall_information_like_cpp(&mut self, time: u32, z: f32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_fall_information_like_cpp(time, z);
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            *self.fixtures.fall_time = time;
            *self.fixtures.fall_z = z;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    pub(in crate::session) fn send_player_move_set_flag_like_cpp(&mut self, opcode: ServerOpcodes) {
        use wow_packet::ServerPacket;

        let Some(player_guid) = self.core.player_guid() else {
            return;
        };
        let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
            return;
        };

        let self_packet = wow_packet::packets::movement::MoveSetFlag {
            opcode,
            mover_guid: player_guid,
            sequence_index,
        }
        .to_bytes();
        if self.core.send_tx().send(self_packet).is_err() {
            warn!("Send channel closed for account {}", self.core.account_id);
        }

        let Some(status) = self
            .current_player_movement_info_like_cpp(player_guid)
        else {
            return;
        };
        self.broadcast_speed_packet_like_cpp(
            wow_packet::packets::movement::MoveUpdate { info: status }.to_bytes(),
            false,
        );
    }

    fn player_position_like_cpp(&self) -> Option<wow_core::Position> {
        self.core.player_position_with_fixture_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.position,
        )
    }

    fn set_player_movement_flags_like_cpp(&mut self, flags: MovementFlag) {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_movement_flags_like_cpp(flags);
        }).is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            *self.fixtures.flags = flags;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = canonical;
    }
}


impl AuraMountControlAccessLikeCpp<'_> {
    pub(crate) fn propagate_represented_player_speed_to_pet_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        if self.resolved_in_combat_like_cpp() != Some(false) {
            return;
        }

        let rate = rate.max(0.01);
        let index = move_type.index();
        let canonical_changed = self.core.with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
            let unit = pet.creature_mut().unit_mut();
            if unit.speed_rate_at_like_cpp(index) == Some(rate) {
                return false;
            }
            unit.set_speed_rate_at_like_cpp(index, rate)
        });
        let changed = match canonical_changed {
            Some(changed) => changed,
            None => {
                #[cfg(any(test, feature = "test-fixtures"))]
                {
                    if self.core.player_handle_like_cpp.is_none() {
                        if self.fixtures.pet_speed_rates[index]
                            == rate
                        {
                            return;
                        }
                        self.fixtures.pet_speed_rates[index] = rate;
                        true
                    } else {
                        false
                    }
                }
                #[cfg(not(any(test, feature = "test-fixtures")))]
                {
                    false
                }
            }
        };
        if !changed {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            *self.fixtures.pet_speed_propagations = self.fixtures.pet_speed_propagations.saturating_add(1);
        }
        self.core.send_pet_spline_speed_with_fixture_like_cpp(pet_guid, move_type, rate,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixtures.position,
        );
    }

    fn player_pet_guid_state_like_cpp(&self) -> Option<Option<wow_core::ObjectGuid>> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.gameplay_state().pet_guid);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.fixtures.pet_guid);
        }
        canonical
    }

    fn resolved_in_combat_like_cpp(&self) -> Option<bool> {
        let canonical = self.core.with_owned_player_like_cpp(|player| player.unit().subsystems().combat.has_combat());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.fixtures.in_combat);
        }
        canonical
    }

    fn broadcast_speed_packet_like_cpp(&self, bytes: Vec<u8>, _include_self: bool) {
        self.core.packet_publication_access_like_cpp()
            .broadcast_to_movement_set_in_range_and_connection_like_cpp(
                bytes, crate::map_manager::VISIBILITY_RADIUS, false,
                #[cfg(any(test, feature = "test-fixtures"))] self.fixtures.position,
            );
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn send_pet_spline_speed_with_fixture_like_cpp(
        &self,
        pet_guid: ObjectGuid,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
        #[cfg(any(test, feature = "test-fixtures"))] position: &Option<wow_core::Position>,
    ) {
        let Some(opcode) =
            crate::session::creature_movement_spline_speed_opcode_like_cpp(move_type)
        else {
            return;
        };
        let packet_bytes = wow_packet::packets::movement::MoveSplineSetSpeed {
            opcode,
            mover_guid: pet_guid,
            speed: PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()] * rate,
        }
        .to_bytes();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        if self.client_visible_guids_like_cpp.contains(&pet_guid)
            && self.send_tx().send(packet_bytes.clone()).is_err()
        {
            warn!("Send channel closed for account {}", self.account_id);
        }

        let (Some(player_guid), Some(registry)) =
            (self.player_guid(), self.player_registry())
        else {
            return;
        };
        let Some(source_position) = self
            .represented_pet_position_like_cpp(pet_guid)
            .or_else(|| self.player_position_with_fixture_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))] position,
            ))
        else {
            return;
        };
        for registration in registry.movement_recipients_within_range(
            player_guid,
            map_id,
            instance_id,
            source_position,
            crate::map_manager::VISIBILITY_RADIUS,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: pet_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                }),
            );
        }
    }
}


impl AuraMountControlAccessLikeCpp<'_> {
    pub fn recompute_represented_run_speed_rate_like_cpp(&mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) {
        let Some(speed) = self.represented_run_speed_rate_like_cpp(presentation) else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(presentation, UnitMoveTypeLikeCpp::Run, speed);
    }

    pub fn recompute_represented_flight_speed_rate_like_cpp(&mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) {
        let Some(speed) = self.represented_flight_speed_rate_like_cpp(presentation) else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(presentation, UnitMoveTypeLikeCpp::Flight, speed);
    }

    pub fn recompute_represented_swim_speed_rate_like_cpp(&mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) {
        let Some(speed) = self.represented_swim_speed_rate_like_cpp(presentation) else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(presentation, UnitMoveTypeLikeCpp::Swim, speed);
    }

    pub fn recompute_represented_backward_speed_rates_like_cpp(&mut self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) {
        let Some(speed) = self.represented_backward_speed_rate_like_cpp(presentation) else {
            return;
        };
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            presentation, UnitMoveTypeLikeCpp::RunBack,
            speed,
        );
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            presentation, UnitMoveTypeLikeCpp::SwimBack,
            speed,
        );
        self.set_player_movement_speed_rate_and_notify_like_cpp(
            presentation, UnitMoveTypeLikeCpp::FlightBack,
            speed,
        );
    }

    fn represented_run_speed_rate_like_cpp(&self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) -> Option<f32> {
        let mounted = presentation.resolved_player_mounted_like_cpp()?;
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
        let main_mod = self.max_represented_aura_amount_like_cpp(presentation, main_mod_effect)?;
        let stack_bonus = self.total_represented_aura_amount_multiplier_like_cpp(presentation, stack_effect)?;
        let not_stack_bonus = 1.0
            + self
                .max_represented_aura_amount_like_cpp(presentation, not_stack_effect)?
                .max(0) as f32
                / 100.0;

        let speed = stack_bonus.max(not_stack_bonus) * (1.0 + main_mod.max(0) as f32 / 100.0);
        Some(self.apply_represented_forward_speed_adjustments_like_cpp(presentation,
            UnitMoveTypeLikeCpp::Run,
            speed,
        )?)
    }

    fn apply_represented_forward_speed_adjustments_like_cpp(
        &self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        move_type: UnitMoveTypeLikeCpp,
        mut speed: f32,
    ) -> Option<f32> {
        let normal_speed_cap = self.max_represented_aura_amount_like_cpp(presentation,
            RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
        )? as f32
            / PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()];
        if normal_speed_cap > 0.0 && speed > normal_speed_cap {
            speed = normal_speed_cap;
        }
        if move_type == UnitMoveTypeLikeCpp::Run {
            let minimum_speed_rate = self.max_represented_aura_amount_like_cpp(presentation,
                RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
            )? as f32
                / PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()];
            if speed < minimum_speed_rate {
                speed = minimum_speed_rate;
            }
        }
        let slow = self.max_negative_represented_aura_amount_like_cpp(presentation,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
        )?;
        if slow < 0 {
            speed *= 1.0 + slow as f32 / 100.0;
        }
        let minimum_speed = self
            .max_represented_aura_amount_like_cpp(presentation, RepresentedAuraEffectLikeCpp::MinimumSpeed)?
            as f32
            / 100.0;
        if speed < minimum_speed {
            speed = minimum_speed;
        }
        Some(speed)
    }

    fn represented_flight_speed_rate_like_cpp(&self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) -> Option<f32> {
        let mounted = presentation.resolved_player_mounted_like_cpp()?;
        let (flight_mod, flight_always) = if mounted {
            (
                self.max_represented_aura_amount_like_cpp(presentation,
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                )?,
                self.total_represented_aura_amount_multiplier_like_cpp(presentation,
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways,
                )?,
            )
        } else {
            (
                self.total_represented_aura_amount_like_cpp(presentation,
                    RepresentedAuraEffectLikeCpp::FlightSpeed,
                )? + self.total_represented_aura_amount_like_cpp(presentation,
                    RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
                )?,
                1.0,
            )
        };
        let flight_not_stack = 1.0
            + self
                .max_represented_aura_amount_like_cpp(presentation,
                    RepresentedAuraEffectLikeCpp::FlightSpeedNotStack,
                )?
                .max(0) as f32
                / 100.0;

        let speed = flight_always.max(flight_not_stack) * (1.0 + flight_mod.max(0) as f32 / 100.0);
        Some(self.apply_represented_forward_speed_adjustments_like_cpp(presentation,
            UnitMoveTypeLikeCpp::Flight,
            speed,
        )?)
    }

    fn represented_swim_speed_rate_like_cpp(&self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) -> Option<f32> {
        let speed = 1.0
            + self
                .max_represented_aura_amount_like_cpp(presentation, RepresentedAuraEffectLikeCpp::SwimSpeed)?
                .max(0) as f32
                / 100.0;
        self.apply_represented_forward_speed_adjustments_like_cpp(presentation, UnitMoveTypeLikeCpp::Swim, speed)
    }

    fn represented_backward_speed_rate_like_cpp(&self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>) -> Option<f32> {
        let mut speed = 1.0;
        let slow = self.max_negative_represented_aura_amount_like_cpp(presentation,
            RepresentedAuraEffectLikeCpp::DecreaseSpeed,
        )?;
        if slow < 0 {
            speed *= 1.0 + slow as f32 / 100.0;
        }
        let minimum_speed = self
            .max_represented_aura_amount_like_cpp(presentation, RepresentedAuraEffectLikeCpp::MinimumSpeed)?
            as f32
            / 100.0;
        if speed < minimum_speed {
            speed = minimum_speed;
        }
        Some(speed)
    }

    pub fn set_player_movement_speed_rate_and_notify_like_cpp(
        &mut self,
        _presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let rate = rate.max(0.01);
        let Some(current_rate) = self
            .resolved_player_movement_speed_rate_like_cpp(move_type)
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

        let Some(player_guid) = self.core.player_guid() else {
            return;
        };
        let Some((set_opcode, update_opcode)) =
            crate::session::player_movement_speed_opcodes_like_cpp(move_type)
        else {
            return;
        };

        if self
            .increment_forced_speed_changes_like_cpp(move_type)
            .is_none()
        {
            return;
        }

        let Some(speed) = self
            .resolved_player_movement_speed_like_cpp(move_type)
        else {
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
        if self.core.send_tx().send(self_packet).is_err() {
            warn!("Send channel closed for account {}", self.core.account_id);
        }

        let Some(status) = self
            .current_player_movement_info_like_cpp(player_guid)
        else {
            return;
        };
        self.broadcast_speed_packet_like_cpp(
            wow_packet::packets::movement::MoveUpdateSpeed {
                opcode: update_opcode,
                status,
                speed,
            }
            .to_bytes(),
            false,
        );
    }

    pub(crate) fn set_player_movement_speed_rate_like_cpp_inner(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) -> bool {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_speed_rate_at_like_cpp(index, rate)
            })
            .unwrap_or(false);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.speed_rates[index] = rate;
        }
        canonical
            || cfg!(any(test, feature = "test-fixtures"))
                && self.core.player_handle_like_cpp.is_none()
    }

    fn increment_forced_speed_changes_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<u8> {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.increment_forced_speed_changes_like_cpp(index)
            })
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let count = &mut self.fixtures.forced_changes[index];
            *count = count.saturating_add(1);
            return Some(*count);
        }
        canonical
    }

    pub fn resolved_player_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        let index = move_type.index();
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().speed_rate_at_like_cpp(index))
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.speed_rates[index]);
        }
        canonical
    }

    pub fn resolved_player_movement_speed_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> Option<f32> {
        Some(
            PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()]
                * self.resolved_player_movement_speed_rate_like_cpp(move_type)?,
        )
    }
}

impl crate::session::state::SessionCore {
    pub(crate) fn max_represented_aura_amount_like_cpp_from_snapshot(
        auras: Option<std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        auras.map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount > 0)
                .max()
                .unwrap_or(0)
        })
    }
    pub(crate) fn max_negative_represented_aura_amount_like_cpp_from_snapshot(
        auras: Option<std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        auras.map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount < 0)
                .min()
                .unwrap_or(0)
        })
    }
    pub(crate) fn total_represented_aura_amount_multiplier_like_cpp_from_snapshot(
        auras: Option<std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        auras.map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |multiplier, aura| {
                    multiplier * (1.0 + aura.represented_amount.max(0) as f32 / 100.0)
                })
        })
    }
    pub(crate) fn total_represented_aura_amount_like_cpp_from_snapshot(
        auras: Option<std::collections::HashMap<u8, wow_entities::AuraApplicationLikeCpp>>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        auras.map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }
}

impl AuraMountControlAccessLikeCpp<'_> {
    fn max_represented_aura_amount_like_cpp(
        &self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        crate::session::state::SessionCore::max_represented_aura_amount_like_cpp_from_snapshot(presentation.visible_auras_snapshot_like_cpp(), effect)
    }
    fn max_negative_represented_aura_amount_like_cpp(
        &self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        crate::session::state::SessionCore::max_negative_represented_aura_amount_like_cpp_from_snapshot(presentation.visible_auras_snapshot_like_cpp(), effect)
    }
    fn total_represented_aura_amount_multiplier_like_cpp(
        &self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        crate::session::state::SessionCore::total_represented_aura_amount_multiplier_like_cpp_from_snapshot(presentation.visible_auras_snapshot_like_cpp(), effect)
    }
    fn total_represented_aura_amount_like_cpp(
        &self, presentation: &PlayerAuraRemovalAccessLikeCpp<'_>,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        crate::session::state::SessionCore::total_represented_aura_amount_like_cpp_from_snapshot(presentation.visible_auras_snapshot_like_cpp(), effect)
    }
}
