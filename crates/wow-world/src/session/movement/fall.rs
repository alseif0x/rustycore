//! Represented falling, jumping and knockback.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn handle_fall_like_cpp(
        &mut self,
        movement_info: &wow_packet::packets::movement::MovementInfo,
    ) -> Option<MovementFallDamageEvent> {
        let (_, last_fall_z) =
            crate::session::hub_ref(self).resolved_fall_information_like_cpp()?;
        let damage_control =
            crate::session::hub_ref(self).resolved_player_damage_control_like_cpp()?;
        let z_diff = last_fall_z - movement_info.position.z;
        let (_, max_health, player_is_alive) =
            crate::session::hub_ref(self).resolved_player_vitals_like_cpp()?;
        if z_diff < 14.57
            || !player_is_alive
            || crate::session::hub_ref(self).player_is_game_master_like_cpp() == Some(true)
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

        let safe_fall = crate::session::hub_ref(self)
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
            * self.resolved_total_represented_aura_multiplier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModifyFallDamagePct,
            )?) as u32;
        if crate::session::hub_ref(self).player_has_visible_aura_spell_like_cpp(43_621)? {
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
        let (_original_health, health_after, _, final_damage, killed_player) =
            crate::session::hub_mut(self).apply_owned_player_damage_like_cpp(
                requested_damage,
                wow_constants::DeathState::JustDied,
            )?;
        self.sync_player_registry_state_like_cpp();
        if final_damage > 0
            && let Some(player_guid) = self.player_guid()
        {
            self.core
                .send_player_health_update_like_cpp(player_guid, u64::from(health_after));
            self.core.send_environmental_damage_log_like_cpp(
                player_guid,
                DAMAGE_FALL_LIKE_CPP,
                damage,
                0,
                0,
            );
            if killed_player {
                self.core
                    .send_player_health_values_update_like_cpp(player_guid, 0);
                // C++ `Player::EnvironmentalDamage` fall-to-death branch
                // (`Player.cpp:663-670`): item durability loss plus the loss
                // message. Non-fall environmental damage never wears items.
                let loss_rate = self.config.durability_loss_on_death_rate_like_cpp();
                if loss_rate > 0.0 {
                    self.apply_represented_durability_loss_all_like_cpp(
                        f64::from(loss_rate),
                        false,
                    );
                    self.send_packet(&wow_packet::packets::misc::DurabilityDamageDeath {
                        percent: (loss_rate * 100.0) as i32,
                    });
                }
            }
        }

        let event = MovementFallDamageEvent {
            z_diff,
            damage,
            final_damage,
        };
        #[cfg(test)]
        self.fixtures
            .movement
            .fall_damage_events_like_cpp
            .push(event);
        Some(event)
    }
    pub(crate) fn apply_knock_back_ack_like_cpp(
        &mut self,
        opcode: ClientOpcodes,
        ack: &mut wow_packet::packets::movement::MovementAck,
    ) -> bool {
        // C++ validates the packet with the Player but admits the status when
        // its GUID is the active `_player->m_unitMovedByMe` (MovementHandler.cpp:548-559).
        // Keep the Player-owned movement-info write below; only the active
        // mover gate decides whether this ACK is accepted.
        if !self.validate_and_sanitize_active_mover_ack_like_cpp(&mut ack.status) {
            crate::session::hub_mut(self).record_movement_ack_event_like_cpp(
                MovementAckEventLikeCpp {
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
                },
            );
            return false;
        }

        let mut status = ack.status.clone();
        status.time =
            crate::session::hub_ref(self).adjust_client_movement_time_like_cpp(status.time);
        crate::session::hub_mut(self).set_player_movement_time_like_cpp(status.time);
        crate::session::hub_mut(self).set_player_movement_flags_like_cpp(status.flags);
        crate::session::hub_mut(self).set_player_position_like_cpp(status.position);
        crate::session::hub_mut(self).record_movement_ack_event_like_cpp(MovementAckEventLikeCpp {
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
}


#[cfg(test)]
#[path = "../../../unit_tests/session/movement/fall/f3_shims.rs"]
mod f3_shims;
