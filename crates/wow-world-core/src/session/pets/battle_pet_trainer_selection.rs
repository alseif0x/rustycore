use wow_data::battle_pet_selection::{
    BattlePetTrainerSelectionLikeCpp, select_battle_pet_trainer_pet_like_cpp,
};

impl crate::session::HubRef<'_> {
    /// The C++ `AddPet` materialization inputs for one admission, frozen
    /// into the durable command so recovery never re-rolls.
    pub fn battle_pet_trainer_selection_like_cpp(
        &self,
        store: &wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp,
        species_entry: &wow_data::BattlePetSpeciesEntry,
    ) -> Option<BattlePetTrainerSelectionLikeCpp> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(selection) = self
            .fixtures
            .pets
            .battle_pet_purchase_selection_override_like_cpp()
        {
            return Some(selection);
        }
        let template = self
            .catalogs
            .creature_template_lifecycle_store_like_cpp()
            .and_then(|templates| {
                u32::try_from(species_entry.creature_id)
                    .ok()
                    .and_then(|entry| templates.get(entry))
            });
        let mut breed_random = rand::thread_rng();
        let mut display_random = rand::thread_rng();
        Some(select_battle_pet_trainer_pet_like_cpp(
            store,
            species_entry,
            template,
            &mut breed_random,
            &mut display_random,
        ))
    }
}
