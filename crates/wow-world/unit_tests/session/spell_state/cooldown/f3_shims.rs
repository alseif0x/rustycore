// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(in crate::session) fn player_spell_history_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::SpellHistory> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.player_spell_history_snapshot_like_cpp(hub)
    }
    pub(in crate::session) fn mutate_player_spell_history_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::SpellHistory) -> R,
    ) -> Option<R> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mutate_player_spell_history_like_cpp(&mut hub, f)
    }
}
