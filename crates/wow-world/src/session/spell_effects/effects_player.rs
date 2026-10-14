//! Represented effects that act on the session player.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    pub(in crate::session) fn persist_player_homebind_like_cpp(
        &mut self,
        homebind: RepresentedHomebindLikeCpp,
    ) {
        let (Some(player_guid), Some(port)) = (
            self.player_guid(),
            self.lifecycle
                .player_lifecycle_port_like_cpp()
                .map(Arc::clone),
        ) else {
            return;
        };
        let guid_counter = player_guid.counter() as u64;
        let request =
            crate::session::player_homebind_update_request_like_cpp(homebind, guid_counter);
        // C++ Player::SetHomebind queues CharacterDatabase.Execute(stmt) on
        // the ordered database worker and immediately sends the bind packets.
        // Send it to one FIFO worker so SQL latency stays off the packet path
        // without allowing an older bind to finish after a newer one.
        if self
            .lifecycle
            .queue_player_homebind_persistence_like_cpp(port, request, guid_counter)
            .is_err()
        {
            warn!(
                player_guid = guid_counter,
                "represented homebind database worker stopped"
            );
        }
    }
    pub(in crate::session) fn apply_self_resurrect_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.core.player_is_in_world_for_registry_like_cpp() {
            return;
        }

        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                if player.unit().is_alive() {
                    return None;
                }
                let max_health = player
                    .unit()
                    .data()
                    .max_health
                    .clamp(1, u64::from(u32::MAX)) as u32;
                let (health, mana) = if damage < 0 {
                    (damage.saturating_abs() as u32, misc_value.max(0))
                } else {
                    let pct = damage.max(0);
                    (
                        max_health
                            .saturating_mul(u32::try_from(pct).unwrap_or(u32::MAX))
                            .saturating_div(100),
                        player
                            .get_max_power(PowerType::Mana)
                            .max(0)
                            .saturating_mul(pct)
                            / 100,
                    )
                };
                let health = health.min(max_health);
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
                player.unit_mut().set_health(u64::from(health));
                player.unit_mut().set_power(PowerType::Mana, mana);
                player.unit_mut().set_power(PowerType::Rage, 0);
                let max_energy = player.get_max_power(PowerType::Energy);
                player.unit_mut().set_power(PowerType::Energy, max_energy);
                player.unit_mut().set_power(PowerType::Focus, 0);
                Some((health, Some(player.values_update(true))))
            })
            .flatten();
        #[cfg(test)]
        let outcome = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some()
                || self.fixtures.combat.player_alive_like_cpp
            {
                return None;
            }
            let max_health = self.fixtures.combat.player_max_health_like_cpp.max(1);
            let health = if damage < 0 {
                damage.saturating_abs() as u32
            } else {
                max_health
                    .saturating_mul(u32::try_from(damage.max(0)).unwrap_or(u32::MAX))
                    .saturating_div(100)
            }
            .min(max_health);
            self.fixtures.combat.player_health_like_cpp = health;
            self.fixtures.combat.player_alive_like_cpp = true;
            Some((health, None))
        });
        #[cfg(not(test))]
        let outcome = canonical;
        let Some((health, values_update)) = outcome else {
            return;
        };
        self.sync_player_registry_state_like_cpp();
        if let Some(values_update) = values_update {
            self.core.send_player_values_update_like_cpp(&values_update);
        } else {
            self.core
                .send_player_health_values_update_like_cpp(player_guid, u64::from(health));
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_effects/effects_player/test_only.rs"]
mod test_only;
