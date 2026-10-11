// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn is_spell_learn_spell_like_cpp(&self, spell_id: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_spell_like_cpp(spell_id))
    }
    pub(crate) fn is_spell_learn_to_spell_like_cpp(&self, spell_id1: u32, spell_id2: u32) -> bool {
        self.catalogs
            .spell_catalogs
            .spell_learn_spell_store
            .as_ref()
            .is_some_and(|store| store.is_spell_learn_to_spell_like_cpp(spell_id1, spell_id2))
    }
    pub(crate) fn represented_favorite_known_spells_like_cpp(&self) -> HashSet<i32> {
        self.with_player_spell_runtime_like_cpp(|runtime| {
            runtime
                .favorite_known_spells_like_cpp()
                .iter()
                .copied()
                .collect()
        })
        .unwrap_or_default()
    }
}
