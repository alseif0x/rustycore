// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn spell_area_for_aura_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.catalogs
            .spell_area_for_aura_map_bounds_like_cpp(spell_id)
    }
    pub(in crate::session) fn represented_has_aura_state_like_cpp(&self, aura_state: u32) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_has_aura_state_like_cpp(hub, aura_state)
    }
}
