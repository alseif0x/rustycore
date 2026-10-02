// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell pet catalogs: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, ScriptIdLikeCpp, ScriptNameInternerLikeCpp, WorldSession};
#[cfg(test)]
use super::{
    BTreeSet, PetAuraLikeCpp, PetDefaultSpellsEntryLikeCpp, PetLevelupSpellSetLikeCpp,
    SpellGroupStackRuleLikeCpp,
};

impl WorldSession {
    pub fn set_script_name_interner(&mut self, store: Arc<ScriptNameInternerLikeCpp>) {
        self.catalogs.script_name_interner = Some(store);
    }

    #[allow(dead_code)]
    pub(crate) fn script_name_like_cpp(&self, id: ScriptIdLikeCpp) -> &str {
        self.catalogs
            .script_name_interner
            .as_ref()
            .map(|store| store.get_script_name_like_cpp(id))
            .unwrap_or("")
    }

    #[allow(dead_code)]
    pub(crate) fn script_id_bound_in_database_like_cpp(&self, id: ScriptIdLikeCpp) -> bool {
        self.catalogs
            .script_name_interner
            .as_ref()
            .is_some_and(|store| store.is_script_database_bound_like_cpp(id))
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/spell_pet_catalogs/f3_shims.rs"]
mod f3_shims;
