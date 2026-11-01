use std::collections::HashMap;

use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPetSpellRowLikeCpp {
    pub spell_id: u32,
    pub active: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPetSpellCooldownRowLikeCpp {
    pub spell_id: u32,
    pub cooldown_end_unix_secs: i64,
    pub category_id: u32,
    pub category_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPetSpellChargeRowLikeCpp {
    pub category_id: u32,
    pub recharge_start_unix_secs: i64,
    pub recharge_end_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPetAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterPetAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterPetDeclinedNamesRowLikeCpp {
    pub names: [String; 5],
}

/// Per-character rows retained for the represented asynchronous
/// `PetLoadQueryHolder` callback (`Pet.cpp:157-203,386-449`).
///
/// This is session lifecycle state, not Player or live Pet gameplay state. A
/// new character load replaces the holder; the typed Pet consumes its rows
/// when Map storage materializes that Pet.
#[derive(Debug, Default)]
pub struct PetLoadQueryHolderRowsLikeCpp {
    spells: HashMap<u32, Vec<CharacterPetSpellRowLikeCpp>>,
    spell_cooldowns: HashMap<u32, Vec<CharacterPetSpellCooldownRowLikeCpp>>,
    spell_charges: HashMap<u32, Vec<CharacterPetSpellChargeRowLikeCpp>>,
    auras: HashMap<u32, Vec<CharacterPetAuraRowLikeCpp>>,
    aura_effects: HashMap<u32, Vec<CharacterPetAuraEffectRowLikeCpp>>,
    declined_names: HashMap<u32, CharacterPetDeclinedNamesRowLikeCpp>,
}

impl PetLoadQueryHolderRowsLikeCpp {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn spells_are_empty_like_cpp(&self) -> bool {
        self.spells.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn spell_cooldowns_are_empty_like_cpp(&self) -> bool {
        self.spell_cooldowns.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn spell_charges_are_empty_like_cpp(&self) -> bool {
        self.spell_charges.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn auras_are_empty_like_cpp(&self) -> bool {
        self.auras.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn aura_effects_are_empty_like_cpp(&self) -> bool {
        self.aura_effects.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn declined_names_are_empty_like_cpp(&self) -> bool {
        self.declined_names.is_empty()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn spells_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellRowLikeCpp>> {
        self.spells.get(&pet_number)
    }

    pub fn insert_spells_for_pet_number(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellRowLikeCpp>> {
        self.spells.insert(pet_number, rows)
    }

    pub fn remove_spells_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellRowLikeCpp>> {
        self.spells.remove(&pet_number)
    }

    pub fn clear_spells(&mut self) {
        self.spells.clear();
    }

    pub fn spell_cooldowns_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.spell_cooldowns.get(&pet_number)
    }

    pub fn insert_spell_cooldowns_for_pet_number(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellCooldownRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.spell_cooldowns.insert(pet_number, rows)
    }

    pub fn remove_spell_cooldowns_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellCooldownRowLikeCpp>> {
        self.spell_cooldowns.remove(&pet_number)
    }

    pub fn spell_charges_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.spell_charges.get(&pet_number)
    }

    pub fn insert_spell_charges_for_pet_number(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetSpellChargeRowLikeCpp>,
    ) -> Option<Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.spell_charges.insert(pet_number, rows)
    }

    pub fn remove_spell_charges_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetSpellChargeRowLikeCpp>> {
        self.spell_charges.remove(&pet_number)
    }

    pub fn auras_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetAuraRowLikeCpp>> {
        self.auras.get(&pet_number)
    }

    pub fn insert_auras_for_pet_number(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetAuraRowLikeCpp>,
    ) -> Option<Vec<CharacterPetAuraRowLikeCpp>> {
        self.auras.insert(pet_number, rows)
    }

    pub fn remove_auras_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetAuraRowLikeCpp>> {
        self.auras.remove(&pet_number)
    }

    pub fn aura_effects_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.aura_effects.get(&pet_number)
    }

    pub fn insert_aura_effects_for_pet_number(
        &mut self,
        pet_number: u32,
        rows: Vec<CharacterPetAuraEffectRowLikeCpp>,
    ) -> Option<Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.aura_effects.insert(pet_number, rows)
    }

    pub fn remove_aura_effects_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<Vec<CharacterPetAuraEffectRowLikeCpp>> {
        self.aura_effects.remove(&pet_number)
    }

    pub fn declined_names_for_pet_number(
        &self,
        pet_number: u32,
    ) -> Option<&CharacterPetDeclinedNamesRowLikeCpp> {
        self.declined_names.get(&pet_number)
    }

    pub fn insert_declined_names_for_pet_number(
        &mut self,
        pet_number: u32,
        row: CharacterPetDeclinedNamesRowLikeCpp,
    ) -> Option<CharacterPetDeclinedNamesRowLikeCpp> {
        self.declined_names.insert(pet_number, row)
    }

    pub fn remove_declined_names_for_pet_number(
        &mut self,
        pet_number: u32,
    ) -> Option<CharacterPetDeclinedNamesRowLikeCpp> {
        self.declined_names.remove(&pet_number)
    }
}
