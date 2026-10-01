// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn canonical_player_pvp_flags_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<UnitPvpFlags> {
        crate::session::hub_ref(self).canonical_player_pvp_flags_like_cpp(guid)
    }
    pub(crate) fn represented_advanced_combat_logging_enabled_like_cpp(&self) -> bool {
        crate::session::hub_ref(self).represented_advanced_combat_logging_enabled_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_player_pvp_state_like_cpp(
        &mut self,
        hostile: bool,
        pvp_enabled: bool,
        in_pvp_flag: bool,
    ) {
        crate::session::hub_mut(self).set_player_pvp_state_like_cpp(
            hostile,
            pvp_enabled,
            in_pvp_flag,
        )
    }
    pub(crate) fn combat_rating_multiplier_like_cpp(&self, level: u8, rating: u32) -> f32 {
        self.catalogs
            .combat_rating_multiplier_like_cpp(level, rating)
    }
}
