// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical player vitals adapters shared with World.

use crate::session::PLAYER_FLAGS_GHOST_LIKE_CPP;
use wow_constants::UnitFlags;

impl crate::session::HubMut<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_game_master_like_cpp(&mut self, is_game_master: bool) {
        let mut canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_game_master_like_cpp(is_game_master)
            })
            .is_some();
        if !canonical
            && self.core.player_handle_like_cpp.is_none()
            && let Some(guid) = self.core.player_guid()
        {
            canonical = self
                .core
                .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.set_game_master_like_cpp(is_game_master)
                })
                .is_some();
        }
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.player_game_master_like_cpp = is_game_master;
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_mounted_like_cpp(&mut self, mounted: bool) {
        let display_id = if mounted { 1 } else { 0 };
        let _ = self.set_player_mount_presentation_like_cpp(display_id, mounted);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_player_cheat_god_like_cpp(&mut self, enabled: bool) {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_cheat_god_like_cpp(enabled))
            .is_some();
        if canonical || self.core.player_handle_like_cpp.is_none() {
            self.fixtures.combat.player_cheat_god_like_cpp = enabled;
        }
    }
}

impl crate::session::HubRef<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_is_alive_like_cpp(&self) -> bool {
        self.resolved_player_is_alive_like_cpp().unwrap()
    }

    pub fn player_has_ghost_flag_like_cpp(&self) -> bool {
        self.core
            .player_guid()
            .and_then(|guid| {
                self.core
                    .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_GHOST_LIKE_CPP)
            })
            .unwrap_or(false)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_mounted_like_cpp(&self) -> bool {
        self.resolved_player_mounted_like_cpp()
            .expect("test Player presentation owner must resolve")
    }
}

impl crate::session::HubRef<'_> {
    pub fn resolved_player_vitals_like_cpp(&self) -> Option<(u32, u32, bool)> {
        let canonical = self.core.with_owned_player_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let health = player.unit().data().health.min(u64::from(max_health)) as u32;
            (health, max_health, player.unit().is_alive() && health > 0)
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some((
                self.fixtures.combat.player_health_like_cpp,
                self.fixtures.combat.player_max_health_like_cpp.max(1),
                self.fixtures.combat.player_alive_like_cpp
                    && self.fixtures.combat.player_health_like_cpp > 0,
            ));
        }
        canonical
    }

    pub fn resolved_player_is_alive_like_cpp(&self) -> Option<bool> {
        self.resolved_player_vitals_like_cpp()
            .map(|(_, _, alive)| alive)
    }

    pub fn resolved_player_mounted_like_cpp(&self) -> Option<bool> {
        self.player_unit_presentation_snapshot_like_cpp()
            .map(|(flags, _, _)| flags.contains(UnitFlags::MOUNT))
    }
}
