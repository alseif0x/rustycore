// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn set_represented_player_phase_shift_like_cpp(
        &mut self,
        phase_shift: PhaseShift,
    ) -> bool {
        let (state, mut hub) = crate::session::split_visibility_mut(self);
        state.set_represented_player_phase_shift_like_cpp(&mut hub, phase_shift)
    }
    #[cfg(test)]
    pub(crate) fn represented_seer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.visibility.represented_seer_guid_like_cpp()
    }
}
