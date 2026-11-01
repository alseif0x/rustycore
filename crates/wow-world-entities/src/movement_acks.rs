// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement acknowledgements, represented validation gates, falling,
//! knockback, speed changes and spline completion (#1363 slice 4b; moved from
//! the World `session/movement/{fall,speed,movement_validation,state}.rs` and
//! `session/movement/state/spline_progression.rs`).
//!
//! C++ refs: `Player::ValidateMovementInfo`, `Player::HandleFall`,
//! `WorldSession::HandleForceSpeedChangeAck`, the movement-force and knockback
//! ACK handlers and `WorldSession::HandleMoveSplineDoneOpcode`
//! (`Handlers/MovementHandler.cpp`).

use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_core::ObjectGuid;
use wow_entities::RepresentedAuraEffectLikeCpp;
use wow_world_core::session::movement_protocol::{
    MoveSplineDoneTaxiActionLikeCpp, MovementAckEventLikeCpp, MovementFallDamageEvent,
    MovementSpeedAckActionLikeCpp, MovementSpeedAckEventLikeCpp, MovementUnderMapDamageEvent,
};
use wow_world_core::session::{DAMAGE_FALL_LIKE_CPP, DAMAGE_FALL_TO_VOID_LIKE_CPP};

use crate::movement_host::PlayerMovementHostLikeCpp;
use crate::player_resurrection::PLAYER_FLAGS_IS_OUT_OF_BOUNDS;

pub async fn handle_fall_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    movement_info: &wow_packet::packets::movement::MovementInfo,
    creature_spawns: &crate::CreatureSpawnCatalogsLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Option<MovementFallDamageEvent> {
    let (_, last_fall_z) = host.hub_ref().resolved_fall_information_like_cpp()?;
    let damage_control = host.hub_ref().resolved_player_damage_control_like_cpp()?;
    let z_diff = last_fall_z - movement_info.position.z;
    let (_, max_health, player_is_alive) = host.hub_ref().resolved_player_vitals_like_cpp()?;
    if z_diff < 14.57
        || !player_is_alive
        || host.hub_ref().player_is_game_master_like_cpp() == Some(true)
        || host
            .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Hover)?
        || host.resolved_has_represented_aura_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::FeatherFall,
        )?
        || host.resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Fly)?
        || damage_control.normal_damage_immune
    {
        return None;
    }

    let safe_fall = host
        .hub_ref()
        .resolved_total_represented_aura_modifier_like_cpp(
            RepresentedAuraEffectLikeCpp::SafeFall,
        )?;
    let damage_percent = 0.018 * (z_diff - safe_fall as f32) - 0.2426;
    if damage_percent <= 0.0 {
        return None;
    }

    let mut damage = (damage_percent * max_health as f32) as u32;
    if damage_control.cheat_god {
        damage = 0;
    }
    damage = (damage as f32
        * host.resolved_total_represented_aura_multiplier_like_cpp(
            RepresentedAuraEffectLikeCpp::ModifyFallDamagePct,
        )?) as u32;
    if host
        .hub_ref()
        .player_has_visible_aura_spell_like_cpp(43_621)?
    {
        damage = max_health / 2;
    }
    damage = damage.min(max_health);
    if damage == 0 {
        return None;
    }

    let requested_damage = if damage_control.environmental_damage_immune {
        0
    } else {
        damage
    };
    let attacker = host.player_guid()?;
    let result = host
        .apply_native_player_damage_like_cpp(
            requested_damage,
            wow_entities::PlayerDeathOperationContext::new(attacker, false),
            creature_spawns,
            item_guid_generator,
        )
        .await
        .ok()?;
    let final_damage = result.applied_damage;
    let killed_player = result.killed;
    host.sync_player_registry_state_like_cpp();
    if final_damage > 0
        && let Some(player_guid) = host.player_guid()
    {
        host.core().send_environmental_damage_log_like_cpp(
            player_guid,
            DAMAGE_FALL_LIKE_CPP,
            damage,
            0,
            0,
        );
        if killed_player {
            // C++ `Player::EnvironmentalDamage` fall-to-death branch
            // (`Player.cpp:663-670`): item durability loss plus the loss
            // message. Non-fall environmental damage never wears items.
            let loss_rate = host
                .hub_ref()
                .config
                .durability_loss_on_death_rate_like_cpp();
            host.apply_represented_durability_loss_all_like_cpp(f64::from(loss_rate), false);
            host.core()
                .send_packet_realm(&wow_packet::packets::misc::DurabilityDamageDeath {
                    percent: (loss_rate * 100.0) as i32,
                });
        }
    }

    let event = MovementFallDamageEvent {
        z_diff,
        damage,
        final_damage,
    };
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.movement.fall_damage_events_like_cpp.push(event);
    }
    Some(event)
}

pub fn apply_knock_back_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: ClientOpcodes,
    ack: &mut wow_packet::packets::movement::MovementAck,
) -> bool {
    // C++ validates the packet with the Player but admits the status when
    // its GUID is the active `_player->m_unitMovedByMe` (MovementHandler.cpp:548-559).
    // Keep the Player-owned movement-info write below; only the active
    // mover gate decides whether this ACK is accepted.
    if !validate_and_sanitize_active_mover_ack_like_cpp(host, &mut ack.status) {
        host.hub_mut()
            .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
                opcode,
                mover_guid: ack.status.guid,
                ack_index: Some(ack.ack_index),
                movement_force_id: None,
                movement_force_type: None,
                adjusted_time: None,
                speed: None,
                time_skipped: None,
                spline_id: None,
                accepted: false,
            });
        return false;
    }

    let mut status = ack.status.clone();
    status.time = host
        .hub_ref()
        .adjust_client_movement_time_like_cpp(status.time);
    host.hub_mut()
        .set_player_movement_time_like_cpp(status.time);
    host.hub_mut()
        .set_player_movement_flags_like_cpp(status.flags);
    host.hub_mut().set_player_position_like_cpp(status.position);
    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode,
            mover_guid: status.guid,
            ack_index: Some(ack.ack_index),
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: Some(status.time),
            speed: None,
            time_skipped: None,
            spline_id: None,
            accepted: true,
        });
    true
}

pub fn handle_force_speed_change_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: ClientOpcodes,
    ack: &mut wow_packet::packets::movement::MovementAck,
    speed: f32,
) -> bool {
    let Some(move_type) =
        wow_world_core::session::movement_protocol::movement_speed_ack_move_type_like_cpp(opcode)
    else {
        host.hub_ref().trace_anticheat_violation_like_cpp(
            "HandleForceSpeedChangeAck.UnknownMoveType",
            Some(opcode),
            "kick",
        );
        host.hub_mut()
            .record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: None,
                ack_speed: speed,
                expected_speed: None,
                remaining_forced_changes: None,
                action: MovementSpeedAckActionLikeCpp::Kicked,
            });
        return false;
    };

    if !record_validated_movement_ack_like_cpp(host, opcode, ack, Some(speed)) {
        host.hub_ref().trace_anticheat_violation_like_cpp(
            "HandleForceSpeedChangeAck.InvalidMovementAck",
            Some(opcode),
            "kick",
        );
        host.hub_mut()
            .record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: Some(move_type),
                ack_speed: speed,
                expected_speed: None,
                remaining_forced_changes: None,
                action: MovementSpeedAckActionLikeCpp::Kicked,
            });
        return false;
    }

    let Some(mut remaining_forced_changes) = host
        .hub_ref()
        .resolved_forced_speed_changes_like_cpp(move_type)
    else {
        return false;
    };
    if remaining_forced_changes > 0 {
        let Some(remaining) = host
            .hub_mut()
            .consume_forced_speed_change_like_cpp(move_type)
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
                    expected_speed: host
                        .hub_ref()
                        .resolved_player_movement_speed_like_cpp(move_type),
                    remaining_forced_changes: Some(remaining_forced_changes),
                    action: MovementSpeedAckActionLikeCpp::SkippedPending,
                };
                host.hub_mut().record_movement_speed_ack_event_like_cpp(a0)
            };
            return true;
        }
    }

    let Some(expected_speed) = host
        .hub_ref()
        .resolved_player_movement_speed_like_cpp(move_type)
    else {
        return false;
    };
    let Some(player_on_transport) = host.hub_ref().player_on_transport_state_like_cpp() else {
        return false;
    };
    let action = if !player_on_transport && (expected_speed - speed).abs() > 0.01 {
        if expected_speed > speed {
            // C++ calls SetSpeedRate(GetSpeedRate()) to force the client back to the server value.
            host.hub_ref().trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.ClientSpeedLower",
                Some(opcode),
                "correct",
            );
            MovementSpeedAckActionLikeCpp::Corrected
        } else {
            host.hub_ref().trace_anticheat_violation_like_cpp(
                "HandleForceSpeedChangeAck.ClientSpeedHigher",
                Some(opcode),
                "kick",
            );
            host.core_mut()
                .kick("WorldSession::HandleForceSpeedChangeAck Incorrect speed");
            MovementSpeedAckActionLikeCpp::Kicked
        }
    } else {
        MovementSpeedAckActionLikeCpp::Accepted
    };

    host.hub_mut()
        .record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
            opcode,
            move_type: Some(move_type),
            ack_speed: speed,
            expected_speed: Some(expected_speed),
            remaining_forced_changes: Some(remaining_forced_changes),
            action,
        });
    !matches!(action, MovementSpeedAckActionLikeCpp::Kicked)
}

pub(crate) fn validate_and_sanitize_movement_ack_status_represented_like_cpp<
    H: PlayerMovementHostLikeCpp,
>(
    host: &H,
    status: &mut wow_packet::packets::movement::MovementInfo,
) -> bool {
    if !host.hub_ref().validate_movement_ack_status_like_cpp(status) {
        return false;
    }

    sanitize_movement_info_flags_represented_like_cpp(host, status);
    true
}

/// Validate a movement-force ACK against the Unit controlled by this
/// session. TrinityCore calls `ValidateMovementInfo` and then compares the
/// packet GUID with `_player->m_unitMovedByMe` in the force handlers
/// (`MovementHandler.cpp:583-615`), so a controlled Creature/Pet is a valid
/// source even though ordinary Player ACKs retain their narrower contract.
pub(crate) fn validate_and_sanitize_active_mover_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &H,
    status: &mut wow_packet::packets::movement::MovementInfo,
) -> bool {
    let Some(active_mover_guid) = host.hub_ref().player_moved_unit_guid_like_cpp() else {
        return false;
    };
    if status.guid != active_mover_guid || !status.position.is_valid_map_coord_like_cpp() {
        return false;
    }

    sanitize_movement_info_flags_represented_like_cpp(host, status);
    true
}

pub fn record_validated_movement_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: ClientOpcodes,
    ack: &mut wow_packet::packets::movement::MovementAck,
    speed: Option<f32>,
) -> bool {
    let accepted =
        validate_and_sanitize_movement_ack_status_represented_like_cpp(host, &mut ack.status);
    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

pub fn apply_move_set_vehicle_rec_id_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    ack: &mut wow_packet::packets::movement::MovementAck,
) {
    sanitize_movement_info_represented_like_cpp(host, &mut ack.status);
}

pub fn record_apply_movement_force_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    ack: &mut wow_packet::packets::movement::MovementAck,
    force: &wow_packet::packets::movement::MovementForce,
) -> bool {
    if !validate_and_sanitize_active_mover_ack_like_cpp(host, &mut ack.status) {
        host.hub_mut()
            .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

    let adjusted_time = host
        .hub_ref()
        .adjust_client_movement_time_like_cpp(ack.status.time);
    ack.status.time = adjusted_time;
    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

pub fn record_remove_movement_force_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    ack: &mut wow_packet::packets::movement::MovementAck,
    force_id: ObjectGuid,
) -> bool {
    if !validate_and_sanitize_active_mover_ack_like_cpp(host, &mut ack.status) {
        host.hub_mut()
            .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

    let adjusted_time = host
        .hub_ref()
        .adjust_client_movement_time_like_cpp(ack.status.time);
    ack.status.time = adjusted_time;
    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

pub fn handle_movement_force_mod_magnitude_ack_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: ClientOpcodes,
    ack: &mut wow_packet::packets::movement::MovementAck,
    speed: f32,
) -> bool {
    if !validate_and_sanitize_active_mover_ack_like_cpp(host, &mut ack.status) {
        host.hub_mut()
            .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
                opcode,
                mover_guid: ack.status.guid,
                ack_index: Some(ack.ack_index),
                movement_force_id: None,
                movement_force_type: None,
                adjusted_time: None,
                speed: Some(speed),
                time_skipped: None,
                spline_id: None,
                accepted: false,
            });
        host.hub_ref().trace_anticheat_violation_like_cpp(
            "HandleMoveSetModMovementForceMagnitudeAck.InvalidMovementAck",
            Some(opcode),
            "kick",
        );
        host.hub_mut()
            .record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
                opcode,
                move_type: None,
                ack_speed: speed,
                expected_speed: None,
                remaining_forced_changes: None,
                action: MovementSpeedAckActionLikeCpp::Kicked,
            });
        return false;
    }

    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode,
            mover_guid: ack.status.guid,
            ack_index: Some(ack.ack_index),
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: None,
            speed: Some(speed),
            time_skipped: None,
            spline_id: None,
            accepted: true,
        });

    let Some(mut remaining_forced_changes) = host
        .hub_ref()
        .resolved_movement_force_mod_magnitude_changes_like_cpp()
    else {
        return false;
    };
    let Some(expected_magnitude) = host
        .hub_ref()
        .mover_movement_force_mod_magnitude_like_cpp(ack.status.guid)
    else {
        return false;
    };
    let mut action = MovementSpeedAckActionLikeCpp::Accepted;
    if remaining_forced_changes > 0 {
        let Some(remaining) = host
            .hub_mut()
            .consume_movement_force_mod_magnitude_change_like_cpp()
        else {
            return false;
        };
        remaining_forced_changes = remaining;
        if remaining_forced_changes == 0 && (expected_magnitude - speed).abs() > 0.01 {
            host.hub_ref().trace_anticheat_violation_like_cpp(
                "HandleMoveSetModMovementForceMagnitudeAck.IncorrectMagnitude",
                Some(opcode),
                "kick",
            );
            host.core_mut().kick(
                "WorldSession::HandleMoveSetModMovementForceMagnitudeAck Incorrect magnitude",
            );
            action = MovementSpeedAckActionLikeCpp::Kicked;
        }
    }

    host.hub_mut()
        .record_movement_speed_ack_event_like_cpp(MovementSpeedAckEventLikeCpp {
            opcode,
            move_type: None,
            ack_speed: speed,
            expected_speed: Some(expected_magnitude),
            remaining_forced_changes: Some(remaining_forced_changes),
            action,
        });
    !matches!(action, MovementSpeedAckActionLikeCpp::Kicked)
}

pub(crate) fn record_move_spline_done_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    status: &mut wow_packet::packets::movement::MovementInfo,
    spline_id: i32,
) -> bool {
    let accepted = validate_and_sanitize_movement_ack_status_represented_like_cpp(host, status);
    host.hub_mut()
        .record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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

pub fn handle_move_spline_done_taxi_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    status: &mut wow_packet::packets::movement::MovementInfo,
    spline_id: i32,
) -> MoveSplineDoneTaxiActionLikeCpp {
    if !record_move_spline_done_like_cpp(host, status, spline_id) {
        return record_move_spline_done_taxi_event_like_cpp(
            host,
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::InvalidMovement,
            None,
            None,
            None,
            false,
        );
    }

    let Some(taxi_state) = host.hub_ref().player_taxi_state_snapshot_like_cpp() else {
        return record_move_spline_done_taxi_event_like_cpp(
            host,
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
            return record_move_spline_done_taxi_event_like_cpp(
                host,
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::InProgressNoFlightGenerator,
                Some(destination_node_id),
                None,
                None,
                false,
            );
        };

        let destination_map_id = host
            .taxi_node_map_ids_like_cpp()
            .get(&destination_node_id)
            .copied();
        let should_teleport = destination_map_id
            .map(|map_id| map_id != host.core().player_map_id_like_cpp())
            .unwrap_or(false)
            || flight.current_node.teleport_flag;

        if should_teleport {
            if let (Some(map_id), Some(node)) = (destination_map_id, flight.node_after_teleport) {
                if host
                    .hub_mut()
                    .advance_player_taxi_flight_after_teleport_like_cpp()
                    .is_none()
                {
                    return record_move_spline_done_taxi_event_like_cpp(
                        host,
                        spline_id,
                        MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                        Some(destination_node_id),
                        None,
                        None,
                        false,
                    );
                }
                host.hub_mut()
                    .set_player_map_position_like_cpp(map_id, node.position);
                return record_move_spline_done_taxi_event_like_cpp(
                    host,
                    spline_id,
                    MoveSplineDoneTaxiActionLikeCpp::TeleportRequested,
                    Some(destination_node_id),
                    Some(map_id),
                    Some(node.position),
                    false,
                );
            }
        }

        return record_move_spline_done_taxi_event_like_cpp(
            host,
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::InProgressNoTeleport,
            Some(destination_node_id),
            None,
            None,
            false,
        );
    }

    if taxi_state.destinations_like_cpp().len() != 1 {
        return record_move_spline_done_taxi_event_like_cpp(
            host,
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
            None,
            None,
            None,
            false,
        );
    }

    if !host.hub_mut().cleanup_player_after_taxi_flight_like_cpp() {
        return record_move_spline_done_taxi_event_like_cpp(
            host,
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
            None,
            None,
            None,
            false,
        );
    }
    let current_z = host
        .hub_ref()
        .player_position_like_cpp()
        .map(|position| position.z)
        .unwrap_or(status.position.z);
    host.hub_mut().set_fall_information_like_cpp(0, current_z);
    let honorless_target_cast = host
        .hub_ref()
        .player_world_local_state_like_cpp()
        .is_some_and(|state| state.is_pvp_hostile_like_cpp());

    record_move_spline_done_taxi_event_like_cpp(
        host,
        spline_id,
        MoveSplineDoneTaxiActionLikeCpp::FinalCleanup,
        None,
        None,
        None,
        honorless_target_cast,
    )
}

pub fn sanitize_movement_info_flags_represented_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &H,
    movement_info: &mut wow_packet::packets::movement::MovementInfo,
) -> MovementFlag {
    sanitize_movement_info_represented_like_cpp(host, movement_info).removed_flags
}

pub fn sanitize_movement_info_represented_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &H,
    movement_info: &mut wow_packet::packets::movement::MovementInfo,
) -> wow_anticheat::ValidationResult {
    wow_anticheat::validate_movement_info(
        movement_info,
        &wow_anticheat::PlayerState {
            // Fixed-position is an authorization proof. An unresolved
            // Player must not preserve a client-supplied ROOT flag.
            mover_fixed_position_vehicle: host
                .hub_ref()
                .resolved_mover_fixed_position_vehicle_like_cpp()
                == Some(true),
            // An unresolved/stale Player may not authorize client-only
            // movement flags. Treat the proof as absent for sanitization,
            // without claiming that the canonical aura set is empty.
            has_hover_aura: host
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Hover)
                == Some(true),
            has_water_walk_aura: host.resolved_has_represented_aura_effect_like_cpp(
                RepresentedAuraEffectLikeCpp::WaterWalk,
            ) == Some(true),
            has_ghost_aura: host
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Ghost)
                == Some(true),
            has_feather_fall_aura: host.resolved_has_represented_aura_effect_like_cpp(
                RepresentedAuraEffectLikeCpp::FeatherFall,
            ) == Some(true),
            has_fly_aura: host
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Fly)
                == Some(true),
            has_mounted_flight_speed_aura: host.resolved_has_represented_aura_effect_like_cpp(
                RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
            ) == Some(true),
            is_player_security: host.hub_ref().player_is_game_master_like_cpp() != Some(true),
        },
    )
}

pub async fn handle_under_map_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    movement_info: &wow_packet::packets::movement::MovementInfo,
    creature_spawns: &crate::CreatureSpawnCatalogsLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Option<MovementUnderMapDamageEvent> {
    let min_height = host
        .hub_ref()
        .player_min_height_like_cpp(movement_info.position);
    if movement_info.position.z >= min_height {
        let _ = host.core().with_owned_player_mut_like_cpp(|player| {
            player.remove_player_flags_like_cpp(PLAYER_FLAGS_IS_OUT_OF_BOUNDS);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures.movement.player_out_of_bounds_like_cpp = false;
        }
        return None;
    }

    let (original_health, max_health, player_is_alive) =
        host.hub_ref().resolved_player_vitals_like_cpp()?;
    if !player_is_alive {
        return None;
    }

    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.set_player_flag(PLAYER_FLAGS_IS_OUT_OF_BOUNDS);
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.movement.player_out_of_bounds_like_cpp = true;
    }
    let damage = max_health;
    let attacker = host.player_guid()?;
    let result = host
        .apply_native_player_damage_like_cpp(
            damage,
            wow_entities::PlayerDeathOperationContext::new(attacker, false),
            creature_spawns,
            item_guid_generator,
        )
        .await
        .ok()?;
    let health_after = result.health_after;
    if health_after != u64::from(original_health)
        && let Some(player_guid) = host.player_guid()
    {
        host.core().send_environmental_damage_log_like_cpp(
            player_guid,
            DAMAGE_FALL_TO_VOID_LIKE_CPP,
            damage,
            0,
            0,
        );
    }

    // C++ calls KillPlayer if EnvironmentalDamage did not kill due to GM/immunity.
    if host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true) {
        crate::set_player_alive_like_cpp(host, false);
    } else {
        host.sync_player_registry_state_like_cpp();
    }

    let event = MovementUnderMapDamageEvent {
        z: movement_info.position.z,
        min_height,
        damage,
    };
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .movement
            .under_map_damage_events_like_cpp
            .push(event);
    }
    Some(event)
}

pub(crate) fn request_jump_proc_like_cpp<H: PlayerMovementHostLikeCpp>(host: &mut H) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.movement.movement_jump_proc_requests_like_cpp = fixtures
            .movement
            .movement_jump_proc_requests_like_cpp
            .saturating_add(1);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = host;
}

pub(crate) fn record_move_spline_done_taxi_event_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    spline_id: i32,
    action: MoveSplineDoneTaxiActionLikeCpp,
    destination_node_id: Option<u32>,
    teleport_map_id: Option<u16>,
    teleport_position: Option<wow_core::Position>,
    honorless_target_cast: bool,
) -> MoveSplineDoneTaxiActionLikeCpp {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .teleport
            .move_spline_done_taxi_events_like_cpp
            .push(
                wow_world_core::session::movement_protocol::MoveSplineDoneTaxiEventLikeCpp {
                    spline_id,
                    action,
                    destination_node_id,
                    teleport_map_id,
                    teleport_position,
                    honorless_target_cast,
                },
            );
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (
        host,
        spline_id,
        destination_node_id,
        teleport_map_id,
        teleport_position,
        honorless_target_cast,
    );
    action
}
