//! Represented player movement state at the Session boundary.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn remove_current_player_from_canonical_current_map_like_cpp(&mut self) -> bool {
        let Some(guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let handle = match self.player_handle_like_cpp {
            Some(handle) if handle.guid() == guid => handle,
            Some(_) => return false,
            None => {
                // Adopt a unique legacy record before removing it. Discarding
                // RemoveFromMap's returned Box would destroy the live Player.
                let Ok(handle) = manager.adopt_active_player_like_cpp(guid) else {
                    return false;
                };
                self.player_handle_like_cpp = Some(handle);
                handle
            }
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager.detach_player_like_cpp(handle).is_ok()
            }
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => true,
            None => false,
        }
    }
    pub(crate) fn represented_move_dismiss_vehicle_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
    ) -> bool {
        let vehicle_guid = self.represented_player_charmed_guid_like_cpp();
        if vehicle_guid.is_empty() || self.player_vehicle_seat_state_like_cpp().is_none() {
            return false;
        }

        self.sanitize_movement_info_represented_like_cpp(status);
        self.set_player_movement_time_like_cpp(status.time);
        self.set_player_movement_flags_like_cpp(status.flags);
        self.set_player_position_like_cpp(status.position);
        self.update_registry_position();
        #[cfg(test)]
        self.represented_vehicle_dismiss_movements_like_cpp.push(
            RepresentedVehicleDismissMovementLikeCpp {
                vehicle_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            },
        );

        if !self.set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }
    pub(crate) fn remove_currency(&mut self, currency_id: u32, amount: u32) -> bool {
        let Some(mut currencies) = self.player_currencies_like_cpp() else {
            return false;
        };
        if !crate::session_rules::plan_remove_currency_like_cpp(
            &mut currencies,
            currency_id,
            amount,
        ) {
            return false;
        }
        self.set_player_currencies_like_cpp(currencies)
    }
    pub(crate) fn remove_account_toy_like_cpp(&mut self, item_id: u32) -> bool {
        self.mutate_player_collection_state_like_cpp(|state| state.toys.remove(&item_id).is_some())
            .unwrap_or(false)
    }
    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        self.mutate_player_rest_state_like_cpp(|state| state.remove_flag_like_cpp(rest_flag))
            .unwrap_or(false)
    }
    pub(in crate::session) fn remove_represented_active_talent_side_effects_like_cpp(
        &mut self,
        talent_id: u32,
        rank: u8,
    ) {
        if let Some(spell_id) = self.represented_talent_spell_id_like_cpp(talent_id, rank) {
            self.remove_known_spell_like_cpp(spell_id);
            for trigger_spell in self.represented_direct_learn_spell_triggers_like_cpp(spell_id) {
                self.remove_known_spell_like_cpp(trigger_spell);
            }
        }

        if let Some((overriden_spell_id, new_spell_id)) =
            self.represented_talent_override_spell_pair_like_cpp(talent_id)
        {
            self.remove_represented_override_spell_like_cpp(overriden_spell_id, new_spell_id);
        }
    }
    pub(crate) fn clear_player_emote_state_on_movement_like_cpp(
        &mut self,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        if self.resolved_player_emote_state_like_cpp() == Some(0) {
            return None;
        }

        self.set_player_emote_state_like_cpp(0)
    }
    /// Remove a GUID from the legit characters list.
    pub fn remove_legit_character(&mut self, guid: &ObjectGuid) {
        self.legit_characters.retain(|g| g != guid);
    }
    pub(in crate::session) fn current_player_movement_info_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<wow_packet::packets::movement::MovementInfo> {
        Some(wow_packet::packets::movement::MovementInfo {
            guid: player_guid,
            position: self.player_position_like_cpp()?,
            flags: self.resolved_player_movement_flags_like_cpp()?,
            flags2: if self.resolved_can_swim_to_fly_transition_like_cpp()? {
                wow_constants::movement::MovementFlag2::CAN_SWIM_TO_FLY_TRANS
            } else {
                wow_constants::movement::MovementFlag2::NONE
            },
            time: self.resolved_player_movement_time_like_cpp()?,
            ..wow_packet::packets::movement::MovementInfo::default()
        })
    }
    pub(crate) fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        let has_died_state = self
            .mutate_canonical_player_like_cpp(|player| {
                player.unit().has_unit_state(UnitState::DIED.bits())
            })
            .unwrap_or(false);
        if !has_died_state {
            return false;
        }

        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return false;
        };
        let slots: Vec<u8> = visible_auras
            .iter()
            .filter_map(|(slot, aura)| {
                (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::FeignDeath))
                    .then_some(*slot)
            })
            .collect();
        if slots.is_empty() {
            return false;
        }

        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().clear_unit_state(UnitState::DIED.bits());
        });
        true
    }
    pub(crate) fn adjust_client_movement_time_like_cpp(&self, time: u32) -> u32 {
        let movement_time = i64::from(time) + self.time_sync_clock_delta;
        if self.time_sync_clock_delta == 0 || !(0..=i64::from(u32::MAX)).contains(&movement_time) {
            warn!(
                account = self.account_id,
                client_time = time,
                clock_delta = self.time_sync_clock_delta,
                "The computed movement time using clockDelta is erroneous. Using fallback instead"
            );
            crate::session_rules::game_time_ms_like_cpp()
        } else {
            movement_time as u32
        }
    }
    pub(in crate::session) fn remove_all_dynamic_objects_for_current_player_like_cpp(
        &self,
    ) -> Option<wow_map::map::RemoveAllDynamicObjectsForCasterOutcomeLikeCpp> {
        let player_guid = self.player_guid()?;
        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        let canonical = self.canonical_map_manager.as_ref()?;
        let mut manager = canonical.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(
            managed
                .map_mut()
                .remove_all_dynamic_objects_for_caster_like_cpp(player_guid),
        )
    }
    pub(in crate::session) fn remove_all_area_triggers_for_current_player_like_cpp(
        &self,
    ) -> Option<wow_map::map::RemoveAllAreaTriggersForCasterOutcomeLikeCpp> {
        let player_guid = self.player_guid()?;
        let map_key = self.current_canonical_player_map_key_like_cpp()?;
        let canonical = self.canonical_map_manager.as_ref()?;
        let mut manager = canonical.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        Some(
            managed
                .map_mut()
                .remove_all_area_triggers_for_caster_like_cpp(player_guid),
        )
    }
    pub(crate) fn set_player_map_position_like_cpp(
        &mut self,
        map_id: u16,
        position: wow_core::Position,
    ) {
        if self.player_map_id_like_cpp() != map_id {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.current_map_id = map_id;
        self.sync_canonical_player_position_if_same_or_detached_like_cpp(map_id, position);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none()
            || self.player_position_like_cpp() == Some(position)
        {
            self.player_position = Some(position);
        }
    }
    fn sync_canonical_player_position_if_same_or_detached_like_cpp(
        &mut self,
        map_id: u16,
        position: Position,
    ) {
        let (Some(manager), Some(handle)) = (
            self.canonical_map_manager.as_ref().map(Arc::clone),
            self.player_handle_like_cpp,
        ) else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => {
                let _ = manager.with_player_mut_like_cpp(handle, |player| {
                    player.unit_mut().world_mut().relocate(position);
                });
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(key))
                if key.map_id == u32::from(map_id) =>
            {
                let _ = manager.relocate_player_like_cpp(handle, position);
            }
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) | None => {}
        }
    }
    pub(crate) fn set_player_position_like_cpp(&mut self, position: wow_core::Position) {
        self.set_player_map_position_like_cpp(self.current_map_id, position);
    }
    pub(crate) fn set_player_movement_time_like_cpp(&mut self, time: u32) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_movement_time_like_cpp(time);
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.player_movement_time_like_cpp = time;
        }
    }
    pub(crate) fn set_player_movement_flags_like_cpp(&mut self, flags: MovementFlag) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_movement_flags_like_cpp(flags);
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.player_movement_flags_like_cpp = flags;
        }
    }
    pub(crate) fn set_represented_mover_fixed_position_vehicle_like_cpp(&mut self, fixed: bool) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_mover_fixed_position_vehicle_like_cpp(fixed);
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.player_handle_like_cpp.is_none() {
            self.represented_mover_fixed_position_vehicle_like_cpp = fixed;
        }
    }
    /// C++ `Unit::m_movementCounter` post-increment: returns the current value and advances
    /// it. Used as the SequenceIndex of movement-control packets (vehicle-rec, collision,
    /// near-teleport, speed/flag) and read for `SMSG_RESUME_TOKEN` on far teleport.
    pub(crate) fn next_movement_counter_like_cpp(&mut self) -> Option<u32> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().next_movement_counter_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let sequence_index = self.movement_counter_like_cpp;
            self.movement_counter_like_cpp = self.movement_counter_like_cpp.wrapping_add(1);
            return Some(sequence_index);
        }
        canonical
    }
    /// C++ `Player::SendInitialPacketsBeforeAddToMap` resets `m_movementCounter` to 0 for a
    /// non-seamless add (login / far teleport). Player.cpp:23483.
    pub(crate) fn reset_movement_counter_like_cpp(&mut self) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().reset_movement_counter_like_cpp();
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.movement_counter_like_cpp = 0;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    /// Current `Unit::m_movementCounter` value (read without advancing). C++ reads this for
    /// `SMSG_RESUME_TOKEN.SequenceIndex` on far teleport (MovementHandler.cpp:109), before
    /// `SendInitialPacketsBeforeAddToMap` resets it.
    pub(crate) fn movement_counter_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().movement_counter_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.movement_counter_like_cpp);
        }
        canonical
    }
    pub(crate) fn player_position_like_cpp(&self) -> Option<wow_core::Position> {
        let canonical = self.with_owned_player_like_cpp(|player| player.unit().world().position());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.player_position;
        }
        canonical
    }
    pub(in crate::session) fn resolved_player_movement_flags_like_cpp(
        &self,
    ) -> Option<MovementFlag> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().movement_flags_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_movement_flags_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_movement_flags_like_cpp(&self) -> MovementFlag {
        self.resolved_player_movement_flags_like_cpp()
            .expect("test Player movement owner must resolve")
    }
    pub(in crate::session) fn resolved_mover_fixed_position_vehicle_like_cpp(
        &self,
    ) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .gameplay_state()
                .movement_control
                .mover_fixed_position_vehicle
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_mover_fixed_position_vehicle_like_cpp);
        }
        canonical
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_remove_event_like_cpp(&mut self, event_id: u64) {
        #[cfg(test)]
        self.represented_calendar_remove_events_like_cpp
            .push(RepresentedCalendarRemoveEventLikeCpp { event_id });
    }
    #[cfg(test)]
    pub(crate) fn represented_calendar_remove_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarRemoveEventLikeCpp] {
        &self.represented_calendar_remove_events_like_cpp
    }
    pub(crate) fn player_moved_unit_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.unit().subsystems().control.unit_moved_by_me
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let guid = if self.player_moved_unit_guid_like_cpp.is_empty() {
                self.player_guid()?
            } else {
                self.player_moved_unit_guid_like_cpp
            };
            return Some(guid);
        }
        canonical.flatten()
    }
    pub fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid) {
        #[cfg_attr(not(test), allow(unused_variables))]
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .control
                    .set_moved_unit((!guid.is_empty()).then_some(guid));
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_moved_unit_guid_like_cpp = guid;
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_dismiss_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleDismissMovementLikeCpp] {
        &self.represented_vehicle_dismiss_movements_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_vehicle_base_movements_like_cpp(
        &self,
    ) -> &[RepresentedVehicleBaseMovementLikeCpp] {
        &self.represented_vehicle_base_movements_like_cpp
    }
    pub(crate) fn represented_move_change_vehicle_seats_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        dst_vehicle: ObjectGuid,
        dst_seat_index: u8,
    ) -> bool {
        let Some(vehicle_base_guid) = self.represented_vehicle_base_guid_for_switch_like_cpp()
        else {
            return false;
        };
        if !self.represented_current_vehicle_seat_can_switch_from_like_cpp() {
            return false;
        }

        self.sanitize_movement_info_represented_like_cpp(status);
        if status.guid != vehicle_base_guid {
            return false;
        }

        let dst_seat_id = dst_seat_index as i8;
        let dst_vehicle_exists_with_empty_seat = !dst_vehicle.is_empty()
            && self.represented_vehicle_seat_spell_click_plan_available_like_cpp(
                dst_vehicle,
                dst_seat_id,
            );
        let action = crate::handlers::vehicle::move_change_vehicle_seats_action_like_cpp(
            true,
            true,
            vehicle_base_guid,
            status.guid,
            dst_vehicle,
            dst_seat_index,
            dst_vehicle_exists_with_empty_seat,
        );

        #[cfg(test)]
        self.represented_vehicle_base_movements_like_cpp.push(
            RepresentedVehicleBaseMovementLikeCpp {
                vehicle_guid: vehicle_base_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            },
        );
        let _ = self.record_represented_vehicle_seat_action_like_cpp(action);
        true
    }
    pub(crate) fn sanitize_movement_info_flags_represented_like_cpp(
        &self,
        movement_info: &mut wow_packet::packets::movement::MovementInfo,
    ) -> MovementFlag {
        self.sanitize_movement_info_represented_like_cpp(movement_info)
            .removed_flags
    }
    pub(crate) fn sanitize_movement_info_represented_like_cpp(
        &self,
        movement_info: &mut wow_packet::packets::movement::MovementInfo,
    ) -> wow_anticheat::ValidationResult {
        wow_anticheat::validate_movement_info(
            movement_info,
            &wow_anticheat::PlayerState {
                // Fixed-position is an authorization proof. An unresolved
                // Player must not preserve a client-supplied ROOT flag.
                mover_fixed_position_vehicle: self.resolved_mover_fixed_position_vehicle_like_cpp()
                    == Some(true),
                // An unresolved/stale Player may not authorize client-only
                // movement flags. Treat the proof as absent for sanitization,
                // without claiming that the canonical aura set is empty.
                has_hover_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::Hover,
                ) == Some(true),
                has_water_walk_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::WaterWalk,
                ) == Some(true),
                has_ghost_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::Ghost,
                ) == Some(true),
                has_feather_fall_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::FeatherFall,
                ) == Some(true),
                has_fly_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::Fly,
                ) == Some(true),
                has_mounted_flight_speed_aura: self.resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                ) == Some(true),
                is_player_security: self.player_is_game_master_like_cpp() != Some(true),
            },
        )
    }
    pub(crate) fn apply_move_time_skipped_like_cpp(
        &mut self,
        mover_guid: ObjectGuid,
        time_skipped: u32,
    ) -> bool {
        let adjusted_time = (self.player_guid() == Some(mover_guid))
            .then(|| {
                self.resolved_player_movement_time_like_cpp()
                    .map(|time| time.saturating_add(time_skipped))
            })
            .flatten();
        if let Some(adjusted_time) = adjusted_time {
            self.set_player_movement_time_like_cpp(adjusted_time);
        }
        let accepted = adjusted_time.is_some();

        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
            opcode: ClientOpcodes::MoveTimeSkipped,
            mover_guid,
            ack_index: None,
            movement_force_id: None,
            movement_force_type: None,
            adjusted_time,
            speed: None,
            time_skipped: Some(time_skipped),
            spline_id: None,
            accepted,
        });
        accepted
    }
    pub(crate) fn record_move_spline_done_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        spline_id: i32,
    ) -> bool {
        let accepted = self.validate_and_sanitize_movement_ack_status_represented_like_cpp(status);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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
    pub(crate) fn handle_move_spline_done_taxi_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        spline_id: i32,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        if !self.record_move_spline_done_like_cpp(status, spline_id) {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::InvalidMovement,
                None,
                None,
                None,
                false,
            );
        }

        let Some(taxi_state) = self.player_taxi_state_snapshot_like_cpp() else {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        };
        let current_destination = taxi_state.destinations.get(1).copied();
        if let Some(destination_node_id) = current_destination {
            let Some(flight) = taxi_state.flight else {
                return self.record_move_spline_done_taxi_event_like_cpp(
                    spline_id,
                    MoveSplineDoneTaxiActionLikeCpp::InProgressNoFlightGenerator,
                    Some(destination_node_id),
                    None,
                    None,
                    false,
                );
            };

            let destination_map_id = self
                .taxi_node_map_ids_like_cpp
                .get(&destination_node_id)
                .copied();
            let should_teleport = destination_map_id
                .map(|map_id| map_id != self.player_map_id_like_cpp())
                .unwrap_or(false)
                || flight.current_node.teleport_flag;

            if should_teleport {
                if let (Some(map_id), Some(node)) = (destination_map_id, flight.node_after_teleport)
                {
                    if self
                        .mutate_player_taxi_state_like_cpp(|taxi| {
                            taxi.flight = Some(wow_entities::PlayerTaxiFlightStateLikeCpp {
                                current_node: node,
                                node_after_teleport: None,
                            });
                        })
                        .is_none()
                    {
                        return self.record_move_spline_done_taxi_event_like_cpp(
                            spline_id,
                            MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                            Some(destination_node_id),
                            None,
                            None,
                            false,
                        );
                    }
                    self.set_player_map_position_like_cpp(map_id, node.position);
                    return self.record_move_spline_done_taxi_event_like_cpp(
                        spline_id,
                        MoveSplineDoneTaxiActionLikeCpp::TeleportRequested,
                        Some(destination_node_id),
                        Some(map_id),
                        Some(node.position),
                        false,
                    );
                }
            }

            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::InProgressNoTeleport,
                Some(destination_node_id),
                None,
                None,
                false,
            );
        }

        if taxi_state.destinations.len() != 1 {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        }

        if self
            .mutate_player_taxi_state_like_cpp(|taxi| {
                taxi.destinations.clear();
                taxi.flight = None;
                taxi.mounted = false;
                taxi.unit_flags &= !(UnitFlags::REMOVE_CLIENT_CONTROL | UnitFlags::ON_TAXI).bits();
            })
            .is_none()
        {
            return self.record_move_spline_done_taxi_event_like_cpp(
                spline_id,
                MoveSplineDoneTaxiActionLikeCpp::IgnoredUnexpectedFinalPath,
                None,
                None,
                None,
                false,
            );
        }
        let current_z = self
            .player_position_like_cpp()
            .map(|position| position.z)
            .unwrap_or(status.position.z);
        self.set_fall_information_like_cpp(0, current_z);
        let honorless_target_cast = self
            .player_world_local_state_like_cpp()
            .is_some_and(|state| state.pvp_hostile);

        self.record_move_spline_done_taxi_event_like_cpp(
            spline_id,
            MoveSplineDoneTaxiActionLikeCpp::FinalCleanup,
            None,
            None,
            None,
            honorless_target_cast,
        )
    }
    fn record_move_spline_done_taxi_event_like_cpp(
        &mut self,
        spline_id: i32,
        action: MoveSplineDoneTaxiActionLikeCpp,
        destination_node_id: Option<u32>,
        teleport_map_id: Option<u16>,
        teleport_position: Option<wow_core::Position>,
        honorless_target_cast: bool,
    ) -> MoveSplineDoneTaxiActionLikeCpp {
        #[cfg(test)]
        self.move_spline_done_taxi_events_like_cpp
            .push(MoveSplineDoneTaxiEventLikeCpp {
                spline_id,
                action,
                destination_node_id,
                teleport_map_id,
                teleport_position,
                honorless_target_cast,
            });
        #[cfg(not(test))]
        let _ = (
            spline_id,
            destination_node_id,
            teleport_map_id,
            teleport_position,
            honorless_target_cast,
        );
        action
    }
    pub(crate) fn remove_represented_at_login_flag_like_cpp(
        &mut self,
        flags: u16,
        persist: bool,
    ) -> bool {
        let Some(removed) = self.mutate_player_persistent_capability_state_like_cpp(|state| {
            let removed = (state.at_login_flags & flags) != 0;
            state.at_login_flags &= !flags;
            removed
        }) else {
            return false;
        };
        if !removed {
            return false;
        }
        if persist {
            #[cfg(test)]
            self.represented_at_login_flag_removals_like_cpp.push(
                RepresentedAtLoginFlagRemovalLikeCpp {
                    flags,
                    persist,
                    db_statement_unrepresented: true,
                },
            );
        }
        true
    }
    #[cfg(test)]
    pub(crate) fn move_spline_done_taxi_events_like_cpp(
        &self,
    ) -> &[MoveSplineDoneTaxiEventLikeCpp] {
        &self.move_spline_done_taxi_events_like_cpp
    }
    pub(crate) fn resolved_player_movement_time_like_cpp(&self) -> Option<u32> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().movement_time_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_movement_time_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_movement_time_like_cpp(&self) -> u32 {
        self.resolved_player_movement_time_like_cpp()
            .expect("test Player movement-time owner must resolve")
    }
    pub(in crate::session) fn resolved_movement_force_mod_magnitude_changes_like_cpp(
        &self,
    ) -> Option<u8> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.movement_force_mod_magnitude_changes_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.movement_force_mod_magnitude_changes_like_cpp);
        }
        canonical
    }
    pub(in crate::session) fn resolved_movement_force_mod_magnitude_like_cpp(&self) -> Option<f32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.unit().movement_force_mod_magnitude_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.movement_force_mod_magnitude_like_cpp);
        }
        canonical
    }
    pub(in crate::session) fn consume_movement_force_mod_magnitude_change_like_cpp(
        &mut self,
    ) -> Option<u8> {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.consume_movement_force_mod_magnitude_change_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            if self.movement_force_mod_magnitude_changes_like_cpp > 0 {
                self.movement_force_mod_magnitude_changes_like_cpp = self
                    .movement_force_mod_magnitude_changes_like_cpp
                    .saturating_sub(1);
            }
            return Some(self.movement_force_mod_magnitude_changes_like_cpp);
        }
        canonical
    }
    pub(crate) fn set_player_transport_position_like_cpp(&mut self, position: Option<Position>) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            if let (Some(state), Some(position)) = (
                self.player_transport_login_state_like_cpp.as_mut(),
                position,
            ) {
                state.info.x = position.x;
                state.info.y = position.y;
                state.info.z = position.z;
                state.info.o = position.orientation;
            }
            return;
        }
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            if let Some(position) = position {
                player.set_transport_position_like_cpp(position);
            }
        });
    }
    pub(crate) fn player_transport_position_like_cpp(&self) -> Option<Position> {
        self.player_transport_state_like_cpp()
            .flatten()
            .map(|state| Position::new(state.x, state.y, state.z, state.orientation))
    }
    #[cfg(test)]
    pub(crate) fn set_movement_force_mod_magnitude_changes_like_cpp(&mut self, count: u8) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_movement_force_mod_magnitude_changes_like_cpp(count);
            })
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.movement_force_mod_magnitude_changes_like_cpp = count;
        }
    }
    #[cfg(test)]
    pub(crate) fn set_movement_force_mod_magnitude_like_cpp(&mut self, magnitude: f32) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_movement_force_mod_magnitude_like_cpp(magnitude);
            })
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.movement_force_mod_magnitude_like_cpp = magnitude;
        }
    }
    pub(crate) fn represented_visibility_source_position_like_cpp(&self) -> Option<Position> {
        let player_position = self.player_position_like_cpp()?;
        let Some(player_guid) = self.player_guid() else {
            return Some(player_position);
        };
        let Some(seer_guid) = self.represented_seer_guid_like_cpp else {
            return Some(player_position);
        };
        if seer_guid.is_empty() || seer_guid == player_guid {
            return Some(player_position);
        }

        let Some(key) = self.current_canonical_player_map_key_like_cpp() else {
            return Some(player_position);
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return Some(player_position);
        };
        let Ok(manager) = manager.lock() else {
            return Some(player_position);
        };
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed.map().with_world_object_by_kinds_like_cpp(
                    seer_guid,
                    crate::session_rules::represented_seer_kinds_like_cpp(),
                    |object| object.position(),
                )
            })
            .or(Some(player_position))
    }
}
