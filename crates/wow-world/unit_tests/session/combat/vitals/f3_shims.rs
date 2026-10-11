// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_health_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).player_health_like_cpp()
    }
    pub(crate) fn canonical_player_power_snapshot_like_cpp(
        &self,
        power_type: PowerType,
    ) -> Option<(i32, i32)> {
        self.core
            .canonical_player_power_snapshot_like_cpp(power_type)
    }
    pub(crate) fn sync_canonical_player_primary_power_like_cpp(
        &mut self,
        power_type: PowerType,
        current: i32,
        max: i32,
        base_mana: i32,
    ) -> bool {
        crate::session::hub_mut(self)
            .sync_canonical_player_primary_power_like_cpp(power_type, current, max, base_mana)
    }
}

impl WorldSession {
    pub(crate) fn canonical_player_health_snapshot_like_cpp(&self) -> Option<(u32, u32)> {
        self.core.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        })
    }
    pub(crate) fn represented_player_power_values_like_cpp(
        &self,
    ) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self.resolved_player_power_values_like_cpp();
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return character_power_snapshot_values_like_cpp(
                &self.fixtures.combat.represented_player_powers_like_cpp,
            );
        }
        canonical
    }
    pub(crate) fn resolved_player_power_values_like_cpp(
        &self,
    ) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().data().power);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return self
                .core
                .mutate_canonical_player_like_cpp(|player| player.unit().data().power);
        }
        canonical
    }
    pub(in crate::session) fn resolved_player_power_snapshot_like_cpp(
        &self,
    ) -> Option<CharacterPowerSnapshotLikeCpp> {
        let canonical = self
            .resolved_player_power_values_like_cpp()
            .map(loaded_character_power_snapshot_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.fixtures.combat.represented_player_powers_like_cpp);
        }
        canonical
    }
    pub(crate) fn set_player_health_like_cpp(&mut self, health: u32, max_health: u32) {
        let _ =
            crate::session::hub_mut(self).sync_canonical_player_health_like_cpp(health, max_health);
        self.sync_player_registry_state_like_cpp();
    }
}
