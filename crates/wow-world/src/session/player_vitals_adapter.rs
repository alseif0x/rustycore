// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player vitals adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{PLAYER_FLAGS_GHOST_LIKE_CPP, UnitFlags, WorldSession};

impl WorldSession {
    pub fn set_player_alive_like_cpp(&mut self, alive: bool) {
        let resolved = self.with_owned_player_mut_like_cpp(|player| {
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
                let max_health = self.player_max_health_like_cpp.max(1);
                let health = if alive {
                    self.player_health_like_cpp.max(1).min(max_health)
                } else {
                    0
                };
                (health, max_health, alive)
            });
            self.player_health_like_cpp = health;
            self.player_max_health_like_cpp = max_health;
            self.player_alive_like_cpp = is_alive;
        }
        if resolved.is_some() || cfg!(test) && self.player_handle_like_cpp.is_none() {
            self.sync_player_registry_state_like_cpp();
        }
    }

    pub(crate) fn resolved_player_vitals_like_cpp(&self) -> Option<(u32, u32, bool)> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let health = player.unit().data().health.min(u64::from(max_health)) as u32;
            (health, max_health, player.unit().is_alive() && health > 0)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some((
                self.player_health_like_cpp,
                self.player_max_health_like_cpp.max(1),
                self.player_alive_like_cpp && self.player_health_like_cpp > 0,
            ));
        }
        canonical
    }

    pub(crate) fn resolved_player_is_alive_like_cpp(&self) -> Option<bool> {
        self.resolved_player_vitals_like_cpp()
            .map(|(_, _, alive)| alive)
    }

    #[cfg(test)]
    pub(crate) fn player_is_alive_like_cpp(&self) -> bool {
        self.resolved_player_is_alive_like_cpp().unwrap()
    }

    pub(crate) fn player_has_ghost_flag_like_cpp(&self) -> bool {
        self.player_guid()
            .and_then(|guid| {
                self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_GHOST_LIKE_CPP)
            })
            .unwrap_or(false)
    }

    pub(crate) fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool) {
        if self.player_guid().is_some() {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                if ghost {
                    player.set_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_GHOST_LIKE_CPP);
                }
            });
        }
        self.sync_player_registry_state_like_cpp();
    }

    #[cfg(test)]
    pub(crate) fn set_player_game_master_like_cpp(&mut self, is_game_master: bool) {
        let mut canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_game_master_like_cpp(is_game_master)
            })
            .is_some();
        if !canonical
            && self.player_handle_like_cpp.is_none()
            && let Some(guid) = self.player_guid()
        {
            canonical = self
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.set_game_master_like_cpp(is_game_master)
                })
                .is_some();
        }
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_game_master_like_cpp = is_game_master;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_player_mounted_like_cpp(&mut self, mounted: bool) {
        let display_id = if mounted { 1 } else { 0 };
        let _ = self.set_player_mount_presentation_like_cpp(display_id, mounted);
    }

    pub(in crate::session) fn resolved_player_mounted_like_cpp(&self) -> Option<bool> {
        self.player_unit_presentation_snapshot_like_cpp()
            .map(|(flags, _, _)| flags.contains(UnitFlags::MOUNT))
    }

    #[cfg(test)]
    pub(crate) fn player_mounted_like_cpp(&self) -> bool {
        self.resolved_player_mounted_like_cpp()
            .expect("test Player presentation owner must resolve")
    }

    #[cfg(test)]
    pub(crate) fn set_player_cheat_god_like_cpp(&mut self, enabled: bool) {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.set_cheat_god_like_cpp(enabled))
            .is_some();
        if canonical || self.player_handle_like_cpp.is_none() {
            self.player_cheat_god_like_cpp = enabled;
        }
    }
}
