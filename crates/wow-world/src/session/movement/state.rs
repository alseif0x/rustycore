//! Represented player movement state at the Session boundary.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

pub(crate) use wow_world_core::session::MovementTransportMembershipLikeCpp;

#[path = "state/spline_progression.rs"]
mod spline_progression;

impl WorldSession {
    pub(crate) fn represented_move_dismiss_vehicle_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
    ) -> bool {
        let vehicle_guid = self.core.represented_player_charmed_guid_like_cpp();
        if vehicle_guid.is_empty()
            || crate::session::hub_ref(self)
                .player_vehicle_seat_state_like_cpp()
                .is_none()
        {
            return false;
        }

        self.sanitize_movement_info_represented_like_cpp(status);
        crate::session::hub_mut(self).set_player_movement_time_like_cpp(status.time);
        crate::session::hub_mut(self).set_player_movement_flags_like_cpp(status.flags);
        crate::session::hub_mut(self).set_player_position_like_cpp(status.position);
        crate::session::hub_ref(self).update_registry_position();
        #[cfg(test)]
        self.fixtures
            .movement
            .represented_vehicle_dismiss_movements_like_cpp
            .push(RepresentedVehicleDismissMovementLikeCpp {
                vehicle_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            });

        if !crate::session::hub_mut(self).set_player_vehicle_seat_state_like_cpp(None, None) {
            return false;
        }
        self.sync_player_registry_state_like_cpp();
        true
    }

    pub(crate) fn remove_currency(&mut self, currency_id: u32, amount: u32) -> bool {
        let access = self.core.owned_player_currency_access_like_cpp();
        self.inventory
            .remove_currency_with_access_like_cpp(&access, currency_id, amount)
    }

    pub(crate) fn remove_represented_rest_flag_like_cpp(&mut self, rest_flag: u32) -> bool {
        self.remove_player_rest_flag_like_cpp(rest_flag)
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

    pub fn remove_legit_character(&mut self, guid: &ObjectGuid) {
        crate::session::hub_mut(self).remove_legit_character(guid)
    }

    pub(crate) fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        self.player_aura_application_cx_like_cpp()
            .remove_represented_feign_death_if_needed_like_cpp()
    }

    pub(crate) fn set_represented_mover_fixed_position_vehicle_like_cpp(&mut self, fixed: bool) {
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_mover_fixed_position_vehicle_like_cpp(fixed);
            })
            .is_some();
        #[cfg(test)]
        if _canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .movement
                .represented_mover_fixed_position_vehicle_like_cpp = fixed;
        }
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn calendar_remove_event_like_cpp(&mut self, event_id: u64) {
        #[cfg(test)]
        self.social
            .record_calendar_remove_event_for_test_like_cpp(event_id);
    }

    #[cfg(test)]
    pub(crate) fn represented_calendar_remove_events_like_cpp(
        &self,
    ) -> &[RepresentedCalendarRemoveEventLikeCpp] {
        self.social
            .represented_calendar_remove_events_for_test_like_cpp()
    }

    pub fn set_player_moved_unit_guid_like_cpp(&mut self, guid: ObjectGuid) {
        crate::session::hub_mut(self).set_player_moved_unit_guid_like_cpp(guid)
    }

    pub(crate) fn represented_move_change_vehicle_seats_like_cpp(
        &mut self,
        status: &mut wow_packet::packets::movement::MovementInfo,
        dst_vehicle: ObjectGuid,
        dst_seat_index: u8,
    ) -> bool {
        let Some(vehicle_base_guid) =
            crate::session::hub_ref(self).represented_vehicle_base_guid_for_switch_like_cpp()
        else {
            return false;
        };
        if !crate::session::hub_ref(self)
            .represented_current_vehicle_seat_can_switch_from_like_cpp()
        {
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
        self.fixtures
            .movement
            .represented_vehicle_base_movements_like_cpp
            .push(RepresentedVehicleBaseMovementLikeCpp {
                vehicle_guid: vehicle_base_guid,
                sanitized_flags: status.flags,
                position: status.position,
                time: status.time,
            });
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
                mover_fixed_position_vehicle: crate::session::hub_ref(self)
                    .resolved_mover_fixed_position_vehicle_like_cpp()
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
                is_player_security: crate::session::hub_ref(self).player_is_game_master_like_cpp()
                    != Some(true),
            },
        )
    }

    pub(crate) fn remove_represented_at_login_flag_like_cpp(
        &mut self,
        flags: u16,
        persist: bool,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.remove_at_login_flags_like_cpp(flags));
        #[cfg(test)]
        let removed = canonical.or_else(|| {
            self.core.player_handle_like_cpp.is_none().then(|| {
                self.mutate_player_persistent_capability_state_like_cpp(|state| {
                    let removed = (state.at_login_flags & flags) != 0;
                    state.at_login_flags &= !flags;
                    removed
                })
                .expect("handle-less fixture capability owner")
            })
        });
        #[cfg(not(test))]
        let removed = canonical;
        let Some(removed) = removed else {
            return false;
        };
        if !removed {
            return false;
        }
        if persist {
            #[cfg(test)]
            self.lifecycle
                .record_at_login_flag_removal_for_test_like_cpp(
                    RepresentedAtLoginFlagRemovalLikeCpp {
                        flags,
                        persist,
                        db_statement_unrepresented: true,
                    },
                );
        }
        true
    }

    pub(in crate::session) fn resolved_movement_force_mod_magnitude_like_cpp(&self) -> Option<f32> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            player.unit().movement_force_mod_magnitude_like_cpp()
        });
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.movement.movement_force_mod_magnitude_like_cpp);
        }
        canonical
    }

    pub(crate) fn set_player_transport_position_like_cpp(&mut self, position: Option<Position>) {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            if let (Some(state), Some(position)) = (
                self.fixtures
                    .vehicles
                    .player_transport_login_state_like_cpp
                    .as_mut(),
                position,
            ) {
                state.info.x = position.x;
                state.info.y = position.y;
                state.info.z = position.z;
                state.info.o = position.orientation;
            }
            return;
        }
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            if let Some(position) = position {
                player.set_transport_position_like_cpp(position);
            }
        });
    }

    pub(crate) fn represented_visibility_source_position_like_cpp(&self) -> Option<Position> {
        let player_position = crate::session::hub_ref(self).player_position_like_cpp()?;
        let Some(player_guid) = self.player_guid() else {
            return Some(player_position);
        };
        let Some(seer_guid) = self.current_seer_guid_like_cpp() else {
            return Some(player_position);
        };
        if seer_guid.is_empty() || seer_guid == player_guid {
            return Some(player_position);
        }

        let Some(key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return Some(player_position);
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
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
                    wow_entities::represented_seer_kinds_like_cpp(),
                    |object| object.position(),
                )
            })
            .or(Some(player_position))
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/movement/state/f3_shims.rs"]
mod f3_shims;
