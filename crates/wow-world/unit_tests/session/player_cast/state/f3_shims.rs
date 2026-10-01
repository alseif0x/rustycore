// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn active_spell_cast_snapshot_like_cpp(&self) -> Option<SpellCastState> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.active_spell_cast_snapshot_like_cpp(hub)
    }
    #[cfg(test)]
    pub(crate) fn pending_spell_cast_for_test_like_cpp(
        &self,
    ) -> Option<RepresentedPendingSpellCastRequestLikeCpp> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.pending_spell_cast_for_test_like_cpp(hub)
    }
}
