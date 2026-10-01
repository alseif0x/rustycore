// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn load_represented_pet_declined_names_like_cpp(
        &mut self,
        pet_number: u32,
        row: Option<CharacterPetDeclinedNamesRowLikeCpp>,
    ) -> bool {
        crate::session::cx_pets(self).load_represented_pet_declined_names_like_cpp(pet_number, row)
    }
}
