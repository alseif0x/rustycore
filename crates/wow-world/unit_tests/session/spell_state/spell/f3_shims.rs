// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_start_all_spells_like_cpp(&mut self, enabled: bool) {
        self.catalogs.set_start_all_spells_like_cpp(enabled)
    }
    #[cfg(test)]
    pub(crate) fn start_all_spells_like_cpp(&self) -> bool {
        self.catalogs.start_all_spells_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn serverside_spell_like_cpp(
        &self,
        spell_id: u32,
        difficulty: u32,
    ) -> Option<&ServersideSpellInfoLikeCpp> {
        self.catalogs
            .serverside_spell_like_cpp(spell_id, difficulty)
    }
}
