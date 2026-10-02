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
        if let Some(teleport) =
            crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
            && teleport.near_pending
        {
            return teleport.near_destination;
        }
        None
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
        if crate::session::hub_ref(self)
            .player_teleport_state_snapshot_like_cpp()
            .is_some_and(|state| state.recovery == wow_entities::PlayerTransferRecovery::Terminal)
        {
            return;
        }
        // C++ Player.cpp:1239 validates the catalog map and all four coordinates
        // before movement, combat, pet, ownership or teleport state is changed.
        if self
            .catalogs
            .maps
            .store
            .as_ref()
            .is_none_or(|maps| maps.get(new_map).is_none())
            || !new_pos.is_valid_map_coord_like_cpp()
        {
            warn!(
                "Invalid map or coordinates for teleport to {} from account {}",
                new_map, self.core.account_id
            );
            return;
        }

        if self.is_map_disabled_for_player_like_cpp(new_map) {
            warn!(
                account = self.core.account_id,
                map_id = new_map,
                "Teleport blocked by C++ DisableMgr map gate"
            );
            crate::session::hub_ref(self)
                .send_transfer_aborted_like_cpp(new_map, TRANSFER_ABORT_MAP_NOT_ALLOWED_LIKE_CPP);
            return;
        }

        if let Some(target_map) = self
            .catalogs
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(new_map).copied())
            && target_map.is_battleground_or_arena()
            && !crate::session::hub_ref(self).player_in_represented_battleground_like_cpp()
        {
            warn!(
                account = self.core.account_id,
                map_id = new_map,
                "Teleport silently blocked by C++ battleground assignment gate"
            );
            return;
        }

        if let Some(target_map) = self
            .catalogs
            .maps
            .store
            .as_ref()
            .and_then(|store| store.get(new_map).copied())
            && self.core.expansion < target_map.expansion_like_cpp()
        {
            warn!(
                account = self.core.account_id,
                map_id = new_map,
                session_expansion = self.core.expansion,
                required_expansion = target_map.expansion_like_cpp(),
                "Teleport blocked by C++ client expansion gate"
            );
            crate::session::hub_ref(self).send_transfer_aborted_with_params_like_cpp(
                new_map,
                TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP,
                target_map.expansion_like_cpp(),
                0,
            );
            return;
        }

        let Some(current_pos) = crate::session::hub_ref(self).player_position_like_cpp() else {
            warn!(
                "Cannot teleport account {}: no current position",
                self.core.account_id
            );
            return;
        };

        self.exit_represented_vehicle_for_teleport_like_cpp();
        crate::session::hub_mut(self).reset_teleport_movement_state_like_cpp();

        if crate::session::hub_ref(self).player_class_like_cpp() == CLASS_DEATH_KNIGHT_LIKE_CPP
            && self.core.player_map_id_like_cpp() == DEATH_KNIGHT_START_MAP_LIKE_CPP
            && u32::from(self.core.player_map_id_like_cpp()) != new_map
            && crate::session::hub_ref(self).player_is_game_master_like_cpp() != Some(true)
            && !self
                .known_spells_like_cpp()
                .contains(&DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP)
        {
            crate::session::hub_ref(self).send_transfer_aborted_with_params_like_cpp(
                new_map,
                TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
                1,
                0,
            );
            return;
        }

        // Approved recovery exception: a detached Player needs AddPlayerToMap,
        // even when returning to its old map. A near ACK only relocates it.
        let active_map = self.core.current_canonical_player_map_key_like_cpp();
        let same_map_near_teleport = active_map.is_some_and(|key| key.map_id == new_map);
        if same_map_near_teleport {
            if !crate::session::hub_mut(self).set_represented_far_teleport_pending_like_cpp(false) {
                return;
            }
            self.initiate_same_map_near_teleport_like_cpp(new_map, new_pos, options);
            return;
        }

        if let Some((transfer_abort, arg, map_difficulty_x_condition_id)) =
            self.player_cannot_enter_target_map_like_cpp(new_map)
        {
            crate::session::hub_ref(self).send_transfer_aborted_with_params_like_cpp(
                new_map,
                transfer_abort,
                arg,
                map_difficulty_x_condition_id,
            );
            return;
        }

        options = crate::session::hub_ref(self)
            .teleport_options_after_seamless_gate_like_cpp(new_map, options);
        if active_map.is_none() {
            options &= !TELE_TO_SEAMLESS_LIKE_CPP;
        }

        if !same_map_near_teleport {
            let Some(can_delay) = crate::session::hub_ref(self)
                .player_teleport_state_snapshot_like_cpp()
                .map(|state| state.can_delay)
            else {
                return;
            };
            if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
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

            crate::session::hub_mut(self).set_selection_guid_like_cpp(None);
            self.combat_stop_like_cpp();
            self.reset_contested_pvp_like_cpp();
            self.maybe_leave_represented_battleground_on_far_teleport_like_cpp(new_map);
            crate::session::hub_mut(self).unsummon_represented_pet_temporary_if_any_like_cpp();
            let _ = crate::session::hub_ref(self)
                .remove_all_dynamic_objects_for_current_player_like_cpp();
            let _ = crate::session::hub_ref(self)
                .remove_all_area_triggers_for_current_player_like_cpp();
            if options & TELE_TO_SPELL_LIKE_CPP == 0 {
                let _ = self.interrupt_non_melee_spells_for_far_teleport_like_cpp();
            }
            let _ = self.remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp();
        }

        info!(
            account = self.core.account_id,
            old_map = self.core.player_map_id_like_cpp(),
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
        if !self.lifecycle.player_logout_like_cpp && options & TELE_TO_SEAMLESS_LIKE_CPP == 0 {
            let transfer_pending = TransferPending {
                map_id: new_map,
                old_map_position: current_pos,
                ship: None,
                transfer_spell_id: None,
            };
            self.send_packet_realm(&transfer_pending);
            self.clear_active_player_transport_server_time_override_for_far_teleport_like_cpp();
        }

        let _ = crate::session::hub_mut(self)
            .remove_current_player_from_canonical_current_map_like_cpp();

        // 2. Store pending destination — completed in handle_world_port_response
        if !self.set_pending_teleport_like_cpp(Some((new_map, new_pos))) {
            return;
        }
        self.view.active_area_trigger = None;

        // Retain native completion authority before an interruptible writer wait.
        if !crate::session::hub_mut(self).set_represented_far_teleport_pending_like_cpp(true) {
            return;
        }
        self.core.state = SessionState::Transfer;

        // 3. SMSG_SUSPEND_TOKEN — pause movement processing on client. C++
        // Player::TeleportTo sets SequenceIndex = m_movementCounter WITHOUT incrementing
        // (Player.cpp:1466), and HandleMoveWorldportAck's ResumeToken uses the SAME counter
        // (the before-add reset happens only after ResumeToken). The client pairs suspend and
        // resume by this index, so they MUST match — a hardcoded 1 here vs the real counter in
        // ResumeToken left the client stuck on the loading screen. #NEXT.R8.ENTITIES.1229.
        if !self.lifecycle.player_logout_like_cpp {
            if options & TELE_TO_SEAMLESS_LIKE_CPP == 0
                && !self
                    .core
                    .wait_for_realm_send_before_instance_update_like_cpp()
                    .await
            {
                self.kick("TransferPending writer fence failed; retain native destination");
                return;
            }
            let Some(suspend_seq) = crate::session::hub_ref(self).movement_counter_like_cpp()
            else {
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
            account = self.core.account_id,
            "Teleport initiated: map {} → {} dest ({:.2}, {:.2}, {:.2}); awaiting WorldPortResponse",
            self.core.player_map_id_like_cpp(),
            new_map,
            new_pos.x,
            new_pos.y,
            new_pos.z
        );
    }
    fn exit_represented_vehicle_for_teleport_like_cpp(&mut self) -> bool {
        if crate::session::hub_ref(self)
            .player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_none()
        {
            return false;
        }

        if !crate::session::hub_mut(self).set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
    fn initiate_same_map_near_teleport_like_cpp(
        &mut self,
        map_id: u32,
        destination: wow_core::Position,
        options: TeleportToOptionsLikeCpp,
    ) {
        let Some(can_delay) = crate::session::hub_ref(self)
            .player_teleport_state_snapshot_like_cpp()
            .map(|state| state.can_delay)
        else {
            return;
        };
        if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
            state.has_delayed = can_delay;
        }) {
            return;
        }
        if can_delay {
            let map_id_u16 = u16::try_from(map_id).unwrap_or(self.core.player_map_id_like_cpp());
            let _ = crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
                state.near_pending = true;
                state.near_destination = Some((map_id_u16, destination));
                state.near_destination_zone_area = None;
                state.delayed = Some((map_id, destination, options));
            });
            return;
        }

        crate::session::hub_mut(self)
            .unsummon_represented_pet_for_same_map_teleport_if_out_of_range_like_cpp(
                destination,
                options,
            );

        if crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() == Some(false)
            && options & TELE_REVIVE_AT_TELEPORT_LIKE_CPP != 0
        {
            self.resurrect_player_percent_for_teleport_like_cpp(0.5);
        }

        if options & TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP == 0 {
            self.combat_stop_like_cpp();
        }

        let map_id = u16::try_from(map_id).unwrap_or(self.core.player_map_id_like_cpp());
        if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
            state.far_destination = None;
            state.near_pending = true;
            state.near_destination = Some((map_id, destination));
            state.near_destination_zone_area = None;
        }) {
            return;
        }
        if let Some(current_pos) = crate::session::hub_ref(self).player_position_like_cpp() {
            crate::session::hub_mut(self).set_fall_information_like_cpp(0, current_pos.z);
        }

        if !self.lifecycle.player_logout_like_cpp {
            let Some(sequence_index) =
                crate::session::hub_mut(self).next_movement_counter_like_cpp()
            else {
                return;
            };
            if let Some(mover_guid) = self.player_guid() {
                crate::session::hub_ref(self)
                    .send_same_map_move_update_teleport_to_visible_set_like_cpp(mover_guid);
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
            account = self.core.account_id,
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
        let Some(teleport) =
            crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
        else {
            return false;
        };
        if !teleport.has_delayed
            || crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() != Some(true)
        {
            return false;
        }

        let Some((map_id, destination, options)) = teleport.delayed else {
            let _ = crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
                state.has_delayed = false;
            });
            return false;
        };

        if !crate::session::hub_mut(self).update_player_teleport_state_like_cpp(|state| {
            state.delayed = None;
            state.can_delay = false;
            state.has_delayed = false;
        }) {
            return false;
        }
        if self
            .core
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
        let restored = self.core.with_owned_player_mut_like_cpp(|player| {
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
    pub(crate) fn schedule_represented_resurrection_after_teleport_like_cpp(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = Some(request);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .combat
                .represented_delayed_resurrection_after_teleport_like_cpp = Some(request);
        }
        canonical || cfg!(test) && self.core.player_handle_like_cpp.is_none()
    }
    pub(crate) fn process_represented_delayed_resurrection_after_teleport_like_cpp(&mut self) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport
                    .take()
            })
            .flatten();
        #[cfg(test)]
        let request = canonical.or_else(|| {
            (self.core.player_handle_like_cpp.is_none())
                .then(|| {
                    self.fixtures
                        .combat
                        .represented_delayed_resurrection_after_teleport_like_cpp
                        .take()
                })
                .flatten()
        });
        #[cfg(not(test))]
        let request = canonical;
        let Some(request) = request else {
            return;
        };

        crate::session::hub_mut(self)
            .apply_represented_resurrection_health_like_cpp(request.health);
    }
    #[cfg(test)]
    pub(crate) fn represented_delayed_resurrection_after_teleport_like_cpp(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.delayed_after_teleport)
    }
    pub(crate) fn handle_move_teleport_ack_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        ack_index: i32,
        move_time: i32,
    ) -> MoveTeleportAckActionLikeCpp {
        let accepted = crate::session::hub_mut(self)
            .record_move_teleport_ack_like_cpp(mover_guid, ack_index, move_time);
        let Some(teleport) =
            crate::session::hub_ref(self).player_teleport_state_snapshot_like_cpp()
        else {
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

        let Some(world_local_before) =
            crate::session::hub_ref(self).player_world_local_state_like_cpp()
        else {
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
        let Some(pvp_enabled_before) =
            crate::session::hub_ref(self).player_is_pvp_like_cpp(player_guid)
        else {
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
        let Some(in_pvp_before) =
            crate::session::hub_ref(self).player_has_in_pvp_flag_like_cpp(player_guid)
        else {
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

        if !crate::session::hub_mut(self)
            .update_player_teleport_state_like_cpp(|state| state.near_pending = false)
        {
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
        crate::session::hub_mut(self).set_player_map_position_like_cpp(map_id, destination);
        crate::session::hub_ref(self).update_registry_position();
        crate::session::hub_mut(self).set_fall_information_like_cpp(0, destination.z);

        let (new_zone, new_area) = teleport
            .near_destination_zone_area
            .unwrap_or(world_local_before.zone_area_like_cpp());
        self.update_zone_represented_like_cpp(new_zone, new_area);

        let zone_changed = old_zone != new_zone;
        let Some(world_local_after) =
            crate::session::hub_ref(self).player_world_local_state_like_cpp()
        else {
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
            crate::session::hub_mut(self).update_player_pvp_like_cpp(false, true);
        }

        crate::session::cx_pets(self).resummon_pet_temporary_unsummoned_like_cpp();
        self.process_represented_delayed_resurrection_after_teleport_like_cpp();
        #[cfg(test)]
        {
            self.fixtures.movement.delayed_operations_processed_like_cpp = self
                .fixtures
                .movement
                .delayed_operations_processed_like_cpp
                .saturating_add(1);
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
        self.fixtures
            .teleport
            .move_teleport_ack_events_like_cpp
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
    pub(crate) fn move_teleport_ack_events_like_cpp(&self) -> &[MoveTeleportAckEventLikeCpp] {
        &self.fixtures.teleport.move_teleport_ack_events_like_cpp
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/transfer/f3_shims.rs"]
mod f3_shims;
