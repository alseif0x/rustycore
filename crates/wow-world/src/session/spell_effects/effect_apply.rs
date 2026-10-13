//! Represented application of individual spell effects.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The battle-pet uncage effect
//! moved to `wow_world_inventory::battle_pet_session` (#1363 slice 7b).

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectStuck` (`SpellEffects.cpp:3265-3303`).
    pub(in crate::session) async fn apply_stuck_effect_like_cpp(&mut self) {
        const HEARTHSTONE_SPELL_ID_LIKE_CPP: i32 = 8690;

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.config.represented_cast_unstuck_enabled_like_cpp {
            return;
        }
        if crate::session::hub_ref(self).resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return;
        }

        match crate::session::hub_ref(self).resolved_player_is_alive_like_cpp() {
            None => return,
            Some(false) => {
                let Some(resurrection) = self.player_resurrection_state_snapshot_like_cpp() else {
                    return;
                };
                if !resurrection.death_timer_active {
                    self.set_player_ghost_flag_like_cpp(true);
                    #[cfg(test)]
                    {
                        self.fixtures.combat.represented_repop_at_graveyard_count = self
                            .fixtures
                            .combat
                            .represented_repop_at_graveyard_count
                            .saturating_add(1);
                    }
                }
                return;
            }
            Some(true) => {}
        }

        let Some(hearthstone_last_cast) =
            self.spell_last_cast_time_like_cpp(HEARTHSTONE_SPELL_ID_LIKE_CPP)
        else {
            return;
        };
        if hearthstone_last_cast.is_some() {
            self.set_player_alive_like_cpp(false);
            let values_update = self.core.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_health(0);
                player.values_update(true)
            });
            if let Some(values_update) = values_update {
                self.core.send_player_values_update_like_cpp(&values_update);
            } else {
                self.core
                    .send_player_health_values_update_like_cpp(player_guid, 0);
            }
            return;
        }

        if let Some(homebind) = represented_homebind_like_cpp(self) {
            self.teleport_to(homebind.map_id, homebind.position).await;
        }

        let Some(hearthstone_cooldown_ms) = self
            .catalogs
            .spell_catalogs
            .spell_store
            .as_deref()
            .and_then(|store| store.get(HEARTHSTONE_SPELL_ID_LIKE_CPP))
            .map(|spell_info| spell_info.recovery_time_ms.max(spell_info.cooldown_ms))
        else {
            return;
        };
        if self
            .mutate_cast_execution_like_cpp(|state| {
                state
                    .last_cast_time_per_spell
                    .insert(HEARTHSTONE_SPELL_ID_LIKE_CPP, Instant::now());
            })
            .is_none()
        {
            return;
        }
        {
            let (s, mut h) = crate::session::split_spell_state_mut(self);
            s.record_cast_character_spell_cooldown_like_cpp(
                &mut h,
                HEARTHSTONE_SPELL_ID_LIKE_CPP,
                hearthstone_cooldown_ms,
            )
        };
        self.send_packet(&wow_packet::packets::spell::CooldownEvent {
            spell_id: HEARTHSTONE_SPELL_ID_LIKE_CPP,
            is_pet: false,
        });
    }
}
