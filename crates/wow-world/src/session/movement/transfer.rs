//! Represented world transfer and teleport, including the far-transfer writer fence.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn pending_teleport_save_destination_like_cpp(
        &self,
    ) -> Option<(u16, Position)> {
        if let Some((map_id, position)) = self.pending_teleport_like_cpp() {
            return Some((u16::try_from(map_id).unwrap_or(u16::MAX), position));
        }
        if let Some(teleport) = self.player_teleport_state_snapshot_like_cpp()
            && teleport.near_pending
        {
            return teleport.near_destination;
        }
        None
    }
    pub(in crate::session) fn send_transfer_aborted_like_cpp(
        &self,
        map_id: u32,
        transfer_abort: u32,
    ) {
        self.send_transfer_aborted_with_params_like_cpp(map_id, transfer_abort, 0, 0);
    }
    pub(in crate::session) fn send_transfer_aborted_with_params_like_cpp(
        &self,
        map_id: u32,
        transfer_abort: u32,
        arg: u8,
        map_difficulty_x_condition_id: i32,
    ) {
        self.send_packet(&wow_packet::packets::misc::TransferAborted {
            map_id,
            arg,
            map_difficulty_x_condition_id,
            transfer_abort,
        });
    }
    /// Teleport the player to a new map and position.
    ///
    /// Sends SMSG_TRANSFER_PENDING (0x25cd) to initiate the transfer.
    /// The client will respond with CMSG_WORLD_PORT_ACK when ready.
    ///
    /// C++ `Player::TeleportTo` → `SendTransferPending`.
    pub async fn teleport_to(&mut self, new_map: u32, new_pos: wow_core::Position) {
        self.teleport_to_with_options(new_map, new_pos, TELE_TO_NONE_LIKE_CPP)
            .await;
    }
    pub(crate) async fn teleport_to_with_options(
        &mut self,
        new_map: u32,
        new_pos: wow_core::Position,
        mut options: TeleportToOptionsLikeCpp,
    ) {
        if self
            .player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.recovery == wow_entities::PlayerTransferRecovery::Terminal)
        {
            return;
        }
        // C++ Player.cpp:1239 validates the catalog map and all four coordinates
        // before movement, combat, pet, ownership or teleport state is changed.
        if self
            .maps
            .store
            .as_ref()
            .is_none_or(|maps| maps.get(new_map).is_none())
            || !new_pos.is_valid_map_coord_like_cpp()
        {
            warn!(
                "Invalid map or coordinates for teleport to {} from account {}",
                new_map, self.account_id
            );
            return;
        }

        if self.is_map_disabled_for_player_like_cpp(new_map) {
            warn!(
                account = self.account_id,
                map_id = new_map,
                "Teleport blocked by C++ DisableMgr map gate"
            );
            self.send_transfer_aborted_like_cpp(new_map, TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP);
            return;
        }

        if let Some(target_map) = self
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(new_map).copied())
            && target_map.is_battleground_or_arena()
            && !self.player_in_represented_battleground_like_cpp()
        {
            warn!(
                account = self.account_id,
                map_id = new_map,
                "Teleport silently blocked by C++ battleground assignment gate"
            );
            return;
        }

        if let Some(target_map) = self
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(new_map).copied())
            && self.expansion < target_map.expansion_like_cpp()
        {
            warn!(
                account = self.account_id,
                map_id = new_map,
                session_expansion = self.expansion,
                required_expansion = target_map.expansion_like_cpp(),
                "Teleport blocked by C++ client expansion gate"
            );
            self.send_transfer_aborted_with_params_like_cpp(
                new_map,
                TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP,
                target_map.expansion_like_cpp(),
                0,
            );
            return;
        }

        let Some(current_pos) = self.player_position_like_cpp() else {
            warn!(
                "Cannot teleport account {}: no current position",
                self.account_id
            );
            return;
        };

        self.exit_represented_vehicle_for_teleport_like_cpp();
        self.reset_teleport_movement_state_like_cpp();

        if self.player_class_like_cpp() == CLASS_DEATH_KNIGHT_LIKE_CPP
            && self.player_map_id_like_cpp() == DEATH_KNIGHT_START_MAP_LIKE_CPP
            && u32::from(self.player_map_id_like_cpp()) != new_map
            && self.player_is_game_master_like_cpp() != Some(true)
            && !self
                .known_spells_like_cpp()
                .contains(&DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP)
        {
            self.send_transfer_aborted_with_params_like_cpp(
                new_map,
                TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
                1,
                0,
            );
            return;
        }

        // Approved recovery exception: a detached Player needs AddPlayerToMap,
        // even when returning to its old map. A near ACK only relocates it.
        let active_map = self.current_canonical_player_map_key_like_cpp();
        let same_map_near_teleport = active_map.is_some_and(|key| key.map_id == new_map);
        if same_map_near_teleport {
            if !self.set_represented_far_teleport_pending_like_cpp(false) {
                return;
            }
            self.initiate_same_map_near_teleport_like_cpp(new_map, new_pos, options);
            return;
        }

        if let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
            self.player_cannot_enter_target_map_like_cpp(new_map)
        {
            self.send_transfer_aborted_with_params_like_cpp(
                new_map,
                transfer_abort,
                arg,
                map_difficulty_x_condition_id,
            );
            return;
        }

        options = self.teleport_options_after_seamless_gate_like_cpp(new_map, options);
        if active_map.is_none() {
            options &= !TELE_TO_SEAMLESS_LIKE_CPP;
        }

        if !same_map_near_teleport {
            let Some(can_delay) = self
                .player_teleport_state_snapshot_like_cpp()
                .map(|state| state.can_delay)
            else {
                return;
            };
            if !self.update_player_teleport_state_like_cpp(|state| {
                state.has_delayed = can_delay;
                state.near_pending = false;
                state.near_destination = None;
                state.near_destination_zone_area = None;
                if can_delay {
                    state.far_pending = true;
                    state.delayed = Some((new_map, new_pos, options));
                }
            }) {
                return;
            }
            if can_delay {
                return;
            }

            self.set_selection_guid_like_cpp(None);
            self.combat_stop_like_cpp();
            self.reset_contested_pvp_like_cpp();
            self.maybe_leave_represented_battleground_on_far_teleport_like_cpp(new_map);
            self.unsummon_represented_pet_temporary_if_any_like_cpp();
            let _ = self.remove_all_dynamic_objects_for_current_player_like_cpp();
            let _ = self.remove_all_area_triggers_for_current_player_like_cpp();
            if options & TELE_TO_SPELL_LIKE_CPP == 0 {
                let _ = self.interrupt_non_melee_spells_for_far_teleport_like_cpp();
            }
            let _ = self.remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp();
        }

        info!(
            account = self.account_id,
            old_map = self.player_map_id_like_cpp(),
            new_map = new_map,
            old_pos = format!(
                "({:.2}, {:.2}, {:.2})",
                current_pos.x, current_pos.y, current_pos.z
            ),
            new_pos = format!("({:.2}, {:.2}, {:.2})", new_pos.x, new_pos.y, new_pos.z),
            "Player teleporting to new map"
        );

        use wow_packet::packets::misc::{SuspendToken, TransferPending};

        // 1. SMSG_TRANSFER_PENDING — tell client to start loading screen
        if !self.player_logout_like_cpp && options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
            let transfer_pending = TransferPending {
                map_id: new_map,
                old_map_position: current_pos,
                ship: None,
                transfer_spell_id: None,
            };
            self.send_packet_realm(&transfer_pending);
            self.clear_active_player_transport_server_time_override_for_far_teleport_like_cpp();
        }

        let _ = self.remove_current_player_from_canonical_current_map_like_cpp();

        // 2. Store pending destination — completed in handle_world_port_response
        if !self.set_pending_teleport_like_cpp(Some((new_map, new_pos))) {
            return;
        }
        self.active_area_trigger = None;

        // Retain native completion authority before an interruptible writer wait.
        if !self.set_represented_far_teleport_pending_like_cpp(true) {
            return;
        }
        self.state = SessionState::Transfer;

        // 3. SMSG_SUSPEND_TOKEN — pause movement processing on client. C++
        // Player::TeleportTo sets SequenceIndex = m_movementCounter WITHOUT incrementing
        // (Player.cpp:1466), and HandleMoveWorldportAck's ResumeToken uses the SAME counter
        // (the before-add reset happens only after ResumeToken). The client pairs suspend and
        // resume by this index, so they MUST match — a hardcoded 1 here vs the real counter in
        // ResumeToken left the client stuck on the loading screen. #NEXT.R8.ENTITIES.1229.
        if !self.player_logout_like_cpp {
            if options & TELE_TO_SEAMLESS_LIKE_CPP == 0
                && !self
                    .wait_for_realm_send_before_instance_update_like_cpp()
                    .await
            {
                self.kick("TransferPending writer fence failed; retain native destination");
                return;
            }
            let Some(suspend_seq) = self.movement_counter_like_cpp() else {
                return;
            };
            self.send_packet(&SuspendToken {
                sequence_index: suspend_seq,
                reason: if options & TELE_TO_SEAMLESS_LIKE_CPP != 0 {
                    2
                } else {
                    1
                },
            });
        }

        info!(
            account = self.account_id,
            "Teleport initiated: map {} → {} dest ({:.2}, {:.2}, {:.2}); awaiting WorldPortResponse",
            self.player_map_id_like_cpp(),
            new_map,
            new_pos.x,
            new_pos.y,
            new_pos.z
        );
    }
    fn exit_represented_vehicle_for_teleport_like_cpp(&mut self) -> bool {
        if self
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return false;
        }

        if !self.set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
    fn reset_teleport_movement_state_like_cpp(&mut self) {
        let Some(movement_flags) = self.resolved_player_movement_flags_like_cpp() else {
            return;
        };
        let movement_flags = movement_flags & MovementFlag::MASK_HAS_PLAYER_STATUS_OPCODE;
        self.set_player_movement_flags_like_cpp(movement_flags);
        #[cfg(test)]
        {
            self.player_movement_jump_like_cpp = wow_packet::packets::movement::JumpInfo::default();
        }
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            let motion = &mut player.unit_mut().subsystems_mut().motion;
            motion.interrupt_spline();
            let _ =
                motion.remove_generator_kind(MovementGeneratorKind::Effect, MovementSlot::Active);
        });
    }
    fn teleport_options_after_seamless_gate_like_cpp(
        &self,
        new_map: u32,
        mut options: TeleportToOptionsLikeCpp,
    ) -> TeleportToOptionsLikeCpp {
        if options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
            return options;
        }

        let Some(map_store) = self.maps.store.as_ref() else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };
        let Some(old_map_entry) = map_store
            .get(u32::from(self.player_map_id_like_cpp()))
            .copied()
        else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };
        let Some(new_map_entry) = map_store.get(new_map).copied() else {
            return options & !TELE_TO_SEAMLESS_LIKE_CPP;
        };

        let old_cosmetic_parent = i32::from(old_map_entry.cosmetic_parent_map_id);
        let new_cosmetic_parent = i32::from(new_map_entry.cosmetic_parent_map_id);
        let current_map_id = i32::from(self.player_map_id_like_cpp());
        let new_map_id = i32::try_from(new_map).unwrap_or(i32::MAX);
        if old_cosmetic_parent != new_map_id
            && current_map_id != new_cosmetic_parent
            && !((old_cosmetic_parent != -1) ^ (old_cosmetic_parent != new_cosmetic_parent))
        {
            options &= !TELE_TO_SEAMLESS_LIKE_CPP;
        }

        options
    }
    fn initiate_same_map_near_teleport_like_cpp(
        &mut self,
        map_id: u32,
        destination: wow_core::Position,
        options: TeleportToOptionsLikeCpp,
    ) {
        let Some(can_delay) = self
            .player_teleport_state_snapshot_like_cpp()
            .map(|state| state.can_delay)
        else {
            return;
        };
        if !self.update_player_teleport_state_like_cpp(|state| {
            state.has_delayed = can_delay;
        }) {
            return;
        }
        if can_delay {
            let map_id_u16 = u16::try_from(map_id).unwrap_or(self.player_map_id_like_cpp());
            let _ = self.update_player_teleport_state_like_cpp(|state| {
                state.near_pending = true;
                state.near_destination = Some((map_id_u16, destination));
                state.near_destination_zone_area = None;
                state.delayed = Some((map_id, destination, options));
            });
            return;
        }

        self.unsummon_represented_pet_for_same_map_teleport_if_out_of_range_like_cpp(
            destination,
            options,
        );

        if self.resolved_player_is_alive_like_cpp() == Some(false)
            && options & TELE_REVIVE_AT_TELEPORT_LIKE_CPP != 0
        {
            self.resurrect_player_percent_for_teleport_like_cpp(0.5);
        }

        if options & TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP == 0 {
            self.combat_stop_like_cpp();
        }

        let map_id = u16::try_from(map_id).unwrap_or(self.player_map_id_like_cpp());
        if !self.update_player_teleport_state_like_cpp(|state| {
            state.far_destination = None;
            state.near_pending = true;
            state.near_destination = Some((map_id, destination));
            state.near_destination_zone_area = None;
        }) {
            return;
        }
        if let Some(current_pos) = self.player_position_like_cpp() {
            self.set_fall_information_like_cpp(0, current_pos.z);
        }

        if !self.player_logout_like_cpp {
            let Some(sequence_index) = self.next_movement_counter_like_cpp() else {
                return;
            };
            if let Some(mover_guid) = self.player_guid() {
                self.send_same_map_move_update_teleport_to_visible_set_like_cpp(mover_guid);
                self.send_packet(&wow_packet::packets::movement::MoveTeleport {
                    mover_guid,
                    position: destination,
                    facing: destination.orientation,
                    sequence_index,
                    preload_world: 0,
                    transport_guid: None,
                });
            }
        }

        info!(
            account = self.account_id,
            map_id,
            new_pos = format!(
                "({:.2}, {:.2}, {:.2})",
                destination.x, destination.y, destination.z
            ),
            "Player same-map near teleport initiated; awaiting MoveTeleportAck"
        );
    }
    pub(in crate::session) async fn process_represented_delayed_teleport_after_update_like_cpp(
        &mut self,
    ) -> bool {
        let Some(teleport) = self.player_teleport_state_snapshot_like_cpp() else {
            return false;
        };
        if !teleport.has_delayed || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return false;
        }

        let Some((map_id, destination, options)) = teleport.delayed else {
            let _ = self.update_player_teleport_state_like_cpp(|state| {
                state.has_delayed = false;
            });
            return false;
        };

        if !self.update_player_teleport_state_like_cpp(|state| {
            state.delayed = None;
            state.can_delay = false;
            state.has_delayed = false;
        }) {
            return false;
        }
        if self
            .current_canonical_player_map_key_like_cpp()
            .is_some_and(|key| key.map_id == map_id)
        {
            self.initiate_same_map_near_teleport_like_cpp(map_id, destination, options);
        } else {
            self.initiate_far_teleport_after_delay_like_cpp(map_id, destination, options)
                .await;
        }
        true
    }
    fn resurrect_player_percent_for_teleport_like_cpp(&mut self, restore_percent: f32) {
        let restored = self.with_owned_player_mut_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let health = ((max_health as f32) * restore_percent)
                .max(0.0)
                .min(max_health as f32) as u32;
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_health(u64::from(health));
            let mana = ((player.get_max_power(PowerType::Mana).max(0) as f32) * restore_percent)
                .max(0.0) as i32;
            let energy = ((player.get_max_power(PowerType::Energy).max(0) as f32) * restore_percent)
                .max(0.0) as i32;
            let focus = ((player.get_max_power(PowerType::Focus).max(0) as f32) * restore_percent)
                .max(0.0) as i32;
            player.unit_mut().set_power(PowerType::Mana, mana);
            player.unit_mut().set_power(PowerType::Rage, 0);
            player.unit_mut().set_power(PowerType::Energy, energy);
            player.unit_mut().set_power(PowerType::Focus, focus);
            health
        });
        if restored.is_none() {
            return;
        }
        self.sync_player_registry_state_like_cpp();
    }
    fn send_same_map_move_update_teleport_to_visible_set_like_cpp(&self, source_guid: ObjectGuid) {
        use wow_packet::ServerPacket;

        let Some(registry) = self.player_registry() else {
            return;
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(status) = self.current_player_movement_info_like_cpp(source_guid) else {
            return;
        };
        let packet_bytes = wow_packet::packets::movement::MoveUpdateTeleport { status }.to_bytes();

        for registration in registry.movement_recipients_within_range(
            source_guid,
            map_id,
            instance_id,
            source_position,
            crate::map_manager::VISIBILITY_RADIUS,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                crate::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                    crate::session::mailbox::SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid,
                        map_id,
                        instance_id,
                        packet_bytes: packet_bytes.clone(),
                    },
                ),
            );
        }
    }
    pub(crate) fn schedule_represented_resurrection_after_teleport_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = Some(request);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.represented_delayed_resurrection_after_teleport_like_cpp = Some(request);
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn process_represented_delayed_resurrection_after_teleport_like_cpp(&mut self) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport
                    .take()
            })
            .flatten();
        #[cfg(test)]
        let request = canonical.or_else(|| {
            (self.player_handle_like_cpp.is_none())
                .then(|| {
                    self.represented_delayed_resurrection_after_teleport_like_cpp
                        .take()
                })
                .flatten()
        });
        #[cfg(not(test))]
        let request = canonical;
        let Some(request) = request else {
            return;
        };

        self.apply_represented_resurrection_health_like_cpp(request.health);
    }
    #[cfg(test)]
    pub(crate) fn represented_delayed_resurrection_after_teleport_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.delayed_after_teleport)
    }
    pub(crate) fn record_move_teleport_ack_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
    ) -> bool {
        let accepted = self.player_guid() == Some(mover_guid);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveTeleportAck,
            mover_guid,
            ack_index: Some(ack_index),
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time: (move_time >= 0).then_some(move_time as u32),
            speed: None,
            time_skipped: None,
            spline_id: None,
            accepted,
        });
        accepted
    }
    pub(crate) fn handle_move_teleport_ack_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
    ) -> MoveTeleportAckActionLikeCpp {
        let accepted = self.record_move_teleport_ack_like_cpp(mover_guid, ack_index, move_time);
        let Some(teleport) = self.player_teleport_state_snapshot_like_cpp() else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };
        if !teleport.near_pending {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::NotBeingTeleportedNear,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        }

        if !accepted {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::WrongMover,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        }

        let Some((map_id, destination)) = teleport.near_destination else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingDestination,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };

        let Some(world_local_before) = self.player_world_local_state_like_cpp() else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };
        let Some(player_guid) = self.player_guid() else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };
        let Some(pvp_enabled_before) = self.player_is_pvp_like_cpp(player_guid) else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };
        let Some(in_pvp_before) = self.player_has_in_pvp_flag_like_cpp(player_guid) else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        };

        if !self.update_player_teleport_state_like_cpp(|state| state.near_pending = false) {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                Some(map_id),
                Some(destination),
                None,
                None,
                None,
                false,
                false,
                false,
                false,
            );
        }
        let old_zone = world_local_before.zone_id_like_cpp();
        self.set_player_map_position_like_cpp(map_id, destination);
        self.update_registry_position();
        self.set_fall_information_like_cpp(0, destination.z);

        let (new_zone, new_area) = teleport
            .near_destination_zone_area
            .unwrap_or(world_local_before.zone_area_like_cpp());
        self.update_zone_represented_like_cpp(new_zone, new_area);

        let zone_changed = old_zone != new_zone;
        let Some(world_local_after) = self.player_world_local_state_like_cpp() else {
            return self.record_move_teleport_ack_event_like_cpp(
                mover_guid,
                ack_index,
                move_time,
                MoveTeleportAckActionLikeCpp::MissingPlayerOwner,
                Some(map_id),
                Some(destination),
                Some(old_zone),
                Some(new_zone),
                Some(new_area),
                false,
                false,
                false,
                false,
            );
        };
        let honorless_target_cast = zone_changed && world_local_after.is_pvp_hostile_like_cpp();
        let pvp_disabled = zone_changed
            && !world_local_after.is_pvp_hostile_like_cpp()
            && pvp_enabled_before
            && !in_pvp_before;
        if pvp_disabled {
            self.update_player_pvp_like_cpp(false, true);
        }

        self.resummon_pet_temporary_unsummoned_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();
        #[cfg(test)]
        {
            self.delayed_operations_processed_like_cpp =
                self.delayed_operations_processed_like_cpp.saturating_add(1);
        }

        self.record_move_teleport_ack_event_like_cpp(
            mover_guid,
            ack_index,
            move_time,
            MoveTeleportAckActionLikeCpp::Accepted,
            Some(map_id),
            Some(destination),
            Some(old_zone),
            Some(new_zone),
            Some(new_area),
            honorless_target_cast,
            pvp_disabled,
            true,
            true,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn record_move_teleport_ack_event_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
        action: MoveTeleportAckActionLikeCpp,
        destination_map_id: Option<u16>,
        destination_position: Option<wow_core::Position>,
        old_zone_id: Option<u32>,
        new_zone_id: Option<u32>,
        new_area_id: Option<u32>,
        honorless_target_cast: bool,
        pvp_disabled: bool,
        pet_resummon_requested: bool,
        delayed_operations_processed: bool,
    ) -> MoveTeleportAckActionLikeCpp {
        #[cfg(test)]
        self.move_teleport_ack_events_like_cpp
            .push(MoveTeleportAckEventLikeCpp {
                mover_guid,
                ack_index,
                move_time,
                action,
                destination_map_id,
                destination_position,
                old_zone_id,
                new_zone_id,
                new_area_id,
                honorless_target_cast,
                pvp_disabled,
                pet_resummon_requested,
                delayed_operations_processed,
            });
        #[cfg(not(test))]
        let _ = (
            mover_guid,
            ack_index,
            move_time,
            destination_map_id,
            destination_position,
            old_zone_id,
            new_zone_id,
            new_area_id,
            honorless_target_cast,
            pvp_disabled,
            pet_resummon_requested,
            delayed_operations_processed,
        );
        action
    }
    #[cfg(test)]
    pub(crate) fn set_near_teleport_pending_like_cpp(
        &mut self,
        pending: bool,
        destination: Option<(u16, wow_core::Position)>,
        zone_area: Option<(u32, u32)>,
    ) -> bool {
        self.update_player_teleport_state_like_cpp(|state| {
            state.near_pending = pending;
            state.near_destination = destination;
            state.near_destination_zone_area = zone_area;
        })
    }
    pub(in crate::session) fn player_teleport_state_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerTeleportStateLikeCpp> {
        let canonical = self.with_owned_player_like_cpp(|player| *player.teleport_state_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(PlayerTeleportStateLikeCpp {
                recovery: Default::default(),
                far_destination: self.pending_teleport,
                post_add: None,
                can_delay: self.represented_can_delay_teleport_like_cpp,
                has_delayed: self.represented_has_delayed_teleport_like_cpp,
                near_pending: self.near_teleport_pending_like_cpp,
                far_pending: self.represented_far_teleport_pending_like_cpp,
                near_destination: self.near_teleport_destination_like_cpp,
                delayed: self.represented_delayed_teleport_like_cpp,
                near_destination_zone_area: self.near_teleport_destination_zone_area_like_cpp,
            });
        }
        canonical
    }
    pub(in crate::session) fn update_player_teleport_state_like_cpp(
        &mut self,
        update: impl FnOnce(&mut PlayerTeleportStateLikeCpp),
    ) -> bool {
        if self.player_handle_like_cpp.is_some() {
            return self
                .with_owned_player_mut_like_cpp(|player| {
                    update(player.teleport_state_mut_like_cpp())
                })
                .is_some();
        }
        #[cfg(test)]
        {
            let mut state = self
                .player_teleport_state_snapshot_like_cpp()
                .unwrap_or_default();
            update(&mut state);
            self.pending_teleport = state.far_destination;
            self.represented_can_delay_teleport_like_cpp = state.can_delay;
            self.represented_has_delayed_teleport_like_cpp = state.has_delayed;
            self.near_teleport_pending_like_cpp = state.near_pending;
            self.represented_far_teleport_pending_like_cpp = state.far_pending;
            self.near_teleport_destination_like_cpp = state.near_destination;
            self.represented_delayed_teleport_like_cpp = state.delayed;
            self.near_teleport_destination_zone_area_like_cpp = state.near_destination_zone_area;
            true
        }
        #[cfg(not(test))]
        {
            let _ = update;
            false
        }
    }
    pub(crate) fn set_represented_can_delay_teleport_like_cpp(&mut self, can_delay: bool) -> bool {
        self.update_player_teleport_state_like_cpp(|state| state.can_delay = can_delay)
    }
    pub(crate) fn represented_can_delay_teleport_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.can_delay)
    }
    #[cfg(test)]
    pub(crate) fn represented_has_delayed_teleport_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.has_delayed)
    }
    #[cfg(test)]
    pub(crate) fn represented_delayed_teleport_like_cpp(
        &self,
    ) -> Option<(u32, wow_core::Position, TeleportToOptionsLikeCpp)> {
        self.player_teleport_state_snapshot_like_cpp()
            .and_then(|state| state.delayed)
    }
    pub(crate) fn near_teleport_pending_like_cpp(&self) -> bool {
        self.player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.near_pending)
    }
    #[cfg(test)]
    pub(crate) fn move_teleport_ack_events_like_cpp(&self) -> &[MoveTeleportAckEventLikeCpp] {
        &self.move_teleport_ack_events_like_cpp
    }
}
