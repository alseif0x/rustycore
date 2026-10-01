// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_live_applications_like_cpp(
        &self,
    ) -> &[RepresentedLiveApplicationLikeCpp] {
        self.fixtures
            .presentation
            .represented_live_applications_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_stand_state_like_cpp(&self) -> UnitStandStateType {
        crate::session::hub_ref(self).player_stand_state_like_cpp()
    }
}
