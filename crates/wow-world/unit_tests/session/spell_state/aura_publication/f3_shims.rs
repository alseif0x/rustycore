// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn visible_aura_slot_for_spell_like_cpp(&self, spell_id: i32) -> Option<u8> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.visible_aura_slot_for_spell_like_cpp(hub, spell_id)
    }
    pub(crate) fn resolved_player_visible_auras_like_cpp(
        &self,
    ) -> Option<HashMap<u8, AuraApplication>> {
        crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
    }
}
