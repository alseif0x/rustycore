//! Represented battle-pet journal entries and their attributes.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// C++ `BattlePetMgr::ChangeBattlePetQuality`. `BattlePet::CalculateStats`
    /// may return early when breed-state DB2 rows are missing; that does not
    /// abort the quality change.
    #[cfg(test)]
    pub(crate) fn battle_pet_change_battle_pet_quality_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        quality: u8,
    ) -> RepresentedBattlePetQualityOutcomeLikeCpp {
        if !crate::session::cx_pets_ref(self).has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
        else {
            return RepresentedBattlePetQualityOutcomeLikeCpp::UnknownPet;
        };

        if quality > BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP {
            return RepresentedBattlePetQualityOutcomeLikeCpp::QualityAboveRare;
        }

        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetQualityOutcomeLikeCpp::CantBattle;
        }

        if pet.quality >= quality {
            return RepresentedBattlePetQualityOutcomeLikeCpp::NotUpgrade;
        }

        let breed = pet.breed;
        let species = pet.species;
        let level = pet.level;
        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);

        let pet = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before stats calculation");
        pet.quality = quality;
        crate::session::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }

        self.send_battle_pet_updates_like_cpp(&[pet_guid], false);
        RepresentedBattlePetQualityOutcomeLikeCpp::Changed
    }
    /// C++ `BattlePetMgr::GrantBattlePetLevel`. `BattlePet::CalculateStats`
    /// may return early when breed-state DB2 rows are missing; that does not
    /// abort the level grant.
    #[cfg(test)]
    pub(crate) fn battle_pet_grant_battle_pet_level_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        granted_levels: u16,
    ) -> RepresentedBattlePetGrantLevelOutcomeLikeCpp {
        if !crate::session::cx_pets_ref(self).has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
        else {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet;
        };

        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::CantBattle;
        }

        let mut level = pet.level;
        if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::AlreadyMaxLevel;
        }

        if granted_levels == 0 {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoGrantedLevels;
        }

        let species = pet.species;
        let breed = pet.breed;
        let quality = pet.quality;
        let mut remaining_levels = granted_levels;
        while remaining_levels > 0 && level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            level += 1;
            remaining_levels -= 1;
            #[cfg(test)]
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_level_criteria_like_cpp
                .push(RepresentedBattlePetLevelCriteriaLikeCpp { species, level });
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);

        let pet = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before stats calculation");
        pet.level = level;
        if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            pet.exp = 0;
        }
        crate::session::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }

        self.send_battle_pet_updates_like_cpp(&[pet_guid], false);
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::Changed
    }
    pub(crate) async fn battle_pet_grant_level_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        granted_levels: u16,
    ) -> RepresentedBattlePetGrantLevelOutcomeLikeCpp {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(test)]
            return self
                .battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, granted_levels);
            #[cfg(not(test))]
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock;
        };
        if !attachment.has_lease_like_cpp() {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock;
        }
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        let Some(pet) = owner.pet_snapshot_like_cpp(pet_guid) else {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet;
        };
        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::CantBattle;
        }
        if pet.level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::AlreadyMaxLevel;
        }
        if granted_levels == 0 {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoGrantedLevels;
        }
        let level = pet
            .level
            .saturating_add(granted_levels)
            .min(MAX_BATTLE_PET_LEVEL_LIKE_CPP);
        #[cfg(test)]
        let criteria: Vec<_> = ((pet.level + 1)..=level)
            .map(|level| RepresentedBattlePetLevelCriteriaLikeCpp {
                species: pet.species,
                level,
            })
            .collect();
        let calculated =
            self.battle_pet_calculate_stats_like_cpp(pet.breed, pet.species, pet.quality, level);
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                pet.level = level;
                if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
                    pet.exp = 0;
                }
                crate::session::apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
            })
            .await
        {
            Ok(((), packet)) => {
                #[cfg(test)]
                self.fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_level_criteria_like_cpp
                    .extend(criteria);
                self.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![packet],
                    pet_added: false,
                });
                RepresentedBattlePetGrantLevelOutcomeLikeCpp::Changed
            }
            Err(
                BattlePetMutationFailureLikeCpp::MissingAuthority
                | BattlePetMutationFailureLikeCpp::JournalLocked,
            ) => RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock,
            Err(BattlePetMutationFailureLikeCpp::UnknownPet) => {
                RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet
            }
            Err(error) => {
                crate::session::hub_ref(self).log_battle_pet_mutation_failure_like_cpp(
                    "grant level",
                    pet_guid,
                    &error,
                );
                RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet
            }
        }
    }
}

impl crate::session::PetsCx<'_> {
    /// C++ `BattlePetMgr::SendJournalLockStatus`, represented as the successful
    /// local acquisition path until the global world journal-lock owner exists.
    ///
    /// Thin World wrapper over the application owner's shared helper (#1263 F5);
    /// the login path (`handlers/character/world_entry.rs`) still calls it.
    pub(crate) async fn send_battle_pet_journal_lock_status_like_cpp(&mut self) {
        wow_world_application::send_battle_pet_journal_lock_status_like_cpp(
            &mut self.hub,
            self.lifecycle,
            cfg!(test),
        )
        .await;
    }

    /// C++ `BattlePetMgr::ModifyName`, represented without live summoned-creature
    /// timestamp propagation until the companion creature runtime exists.
    #[cfg(test)]
    pub(crate) fn battle_pet_modify_name_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        name: String,
        declined_names: Option<wow_packet::packets::misc::DeclinedNamesLikeCpp>,
        timestamp: i64,
    ) -> bool {
        if !self
            .shared()
            .has_represented_battle_pet_journal_lock_like_cpp()
        {
            return false;
        }

        let Some(pet) = self
            .hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        pet.name = name;
        pet.name_timestamp = timestamp;
        pet.declined_names = declined_names;
        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }
        true
    }

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
                self.hub.shared().log_battle_pet_mutation_failure_like_cpp(
                    "modify name",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }
}

impl crate::session::PetsCxRef<'_> {
    /// Acquire this session's journal lease through the #160 attachment
    /// (issue #161 admission/recovery).
    pub(crate) async fn battle_pet_try_acquire_journal_lease_like_cpp(&self) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            return false;
        };
        attachment.try_acquire_lease_like_cpp().await
    }

    /// One cloned DB2 species row for admission-time materialization (#161).
    pub(crate) fn battle_pet_species_entry_like_cpp(
        &self,
        species: u32,
    ) -> Option<wow_data::BattlePetSpeciesEntry> {
        if let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() {
            return attachment.owner_like_cpp().species_entry_like_cpp(species);
        }
        #[cfg(test)]
        return self
            .hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned();
        #[cfg(not(test))]
        None
    }

    pub(in crate::session) fn battle_pet_xp_per_level_like_cpp(&self, level: u16) -> Option<u16> {
        let canonical = self
            .lifecycle
            .battle_pet_account_attachment_like_cpp()
            .and_then(|attachment| attachment.owner_like_cpp().xp_per_level_like_cpp(level));
        #[cfg(test)]
        return canonical.or_else(|| {
            self.hub
                .fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .battle_pet_xp_game_table
                .as_ref()
                .and_then(|table| table.xp_per_level_like_cpp(level))
                .or_else(|| {
                    self.hub
                        .fixtures
                        .pets
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_xp_per_level_like_cpp
                        .get(&level)
                        .copied()
                })
        });
        #[cfg(not(test))]
        canonical
    }

    /// C++ `BattlePetMgr::HasJournalLock`.
    ///
    /// Thin World wrapper over the application owner's shared helper (#1263 F5);
    /// the battle-pet spell-effect admission path
    /// (`session/spell_effects/checks.rs`) still calls it.
    pub(crate) fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        wow_world_application::has_represented_battle_pet_journal_lock_like_cpp(
            self.hub,
            self.lifecycle,
            cfg!(test),
        )
    }

    /// C++ `BattlePetMgr::GetMaxPetLevel`.
    pub(crate) fn battle_pet_max_pet_level_like_cpp(&self) -> Option<u16> {
        if let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() {
            return Some(attachment.owner_like_cpp().max_pet_level_like_cpp());
        }
        #[cfg(test)]
        return Some(
            self.hub
                .fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pets_like_cpp
                .values()
                .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
                .map(|pet| pet.level)
                .max()
                .unwrap_or(0),
        );
        #[cfg(not(test))]
        None
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/pets/battle_pet_journal/f3_shims.rs"]
mod f3_shims;
