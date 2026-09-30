//! Battle-pet experience progression and acquisition criteria.

use super::super::*;

impl WorldSession {
    /// Record the C++ `BattlePetMgr::AddPet` criteria hooks from durable
    /// current state. Both C++ criteria are set-like (`UniquePetsOwned` uses
    /// the current unique-species count and `LearnedNewPet` sets one species
    /// to 1), so receipt recovery and packet re-sends are idempotent.
    pub(crate) fn record_battle_pet_trainer_purchase_criteria_like_cpp(&mut self, species: u32) {
        #[cfg(not(test))]
        let _ = species;
        #[cfg(test)]
        {
            self.battle_pet_test_fixture_like_cpp
                .represented_battle_pet_unique_owned_criteria_like_cpp = self
                .lifecycle
                .battle_pet_account_attachment_like_cpp
                .as_ref()
                .map(|attachment| attachment.owner_like_cpp().unique_species_count_like_cpp())
                .unwrap_or_else(|| {
                    u32::try_from(
                        self.battle_pet_test_fixture_like_cpp
                            .represented_battle_pets_like_cpp
                            .values()
                            .map(|pet| pet.species)
                            .collect::<BTreeSet<_>>()
                            .len(),
                    )
                    .unwrap_or(u32::MAX)
                });
            if !self
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_learned_new_pet_criteria_like_cpp
                .contains(&species)
            {
                self.battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_learned_new_pet_criteria_like_cpp
                    .push(species);
            }
        }
    }
    /// C++ `BattlePetMgr::HealBattlePetsPct`.
    ///
    /// Fidelity note: legacy C++ does not skip removed pets and would rewrite a
    /// damaged `BATTLE_PET_REMOVED` row to `BATTLE_PET_CHANGED`. Rust keeps the
    /// represented removed row immutable here; if we decide to patch that legacy
    /// bug upstream, this is the intended shared behavior.
    #[cfg(test)]
    pub(crate) fn battle_pet_heal_battle_pets_pct_like_cpp(&mut self, pct: u8) -> usize {
        let mut updated = Vec::new();

        for (pet_guid, pet) in &mut self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
        {
            if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                continue;
            }

            if pet.health == pet.max_health {
                continue;
            }

            let heal = (pet.max_health as f32 * f32::from(pct) / 100.0f32) as u32;
            pet.health = pet.health.saturating_add(heal).min(pet.max_health);
            if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
                pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
            }
            updated.push(*pet_guid);
        }

        self.send_battle_pet_updates_like_cpp(&updated, false)
    }
    /// C++ `BattlePetMgr::GrantBattlePetExperience`, represented after
    /// external aura multiplier resolution.
    #[cfg(test)]
    pub(crate) fn battle_pet_grant_battle_pet_experience_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        xp: u16,
        xp_source: RepresentedBattlePetXpSourceLikeCpp,
        pet_battle_xp_multiplier: f32,
    ) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
        else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet;
        };

        if xp == 0 || xp_source == RepresentedBattlePetXpSourceLikeCpp::Invalid {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::InvalidXpOrSource;
        }

        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::CantBattle;
        }

        let mut level = pet.level;
        if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::AlreadyMaxLevel;
        }

        let Some(mut next_level_xp) = self.battle_pet_xp_per_level_like_cpp(level) else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
        };

        let species = pet.species;
        let breed = pet.breed;
        let quality = pet.quality;
        let mut total_xp = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
            (f32::from(xp) * pet_battle_xp_multiplier) as u16
        } else {
            xp
        };
        total_xp = total_xp.saturating_add(pet.exp);

        while total_xp >= next_level_xp && level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            total_xp = total_xp.saturating_sub(next_level_xp);
            level += 1;

            let Some(row_xp) = self.battle_pet_xp_per_level_like_cpp(level) else {
                return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
            };
            next_level_xp = row_xp;

            #[cfg(test)]
            {
                let criteria = RepresentedBattlePetLevelCriteriaLikeCpp { species, level };
                self.battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_level_criteria_like_cpp
                    .push(criteria);
                if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                    self.battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_active_level_criteria_like_cpp
                        .push(criteria);
                }
            }
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);

        let pet = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before XP calculation");
        pet.level = level;
        pet.exp = if level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            total_xp
        } else {
            0
        };
        crate::session::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }

        self.send_battle_pet_updates_like_cpp(&[pet_guid], false);
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    }
    pub(crate) async fn battle_pet_grant_experience_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        xp: u16,
        xp_source: RepresentedBattlePetXpSourceLikeCpp,
        pet_battle_xp_multiplier: f32,
    ) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self.battle_pet_grant_battle_pet_experience_represented_like_cpp(
                pet_guid,
                xp,
                xp_source,
                pet_battle_xp_multiplier,
            );
            #[cfg(not(test))]
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::NoJournalLock;
        };
        if !attachment.has_lease_like_cpp() {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::NoJournalLock;
        }
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        let Some(pet) = owner.pet_snapshot_like_cpp(pet_guid) else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet;
        };
        if xp == 0 || xp_source == RepresentedBattlePetXpSourceLikeCpp::Invalid {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::InvalidXpOrSource;
        }
        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::CantBattle;
        }
        if pet.level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::AlreadyMaxLevel;
        }
        let Some(mut next_level_xp) = self.battle_pet_xp_per_level_like_cpp(pet.level) else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
        };
        let mut level = pet.level;
        let mut total_xp = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
            (f32::from(xp) * pet_battle_xp_multiplier) as u16
        } else {
            xp
        };
        total_xp = total_xp.saturating_add(pet.exp);
        #[cfg(test)]
        let mut criteria = Vec::new();
        while total_xp >= next_level_xp && level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            total_xp = total_xp.saturating_sub(next_level_xp);
            level += 1;
            let Some(row_xp) = self.battle_pet_xp_per_level_like_cpp(level) else {
                return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
            };
            next_level_xp = row_xp;
            #[cfg(test)]
            criteria.push(RepresentedBattlePetLevelCriteriaLikeCpp {
                species: pet.species,
                level,
            });
        }
        let calculated =
            self.battle_pet_calculate_stats_like_cpp(pet.breed, pet.species, pet.quality, level);
        let persisted_exp = if level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            total_xp
        } else {
            0
        };
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                pet.level = level;
                pet.exp = persisted_exp;
                crate::session::apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
            })
            .await
        {
            Ok(((), packet)) => {
                #[cfg(test)]
                {
                    self.battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_level_criteria_like_cpp
                        .extend(criteria.iter().copied());
                    if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                        self.battle_pet_test_fixture_like_cpp
                            .represented_battle_pet_active_level_criteria_like_cpp
                            .extend(criteria);
                    }
                }
                self.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![packet],
                    pet_added: false,
                });
                RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
            }
            Err(
                BattlePetMutationFailureLikeCpp::MissingAuthority
                | BattlePetMutationFailureLikeCpp::JournalLocked,
            ) => RepresentedBattlePetGrantExperienceOutcomeLikeCpp::NoJournalLock,
            Err(BattlePetMutationFailureLikeCpp::UnknownPet) => {
                RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet
            }
            Err(error) => {
                self.log_battle_pet_mutation_failure_like_cpp("grant experience", pet_guid, &error);
                RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_unique_owned_criteria_like_cpp(&self) -> u32 {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_unique_owned_criteria_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_learned_new_pet_criteria_like_cpp(&self) -> &[u32] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_learned_new_pet_criteria_like_cpp
    }
}
