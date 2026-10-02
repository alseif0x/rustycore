// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player vitals adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{PLAYER_FLAGS_GHOST_LIKE_CPP, WorldSession};

impl WorldSession {
    pub fn set_player_alive_like_cpp(&mut self, alive: bool) {
        let resolved = self.core.with_owned_player_mut_like_cpp(|player| {
            if !alive {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
                player.unit_mut().set_health(0);
            } else {
                let max_health = player.unit().data().max_health.max(1);
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
                if player.unit().data().health == 0 {
                    player.unit_mut().set_health(max_health);
                }
            }
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player
                    .unit()
                    .data()
                    .max_health
                    .clamp(1, u64::from(u32::MAX)) as u32,
                player.unit().is_alive(),
            )
        });
        #[cfg(test)]
        {
            let (health, max_health, is_alive) = resolved.unwrap_or_else(|| {
                let max_health = self.fixtures.combat.player_max_health_like_cpp.max(1);
                let health = if alive {
                    self.fixtures
                        .combat
                        .player_health_like_cpp
                        .max(1)
                        .min(max_health)
                } else {
                    0
                };
                (health, max_health, alive)
            });
            self.fixtures.combat.player_health_like_cpp = health;
            self.fixtures.combat.player_max_health_like_cpp = max_health;
            self.fixtures.combat.player_alive_like_cpp = is_alive;
        }
        if resolved.is_some() || cfg!(test) && self.core.player_handle_like_cpp.is_none() {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub(crate) fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool) {
        if self.player_guid().is_some() {
            let _ = self.core.mutate_canonical_player_like_cpp(|player| {
                if ghost {
                    player.set_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                }
            });
        }
        self.sync_player_registry_state_like_cpp();
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/player_vitals_adapter/f3_shims.rs"]
mod f3_shims;
