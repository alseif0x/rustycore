// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(crate) fn battle_pet_selection_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>> {
        self.fixtures.pets.battle_pet_selection_store_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_battle_pet_purchase_selection_override_like_cpp(
        &mut self,
        selection: Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    ) {
        self.fixtures
            .pets
            .set_battle_pet_purchase_selection_override_like_cpp(selection)
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_purchase_selection_override_like_cpp(
        &self,
    ) -> Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp> {
        self.fixtures
            .pets
            .battle_pet_purchase_selection_override_like_cpp()
    }
    #[cfg(test)]
    pub fn set_battle_pet_breed_state_store(&mut self, store: Arc<BattlePetBreedStateStore>) {
        self.fixtures.pets.set_battle_pet_breed_state_store(store)
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_store(&mut self, store: Arc<BattlePetSpeciesStore>) {
        self.fixtures.pets.set_battle_pet_species_store(store)
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_state_store(&mut self, store: Arc<BattlePetSpeciesStateStore>) {
        self.fixtures.pets.set_battle_pet_species_state_store(store)
    }
    #[cfg(test)]
    pub fn set_battle_pet_xp_game_table(&mut self, table: Arc<BattlePetXpGameTableLikeCpp>) {
        self.fixtures.pets.set_battle_pet_xp_game_table(table)
    }
    #[cfg(test)]
    pub(crate) fn add_represented_battle_pet_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        save_info: RepresentedBattlePetSaveInfoLikeCpp,
    ) {
        self.fixtures
            .pets
            .add_represented_battle_pet_like_cpp(pet_guid, flags, save_info)
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_clear_fanfare_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        self.fixtures
            .pets
            .battle_pet_clear_fanfare_like_cpp(pet_guid)
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_set_flags_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> bool {
        self.fixtures
            .pets
            .battle_pet_set_flags_like_cpp(pet_guid, flags, control_type)
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_heal_battle_pets_pct_like_cpp(&mut self, pct: u8) -> usize {
        crate::session::hub_mut(self).battle_pet_heal_battle_pets_pct_like_cpp(pct)
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_unique_owned_criteria_like_cpp(&self) -> u32 {
        self.fixtures
            .pets
            .represented_battle_pet_unique_owned_criteria_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_learned_new_pet_criteria_like_cpp(&self) -> &[u32] {
        self.fixtures
            .pets
            .represented_battle_pet_learned_new_pet_criteria_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_cage_items_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetCageItemLikeCpp] {
        self.fixtures
            .pets
            .represented_battle_pet_cage_items_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_represented_battle_pet_query_companion_like_cpp(
        &mut self,
        unit_guid: ObjectGuid,
        companion: RepresentedBattlePetQueryCompanionLikeCpp,
    ) {
        self.fixtures
            .pets
            .set_represented_battle_pet_query_companion_like_cpp(unit_guid, companion)
    }
}
