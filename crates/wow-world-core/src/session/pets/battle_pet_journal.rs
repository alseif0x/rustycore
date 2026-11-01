#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::battle_pet_adapter::RepresentedBattlePetLevelCriteriaLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use std::sync::Arc;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::BattlePetBreedQualityStore;

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_battle_pet_breed_quality_store(&mut self, store: Arc<BattlePetBreedQualityStore>) {
        self.battle_pet_test_fixture_like_cpp
            .battle_pet_breed_quality_store = Some(store);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_represented_battle_pet_xp_per_level_like_cpp(
        &mut self,
        level: u16,
        xp_per_level: u16,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_xp_per_level_like_cpp
            .insert(level, xp_per_level);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_level_criteria_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_battle_pet_active_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_active_level_criteria_like_cpp
    }
}
