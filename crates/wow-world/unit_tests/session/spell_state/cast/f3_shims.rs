// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_player_create_cast_spell_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoCastSpellStoreLikeCpp>,
    ) {
        self.catalogs
            .set_player_create_cast_spell_store_like_cpp(store)
    }
}
