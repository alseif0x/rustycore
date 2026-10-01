// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn load_represented_pet_aura_rows_with_timediff_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        crate::session::cx_pets(self).load_represented_pet_aura_rows_with_timediff_like_cpp(
            pet_number,
            rows,
            timediff_secs,
        )
    }
    pub(crate) fn load_represented_pet_spell_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellRowLikeCpp>,
    ) -> usize {
        crate::session::cx_pets(self).load_represented_pet_spell_rows_like_cpp(pet_number, rows)
    }
    pub(crate) fn load_represented_pet_aura_effect_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraEffectRowLikeCpp>,
    ) -> usize {
        crate::session::cx_pets(self)
            .load_represented_pet_aura_effect_rows_like_cpp(pet_number, rows)
    }
    pub(crate) fn load_represented_pet_spell_cooldown_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellCooldownRowLikeCpp>,
    ) -> usize {
        crate::session::cx_pets(self)
            .load_represented_pet_spell_cooldown_rows_like_cpp(pet_number, rows)
    }
    pub(crate) fn load_represented_pet_aura_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
    ) -> usize {
        crate::session::cx_pets(self).load_represented_pet_aura_rows_like_cpp(pet_number, rows)
    }
    pub(crate) fn load_represented_pet_spell_charge_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellChargeRowLikeCpp>,
    ) -> usize {
        crate::session::cx_pets(self)
            .load_represented_pet_spell_charge_rows_like_cpp(pet_number, rows)
    }
}
