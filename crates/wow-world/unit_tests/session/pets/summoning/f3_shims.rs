// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn set_represented_pet_stable_like_cpp(&mut self, stable: PetStable) {
        crate::session::hub_mut(self).set_represented_pet_stable_like_cpp(stable)
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_stable_current_index_like_cpp(&self) -> Option<u32> {
        crate::session::hub_ref(self).represented_pet_stable_current_index_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_temporary_unsummoned_pet_number_like_cpp(&self) -> u32 {
        crate::session::hub_ref(self).represented_temporary_unsummoned_pet_number_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn resummon_pet_temporary_unsummoned_if_any_like_cpp(&mut self) {
        crate::session::cx_pets(self).resummon_pet_temporary_unsummoned_if_any_like_cpp()
    }
    pub(crate) fn load_represented_pet_stable_rows_like_cpp(
        &mut self,
        summoned_pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetStableRowLikeCpp>,
    ) -> usize {
        crate::session::hub_mut(self)
            .load_represented_pet_stable_rows_like_cpp(summoned_pet_number, rows)
    }
}
