// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn player_aura_authority_complete_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.player_aura_authority_complete_like_cpp(hub)
    }
    pub(crate) fn resolved_player_aura_authority_complete_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.resolved_player_aura_authority_complete_like_cpp(hub)
    }
}
