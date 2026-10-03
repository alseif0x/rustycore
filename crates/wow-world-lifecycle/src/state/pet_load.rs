use crate::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
};

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn pet_load_spells_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .spells_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_spells_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_spells_for_pet_number(pet_number, rows)
    }

    pub fn pet_load_remove_spells_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_spells_for_pet_number(pet_number)
    }

    pub fn pet_load_clear_spells_like_cpp(&mut self) {
        self.pet_load_query_holder_rows_like_cpp.clear_spells();
    }

    pub fn pet_load_spell_cooldowns_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .spell_cooldowns_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_spell_cooldowns_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellCooldownRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_spell_cooldowns_for_pet_number(pet_number, rows)
    }

    pub fn pet_load_remove_spell_cooldowns_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_spell_cooldowns_for_pet_number(pet_number)
    }

    pub fn pet_load_spell_charges_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .spell_charges_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_spell_charges_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellChargeRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_spell_charges_for_pet_number(pet_number, rows)
    }

    pub fn pet_load_remove_spell_charges_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_spell_charges_for_pet_number(pet_number)
    }

    pub fn pet_load_auras_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetAuraRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .auras_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_auras_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetAuraRowLikeCpp>,
    ) -> Option<Vec<CharacterPetAuraRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_auras_for_pet_number(pet_number, rows)
    }

    pub fn pet_load_remove_auras_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetAuraRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_auras_for_pet_number(pet_number)
    }

    pub fn pet_load_aura_effects_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .aura_effects_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_aura_effects_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetAuraEffectRowLikeCpp>,
    ) -> Option<Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_aura_effects_for_pet_number(pet_number, rows)
    }

    pub fn pet_load_remove_aura_effects_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_aura_effects_for_pet_number(pet_number)
    }

    pub fn pet_load_declined_names_for_pet_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<&CharacterPetDeclinedNamesRowLikeCpp> {
        self.pet_load_query_holder_rows_like_cpp
            .declined_names_for_pet_number(pet_number)
    }

    pub fn pet_load_insert_declined_names_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
        row: CharacterPetDeclinedNamesRowLikeCpp,
    ) -> Option<CharacterPetDeclinedNamesRowLikeCpp> {
        self.pet_load_query_holder_rows_like_cpp
            .insert_declined_names_for_pet_number(pet_number, row)
    }

    pub fn pet_load_remove_declined_names_for_pet_number_like_cpp(
        &mut self,
        pet_number: u32,
    ) -> Option<CharacterPetDeclinedNamesRowLikeCpp> {
        self.pet_load_query_holder_rows_like_cpp
            .remove_declined_names_for_pet_number(pet_number)
    }

    pub fn pet_load_reset_like_cpp(&mut self) {
        self.pet_load_query_holder_rows_like_cpp.reset();
    }
}
