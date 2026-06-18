// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_*.
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Sanitize movement flags like `Player::ValidateMovementInfo`
//!   3. Validate: GUID must match the player, position must be finite
//!   4. Update server-side player position
//!   5. Broadcast SMSG_MOVE_UPDATE to nearby sessions (TODO: multi-session map)
//!
//! Reference: C++ `WorldSession::HandleMovementOpcode`.

use tracing::{trace, warn};

use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitStandStateType;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ServerPacket;
use wow_packet::packets::movement::{
    ClientPlayerMovement, MoveApplyMovementForceAck, MoveInitActiveMoverComplete, MoveKnockBackAck,
    MoveRemoveMovementForceAck, MoveSetCollisionHeightAck, MoveSkipTime, MoveSplineDone,
    MoveTeleportAck, MoveTimeSkipped, MoveUpdate, MoveUpdateApplyMovementForce,
    MoveUpdateKnockBack, MoveUpdateModMovementForceMagnitude, MoveUpdateRemoveMovementForce,
    MovementAckMessage, MovementInfo, MovementSpeedAck, SetActiveMover,
};

use crate::session::{
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
    WorldSession,
};

// ── Handler registrations ─────────────────────────────────────────
// All CMSG_MOVE_* share the same handler (ThreadSafe in C#).

macro_rules! register_move {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: concat!("handle_movement_", stringify!($opcode)),
            }
        }
    };
}

register_move!(MoveStartForward);
register_move!(MoveStartBackward);
register_move!(MoveStop);
register_move!(MoveStartStrafeLeft);
register_move!(MoveStartStrafeRight);
register_move!(MoveStopStrafe);
register_move!(MoveStartTurnLeft);
register_move!(MoveStartTurnRight);
register_move!(MoveStopTurn);
register_move!(MoveStartPitchUp);
register_move!(MoveStartPitchDown);
register_move!(MoveStopPitch);
register_move!(MoveSetRunMode);
register_move!(MoveSetWalkMode);
register_move!(MoveHeartbeat);
register_move!(MoveFallLand);
register_move!(MoveFallReset);
register_move!(MoveJump);
register_move!(MoveSetFacing);
register_move!(MoveSetFacingHeartbeat);
register_move!(MoveSetPitch);
register_move!(MoveSetFly);
register_move!(MoveStartAscend);
register_move!(MoveStopAscend);
register_move!(MoveStartDescend);
register_move!(MoveStartSwim);
register_move!(MoveStopSwim);
register_move!(MoveUpdateFallSpeed);

// ── Handler implementation ─────────────────────────────────────────

impl WorldSession {
    /// Handle any CMSG_MOVE_* packet.
    ///
    /// Parses MovementInfo, validates it, updates player position,
    /// and queues a broadcast to nearby players.
    pub async fn handle_movement(&mut self, mut pkt: wow_packet::WorldPacket) {
        let opcode = pkt.client_opcode();
        let info = match ClientPlayerMovement::read(&mut pkt) {
            Ok(m) => m,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse movement packet: {e}"
                );
                return;
            }
        };

        self.handle_movement_info_like_cpp(opcode, info.info).await;
    }

    pub(crate) async fn handle_movement_info_like_cpp(
        &mut self,
        opcode: Option<ClientOpcodes>,
        mut info: MovementInfo,
    ) {
        let Some(player_guid) = self.player_guid() else {
            warn!(
                account = self.account_id,
                "Movement packet received without loaded player"
            );
            return;
        };

        // C++ calls Player::ValidateMovementInfo before rejecting mismatched
        // GUIDs or invalid positions, then broadcasts only the sanitized state.
        let movement_validation = self.sanitize_movement_info_represented_like_cpp(&mut info);
        if !movement_validation.removed_flags.is_empty() {
            for rule in movement_validation
                .stripped_rules
                .iter()
                .copied()
                .filter(|rule| rule.removes_flags_like_cpp())
            {
                self.trace_anticheat_violation_like_cpp(
                    rule.trace_rule_name_like_cpp(),
                    opcode,
                    "strip",
                );
            }
            trace!(
                account = self.account_id,
                removed = ?movement_validation.removed_flags,
                rules = ?movement_validation.stripped_rules,
                "MovementInfo flags sanitized before position update and broadcast"
            );
        }

        if info.guid != player_guid {
            self.trace_anticheat_violation_like_cpp(
                "HandleMovementOpcode.GuidMismatch",
                opcode,
                "reject",
            );
            warn!(
                account = self.account_id,
                "Movement GUID mismatch: expected {:?}, got {:?}", player_guid, info.guid
            );
            return;
        }

        let pos = info.position;
        if !pos.is_valid_map_coord_like_cpp() {
            self.trace_anticheat_violation_like_cpp(
                "HandleMovementOpcode.InvalidPosition",
                opcode,
                "reject",
            );
            warn!(
                account = self.account_id,
                "Invalid movement position: {pos:?}"
            );
            return;
        }

        if let Some(transport) = &info.transport {
            if self.player_position_like_cpp().is_some_and(|current| {
                pos.distance_2d(&current) > wow_core::Position::GRID_SIZE_LIKE_CPP
            }) {
                trace!(
                    account = self.account_id,
                    "Ignoring stale transport movement after large position delta"
                );
                return;
            }

            if transport.x.abs() > 75.0 || transport.y.abs() > 75.0 || transport.z.abs() > 75.0 {
                trace!(
                    account = self.account_id,
                    "Ignoring movement with invalid transport offset"
                );
                return;
            }

            if !wow_core::Position::new(
                pos.x + transport.x,
                pos.y + transport.y,
                pos.z + transport.z,
                pos.orientation + transport.o,
            )
            .is_valid_map_coord_like_cpp()
            {
                trace!(
                    account = self.account_id,
                    "Ignoring movement with invalid world transport coordinate"
                );
                return;
            }
        }

        self.apply_movement_side_effects_like_cpp(opcode, &info);
        info.time = self.adjust_client_movement_time_like_cpp(info.time);
        self.set_player_movement_time_like_cpp(info.time);
        self.set_player_movement_flags_like_cpp(info.flags);

        // Update server-side player position.
        self.set_player_position_like_cpp(info.position);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().world_mut().relocate(info.position);
        });
        // Keep the broadcast registry in sync so chat range checks are accurate.
        self.update_registry_position();
        trace!(
            account = self.account_id,
            x = pos.x,
            y = pos.y,
            z = pos.z,
            "Player moved"
        );

        // Dynamic visibility update: send new creatures/GOs that came into
        // range and remove those that left. Internally throttled to 50 yards.
        self.update_visibility().await;

        // Check area triggers at the new position
        self.check_area_triggers().await;

        // TODO: aggro proximity check re-enable once combat system is stable
        // self.check_creature_aggro().await;

        // Broadcast movement to other players on the same map.
        if let (Some(guid), Some(registry)) = (self.player_guid(), self.player_registry()) {
            use wow_core::ObjectGuid;
            use wow_network::PlayerBroadcastInfo;

            let move_update = MoveUpdate { info };
            let bytes = move_update.to_bytes();
            let current_map_id = self.player_map_id_like_cpp();

            for entry in registry.iter() {
                let (other_guid, other_info): (&ObjectGuid, &PlayerBroadcastInfo) = entry.pair();
                if *other_guid == guid {
                    continue;
                }
                if other_info.map_id != current_map_id {
                    continue;
                }
                let _ = other_info.send_tx.send(bytes.clone());
            }
        }
    }

    fn apply_movement_side_effects_like_cpp(
        &mut self,
        opcode: Option<ClientOpcodes>,
        info: &MovementInfo,
    ) {
        if matches!(opcode, Some(ClientOpcodes::MoveFallLand)) {
            self.handle_fall_like_cpp(info);
        }

        match opcode {
            Some(ClientOpcodes::MoveFallLand)
            | Some(ClientOpcodes::MoveStartSwim)
            | Some(ClientOpcodes::MoveSetFly) => {
                self.remove_auras_with_interrupt_flags_like_cpp(
                    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP,
                    0,
                );
            }
            _ => {}
        }

        if matches!(
            opcode,
            Some(ClientOpcodes::MoveSetFly) | Some(ClientOpcodes::MoveSetAdvFly)
        ) {
            self.request_temporary_pet_unsummon_like_cpp();
        }

        if self.player_is_sit_state_like_cpp()
            && info
                .flags
                .intersects(MovementFlag::MASK_MOVING | MovementFlag::MASK_TURNING)
        {
            self.set_player_stand_state_like_cpp(UnitStandStateType::Stand);
        }

        if matches!(opcode, Some(ClientOpcodes::MoveJump)) {
            self.remove_auras_with_interrupt_flags_like_cpp(
                0,
                SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
            );
            self.request_jump_proc_like_cpp();
        }

        self.update_fall_information_if_needed_like_cpp(
            info,
            matches!(opcode, Some(ClientOpcodes::MoveFallLand)),
        );
        self.handle_under_map_like_cpp(info);
    }

    /// Handle CMSG_SET_ACTIVE_MOVER — client sets which unit is currently being moved.
    ///
    /// The client sends this after login to establish the active mover GUID.
    /// The mover must match C++ `Player::GetUnitBeingMoved()`.
    pub async fn handle_set_active_mover(&mut self, pkt: SetActiveMover) {
        trace!(
            account = self.account_id,
            mover = ?pkt.active_mover,
            "SetActiveMover"
        );

        let expected_mover = self.player_moved_unit_guid_like_cpp();
        if !expected_mover.is_empty() && pkt.active_mover != expected_mover {
            warn!(
                account = self.account_id,
                "SetActiveMover GUID mismatch: expected {:?}, got {:?}",
                expected_mover,
                pkt.active_mover
            );
            // C++ only logs this mismatch.
        }
    }

    /// Handle CMSG_MOVE_INIT_ACTIVE_MOVER_COMPLETE — client acknowledges active mover ready.
    ///
    /// In C# this updates transport timing flags and triggers visibility update.
    /// For now we just log receipt; transport timing is not yet implemented.
    pub async fn handle_move_init_active_mover_complete(
        &mut self,
        pkt: MoveInitActiveMoverComplete,
    ) {
        trace!(
            account = self.account_id,
            ticks = pkt.ticks,
            "MoveInitActiveMoverComplete"
        );
        self.apply_move_init_active_mover_complete_like_cpp(pkt.ticks);
        self.update_visibility().await;
    }

    /// Handle C++ `HandleMovementAckMessage` opcodes.
    pub async fn handle_movement_ack_message(
        &mut self,
        opcode: ClientOpcodes,
        mut pkt: MovementAckMessage,
    ) {
        trace!(account = self.account_id, ?opcode, "MovementAckMessage");
        self.record_validated_movement_ack_like_cpp(opcode, &mut pkt.ack, None);
    }

    /// Handle C++ `HandleMoveSetVehicleRecAck`.
    pub async fn handle_move_set_vehicle_rec_id_ack(
        &mut self,
        opcode: ClientOpcodes,
        mut pkt: wow_packet::packets::vehicle::MoveSetVehicleRecIdAck,
    ) {
        trace!(
            account = self.account_id,
            ?opcode,
            vehicle_rec_id = pkt.vehicle_rec_id,
            "MoveSetVehicleRecIdAck"
        );
        self.apply_move_set_vehicle_rec_id_ack_like_cpp(&mut pkt.data);
    }

    /// Handle C++ `HandleForceSpeedChangeAck` and movement-force magnitude ACKs.
    pub async fn handle_movement_speed_ack(
        &mut self,
        opcode: ClientOpcodes,
        mut pkt: MovementSpeedAck,
    ) {
        trace!(
            account = self.account_id,
            ?opcode,
            speed = pkt.speed,
            "MovementSpeedAck"
        );
        let accepted = if matches!(opcode, ClientOpcodes::MoveSetModMovementForceMagnitudeAck) {
            self.handle_movement_force_mod_magnitude_ack_like_cpp(opcode, &mut pkt.ack, pkt.speed)
        } else {
            self.handle_force_speed_change_ack_like_cpp(opcode, &mut pkt.ack, pkt.speed)
        };

        if accepted && matches!(opcode, ClientOpcodes::MoveSetModMovementForceMagnitudeAck) {
            let mut status = pkt.ack.status.clone();
            status.time = self.adjust_client_movement_time_like_cpp(status.time);
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateModMovementForceMagnitude {
                    status,
                    speed: pkt.speed,
                }
                .to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleMoveKnockBackAck`.
    pub async fn handle_move_knock_back_ack(&mut self, mut pkt: MoveKnockBackAck) {
        trace!(
            account = self.account_id,
            has_speeds = pkt.speeds.is_some(),
            "MoveKnockBackAck"
        );
        if self.apply_knock_back_ack_like_cpp(ClientOpcodes::MoveKnockBackAck, &mut pkt.ack) {
            let mut status = pkt.ack.status.clone();
            status.time = self.player_movement_time_like_cpp();
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateKnockBack { status }.to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleSetCollisionHeightAck`.
    pub async fn handle_move_set_collision_height_ack(
        &mut self,
        mut pkt: MoveSetCollisionHeightAck,
    ) {
        trace!(
            account = self.account_id,
            height = pkt.height,
            mount_display_id = pkt.mount_display_id,
            reason = pkt.reason,
            "MoveSetCollisionHeightAck"
        );
        self.record_validated_movement_ack_like_cpp(
            ClientOpcodes::MoveSetCollisionHeightAck,
            &mut pkt.data,
            None,
        );
    }

    /// Handle C++ `HandleMoveApplyMovementForceAck` bookkeeping until movement-force broadcasts exist.
    pub async fn handle_move_apply_movement_force_ack(
        &mut self,
        mut pkt: MoveApplyMovementForceAck,
    ) {
        trace!(
            account = self.account_id,
            force = ?pkt.force.id,
            "MoveApplyMovementForceAck"
        );
        if self.record_apply_movement_force_ack_like_cpp(&mut pkt.ack, &pkt.force) {
            let mut status = pkt.ack.status.clone();
            if let Some(adjusted_time) = self.latest_movement_ack_adjusted_time_like_cpp() {
                status.time = adjusted_time;
            }
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateApplyMovementForce {
                    status,
                    force: pkt.force,
                }
                .to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleMoveRemoveMovementForceAck` bookkeeping until movement-force broadcasts exist.
    pub async fn handle_move_remove_movement_force_ack(
        &mut self,
        mut pkt: MoveRemoveMovementForceAck,
    ) {
        trace!(
            account = self.account_id,
            force = ?pkt.id,
            "MoveRemoveMovementForceAck"
        );
        if self.record_remove_movement_force_ack_like_cpp(&mut pkt.ack, pkt.id) {
            let mut status = pkt.ack.status.clone();
            if let Some(adjusted_time) = self.latest_movement_ack_adjusted_time_like_cpp() {
                status.time = adjusted_time;
            }
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateRemoveMovementForce {
                    status,
                    trigger_guid: pkt.id,
                }
                .to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleMoveTimeSkippedOpcode`.
    pub async fn handle_move_time_skipped(&mut self, pkt: MoveTimeSkipped) {
        trace!(
            account = self.account_id,
            mover = ?pkt.mover_guid,
            time_skipped = pkt.time_skipped,
            "MoveTimeSkipped"
        );
        if self.apply_move_time_skipped_like_cpp(pkt.mover_guid, pkt.time_skipped) {
            self.broadcast_to_movement_set_like_cpp(
                MoveSkipTime {
                    mover_guid: pkt.mover_guid,
                    time_skipped: pkt.time_skipped,
                }
                .to_bytes(),
                false,
            );
        }
    }

    /// Handle C++ `HandleMoveSplineDoneOpcode` bookkeeping until taxi runtime is complete.
    pub async fn handle_move_spline_done(&mut self, mut pkt: MoveSplineDone) {
        trace!(
            account = self.account_id,
            spline_id = pkt.spline_id,
            "MoveSplineDone"
        );
        self.handle_move_spline_done_taxi_like_cpp(&mut pkt.status, pkt.spline_id);
    }

    /// Handle C++ `HandleMoveTeleportAck` bookkeeping until near-teleport runtime is complete.
    pub async fn handle_move_teleport_ack(&mut self, pkt: MoveTeleportAck) {
        trace!(
            account = self.account_id,
            mover = ?pkt.mover_guid,
            ack_index = pkt.ack_index,
            move_time = pkt.move_time,
            "MoveTeleportAck"
        );
        self.handle_move_teleport_ack_like_cpp(pkt.mover_guid, pkt.ack_index, pkt.move_time);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{
        AuraApplication, MoveSplineDoneTaxiActionLikeCpp, MoveTeleportAckActionLikeCpp,
        MovementSpeedAckActionLikeCpp, RepresentedAuraEffectLikeCpp,
        RepresentedTaxiFlightNodeLikeCpp, SessionPlayerController, UnitMoveTypeLikeCpp,
    };
    use std::sync::{Arc, Mutex};
    use wow_constants::ServerOpcodes;
    use wow_constants::movement::MovementFlag;
    use wow_constants::unit::UnitFlags;
    use wow_core::{ObjectGuid, Position};

    fn make_session() -> WorldSession {
        make_session_with_send_rx().0
    }

    fn make_session_with_send_rx() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded(8);
        let (send_tx, send_rx) = flume::bounded(8);
        let session = WorldSession::new(
            1,
            "MovementTest".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "esES".into(),
            pkt_rx,
            send_tx,
        );
        (session, send_rx)
    }

    fn visible_aura(slot: u8, flags: u32, flags2: u32) -> AuraApplication {
        AuraApplication {
            spell_id: 1000 + i32::from(slot),
            caster_guid: ObjectGuid::EMPTY,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x1,
            effect_mask: 0x1,
            aura_interrupt_flags: flags,
            aura_interrupt_flags2: flags2,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        }
    }

    fn fall_aura(
        slot: u8,
        effect: RepresentedAuraEffectLikeCpp,
        amount: i32,
        multiplier: f32,
    ) -> AuraApplication {
        AuraApplication {
            represented_effect: Some(effect),
            represented_amount: amount,
            represented_multiplier: multiplier,
            ..visible_aura(slot, 0, 0)
        }
    }

    #[test]
    fn movement_landing_and_jump_remove_cpp_interruptible_auras() {
        let mut session = make_session();
        session.visible_auras.insert(
            1,
            visible_aura(1, SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, 0),
        );
        session.visible_auras.insert(
            2,
            visible_aura(2, 0, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP),
        );
        session.visible_auras.insert(3, visible_aura(3, 0, 0));

        session.apply_movement_side_effects_like_cpp(
            Some(ClientOpcodes::MoveFallLand),
            &MovementInfo::default(),
        );
        assert!(!session.visible_auras.contains_key(&1));
        assert!(session.visible_auras.contains_key(&2));
        assert!(session.visible_auras.contains_key(&3));

        session.apply_movement_side_effects_like_cpp(
            Some(ClientOpcodes::MoveJump),
            &MovementInfo::default(),
        );
        assert!(!session.visible_auras.contains_key(&2));
        assert!(session.visible_auras.contains_key(&3));
        assert_eq!(session.movement_jump_proc_requests_like_cpp(), 1);
    }

    #[test]
    fn movement_stands_sitting_player_and_records_flying_pet_unsummon() {
        let mut session = make_session();
        session.set_player_stand_state_like_cpp(UnitStandStateType::SitChair);
        let mut info = MovementInfo::default();
        info.flags = MovementFlag::FORWARD;

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveSetFly), &info);

        assert_eq!(
            session.player_stand_state_like_cpp(),
            UnitStandStateType::Stand
        );
        assert_eq!(session.temporary_pet_unsummon_requests_like_cpp(), 1);
    }

    #[test]
    fn movement_fall_land_applies_cpp_base_fall_damage_and_updates_fall_info() {
        let (mut session, send_rx) = make_session_with_send_rx();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 41)));
        session.set_player_health_like_cpp(1_000, 1_000);
        session.set_fall_information_like_cpp(1_200, 120.0);
        let mut info = MovementInfo::default();
        info.position.z = 100.0;
        info.jump.fall_time = 1_500;

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

        let events = session.fall_damage_events_like_cpp();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].damage, 117);
        assert_eq!(events[0].final_damage, 117);
        assert_eq!(session.player_health_like_cpp(), 883);
        let sent = send_rx.try_recv().expect("fall health update");
        let opcode = u16::from_le_bytes([sent[0], sent[1]]);
        assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
        let sent = send_rx.try_recv().expect("fall environmental damage log");
        let opcode = u16::from_le_bytes([sent[0], sent[1]]);
        assert_eq!(opcode, ServerOpcodes::EnvironmentalDamageLog as u16);

        let mut harmless = MovementInfo::default();
        harmless.position.z = 99.0;
        harmless.jump.fall_time = 1_600;
        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &harmless);
        assert_eq!(session.fall_damage_events_like_cpp().len(), 1);
    }

    #[test]
    fn movement_fall_damage_applies_cpp_aura_modifiers_and_guards() {
        let mut session = make_session();
        session.set_player_health_like_cpp(1_000, 1_000);
        session.set_fall_information_like_cpp(1_200, 150.0);
        session.visible_auras.insert(
            4,
            fall_aura(4, RepresentedAuraEffectLikeCpp::SafeFall, 10, 1.0),
        );
        session.visible_auras.insert(
            5,
            fall_aura(5, RepresentedAuraEffectLikeCpp::ModifyFallDamagePct, 0, 0.5),
        );
        let mut info = MovementInfo::default();
        info.position.z = 100.0;
        info.jump.fall_time = 1_500;

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

        let events = session.fall_damage_events_like_cpp();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].damage, 238);
        assert_eq!(events[0].final_damage, 238);
        assert_eq!(session.player_health_like_cpp(), 762);

        let mut guarded = make_session();
        guarded.set_player_health_like_cpp(1_000, 1_000);
        guarded.set_fall_information_like_cpp(1_200, 150.0);
        guarded.visible_auras.insert(
            6,
            fall_aura(6, RepresentedAuraEffectLikeCpp::FeatherFall, 0, 1.0),
        );
        guarded.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
        assert!(guarded.fall_damage_events_like_cpp().is_empty());

        let mut god = make_session();
        god.set_player_health_like_cpp(1_000, 1_000);
        god.set_fall_information_like_cpp(1_200, 150.0);
        god.set_player_cheat_god_like_cpp(true);
        god.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
        assert!(god.fall_damage_events_like_cpp().is_empty());

        let mut gm = make_session();
        gm.set_player_health_like_cpp(1_000, 1_000);
        gm.set_fall_information_like_cpp(1_200, 150.0);
        gm.set_player_game_master_like_cpp(true);
        gm.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
        assert!(gm.fall_damage_events_like_cpp().is_empty());

        let mut immune = make_session();
        immune.set_player_health_like_cpp(1_000, 1_000);
        immune.set_fall_information_like_cpp(1_200, 150.0);
        immune.set_player_normal_damage_immune_like_cpp(true);
        immune.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
        assert!(immune.fall_damage_events_like_cpp().is_empty());

        let mut environmental = make_session();
        environmental.set_player_health_like_cpp(1_000, 1_000);
        environmental.set_fall_information_like_cpp(1_200, 150.0);
        environmental.set_player_environmental_damage_immune_like_cpp(true);
        environmental
            .apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);
        assert_eq!(environmental.fall_damage_events_like_cpp()[0].damage, 657);
        assert_eq!(
            environmental.fall_damage_events_like_cpp()[0].final_damage,
            0
        );
        assert_eq!(environmental.player_health_like_cpp(), 1_000);
    }

    #[test]
    fn movement_under_map_applies_cpp_void_damage_and_flag() {
        let (mut session, send_rx) = make_session_with_send_rx();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_player_health_like_cpp(1_000, 1_000);
        let mut info = MovementInfo::default();
        info.position.z = -501.0;

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);

        assert_eq!(session.under_map_damage_events_like_cpp().len(), 1);
        assert_eq!(
            session.under_map_damage_events_like_cpp()[0].min_height,
            crate::map_manager::DEFAULT_MIN_HEIGHT_LIKE_CPP
        );
        assert_eq!(session.player_health_like_cpp(), 0);
        assert!(!session.player_is_alive_like_cpp());
        assert!(session.player_out_of_bounds_like_cpp());
        let sent = send_rx.try_recv().expect("void health update");
        let opcode = u16::from_le_bytes([sent[0], sent[1]]);
        assert_eq!(opcode, ServerOpcodes::HealthUpdate as u16);
        let sent = send_rx.try_recv().expect("void environmental damage log");
        let opcode = u16::from_le_bytes([sent[0], sent[1]]);
        assert_eq!(opcode, ServerOpcodes::EnvironmentalDamageLog as u16);

        info.position.z = -499.0;
        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);
        assert!(!session.player_out_of_bounds_like_cpp());
    }

    #[test]
    fn move_init_active_mover_complete_sets_cpp_transport_state() {
        let mut session = make_session();
        let before = WorldSession::game_time_ms_like_cpp();

        session.apply_move_init_active_mover_complete_like_cpp(25);

        assert!(
            session.active_player_local_flags_like_cpp()
                & crate::session::PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP
                != 0
        );
        assert!(session.active_player_transport_server_time_like_cpp() >= 0);
        assert!(
            session.active_player_transport_server_time_like_cpp()
                <= WorldSession::game_time_ms_like_cpp() as i32
        );
        assert!(
            session.active_player_transport_server_time_like_cpp()
                >= before.saturating_sub(25) as i32
        );
        assert_eq!(session.movement_visibility_refresh_requests_like_cpp(), 1);
    }

    #[test]
    fn movement_ack_helpers_validate_and_apply_cpp_side_effects() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));

        let status = MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
            ..MovementInfo::default()
        };
        let mut ack = wow_packet::packets::movement::MovementAck {
            status: status.clone(),
            ack_index: 7,
        };

        assert!(session.apply_knock_back_ack_like_cpp(ClientOpcodes::MoveKnockBackAck, &mut ack));
        assert_eq!(session.player_position_like_cpp(), Some(status.position));
        assert_eq!(session.movement_ack_events_like_cpp().len(), 1);
        assert!(session.movement_ack_events_like_cpp()[0].accepted);
        assert_eq!(session.movement_ack_events_like_cpp()[0].ack_index, Some(7));
        assert_eq!(
            session.movement_ack_events_like_cpp()[0].adjusted_time,
            Some(session.player_movement_time_like_cpp())
        );

        session.set_player_movement_time_like_cpp(100);
        assert!(session.apply_move_time_skipped_like_cpp(guid, 25));
        assert_eq!(session.player_movement_time_like_cpp(), 125);
        assert_eq!(session.movement_ack_events_like_cpp().len(), 2);
        assert_eq!(
            session.movement_ack_events_like_cpp()[1].opcode,
            ClientOpcodes::MoveTimeSkipped
        );
        assert_eq!(
            session.movement_ack_events_like_cpp()[1].time_skipped,
            Some(25)
        );

        let wrong_guid = ObjectGuid::create_player(1, 43);
        assert!(!session.apply_move_time_skipped_like_cpp(wrong_guid, 25));
        assert_eq!(session.player_movement_time_like_cpp(), 125);
        assert!(!session.movement_ack_events_like_cpp()[2].accepted);
    }

    #[test]
    fn movement_force_ack_helpers_record_cpp_adjusted_time_and_force_id() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let force_guid = ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::GameObject,
            0,
            1,
            0,
            0,
            9,
            88,
        );
        session.set_player_guid(Some(guid));

        let mut ack = wow_packet::packets::movement::MovementAck {
            status: MovementInfo {
                guid,
                time: 1_000,
                position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                ..MovementInfo::default()
            },
            ack_index: 44,
        };
        let force = wow_packet::packets::movement::MovementForce {
            id: force_guid,
            origin: [1.0, 2.0, 3.0],
            direction: [4.0, 5.0, 6.0],
            transport_id: 0,
            magnitude: 7.0,
            unused_910: 0,
            force_type: wow_packet::packets::movement::MovementForceType::Gravity,
        };

        assert!(session.record_apply_movement_force_ack_like_cpp(&mut ack, &force));
        assert_eq!(session.movement_ack_events_like_cpp().len(), 1);
        assert_eq!(
            session.movement_ack_events_like_cpp()[0].opcode,
            ClientOpcodes::MoveApplyMovementForceAck
        );
        assert_eq!(
            session.movement_ack_events_like_cpp()[0].movement_force_id,
            Some(force_guid)
        );
        assert_eq!(
            session.movement_ack_events_like_cpp()[0].movement_force_type,
            Some(1)
        );
        assert!(
            session.movement_ack_events_like_cpp()[0]
                .adjusted_time
                .is_some()
        );

        assert!(session.record_remove_movement_force_ack_like_cpp(&mut ack, force_guid));
        assert_eq!(
            session.movement_ack_events_like_cpp()[1].opcode,
            ClientOpcodes::MoveRemoveMovementForceAck
        );
        assert_eq!(
            session.movement_ack_events_like_cpp()[1].movement_force_id,
            Some(force_guid)
        );
    }

    #[test]
    fn movement_speed_ack_matches_cpp_counters_and_anticheat() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        let mut ack = wow_packet::packets::movement::MovementAck {
            status: MovementInfo {
                guid,
                time: 1_000,
                position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                ..MovementInfo::default()
            },
            ack_index: 10,
        };

        session.set_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run, 1.0);
        session.set_forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run, 2);
        assert!(session.handle_force_speed_change_ack_like_cpp(
            ClientOpcodes::MoveForceRunSpeedChangeAck,
            &mut ack,
            1.0,
        ));
        let first = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(first.action, MovementSpeedAckActionLikeCpp::SkippedPending);
        assert_eq!(first.remaining_forced_changes, Some(1));
        assert!(!session.is_disconnecting());

        assert!(session.handle_force_speed_change_ack_like_cpp(
            ClientOpcodes::MoveForceRunSpeedChangeAck,
            &mut ack,
            6.0,
        ));
        let corrected = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(corrected.expected_speed, Some(7.0));
        assert_eq!(corrected.action, MovementSpeedAckActionLikeCpp::Corrected);
        assert!(!session.is_disconnecting());

        session.set_player_on_transport_like_cpp(true);
        assert!(session.handle_force_speed_change_ack_like_cpp(
            ClientOpcodes::MoveForceRunSpeedChangeAck,
            &mut ack,
            8.0,
        ));
        let transport = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(transport.action, MovementSpeedAckActionLikeCpp::Accepted);
        assert!(!session.is_disconnecting());

        session.set_player_on_transport_like_cpp(false);
        assert!(!session.handle_force_speed_change_ack_like_cpp(
            ClientOpcodes::MoveForceRunSpeedChangeAck,
            &mut ack,
            8.0,
        ));
        let kicked = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(kicked.action, MovementSpeedAckActionLikeCpp::Kicked);
        assert!(session.is_disconnecting());
    }

    #[tokio::test]
    async fn movement_speed_ack_correction_matches_legacy_no_resync_packet() {
        let (mut session, send_rx) = make_session_with_send_rx();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.set_player_movement_speed_rate_like_cpp(UnitMoveTypeLikeCpp::Run, 1.0);

        session
            .handle_movement_speed_ack(
                ClientOpcodes::MoveForceRunSpeedChangeAck,
                wow_packet::packets::movement::MovementSpeedAck {
                    ack: wow_packet::packets::movement::MovementAck {
                        status: MovementInfo {
                            guid,
                            time: 1_000,
                            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                            ..MovementInfo::default()
                        },
                        ack_index: 10,
                    },
                    speed: 6.0,
                },
            )
            .await;

        let corrected = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(corrected.expected_speed, Some(7.0));
        assert_eq!(corrected.action, MovementSpeedAckActionLikeCpp::Corrected);
        assert!(
            send_rx.try_recv().is_err(),
            "legacy C++ calls SetSpeedRate(GetSpeedRate()), but Unit::SetSpeedRate returns early when the rate is unchanged"
        );
    }

    #[test]
    fn movement_force_magnitude_ack_matches_cpp_counter_validation() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.set_movement_force_mod_magnitude_changes_like_cpp(1);
        session.set_movement_force_mod_magnitude_like_cpp(1.25);
        let mut ack = wow_packet::packets::movement::MovementAck {
            status: MovementInfo {
                guid,
                time: 1_000,
                position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                ..MovementInfo::default()
            },
            ack_index: 11,
        };

        assert!(session.handle_movement_force_mod_magnitude_ack_like_cpp(
            ClientOpcodes::MoveSetModMovementForceMagnitudeAck,
            &mut ack,
            1.25,
        ));
        let accepted = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(accepted.action, MovementSpeedAckActionLikeCpp::Accepted);
        assert_eq!(accepted.remaining_forced_changes, Some(0));

        session.set_movement_force_mod_magnitude_changes_like_cpp(1);
        assert!(!session.handle_movement_force_mod_magnitude_ack_like_cpp(
            ClientOpcodes::MoveSetModMovementForceMagnitudeAck,
            &mut ack,
            1.5,
        ));
        let kicked = session.movement_speed_ack_events_like_cpp().last().unwrap();
        assert_eq!(kicked.action, MovementSpeedAckActionLikeCpp::Kicked);
        assert!(session.is_disconnecting());
    }

    #[test]
    fn move_spline_done_taxi_final_cleanup_matches_cpp_represented_side_effects() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(wow_core::Position::new(1.0, 2.0, 30.0, 0.5));
        session.set_fall_information_like_cpp(1_200, 120.0);
        session.set_taxi_destinations_like_cpp(vec![100]);
        session.set_taxi_cleanup_state_like_cpp(
            UnitFlags::REMOVE_CLIENT_CONTROL | UnitFlags::ON_TAXI,
            true,
        );
        session.set_player_pvp_hostile_like_cpp(true);

        let mut status = MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(1.0, 2.0, 30.0, 0.5),
            ..MovementInfo::default()
        };

        let action = session.handle_move_spline_done_taxi_like_cpp(&mut status, 55);
        assert_eq!(action, MoveSplineDoneTaxiActionLikeCpp::FinalCleanup);
        assert!(session.taxi_destinations_like_cpp().is_empty());
        assert!(!session.taxi_mounted_like_cpp());
        assert_eq!(session.taxi_unit_flags_like_cpp(), UnitFlags::empty());
        assert_eq!(session.fall_information_like_cpp(), (0, 30.0));
        let event = session
            .move_spline_done_taxi_events_like_cpp()
            .last()
            .unwrap();
        assert!(event.honorless_target_cast);
    }

    #[test]
    fn move_spline_done_taxi_far_teleport_matches_cpp_represented_branch() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        session.set_player_map_position_like_cpp(0, wow_core::Position::new(1.0, 2.0, 3.0, 1.0));
        session.set_taxi_destinations_like_cpp(vec![10, 20]);
        session.set_taxi_node_map_id_like_cpp(20, 1);
        session.set_taxi_flight_state_like_cpp(
            RepresentedTaxiFlightNodeLikeCpp {
                map_id: 0,
                position: wow_core::Position::new(5.0, 6.0, 7.0, 1.0),
                teleport_flag: false,
            },
            Some(RepresentedTaxiFlightNodeLikeCpp {
                map_id: 1,
                position: wow_core::Position::new(50.0, 60.0, 70.0, 1.0),
                teleport_flag: false,
            }),
        );

        let mut status = MovementInfo {
            guid,
            time: 1_000,
            position: wow_core::Position::new(1.0, 2.0, 3.0, 1.0),
            ..MovementInfo::default()
        };

        let action = session.handle_move_spline_done_taxi_like_cpp(&mut status, 56);
        assert_eq!(action, MoveSplineDoneTaxiActionLikeCpp::TeleportRequested);
        assert_eq!(session.player_map_id_like_cpp(), 1);
        assert_eq!(
            session.player_position_like_cpp().unwrap(),
            wow_core::Position::new(50.0, 60.0, 70.0, 1.0)
        );
        let event = session
            .move_spline_done_taxi_events_like_cpp()
            .last()
            .unwrap();
        assert_eq!(event.destination_node_id, Some(20));
        assert_eq!(event.teleport_map_id, Some(1));
        assert_eq!(
            event.teleport_position,
            Some(wow_core::Position::new(50.0, 60.0, 70.0, 1.0))
        );
    }

    #[test]
    fn move_teleport_ack_applies_near_teleport_cpp_side_effects() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let destination = wow_core::Position::new(12.0, 13.0, 14.0, 1.5);
        session.set_player_guid(Some(guid));
        session.set_player_map_position_like_cpp(0, wow_core::Position::new(1.0, 2.0, 3.0, 0.5));
        session.set_fall_information_like_cpp(1_200, 80.0);
        session.set_player_zone_area_like_cpp(10, 11);
        session.set_player_pvp_state_like_cpp(true, false, false);
        session.set_near_teleport_pending_like_cpp(true, Some((0, destination)), Some((20, 21)));

        let action = session.handle_move_teleport_ack_like_cpp(guid, 77, 1_234);
        assert_eq!(action, MoveTeleportAckActionLikeCpp::Accepted);
        assert!(!session.near_teleport_pending_like_cpp());
        assert_eq!(session.player_position_like_cpp(), Some(destination));
        assert_eq!(session.fall_information_like_cpp(), (0, 14.0));
        assert_eq!(session.player_zone_area_like_cpp(), (20, 21));
        assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 1);
        assert_eq!(session.delayed_operations_processed_like_cpp(), 1);

        let event = session.move_teleport_ack_events_like_cpp().last().unwrap();
        assert_eq!(event.action, MoveTeleportAckActionLikeCpp::Accepted);
        assert_eq!(event.old_zone_id, Some(10));
        assert_eq!(event.new_zone_id, Some(20));
        assert!(event.honorless_target_cast);
        assert!(!event.pvp_disabled);
        assert!(event.pet_resummon_requested);
        assert!(event.delayed_operations_processed);
    }

    #[test]
    fn move_teleport_ack_ignores_wrong_or_missing_near_teleport_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
        session.set_player_guid(Some(guid));
        session.set_player_map_position_like_cpp(0, original_position);

        let action = session.handle_move_teleport_ack_like_cpp(guid, 1, 2);
        assert_eq!(action, MoveTeleportAckActionLikeCpp::NotBeingTeleportedNear);
        assert_eq!(session.player_position_like_cpp(), Some(original_position));

        session.set_near_teleport_pending_like_cpp(
            true,
            Some((0, wow_core::Position::new(9.0, 9.0, 9.0, 0.0))),
            Some((30, 31)),
        );
        let action = session.handle_move_teleport_ack_like_cpp(other_guid, 3, 4);
        assert_eq!(action, MoveTeleportAckActionLikeCpp::WrongMover);
        assert!(session.near_teleport_pending_like_cpp());
        assert_eq!(session.player_position_like_cpp(), Some(original_position));
        assert_eq!(session.temporary_pet_resummon_requests_like_cpp(), 0);
        assert_eq!(session.delayed_operations_processed_like_cpp(), 0);
    }

    #[test]
    fn validate_movement_info_sanitizes_representable_cpp_flag_violations() {
        let session = make_session();
        let mut info = MovementInfo {
            flags: MovementFlag::FORWARD
                | MovementFlag::BACKWARD
                | MovementFlag::LEFT
                | MovementFlag::RIGHT
                | MovementFlag::ASCENDING
                | MovementFlag::DESCENDING
                | MovementFlag::HOVER
                | MovementFlag::WATER_WALK
                | MovementFlag::FALLING_SLOW
                | MovementFlag::FLYING
                | MovementFlag::CAN_FLY
                | MovementFlag::DISABLE_GRAVITY
                | MovementFlag::FALLING
                | MovementFlag::SPLINE_ELEVATION,
            step_up_start_elevation: 0.0,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);
        assert!(removed.contains(MovementFlag::FORWARD | MovementFlag::BACKWARD));
        assert!(removed.contains(MovementFlag::LEFT | MovementFlag::RIGHT));
        assert!(removed.contains(MovementFlag::ASCENDING | MovementFlag::DESCENDING));
        assert!(removed.contains(MovementFlag::HOVER));
        assert!(removed.contains(MovementFlag::WATER_WALK));
        assert!(removed.contains(MovementFlag::FALLING_SLOW));
        assert!(removed.contains(MovementFlag::FLYING | MovementFlag::CAN_FLY));
        assert!(removed.contains(MovementFlag::FALLING));
        assert!(removed.contains(MovementFlag::SPLINE_ELEVATION));
        assert_eq!(info.flags, MovementFlag::DISABLE_GRAVITY);
    }

    #[test]
    fn validate_movement_info_strips_each_cpp_incompatible_pair() {
        let session = make_session();
        for (left, right) in [
            (MovementFlag::ASCENDING, MovementFlag::DESCENDING),
            (MovementFlag::LEFT, MovementFlag::RIGHT),
            (MovementFlag::STRAFE_LEFT, MovementFlag::STRAFE_RIGHT),
            (MovementFlag::PITCH_UP, MovementFlag::PITCH_DOWN),
            (MovementFlag::FORWARD, MovementFlag::BACKWARD),
        ] {
            let mut info = MovementInfo {
                flags: left | right,
                ..MovementInfo::default()
            };

            let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

            assert!(removed.contains(left | right), "{left:?} | {right:?}");
            assert!(info.flags.is_empty(), "{left:?} | {right:?}");
        }
    }

    #[test]
    fn validate_movement_info_reports_rule_evidence_from_anticheat_core() {
        let session = make_session();
        let mut info = MovementInfo {
            flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
            ..MovementInfo::default()
        };

        let result = session.sanitize_movement_info_represented_like_cpp(&mut info);

        assert_eq!(info.flags, MovementFlag::empty());
        assert!(result.removed_flags.contains(MovementFlag::HOVER));
        assert!(result.removed_flags.contains(MovementFlag::WATER_WALK));
        assert_eq!(
            result.stripped_rules,
            vec![
                wow_anticheat::MovementSanitizerRule::HoverWithoutAura,
                wow_anticheat::MovementSanitizerRule::WaterWalkWithoutAuraOrGhost,
            ]
        );
    }

    #[test]
    fn validate_movement_info_root_order_matches_cpp_without_fixed_vehicle() {
        let session = make_session();
        let mut info = MovementInfo {
            flags: MovementFlag::ROOT | MovementFlag::FORWARD,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

        assert!(removed.contains(MovementFlag::ROOT));
        assert!(!removed.contains(MovementFlag::FORWARD));
        assert_eq!(info.flags, MovementFlag::FORWARD);
    }

    #[test]
    fn validate_movement_info_keeps_root_for_fixed_position_vehicle_like_cpp() {
        let mut session = make_session();
        session.set_represented_mover_fixed_position_vehicle_like_cpp(true);

        let mut rooted = MovementInfo {
            flags: MovementFlag::ROOT,
            ..MovementInfo::default()
        };
        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut rooted);
        assert!(removed.is_empty());
        assert_eq!(rooted.flags, MovementFlag::ROOT);

        let mut rooted_moving = MovementInfo {
            flags: MovementFlag::ROOT | MovementFlag::FORWARD,
            ..MovementInfo::default()
        };
        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut rooted_moving);
        assert!(removed.contains(MovementFlag::FORWARD));
        assert!(!removed.contains(MovementFlag::ROOT));
        assert_eq!(rooted_moving.flags, MovementFlag::ROOT);
    }

    #[test]
    fn validate_movement_info_keeps_represented_allowed_aura_flags() {
        let mut session = make_session();
        session
            .visible_auras
            .insert(1, fall_aura(1, RepresentedAuraEffectLikeCpp::Hover, 0, 1.0));
        session.visible_auras.insert(
            2,
            fall_aura(2, RepresentedAuraEffectLikeCpp::FeatherFall, 0, 1.0),
        );
        session
            .visible_auras
            .insert(3, fall_aura(3, RepresentedAuraEffectLikeCpp::Fly, 0, 1.0));
        session.visible_auras.insert(
            4,
            fall_aura(4, RepresentedAuraEffectLikeCpp::WaterWalk, 0, 1.0),
        );
        let mut info = MovementInfo {
            flags: MovementFlag::HOVER
                | MovementFlag::WATER_WALK
                | MovementFlag::FALLING_SLOW
                | MovementFlag::FLYING
                | MovementFlag::CAN_FLY,
            step_up_start_elevation: 1.0,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);
        assert!(removed.is_empty());
        assert!(info.flags.contains(MovementFlag::HOVER));
        assert!(info.flags.contains(MovementFlag::WATER_WALK));
        assert!(info.flags.contains(MovementFlag::FALLING_SLOW));
        assert!(
            info.flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );
        assert!(info.flags.contains(MovementFlag::SPLINE_ELEVATION));
    }

    #[test]
    fn validate_movement_info_keeps_water_walk_for_ghost_like_cpp() {
        let mut session = make_session();
        session
            .visible_auras
            .insert(1, fall_aura(1, RepresentedAuraEffectLikeCpp::Ghost, 0, 1.0));
        let mut info = MovementInfo {
            flags: MovementFlag::WATER_WALK,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

        assert!(removed.is_empty());
        assert!(info.flags.contains(MovementFlag::WATER_WALK));
    }

    #[test]
    fn validate_movement_info_keeps_fly_for_gm_like_cpp() {
        let mut session = make_session();
        session.set_player_game_master_like_cpp(true);
        let mut info = MovementInfo {
            flags: MovementFlag::FLYING | MovementFlag::CAN_FLY,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

        assert!(removed.is_empty());
        assert!(
            info.flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );
    }

    #[test]
    fn validate_movement_info_keeps_fly_for_mounted_flight_speed_aura_like_cpp() {
        let mut session = make_session();
        session.visible_auras.insert(
            1,
            fall_aura(1, RepresentedAuraEffectLikeCpp::MountedFlightSpeed, 0, 1.0),
        );
        let mut info = MovementInfo {
            flags: MovementFlag::FLYING | MovementFlag::CAN_FLY,
            ..MovementInfo::default()
        };

        let removed = session.sanitize_movement_info_flags_represented_like_cpp(&mut info);

        assert!(removed.is_empty());
        assert!(
            info.flags
                .contains(MovementFlag::FLYING | MovementFlag::CAN_FLY)
        );
    }

    #[tokio::test]
    async fn handle_movement_broadcasts_sanitized_flags_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let registry = std::sync::Arc::new(wow_network::PlayerRegistry::default());
        let (self_tx, self_rx) = flume::bounded(1);
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.insert(guid, broadcast_info(guid, self_tx));
        registry.insert(other_guid, broadcast_info(other_guid, other_tx));

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD
                | MovementFlag::BACKWARD
                | MovementFlag::HOVER
                | MovementFlag::WATER_WALK,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
            ..MovementInfo::default()
        };
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        session.handle_movement(inbound).await;

        assert!(self_rx.try_recv().is_err());
        let bytes = other_rx.try_recv().expect("movement broadcast");
        let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            packet.server_opcode(),
            Some(wow_constants::ServerOpcodes::MoveUpdate)
        );
        packet.read_uint16().expect("move update opcode");
        let sanitized = MovementInfo::read(&mut packet).expect("move update status");
        assert_eq!(sanitized.flags, MovementFlag::empty());
        assert_eq!(
            session.player_movement_flags_like_cpp(),
            MovementFlag::empty()
        );
    }

    #[tokio::test]
    async fn handle_movement_syncs_canonical_player_position_for_logout_save_like_cpp() {
        let mut session = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1042);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let moved_position = Position::new(10.0, 20.0, 30.0, 1.0);

        canonical.lock().unwrap().create_world_map(571, 0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 571,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            "MovementSaver".to_string(),
            login_position,
            571,
            1,
            3,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: moved_position,
            ..MovementInfo::default()
        };
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        session.handle_movement(inbound).await;

        let canonical_position = canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .and_then(|map| map.map().get_typed_player(guid))
            .map(|player| player.unit().world().position())
            .expect("canonical player");
        assert_eq!(canonical_position, moved_position);
        assert_eq!(
            session
                .sync_session_from_save_to_db_snapshot_like_cpp()
                .unwrap()
                .position,
            moved_position
        );
    }

    #[tokio::test]
    async fn logout_save_snapshot_prefers_live_canonical_player_position_like_cpp() {
        let mut session = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1043);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let stale_session_position = Position::new(4.0, 5.0, 6.0, 0.5);
        let live_canonical_position = Position::new(10.0, 20.0, 30.0, 1.0);

        canonical.lock().unwrap().create_world_map(571, 0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 571,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            "LogoutSaver".to_string(),
            login_position,
            571,
            1,
            3,
            10,
            0,
        ));
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
        session.set_player_position_like_cpp(stale_session_position);
        session.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(live_canonical_position);
        });

        let snapshot = session
            .sync_session_from_save_to_db_snapshot_like_cpp()
            .expect("save snapshot");

        assert_eq!(snapshot.position, live_canonical_position);
        assert_eq!(
            session.player_position_like_cpp(),
            Some(live_canonical_position)
        );
    }

    #[tokio::test]
    async fn handle_movement_rejects_guid_mismatch_without_state_or_broadcast_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let spoofed_guid = ObjectGuid::create_player(1, 99);
        let other_guid = ObjectGuid::create_player(1, 43);
        let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
        let registry = std::sync::Arc::new(wow_network::PlayerRegistry::default());
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(original_position);
        session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.insert(other_guid, broadcast_info(other_guid, other_tx));

        let movement = MovementInfo {
            guid: spoofed_guid,
            flags: MovementFlag::FORWARD | MovementFlag::BACKWARD,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
            ..MovementInfo::default()
        };
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        session.handle_movement(inbound).await;

        assert_eq!(session.player_position_like_cpp(), Some(original_position));
        assert_eq!(
            session.player_movement_flags_like_cpp(),
            MovementFlag::SWIMMING
        );
        assert!(other_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn handle_movement_rejects_invalid_position_without_state_or_broadcast_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
        let registry = std::sync::Arc::new(wow_network::PlayerRegistry::default());
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(original_position);
        session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.insert(other_guid, broadcast_info(other_guid, other_tx));

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
            position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.0),
            ..MovementInfo::default()
        };
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        session.handle_movement(inbound).await;

        assert_eq!(session.player_position_like_cpp(), Some(original_position));
        assert_eq!(
            session.player_movement_flags_like_cpp(),
            MovementFlag::SWIMMING
        );
        assert!(other_rx.try_recv().is_err());
    }

    #[test]
    fn movement_ack_validation_sanitizes_status_flags_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(guid));
        let mut ack = wow_packet::packets::movement::MovementAck {
            status: MovementInfo {
                guid,
                flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
                position: wow_core::Position::new(10.0, 20.0, 30.0, 1.5),
                ..MovementInfo::default()
            },
            ack_index: 12,
        };

        assert!(session.record_validated_movement_ack_like_cpp(
            ClientOpcodes::MoveHoverAck,
            &mut ack,
            None
        ));
        assert!(ack.status.flags.is_empty());
        assert!(session.movement_ack_events_like_cpp()[0].accepted);
    }

    #[test]
    fn move_set_vehicle_rec_ack_only_sanitizes_status_like_cpp() {
        let mut session = make_session();
        let mut ack = wow_packet::packets::movement::MovementAck {
            status: MovementInfo {
                guid: ObjectGuid::create_player(1, 77),
                flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
                position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.5),
                ..MovementInfo::default()
            },
            ack_index: 77,
        };

        session.apply_move_set_vehicle_rec_id_ack_like_cpp(&mut ack);

        assert!(
            ack.status.flags.is_empty(),
            "C++ Player::ValidateMovementInfo strips invalid flags for this ACK"
        );
        assert!(
            session.movement_ack_events_like_cpp().is_empty(),
            "C++ HandleMoveSetVehicleRecAck does not run the generic movement ACK path"
        );
    }

    #[tokio::test]
    async fn handle_move_set_vehicle_rec_ack_does_not_record_generic_ack_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 78);
        session.set_player_guid(Some(guid));

        session
            .handle_move_set_vehicle_rec_id_ack(
                ClientOpcodes::MoveSetVehicleRecIdAck,
                wow_packet::packets::vehicle::MoveSetVehicleRecIdAck {
                    data: wow_packet::packets::movement::MovementAck {
                        status: MovementInfo {
                            guid: ObjectGuid::create_player(1, 79),
                            flags: MovementFlag::HOVER | MovementFlag::WATER_WALK,
                            position: wow_core::Position::new(f32::NAN, 20.0, 30.0, 1.5),
                            ..MovementInfo::default()
                        },
                        ack_index: 79,
                    },
                    vehicle_rec_id: 123,
                },
            )
            .await;

        assert!(session.movement_ack_events_like_cpp().is_empty());
    }

    fn broadcast_info(
        guid: ObjectGuid,
        send_tx: flume::Sender<Vec<u8>>,
    ) -> wow_network::PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(1);
        wow_network::PlayerBroadcastInfo {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            current_health: 100,
            max_health: 100,
            power_type: 0,
            current_power: 0,
            max_power: 0,
            is_pvp: false,
            is_ffa_pvp: false,
            is_ghost: false,
            is_afk: false,
            is_dnd: false,
            auto_reply_msg_like_cpp: String::new(),
            in_vehicle: false,
            has_vehicle_kit_like_cpp: false,
            party_member_vehicle_seat: 0,
            zone_id: 0,
            spec_id: 0,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            completed_achievements: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_party_type: [0; 2],
            party_member_phase_states: Default::default(),
            party_member_auras: Vec::new(),
            party_member_pet_stats: None,
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
            lifetime_honorable_kills: 0,
            this_week_contribution: 0,
            yesterday_contribution: 0,
            today_honorable_kills: 0,
            yesterday_honorable_kills: 0,
            lifetime_max_rank: 0,
            honor_level: 0,
        }
    }

    #[tokio::test]
    async fn move_time_skipped_broadcasts_skip_time_to_other_players_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let registry = std::sync::Arc::new(wow_network::PlayerRegistry::default());
        let (self_tx, self_rx) = flume::bounded(1);
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_registry(std::sync::Arc::clone(&registry));
        session.set_player_movement_time_like_cpp(100);
        registry.insert(guid, broadcast_info(guid, self_tx));
        registry.insert(other_guid, broadcast_info(other_guid, other_tx));

        session
            .handle_move_time_skipped(wow_packet::packets::movement::MoveTimeSkipped {
                mover_guid: guid,
                time_skipped: 25,
            })
            .await;

        assert!(self_rx.try_recv().is_err());
        let bytes = other_rx.try_recv().unwrap();
        let pkt = wow_packet::WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.server_opcode(),
            Some(wow_constants::ServerOpcodes::MoveSkipTime)
        );
        assert_eq!(session.player_movement_time_like_cpp(), 125);
    }
}

// ── Handler registration (SetActiveMover) ────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActiveMover,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_active_mover",
    }
}

// ── Handler registration (MoveInitActiveMoverComplete) ───────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveInitActiveMoverComplete,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_init_active_mover_complete",
    }
}

macro_rules! register_movement_ack_message {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_movement_ack_message",
            }
        }
    };
}

macro_rules! register_movement_speed_ack {
    ($opcode:ident) => {
        inventory::submit! {
            PacketHandlerEntry {
                opcode: ClientOpcodes::$opcode,
                status: SessionStatus::LoggedIn,
                processing: PacketProcessing::ThreadSafe,
                handler_name: "handle_movement_speed_ack",
            }
        }
    };
}

register_movement_ack_message!(MoveCollisionDisableAck);
register_movement_ack_message!(MoveCollisionEnableAck);
register_movement_ack_message!(MoveEnableDoubleJumpAck);
register_movement_ack_message!(MoveEnableSwimToFlyTransAck);
register_movement_ack_message!(MoveFeatherFallAck);
register_movement_ack_message!(MoveForceRootAck);
register_movement_ack_message!(MoveForceUnrootAck);
register_movement_ack_message!(MoveGravityDisableAck);
register_movement_ack_message!(MoveGravityEnableAck);
register_movement_ack_message!(MoveHoverAck);
register_movement_ack_message!(MoveInertiaDisableAck);
register_movement_ack_message!(MoveInertiaEnableAck);
register_movement_ack_message!(MoveSetCanFlyAck);
register_movement_ack_message!(MoveSetCanTurnWhileFallingAck);
register_movement_ack_message!(MoveSetIgnoreMovementForcesAck);
register_movement_ack_message!(MoveWaterWalkAck);

register_movement_speed_ack!(MoveForceWalkSpeedChangeAck);
register_movement_speed_ack!(MoveForceRunSpeedChangeAck);
register_movement_speed_ack!(MoveForceRunBackSpeedChangeAck);
register_movement_speed_ack!(MoveForceSwimSpeedChangeAck);
register_movement_speed_ack!(MoveForceSwimBackSpeedChangeAck);
register_movement_speed_ack!(MoveForceTurnRateChangeAck);
register_movement_speed_ack!(MoveForceFlightSpeedChangeAck);
register_movement_speed_ack!(MoveForceFlightBackSpeedChangeAck);
register_movement_speed_ack!(MoveForcePitchRateChangeAck);
register_movement_speed_ack!(MoveSetModMovementForceMagnitudeAck);

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveKnockBackAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_knock_back_ack",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetCollisionHeightAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_collision_height_ack",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveApplyMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_apply_movement_force_ack",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_remove_movement_force_ack",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTimeSkipped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_time_skipped",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSplineDone,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_spline_done",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTeleportAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_teleport_ack",
    }
}
