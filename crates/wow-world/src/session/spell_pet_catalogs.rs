// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell pet catalogs: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, ScriptNameInternerLikeCpp, WorldSession};
#[cfg(test)]
use super::{
    BTreeSet, PetAuraLikeCpp, PetDefaultSpellsEntryLikeCpp, PetLevelupSpellSetLikeCpp,
    SpellGroupStackRuleLikeCpp,
};

impl WorldSession {
    pub fn set_script_name_interner(&mut self, store: Arc<ScriptNameInternerLikeCpp>) {
        self.catalogs.script_name_interner = Some(store);
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/spell_pet_catalogs/f3_shims.rs"]
mod f3_shims;
