// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_*.
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Sanitize movement flags like `Player::ValidateMovementInfo`
//!   3. Validate: GUID must match `Player::GetUnitBeingMoved()`, position must be finite
//!   4. Update server-side mover position when represented
//!   5. Broadcast SMSG_MOVE_UPDATE to nearby visible sessions
//!
//! Reference: C++ `WorldSession::HandleMovementOpcode`.

use tracing::{info, trace, warn};
use wow_packet::ClientPacket;

use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitStandStateType;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ServerPacket;
use wow_packet::packets::movement::{
    ClientPlayerMovement, MoveApplyMovementForceAck, MoveInitActiveMoverComplete, MoveKnockBackAck,
    MoveRemoveMovementForceAck, MoveSetCollisionHeightAck, MoveSkipTime, MoveSplineDone,
    MoveTeleportAck, MoveTimeSkipped, MoveUpdate, MoveUpdateApplyMovementForce,
    MoveUpdateKnockBack, MoveUpdateModMovementForceMagnitude, MoveUpdateRemoveMovementForce,
    MovementAckMessage, MovementInfo, MovementSpeedAck, SetActiveMover,
};

use crate::map_manager::zone_and_area_for_position_like_cpp;
use crate::session::{
    AreaTriggerCatalogsLikeCpp, ProgressionCatalogsLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP, SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
    WorldSession,
};

// C++ `HandleMoveSetVehicleRecAck` has no session-visible side effect, so
// the #142 wire-dispatch test proves reachability with a test-only call
// counter instead of inventing production state.
#[cfg(test)]
static MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST: std::sync::Mutex<
    Vec<(std::thread::ThreadId, usize)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn record_move_set_vehicle_rec_id_ack_handler_call_for_test() {
    let thread_id = std::thread::current().id();
    let mut calls_by_thread = MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST
        .lock()
        .expect("vehicle ACK test-call counter poisoned");
    if let Some((_, calls)) = calls_by_thread
        .iter_mut()
        .find(|(candidate, _)| *candidate == thread_id)
    {
        *calls += 1;
    } else {
        calls_by_thread.push((thread_id, 1));
    }
}

#[cfg(test)]
pub(crate) fn take_move_set_vehicle_rec_id_ack_handler_calls_for_test() -> usize {
    let thread_id = std::thread::current().id();
    let mut calls_by_thread = MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST
        .lock()
        .expect("vehicle ACK test-call counter poisoned");
    calls_by_thread
        .iter()
        .position(|(candidate, _)| *candidate == thread_id)
        .map(|index| calls_by_thread.swap_remove(index).1)
        .unwrap_or(0)
}

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
                handler: |session, catalogs, pkt| {
                    Box::pin(async move {
                        session
                            .handle_movement_with_catalogs_like_cpp(
                                catalogs.area_triggers.as_ref(),
                                catalogs.creature_spawns.as_ref(),
                                catalogs.progression.as_ref(),
                                &catalogs.player_grid_loader,
                                pkt,
                            )
                            .await
                    })
                },
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
    pub async fn handle_movement_with_catalogs_like_cpp(
        &mut self,
        area_trigger_catalogs: &AreaTriggerCatalogsLikeCpp,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        progression: &ProgressionCatalogsLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
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

        self.handle_movement_info_with_catalogs_like_cpp(
            area_trigger_catalogs,
            creature_spawn_catalogs,
            progression,
            player_grid_loader,
            opcode,
            info.info,
        )
        .await;
    }

    pub(crate) async fn handle_movement_info_with_catalogs_like_cpp(
        &mut self,
        area_trigger_catalogs: &AreaTriggerCatalogsLikeCpp,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        progression: &ProgressionCatalogsLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
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
        let Some(mover_guid) = self.player_moved_unit_guid_like_cpp() else {
            warn!(
                account = self.account_id,
                "Movement packet received without active mover"
            );
            return;
        };
        let mover_is_player = mover_guid == player_guid;
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                ?opcode,
                mover = ?info.guid,
                expected_mover = ?mover_guid,
                player = ?player_guid,
                flags = ?info.flags,
                client_time = info.time,
                x = info.position.x,
                y = info.position.y,
                z = info.position.z,
                o = info.position.orientation,
                has_transport = info.transport.is_some(),
                "RUST_LOGIN_TRACE movement_received"
            );
        }

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

        if info.guid != mover_guid {
            self.trace_anticheat_violation_like_cpp(
                "HandleMovementOpcode.GuidMismatch",
                opcode,
                "reject",
            );
            warn!(
                account = self.account_id,
                "Movement GUID mismatch: expected {:?}, got {:?}", mover_guid, info.guid
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

        if mover_is_player {
            self.clear_player_emote_state_on_player_movement_like_cpp();
        }

        let current_mover_position = if mover_is_player {
            self.player_position_like_cpp()
        } else {
            None
        };
        let new_player_cell_like_cpp =
            mover_is_player.then(|| wow_map::cell_from_world(pos.x, pos.y));
        let old_player_cell_like_cpp = current_mover_position
            .filter(|_| mover_is_player)
            .map(|current| wow_map::cell_from_world(current.x, current.y));
        let load_player_active_grid_like_cpp = match (
            old_player_cell_like_cpp.as_ref(),
            new_player_cell_like_cpp.as_ref(),
        ) {
            (Some(old_cell), Some(new_cell)) => old_cell.diff_grid(new_cell),
            (None, Some(_)) => true,
            _ => false,
        };
        if let Some(transport) = &info.transport {
            if current_mover_position.is_some_and(|current| {
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

        if mover_is_player {
            self.set_player_transport_info_like_cpp(info.transport.clone());
            self.apply_movement_side_effects_like_cpp(opcode, &info);
        } else if matches!(
            opcode,
            Some(ClientOpcodes::MoveSetFly) | Some(ClientOpcodes::MoveSetAdvFly)
        ) {
            // C++ removes the temporary pet from the active player, not from the
            // moved unit, even when a controlled unit is the mover.
            self.request_temporary_pet_unsummon_like_cpp();
        }
        info.guid = mover_guid;
        info.time = self.adjust_client_movement_time_like_cpp(info.time);
        let adjusted_time = info.time;

        if mover_is_player {
            self.set_player_movement_time_like_cpp(info.time);
            self.set_player_movement_flags_like_cpp(info.flags);
            self.set_player_movement_jump_like_cpp(info.jump.clone());

            // Update server-side player position.
            self.set_player_position_like_cpp(info.position);
            let authoritative_grid_map_key = self
                .current_canonical_player_map_key_like_cpp()
                .filter(|key| key.map_id == u32::from(self.player_map_id_like_cpp()));
            let grid_instance_id = authoritative_grid_map_key
                .map(|key| key.instance_id)
                .unwrap_or(0);
            if load_player_active_grid_like_cpp {
                let outcome = player_grid_loader(
                    self.player_map_id_like_cpp(),
                    authoritative_grid_map_key.map(|key| key.instance_id),
                    pos,
                );
                trace!(
                    account = self.account_id,
                    map_id = self.player_map_id_like_cpp(),
                    instance_id = grid_instance_id,
                    old_grid_x = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                    old_grid_y = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                    new_grid_x = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                    new_grid_y = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                    map_unavailable = outcome.map_unavailable,
                    grid_loaded_now = outcome.grid_loaded_now,
                    creature_records_added = outcome.creature_records_added,
                    gameobject_records_added = outcome.gameobject_records_added,
                    area_trigger_records_added = outcome.area_trigger_records_added,
                    legacy_creature_mirrors = outcome.legacy_creature_mirrors,
                    "C++ Map::PlayerRelocation active grid loaded before player visibility"
                );
                if (std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
                    || std::env::var_os("RUSTYCORE_CREATURE_VIS_TRACE").is_some())
                    && (outcome.map_unavailable
                        || outcome.map_created
                        || outcome.grid_loaded_now
                        || outcome.metadata_entries != 0
                        || outcome.skipped_already_loaded != 0
                        || outcome.skipped_should_not_spawn != 0
                        || outcome.skipped_difficulty_mismatch != 0
                        || outcome.stale_index_entries != 0
                        || outcome.creature_records_added != 0
                        || outcome.gameobject_records_added != 0
                        || outcome.area_trigger_records_added != 0
                        || outcome.pre_add_records_added != 0
                        || outcome.add_to_map_errors != 0
                        || outcome.load_record_missing != 0
                        || outcome.legacy_creature_mirrors != 0)
                {
                    info!(
                        account = self.account_id,
                        map_id = self.player_map_id_like_cpp(),
                        instance_id = grid_instance_id,
                        x = pos.x,
                        y = pos.y,
                        z = pos.z,
                        old_grid_x = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                        old_grid_y = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                        new_grid_x = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                        new_grid_y = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                        map_unavailable = outcome.map_unavailable,
                        map_created = outcome.map_created,
                        grid_loaded_now = outcome.grid_loaded_now,
                        metadata_entries = outcome.metadata_entries,
                        skipped_already_loaded = outcome.skipped_already_loaded,
                        skipped_should_not_spawn = outcome.skipped_should_not_spawn,
                        skipped_difficulty_mismatch = outcome.skipped_difficulty_mismatch,
                        stale_index_entries = outcome.stale_index_entries,
                        creature_records_added = outcome.creature_records_added,
                        gameobject_records_added = outcome.gameobject_records_added,
                        area_trigger_records_added = outcome.area_trigger_records_added,
                        pre_add_records_added = outcome.pre_add_records_added,
                        add_to_map_errors = outcome.add_to_map_errors,
                        load_record_missing = outcome.load_record_missing,
                        creature_load_record_missing = outcome.creature_load_record_missing,
                        gameobject_load_record_missing = outcome.gameobject_load_record_missing,
                        area_trigger_load_record_missing = outcome.area_trigger_load_record_missing,
                        legacy_creature_mirrors = outcome.legacy_creature_mirrors,
                        "RUST_CREATURE_VIS movement_grid_load"
                    );
                }
            }
            let area_id = match zone_and_area_for_position_like_cpp(
                &self.mmap_runtime_config_like_cpp().data_dir,
                u32::from(self.player_map_id_like_cpp()),
                info.position.x,
                info.position.y,
                self.area_table_store().map(|store| store.as_ref()),
                |map_id| {
                    self.map_store()
                        .as_deref()
                        .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                        .unwrap_or(0)
                },
            ) {
                Ok((zone_id, area_id)) => {
                    if area_id != 0 {
                        self.update_zone_represented_like_cpp(zone_id, area_id);
                        area_id
                    } else {
                        let Some((_, current_area_id)) = self.player_zone_area_like_cpp() else {
                            return;
                        };
                        current_area_id
                    }
                }
                Err(error) => {
                    let Some((_, area_id)) = self.player_zone_area_like_cpp() else {
                        return;
                    };
                    warn!(
                        account = self.account_id,
                        map_id = self.player_map_id_like_cpp(),
                        x = info.position.x,
                        y = info.position.y,
                        %error,
                        "failed to resolve C++ terrain zone/area after movement; keeping existing zone/area"
                    );
                    area_id
                }
            };
            self.check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
                progression,
                area_id,
            )
            .await;
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
            self.update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;

            // Check area triggers at the new position
            self.check_area_triggers_with_catalogs_like_cpp(area_trigger_catalogs)
                .await;
        } else {
            let moved = self
                .mutate_world_creature(mover_guid, |creature| {
                    creature.creature.set_ai_position(info.position);
                    creature
                        .creature
                        .unit_mut()
                        .set_movement_time_like_cpp(info.time);
                    creature
                        .creature
                        .unit_mut()
                        .set_movement_flags_like_cpp(info.flags);
                    creature
                        .creature
                        .set_movement_flags_runtime_like_cpp(info.flags);
                    creature.create_data.movement_flags = info.flags.bits();
                })
                .is_some();
            trace!(
                account = self.account_id,
                mover = ?mover_guid,
                x = pos.x,
                y = pos.y,
                z = pos.z,
                represented = moved,
                "Controlled mover moved"
            );
        }

        // TODO: aggro proximity check re-enable once combat system is stable
        // self.check_creature_aggro().await;

        // C++ `mover->SendMessageToSet(moveUpdate.Write(), _player)` uses
        // the mover visibility range and skips this player's session.
        // Candidate routing is cheap here; the receiver session applies the
        // final HaveAtClient gate through `SendIfVisibleLikeCpp`.
        if let Some(registry) = self.player_registry() {
            let move_update = MoveUpdate { info };
            let packet_bytes = move_update.to_bytes();
            let map_id = self.player_map_id_like_cpp();
            let instance_id = self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0);
            for registration in registry.movement_recipients_within_range(
                player_guid,
                map_id,
                instance_id,
                pos,
                crate::map_manager::VISIBILITY_RADIUS,
            ) {
                let _ = registry.try_send_current_command(
                    registration,
                    crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                        crate::session::mailbox::SendIfVisibleLikeCppCommand {
                            queued_at: std::time::Instant::now(),
                            source_guid: mover_guid,
                            map_id,
                            instance_id,
                            packet_bytes: packet_bytes.clone(),
                        },
                    ),
                );
            }
        }
        if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
            info!(
                account = self.account_id,
                ?opcode,
                adjusted_time,
                x = pos.x,
                y = pos.y,
                z = pos.z,
                "RUST_LOGIN_TRACE movement_applied"
            );
        }
    }

    #[cfg(test)]
    pub async fn handle_movement(&mut self, pkt: wow_packet::WorldPacket) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        self.handle_movement_with_catalogs_like_cpp(
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            pkt,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn handle_movement_info_like_cpp(
        &mut self,
        opcode: Option<ClientOpcodes>,
        info: MovementInfo,
    ) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        self.handle_movement_info_with_catalogs_like_cpp(
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            opcode,
            info,
        )
        .await;
    }

    fn apply_movement_side_effects_like_cpp(
        &mut self,
        opcode: Option<ClientOpcodes>,
        info: &MovementInfo,
    ) {
        self.clear_player_emote_state_on_player_movement_like_cpp();

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

    fn clear_player_emote_state_on_player_movement_like_cpp(&mut self) {
        if let Some(update) = self.clear_player_emote_state_on_movement_like_cpp() {
            self.send_packet(&update);
            self.broadcast_to_movement_set_like_cpp(update.to_bytes(), false);
        }
    }

    /// Handle CMSG_SET_ACTIVE_MOVER — client sets which unit is currently being moved.
    ///
    /// The client sends this after login to establish the active mover GUID.
    /// The mover must match C++ `Player::GetUnitBeingMoved()`.
    pub async fn handle_set_active_mover(&mut self, pkt: SetActiveMover) {
        info!(
            account = self.account_id,
            mover = ?pkt.active_mover,
            expected = ?self.player_moved_unit_guid_like_cpp(),
            "RUST_LOGIN_TRACE SetActiveMover"
        );

        let Some(expected_mover) = self.player_moved_unit_guid_like_cpp() else {
            warn!(
                account = self.account_id,
                "SetActiveMover received without canonical active mover"
            );
            return;
        };
        if pkt.active_mover != expected_mover {
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
    /// C++ updates transport timing, then calls `UpdateObjectVisibility(false)`.
    /// That marks `NOTIFY_VISIBILITY_CHANGED`; the visible object batch is sent
    /// later by the normal map/object visibility pass, not directly from this
    /// packet handler.
    pub async fn handle_move_init_active_mover_complete(
        &mut self,
        pkt: MoveInitActiveMoverComplete,
    ) {
        info!(
            account = self.account_id,
            ticks = pkt.ticks,
            "RUST_LOGIN_TRACE MoveInitActiveMoverComplete"
        );
        self.apply_move_init_active_mover_complete_like_cpp(pkt.ticks);
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
        #[cfg(test)]
        record_move_set_vehicle_rec_id_ack_handler_call_for_test();
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
            let Some(adjusted_time) = self.resolved_player_movement_time_like_cpp() else {
                return;
            };
            status.time = adjusted_time;
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
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateApplyMovementForce {
                    status: pkt.ack.status,
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
            self.broadcast_to_movement_set_like_cpp(
                MoveUpdateRemoveMovementForce {
                    status: pkt.ack.status,
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
        AuraApplication, MMapRuntimeConfigLikeCpp, MoveSplineDoneTaxiActionLikeCpp,
        MoveTeleportAckActionLikeCpp, MovementSpeedAckActionLikeCpp, PlayerGridLoadOutcomeLikeCpp,
        RepresentedAuraEffectLikeCpp, RepresentedTaxiFlightNodeLikeCpp, SessionPlayerController,
        UnitMoveTypeLikeCpp,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, RwLock};
    use wow_constants::ServerOpcodes;
    use wow_constants::movement::MovementFlag;
    use wow_constants::unit::UnitFlags;
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_packet::packets::movement::TransportInfo;

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

    fn movement_packet(opcode: ClientOpcodes, movement: &MovementInfo) -> wow_packet::WorldPacket {
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(opcode as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        inbound
    }

    fn unique_temp_data_dir(test_name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "rustycore-movement-{test_name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("maps")).expect("create maps dir");
        dir
    }

    fn write_single_area_map_tile_like_cpp(
        data_dir: &std::path::Path,
        map_id: u32,
        x: f32,
        y: f32,
        area_id: u16,
    ) {
        const MAP_FILE_HEADER_SIZE_LIKE_CPP: usize = 44;
        const MAP_AREA_HEADER_SIZE_LIKE_CPP: usize = 8;
        const MAP_AREA_CELLS_PER_GRID_LIKE_CPP: usize = 16;

        let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
        let area_size = (MAP_AREA_HEADER_SIZE_LIKE_CPP
            + MAP_AREA_CELLS_PER_GRID_LIKE_CPP
                * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
                * std::mem::size_of::<u16>()) as u32;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MAPS");
        bytes.extend_from_slice(&10_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&area_offset.to_le_bytes());
        bytes.extend_from_slice(&area_size.to_le_bytes());
        for _ in 0..6 {
            bytes.extend_from_slice(&0_u32.to_le_bytes());
        }
        assert_eq!(bytes.len(), MAP_FILE_HEADER_SIZE_LIKE_CPP);
        bytes.extend_from_slice(b"AREA");
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&area_id.to_le_bytes());
        for _ in 0..(MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP) {
            bytes.extend_from_slice(&area_id.to_le_bytes());
        }

        let (gx, gy) = crate::map_manager::terrain_grid_coords_for_wow_position_like_cpp(x, y);
        fs::write(
            data_dir
                .join("maps")
                .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
            bytes,
        )
        .expect("write movement area map");
    }

    fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
        let mut opcodes = Vec::new();
        while let Ok(bytes) = send_rx.try_recv() {
            if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
                opcodes.push(opcode);
            }
        }
        opcodes
    }

    fn visible_aura(slot: u8, flags: u32, flags2: u32) -> AuraApplication {
        AuraApplication {
            spell_id: 1000 + i32::from(slot),
            difficulty_id: 0,
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
    fn movement_clears_player_emote_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_rx();
        let guid = ObjectGuid::create_player(1, 46);
        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
        session
            .set_player_emote_state_like_cpp(10)
            .expect("emote state update packet");
        assert_eq!(session.player_emote_state_like_cpp(), 10);

        let mut info = MovementInfo::default();
        info.flags = MovementFlag::FORWARD;
        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);

        assert_eq!(session.player_emote_state_like_cpp(), 0);
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::UpdateObject],
            "C++ MovementHandler clears UnitData::EmoteState on accepted player movement"
        );

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveHeartbeat), &info);
        assert_eq!(
            drain_server_opcodes(&send_rx),
            Vec::<ServerOpcodes>::new(),
            "C++ only updates EmoteState when a stateful emote was active"
        );
    }

    #[tokio::test]
    async fn rejected_transport_movement_clears_player_emote_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_rx();
        let guid = ObjectGuid::create_player(1, 47);
        session.set_player_guid(Some(guid));
        session.set_player_moved_unit_guid_like_cpp(guid);
        session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.0));
        session
            .set_player_emote_state_like_cpp(10)
            .expect("emote state update packet");

        let info = MovementInfo {
            guid,
            position: Position::new(1.0, 2.0, 3.0, 0.0),
            transport: Some(TransportInfo {
                guid: ObjectGuid::EMPTY,
                x: 76.0,
                y: 0.0,
                z: 0.0,
                o: 0.0,
                seat: 0,
                time: 0,
                prev_time: None,
                vehicle_id: None,
            }),
            ..MovementInfo::default()
        };

        session
            .handle_movement_info_like_cpp(Some(ClientOpcodes::MoveHeartbeat), info)
            .await;

        assert_eq!(session.player_emote_state_like_cpp(), 0);
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::UpdateObject],
            "C++ clears EmoteState before transport movement early returns"
        );
    }

    #[tokio::test]
    async fn accepted_movement_updates_represented_jump_info_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 44);
        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(wow_core::Position::new(1.0, 2.0, 3.0, 0.0));

        let mut info = MovementInfo {
            guid,
            time: 2_000,
            position: wow_core::Position::new(10.0, 20.0, 30.0, 1.0),
            ..MovementInfo::default()
        };
        info.jump.fall_time = 1_234;
        info.jump.z_speed = 6.25;
        info.jump.has_direction = true;
        info.jump.sin_angle = 0.1;
        info.jump.cos_angle = 0.9;
        info.jump.xy_speed = 7.5;

        session
            .handle_movement_info_like_cpp(Some(ClientOpcodes::MoveJump), info)
            .await;

        let jump = session.player_movement_jump_like_cpp();
        assert_eq!(jump.fall_time, 1_234);
        assert_eq!(jump.z_speed, 6.25);
        assert!(jump.has_direction);
        assert_eq!(jump.sin_angle, 0.1);
        assert_eq!(jump.cos_angle, 0.9);
        assert_eq!(jump.xy_speed, 7.5);
    }

    #[tokio::test]
    async fn player_movement_loads_active_grid_before_visibility_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 45);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let same_grid_position = Position::new(10.0, 20.0, 30.0, 1.0);
        let new_grid_position = Position::new(600.0, 20.0, 30.0, 1.0);
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(Vec::new()));

        let canonical: crate::SharedCanonicalMapManager =
            Arc::new(Mutex::new(wow_map::MapManager::default()));
        let mut canonical_player = wow_entities::Player::new(Some(1), false);
        canonical_player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(guid);
        canonical_player
            .unit_mut()
            .world_mut()
            .set_map(1, 77)
            .unwrap();
        canonical_player
            .unit_mut()
            .world_mut()
            .relocate(login_position);
        canonical_player
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        canonical
            .lock()
            .unwrap()
            .create_map_entry(
                1,
                77,
                0,
                wow_map::ManagedMapKind::Dungeon {
                    has_reset_schedule: false,
                },
            )
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_player(canonical_player).unwrap(),
            )
            .unwrap();
        session.set_canonical_map_manager(canonical);

        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            "MovementGridLoader".to_string(),
            login_position,
            1,
            1,
            3,
            10,
            0,
        ));
        session.set_player_moved_unit_guid_like_cpp(guid);
        let calls_for_resolver = Arc::clone(&calls);
        let seen_for_resolver = Arc::clone(&seen);
        let player_grid_loader: crate::session::PlayerGridLoadResolverLikeCpp =
            Arc::new(move |map_id, instance_id, pos| {
                calls_for_resolver.fetch_add(1, Ordering::SeqCst);
                seen_for_resolver
                    .lock()
                    .unwrap()
                    .push((map_id, instance_id, pos));
                PlayerGridLoadOutcomeLikeCpp {
                    grid_loaded_now: true,
                    creature_records_added: 7,
                    legacy_creature_mirrors: 7,
                    ..PlayerGridLoadOutcomeLikeCpp::default()
                }
            });
        let area_triggers = session.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
        let progression = session.progression_catalogs_for_test_like_cpp();

        let same_grid = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: same_grid_position,
            ..MovementInfo::default()
        };
        session
            .handle_movement_info_with_catalogs_like_cpp(
                &area_triggers,
                &creature_spawns,
                &progression,
                &player_grid_loader,
                Some(ClientOpcodes::MoveHeartbeat),
                same_grid,
            )
            .await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "C++ Map::PlayerRelocation does not EnsureGridLoadedForActiveObject when the player stays in the same grid"
        );

        let new_grid = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: new_grid_position,
            ..MovementInfo::default()
        };
        session
            .handle_movement_info_with_catalogs_like_cpp(
                &area_triggers,
                &creature_spawns,
                &progression,
                &player_grid_loader,
                Some(ClientOpcodes::MoveHeartbeat),
                new_grid,
            )
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &[(1, Some(77), new_grid_position)]
        );
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
        assert!(
            send_rx.try_recv().is_err(),
            "non-lethal fall damage must not send a death values update"
        );

        let mut harmless = MovementInfo::default();
        harmless.position.z = 99.0;
        harmless.jump.fall_time = 1_600;
        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &harmless);
        assert_eq!(session.fall_damage_events_like_cpp().len(), 1);
    }

    #[test]
    fn movement_fall_land_lethal_damage_sends_player_values_update_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_rx();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 43)));
        session.set_player_health_like_cpp(1_000, 1_000);
        session.set_fall_information_like_cpp(1_200, 300.0);
        let mut info = MovementInfo::default();
        info.position.z = 100.0;
        info.jump.fall_time = 1_500;

        session.apply_movement_side_effects_like_cpp(Some(ClientOpcodes::MoveFallLand), &info);

        let events = session.fall_damage_events_like_cpp();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].damage, 1_000);
        assert_eq!(events[0].final_damage, 1_000);
        assert_eq!(session.player_health_like_cpp(), 0);
        assert!(!session.player_is_alive_like_cpp());
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![
                ServerOpcodes::HealthUpdate,
                ServerOpcodes::EnvironmentalDamageLog,
                ServerOpcodes::UpdateObject,
            ],
            "C++ lethal EnvironmentalDamage goes through Unit::Kill/Player::setDeathState before release/cemetery flows; Rust must publish the zero-health values update, not only the combat log"
        );
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
        let sent = send_rx.try_recv().expect("void death values update");
        let opcode = u16::from_le_bytes([sent[0], sent[1]]);
        assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);

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
        assert_eq!(session.movement_visibility_refresh_requests_like_cpp(), 0);
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
        assert_eq!(session.player_zone_area_like_cpp(), Some((20, 21)));
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
        let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
        let (self_tx, self_rx) = flume::bounded(1);
        let (other_tx, other_rx) = flume::bounded(1);
        let (self_command_tx, self_command_rx) = flume::bounded(1);
        let (other_command_tx, other_command_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.register_or_replace(
            guid,
            broadcast_info_with_command(guid, self_tx, self_command_tx),
            Default::default(),
        );
        registry.register_or_replace(
            other_guid,
            broadcast_info_with_command(other_guid, other_tx, other_command_tx),
            Default::default(),
        );

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
        assert!(other_rx.try_recv().is_err());
        assert!(self_command_rx.try_recv().is_err());
        let command = other_command_rx
            .try_recv()
            .expect("visible movement command");
        let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
            panic!("expected SendIfVisibleLikeCpp movement command");
        };
        assert_eq!(command.source_guid, guid);
        assert_eq!(command.map_id, 0);
        assert_eq!(command.instance_id, 0);
        let bytes = command.packet_bytes;
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

    #[test]
    fn movement_directory_rejects_replaced_recipient_generation_like_cpp() {
        let source_guid = ObjectGuid::create_player(1, 44);
        let recipient_guid = ObjectGuid::create_player(1, 45);
        let registry = crate::session::directory::PlayerRegistry::default();
        let (old_send_tx, _old_send_rx) = flume::bounded(1);
        let (old_command_tx, old_command_rx) = flume::bounded(1);
        registry.register_or_replace(
            recipient_guid,
            broadcast_info_with_command(recipient_guid, old_send_tx, old_command_tx),
            Default::default(),
        );
        let recipients = registry.movement_recipients_within_range(
            source_guid,
            0,
            0,
            Position::ZERO,
            crate::map_manager::VISIBILITY_RADIUS,
        );
        let [stale] = recipients.as_slice() else {
            panic!("expected the first recipient generation");
        };
        let stale = *stale;

        let (replacement_send_tx, _replacement_send_rx) = flume::bounded(1);
        let (replacement_command_tx, replacement_command_rx) = flume::bounded(1);
        registry.register_or_replace(
            recipient_guid,
            broadcast_info_with_command(
                recipient_guid,
                replacement_send_tx,
                replacement_command_tx,
            ),
            Default::default(),
        );

        let result = registry.try_send_current_command(
            stale,
            crate::session::mailbox::SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
        );
        assert_eq!(
            result,
            Err(crate::session::directory::PlayerDirectorySendError::StaleRegistration)
        );
        assert!(old_command_rx.try_recv().is_err());
        assert!(replacement_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn handle_movement_uses_current_mover_guid_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 142);
        let mover_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 777, 1_142);
        let other_guid = ObjectGuid::create_player(1, 143);
        let player_position = Position::new(1.0, 2.0, 3.0, 0.5);
        let mover_start = Position::new(10.0, 10.0, 0.0, 0.0);
        let moved_position = Position::new(12.0, 13.0, 1.0, 1.25);
        let manager = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));
        let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
        let (self_tx, _self_rx) = flume::bounded(1);
        let (other_tx, other_rx) = flume::bounded(1);
        let (self_command_tx, self_command_rx) = flume::bounded(1);
        let (other_command_tx, other_command_rx) = flume::bounded(1);

        session.set_player_guid(Some(player_guid));
        session.set_player_moved_unit_guid_like_cpp(mover_guid);
        session.set_player_position_like_cpp(player_position);
        session.set_player_movement_time_like_cpp(7_777);
        session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
        session.set_map_manager(Arc::clone(&manager));
        session.set_player_registry(Arc::clone(&registry));

        let (grid_x, grid_y) =
            crate::map_manager::world_to_grid_coords(mover_start.x, mover_start.y);
        manager.write().unwrap().add_creature(
            0,
            0,
            grid_x,
            grid_y,
            crate::map_manager::WorldCreature::new(
                mover_guid,
                777,
                mover_start,
                100,
                80,
                1,
                2,
                0.0,
                1,
                35,
                0,
                0,
            ),
        );

        registry.register_or_replace(
            player_guid,
            broadcast_info_with_command(player_guid, self_tx, self_command_tx),
            Default::default(),
        );
        let mut other_info = broadcast_info_with_command(other_guid, other_tx, other_command_tx);
        other_info.placement.position = moved_position;
        registry.register_or_replace(other_guid, other_info, Default::default());

        let movement = MovementInfo {
            guid: mover_guid,
            flags: MovementFlag::FORWARD,
            time: 1_234,
            position: moved_position,
            ..MovementInfo::default()
        };
        session
            .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
            .await;

        assert_eq!(session.player_position_like_cpp(), Some(player_position));
        assert_eq!(session.player_movement_time_like_cpp(), 7_777);
        assert_eq!(
            session.player_movement_flags_like_cpp(),
            MovementFlag::SWIMMING
        );
        let (creature_position, creature_flags, creature_time) = {
            let guard = manager.read().unwrap();
            let creature = guard
                .find_creature(0, 0, mover_guid)
                .expect("controlled mover creature");
            (
                creature.position(),
                creature.creature.unit().movement_flags_like_cpp(),
                creature.creature.unit().movement_time_like_cpp(),
            )
        };
        assert_eq!(creature_position, moved_position);
        assert_eq!(creature_flags, MovementFlag::FORWARD);

        assert!(self_command_rx.try_recv().is_err());
        assert!(other_rx.try_recv().is_err());
        let command = other_command_rx
            .try_recv()
            .expect("visible controlled-mover movement command");
        let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
            panic!("expected SendIfVisibleLikeCpp movement command");
        };
        assert_eq!(command.source_guid, mover_guid);
        let mut packet = wow_packet::WorldPacket::from_bytes(&command.packet_bytes);
        assert_eq!(
            packet.server_opcode(),
            Some(wow_constants::ServerOpcodes::MoveUpdate)
        );
        packet.read_uint16().expect("move update opcode");
        let status = MovementInfo::read(&mut packet).expect("move update status");
        assert_eq!(status.guid, mover_guid);
        assert_eq!(status.flags, MovementFlag::FORWARD);
        assert_eq!(status.position, moved_position);
        assert_eq!(creature_time, status.time);
    }

    #[tokio::test]
    async fn handle_movement_does_not_broadcast_outside_visibility_range_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let far_guid = ObjectGuid::create_player(1, 44);
        let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
        let (self_tx, _self_rx) = flume::bounded(1);
        let (far_tx, far_rx) = flume::bounded(1);
        let (self_command_tx, self_command_rx) = flume::bounded(1);
        let (far_command_tx, far_command_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.register_or_replace(
            guid,
            broadcast_info_with_command(guid, self_tx, self_command_tx),
            Default::default(),
        );
        let mut far_info = broadcast_info_with_command(far_guid, far_tx, far_command_tx);
        far_info.placement.position =
            wow_core::Position::new(crate::map_manager::VISIBILITY_RADIUS + 10.0, 0.0, 0.0, 0.0);
        registry.register_or_replace(far_guid, far_info, Default::default());

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: wow_core::Position::ZERO,
            ..MovementInfo::default()
        };
        let mut inbound = wow_packet::WorldPacket::new_empty();
        inbound.write_uint16(ClientOpcodes::MoveHeartbeat as u16);
        movement.write(&mut inbound);
        inbound.read_uint16().expect("movement opcode");
        session.handle_movement(inbound).await;

        assert!(self_command_rx.try_recv().is_err());
        assert!(far_command_rx.try_recv().is_err());
        assert!(far_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn handle_movement_syncs_canonical_player_position_for_logout_save_like_cpp() {
        let mut session = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1042);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let moved_position = Position::new(90.0, 20.0, 30.0, 1.0);

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
            time: 12_345,
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
        let canonical_cell = canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .and_then(|map| map.map().get_typed_player(guid))
            .and_then(|player| player.unit().world().current_cell())
            .expect("canonical player cell");
        let expected_cell = wow_map::cell_from_world(moved_position.x, moved_position.y);
        assert_eq!(
            canonical_cell,
            (expected_cell.cell_x(), expected_cell.cell_y()),
            "C++ Map::PlayerRelocation moves the Player between derived cell indexes"
        );
        let canonical_movement_flags = canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .and_then(|map| map.map().get_typed_player(guid))
            .map(|player| player.unit().movement_flags_like_cpp())
            .expect("canonical player movement flags");
        assert_eq!(
            canonical_movement_flags,
            MovementFlag::FORWARD,
            "C++ stores accepted player MovementInfo flags on Unit::m_movementInfo"
        );
        let canonical_movement_time = canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .and_then(|map| map.map().get_typed_player(guid))
            .map(|player| player.unit().movement_time_like_cpp())
            .expect("canonical player movement time");
        assert_eq!(
            canonical_movement_time,
            session.player_movement_time_like_cpp(),
            "C++ stores accepted player MovementInfo time on Unit::m_movementInfo"
        );
        assert_eq!(
            session
                .sync_session_from_save_to_db_snapshot_like_cpp()
                .unwrap()
                .position,
            moved_position
        );
    }

    #[tokio::test]
    async fn handle_movement_discovers_current_area_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_rx();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1094);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let moved_position = Position::new(11.0, 22.0, 33.0, 1.0);

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
        session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
            wow_data::AreaTableEntry {
                id: 9_104,
                continent_id: 571,
                parent_area_id: 0,
                area_bit: 65,
                exploration_level: 12,
                mount_flags: 0,
                flags: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            "MovementExplorer".to_string(),
            login_position,
            571,
            1,
            3,
            10,
            0,
        ));
        session.set_player_zone_area_like_cpp(9_104, 9_104);
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: moved_position,
            ..MovementInfo::default()
        };
        session
            .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
            .await;

        assert_eq!(
            session
                .represented_explored_zones_db_string_like_cpp()
                .expect("test Player explored-zones owner resolves")
                .split_whitespace()
                .take(4)
                .collect::<Vec<_>>(),
            vec!["0", "0", "2", "0"]
        );
        assert_eq!(
            session.represented_reveal_world_map_overlay_criteria_like_cpp(),
            &[9_104]
        );
        assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject));

        session
            .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
            .await;
        assert_eq!(
            session.represented_reveal_world_map_overlay_criteria_like_cpp(),
            &[9_104],
            "C++ discovery criteria only fires when the explored-zone bit changes"
        );
        assert!(!drain_server_opcodes(&send_rx).contains(&ServerOpcodes::UpdateObject));
    }

    #[tokio::test]
    async fn handle_movement_resolves_zone_area_for_cemetery_flow_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_rx();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1095);
        let map_id = 1_u32;
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let moved_position = Position::new(1922.0, -4345.0, 25.0, 1.0);
        let data_dir = unique_temp_data_dir("zone-area-cemetery");
        write_single_area_map_tile_like_cpp(
            &data_dir,
            map_id,
            moved_position.x,
            moved_position.y,
            5170,
        );

        canonical.lock().unwrap().create_world_map(map_id, 0);
        session.set_mmap_runtime_config_like_cpp(MMapRuntimeConfigLikeCpp {
            data_dir: data_dir.to_string_lossy().into_owned(),
            ..MMapRuntimeConfigLikeCpp::default()
        });
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: map_id,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([
            wow_data::AreaTableEntry {
                id: 1637,
                continent_id: map_id as u16,
                parent_area_id: 0,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            wow_data::AreaTableEntry {
                id: 5170,
                continent_id: map_id as u16,
                parent_area_id: 1637,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0x4000_0000,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid,
            "MovementZone".to_string(),
            login_position,
            map_id as u16,
            10,
            5,
            80,
            0,
        ));
        session.set_player_moved_unit_guid_like_cpp(guid);
        session.set_player_zone_area_like_cpp(1, 1);
        let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

        let movement = MovementInfo {
            guid,
            flags: MovementFlag::FORWARD,
            position: moved_position,
            ..MovementInfo::default()
        };
        session
            .handle_movement(movement_packet(ClientOpcodes::MoveHeartbeat, &movement))
            .await;

        assert_eq!(
            session.player_zone_area_like_cpp(),
            Some((1637, 5170)),
            "C++ Player::Update uses terrain GetZoneAndAreaId, so cemetery requests after movement must use the Orgrimmar zone, not stale DB zone"
        );
    }

    #[tokio::test]
    async fn logout_save_snapshot_uses_canonical_position_not_stale_session_mirror_like_cpp() {
        let mut session = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let guid = ObjectGuid::create_player(1, 1043);
        let login_position = Position::new(1.0, 2.0, 3.0, 0.25);
        let latest_session_position = Position::new(4.0, 5.0, 6.0, 0.5);
        let stale_canonical_position = Position::new(10.0, 20.0, 30.0, 1.0);

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
        session.set_player_position_like_cpp(latest_session_position);
        session.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .relocate(stale_canonical_position);
        });

        let snapshot = session
            .sync_session_from_save_to_db_snapshot_like_cpp()
            .expect("save snapshot");

        assert_eq!(snapshot.position, stale_canonical_position);
        assert_eq!(
            session.player_position_like_cpp(),
            Some(stale_canonical_position)
        );
    }

    #[tokio::test]
    async fn handle_movement_rejects_guid_mismatch_without_state_or_broadcast_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let spoofed_guid = ObjectGuid::create_player(1, 99);
        let other_guid = ObjectGuid::create_player(1, 43);
        let original_position = wow_core::Position::new(1.0, 2.0, 3.0, 0.5);
        let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(original_position);
        session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.register_or_replace(
            other_guid,
            broadcast_info(other_guid, other_tx),
            Default::default(),
        );

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
        let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
        let (other_tx, other_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_position_like_cpp(original_position);
        session.set_player_movement_flags_like_cpp(MovementFlag::SWIMMING);
        session.set_player_registry(std::sync::Arc::clone(&registry));
        registry.register_or_replace(
            other_guid,
            broadcast_info(other_guid, other_tx),
            Default::default(),
        );

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
    ) -> crate::session::directory::PlayerSessionRegistrationLikeCpp {
        let (command_tx, _command_rx) = flume::bounded(1);
        broadcast_info_with_command(guid, send_tx, command_tx)
    }

    fn broadcast_info_with_command(
        guid: ObjectGuid,
        send_tx: flume::Sender<Vec<u8>>,
        command_tx: flume::Sender<crate::session::mailbox::SessionCommand>,
    ) -> crate::session::directory::PlayerSessionRegistrationLikeCpp {
        crate::session::directory::PlayerSessionRegistrationLikeCpp {
            identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp {
                player_name: format!("Player{}", guid.counter()),
                account_id: guid.counter() as u32,
                recruiter_id: 0,
                race: 1,
                class: 1,
                sex: 0,
                active_expansion: 2,
            },
            placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
                map_id: 0,
                instance_id: 0,
                position: wow_core::Position::ZERO,
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: Vec::new(),
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            durable_creature_runtime_commands_like_cpp: Default::default(),
            client_visible_guids_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        }
    }

    #[tokio::test]
    async fn move_time_skipped_broadcasts_skip_time_to_other_players_like_cpp() {
        let mut session = make_session();
        let guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let registry = std::sync::Arc::new(crate::session::directory::PlayerRegistry::default());
        let (self_tx, self_rx) = flume::bounded(1);
        let (other_tx, other_rx) = flume::bounded(1);
        let (self_command_tx, self_command_rx) = flume::bounded(1);
        let (other_command_tx, other_command_rx) = flume::bounded(1);

        session.set_player_guid(Some(guid));
        session.set_player_registry(std::sync::Arc::clone(&registry));
        session.set_player_position_like_cpp(wow_core::Position::ZERO);
        session.set_player_movement_time_like_cpp(100);
        registry.register_or_replace(
            guid,
            broadcast_info_with_command(guid, self_tx, self_command_tx),
            Default::default(),
        );
        registry.register_or_replace(
            other_guid,
            broadcast_info_with_command(other_guid, other_tx, other_command_tx),
            Default::default(),
        );

        session
            .handle_move_time_skipped(wow_packet::packets::movement::MoveTimeSkipped {
                mover_guid: guid,
                time_skipped: 25,
            })
            .await;

        assert!(self_rx.try_recv().is_err());
        assert!(other_rx.try_recv().is_err());
        assert!(self_command_rx.try_recv().is_err());
        let command = other_command_rx
            .try_recv()
            .expect("visible movement-set command");
        let crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(command) = command else {
            panic!("expected SendIfVisibleLikeCpp move-skip-time command");
        };
        assert_eq!(command.source_guid, guid);
        let bytes = command.packet_bytes;
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
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::SetActiveMover::read(&mut pkt) {
                    Ok(mover) => session.handle_set_active_mover(mover).await,
                    Err(e) => tracing::warn!("Failed to read SetActiveMover: {e}"),
                }
            })
        },
    }
}

// ── Handler registration (MoveInitActiveMoverComplete) ───────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveInitActiveMoverComplete,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_init_active_mover_complete",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveInitActiveMoverComplete::read(&mut pkt) {
                    Ok(init) => session.handle_move_init_active_mover_complete(init).await,
                    Err(e) => tracing::warn!("Failed to read MoveInitActiveMoverComplete: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetVehicleRecIdAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_vehicle_rec_id_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::MoveSetVehicleRecIdAck); match wow_packet::packets::vehicle::MoveSetVehicleRecIdAck::read(&mut pkt) { Ok(ack) => session.handle_move_set_vehicle_rec_id_ack(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MoveSetVehicleRecIdAck: {e}"), } })
        },
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
                handler: |session, _catalogs, mut pkt| {
                    Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::$opcode); match wow_packet::packets::movement::MovementAckMessage::read(&mut pkt) { Ok(ack) => session.handle_movement_ack_message(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MovementAckMessage: {e}"), } })
                },
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
                handler: |session, _catalogs, mut pkt| {
                    Box::pin(async move { let opcode = pkt.client_opcode().unwrap_or(ClientOpcodes::$opcode); match wow_packet::packets::movement::MovementSpeedAck::read(&mut pkt) { Ok(ack) => session.handle_movement_speed_ack(opcode, ack).await, Err(e) => tracing::warn!("Failed to read MovementSpeedAck: {e}"), } })
                },
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
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveKnockBackAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_knock_back_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveKnockBackAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSetCollisionHeightAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_set_collision_height_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveSetCollisionHeightAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_set_collision_height_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveSetCollisionHeightAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveApplyMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_apply_movement_force_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveApplyMovementForceAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_apply_movement_force_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveApplyMovementForceAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveRemoveMovementForceAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_remove_movement_force_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveRemoveMovementForceAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_remove_movement_force_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveRemoveMovementForceAck: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTimeSkipped,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_move_time_skipped",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveTimeSkipped::read(&mut pkt) {
                    Ok(skipped) => session.handle_move_time_skipped(skipped).await,
                    Err(e) => tracing::warn!("Failed to read MoveTimeSkipped: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveSplineDone,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_spline_done",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveSplineDone::read(&mut pkt) {
                    Ok(done) => session.handle_move_spline_done(done).await,
                    Err(e) => tracing::warn!("Failed to read MoveSplineDone: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MoveTeleportAck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_move_teleport_ack",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::movement::MoveTeleportAck::read(&mut pkt) {
                    Ok(ack) => session.handle_move_teleport_ack(ack).await,
                    Err(e) => tracing::warn!("Failed to read MoveTeleportAck: {e}"),
                }
            })
        },
    }
}
