//! Represented falling, jumping and knockback.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn set_player_movement_jump_like_cpp(
        &mut self,
        jump: wow_packet::packets::movement::JumpInfo,
    ) {
        #[cfg(test)]
        {
            self.player_movement_jump_like_cpp = jump;
        }
        #[cfg(not(test))]
        let _ = jump;
    }
    pub(in crate::session) fn resolved_fall_information_like_cpp(&self) -> Option<(u32, f32)> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.fall_information_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((self.last_fall_time_like_cpp, self.last_fall_z_like_cpp));
        }
        canonical
    }
    pub(crate) fn set_fall_information_like_cpp(&mut self, time: u32, z: f32) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_fall_information_like_cpp(time, z);
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.last_fall_time_like_cpp = time;
            self.last_fall_z_like_cpp = z;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn update_fall_information_if_needed_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
        is_fall_land: bool,
    ) {
        let Some((last_fall_time, last_fall_z)) = self.resolved_fall_information_like_cpp() else {
            return;
        };
        if last_fall_time >= movement_info.jump.fall_time
            || last_fall_z <= movement_info.position.z
            || is_fall_land
        {
            self.set_fall_information_like_cpp(
                movement_info.jump.fall_time,
                movement_info.position.z,
            );
        }
    }
    pub(crate) fn handle_fall_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
    ) -> Option<MovementFallDamageEvent> {
        let (_, last_fall_z) = self.resolved_fall_information_like_cpp()?;
        let damage_control = self.resolved_player_damage_control_like_cpp()?;
        let z_diff = last_fall_z - movement_info.position.z;
        let (_, max_health, player_is_alive) = self.resolved_player_vitals_like_cpp()?;
        if z_diff < 14.57
            || !player_is_alive
            || self.player_is_game_master_like_cpp() == Some(true)
            || self.resolved_has_represented_aura_effect_like_cpp(
                RepresentedAuraEffectLikeCpp::Hover,
            )?
            || self.resolved_has_represented_aura_effect_like_cpp(
                RepresentedAuraEffectLikeCpp::FeatherFall,
            )?
            || self
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Fly)?
            || damage_control.normal_damage_immune
        {
            return None;
        }

        let safe_fall = self.resolved_total_represented_aura_modifier_like_cpp(
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
            * self.resolved_total_represented_aura_multiplier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModifyFallDamagePct,
            )?) as u32;
        if self.player_has_visible_aura_spell_like_cpp(43_621)? {
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
        let (_original_health, health_after, _, final_damage, killed_player) = self
            .apply_owned_player_damage_like_cpp(
                requested_damage,
                wow_constants::DeathState::JustDied,
            )?;
        self.sync_player_registry_state_like_cpp();
        if final_damage > 0
            && let Some(player_guid) = self.player_guid()
        {
            self.send_player_health_update_like_cpp(player_guid, u64::from(health_after));
            self.send_environmental_damage_log_like_cpp(
                player_guid,
                DAMAGE_FALL_LIKE_CPP,
                damage,
                0,
                0,
            );
            if killed_player {
                self.send_player_health_values_update_like_cpp(player_guid, 0);
            }
        }

        let event = MovementFallDamageEvent {
            z_diff,
            damage,
            final_damage,
        };
        #[cfg(test)]
        self.fall_damage_events_like_cpp.push(event);
        Some(event)
    }
    #[cfg(test)]
    pub(crate) fn fall_damage_events_like_cpp(&self) -> &[MovementFallDamageEvent] {
        &self.fall_damage_events_like_cpp
    }
    pub(crate) fn apply_knock_back_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
    ) -> bool {
        if !self.validate_and_sanitize_movement_ack_status_represented_like_cpp(&mut ack.status) {
            self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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
        status.time = self.adjust_client_movement_time_like_cpp(status.time);
        self.set_player_movement_time_like_cpp(status.time);
        self.set_player_movement_flags_like_cpp(status.flags);
        self.set_player_position_like_cpp(status.position);
        self.record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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
    #[cfg(test)]
    pub(crate) fn fall_information_like_cpp(&self) -> (u32, f32) {
        self.resolved_fall_information_like_cpp()
            .expect("test Player fall-information owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn player_movement_jump_like_cpp(&self) -> &wow_packet::packets::movement::JumpInfo {
        &self.player_movement_jump_like_cpp
    }
    pub(in crate::session) fn move_represented_player_fall_like_cpp(&mut self) -> bool {
        let Some(mut movement_flags) = self.resolved_player_movement_flags_like_cpp() else {
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
}
