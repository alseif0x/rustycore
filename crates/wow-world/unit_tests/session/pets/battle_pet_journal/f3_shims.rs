// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use crate::session::*;

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
    pub(crate) fn represented_battle_pet_journal_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::BattlePetJournal> {
        wow_world_inventory::represented_battle_pet_journal_like_cpp(
            crate::session::hub_ref(self),
            &self.lifecycle,
            cfg!(test),
        )
    }
}

impl crate::session::WorldSession {
    pub(crate) async fn battle_pet_modify_name_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        name: String,
        declined_names: Option<wow_packet::packets::misc::DeclinedNamesLikeCpp>,
        timestamp: i64,
    ) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(test)]
            return self.battle_pet_modify_name_like_cpp(pet_guid, name, declined_names, timestamp);
            #[cfg(not(test))]
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                pet.name = name;
                pet.name_timestamp = timestamp;
                pet.declined_names = declined_names;
            })
            .await
        {
            Ok(_) => true,
            Err(error) => {
                crate::session::hub_ref(self).log_battle_pet_mutation_failure_like_cpp(
                    "modify name",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }
}
