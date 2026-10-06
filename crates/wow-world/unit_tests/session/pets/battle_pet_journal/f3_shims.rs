// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub fn set_battle_pet_breed_quality_store(&mut self, store: Arc<BattlePetBreedQualityStore>) {
        self.fixtures.pets.set_battle_pet_breed_quality_store(store)
    }
    #[cfg(test)]
    pub(crate) fn set_represented_battle_pet_xp_per_level_like_cpp(
        &mut self,
        level: u16,
        xp_per_level: u16,
    ) {
        self.fixtures
            .pets
            .set_represented_battle_pet_xp_per_level_like_cpp(level, xp_per_level)
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        self.fixtures
            .pets
            .represented_battle_pet_level_criteria_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_active_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        self.fixtures
            .pets
            .represented_battle_pet_active_level_criteria_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_modify_name_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        name: String,
        declined_names: Option<wow_packet::packets::misc::DeclinedNamesLikeCpp>,
        timestamp: i64,
    ) -> bool {
        crate::session::cx_pets(self).battle_pet_modify_name_like_cpp(
            pet_guid,
            name,
            declined_names,
            timestamp,
        )
    }
    pub(crate) fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        crate::session::cx_pets_ref(self).has_represented_battle_pet_journal_lock_like_cpp()
    }
    pub(crate) fn represented_battle_pet_journal_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::BattlePetJournal> {
        wow_world_application::represented_battle_pet_journal_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
        )
    }
    pub(crate) async fn battle_pet_try_acquire_journal_lease_like_cpp(&self) -> bool {
        crate::session::cx_pets_ref(self)
            .battle_pet_try_acquire_journal_lease_like_cpp()
            .await
    }
}
