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
}
