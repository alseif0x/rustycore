//! Movement handlers operations, part 1 of 1.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #654; every method keeps its original body.

use super::*;

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
    pub(super) fn apply_movement_side_effects_like_cpp(
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
    pub(super) fn clear_player_emote_state_on_player_movement_like_cpp(&mut self) {
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
