//! Represented battle-pet journal entries and their attributes.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Acquire this session's journal lease through the #160 attachment
    /// (issue #161 admission/recovery).
    pub(crate) async fn battle_pet_try_acquire_journal_lease_like_cpp(&self) -> bool {
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
            return false;
        };
        attachment.try_acquire_lease_like_cpp().await
    }
    /// One cloned DB2 species row for admission-time materialization (#161).
    pub(crate) fn battle_pet_species_entry_like_cpp(
        &self,
        species: u32,
    ) -> Option<wow_data::BattlePetSpeciesEntry> {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment.owner_like_cpp().species_entry_like_cpp(species);
        }
        #[cfg(test)]
        return self
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned();
        #[cfg(not(test))]
        None
    }
    #[cfg(test)]
    pub fn set_battle_pet_breed_quality_store(&mut self, store: Arc<BattlePetBreedQualityStore>) {
        self.battle_pet_breed_quality_store = Some(store);
    }
    pub(in crate::session) fn battle_pet_xp_per_level_like_cpp(&self, level: u16) -> Option<u16> {
        let canonical = self
            .battle_pet_account_attachment_like_cpp
            .as_ref()
            .and_then(|attachment| attachment.owner_like_cpp().xp_per_level_like_cpp(level));
        #[cfg(test)]
        return canonical.or_else(|| {
            self.battle_pet_xp_game_table
                .as_ref()
                .and_then(|table| table.xp_per_level_like_cpp(level))
                .or_else(|| {
                    self.represented_battle_pet_xp_per_level_like_cpp
                        .get(&level)
                        .copied()
                })
        });
        #[cfg(not(test))]
        canonical
    }
    /// C++ `BattlePetMgr::HasJournalLock`.
    pub(crate) fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment.has_lease_like_cpp();
        }
        #[cfg(test)]
        return self.represented_battle_pet_journal_lock_like_cpp;
        #[cfg(not(test))]
        false
    }
    /// C++ `BattlePetMgr::SendJournalLockStatus`, represented as the successful
    /// local acquisition path until the global world journal-lock owner exists.
    pub(crate) async fn send_battle_pet_journal_lock_status_like_cpp(&mut self) {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            let acquired = attachment.try_acquire_lease_like_cpp().await;
            if acquired {
                self.send_packet_realm(&wow_packet::packets::misc::BattlePetJournalLockAcquired);
            } else {
                self.send_packet_realm(&wow_packet::packets::misc::BattlePetJournalLockDenied);
            }
            return;
        }
        #[cfg(test)]
        {
            self.represented_battle_pet_journal_lock_like_cpp = true;
            self.send_packet_realm(&wow_packet::packets::misc::BattlePetJournalLockAcquired);
        }
        #[cfg(not(test))]
        self.send_packet_realm(&wow_packet::packets::misc::BattlePetJournalLockDenied);
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
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return false;
        }

        let Some(pet) = self.represented_battle_pets_like_cpp.get_mut(&pet_guid) else {
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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
                self.log_battle_pet_mutation_failure_like_cpp("modify name", pet_guid, &error);
                false
            }
        }
    }
    /// C++ `BattlePetMgr::GetMaxPetLevel`.
    pub(crate) fn battle_pet_max_pet_level_like_cpp(&self) -> Option<u16> {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return Some(attachment.owner_like_cpp().max_pet_level_like_cpp());
        }
        #[cfg(test)]
        return Some(
            self.represented_battle_pets_like_cpp
                .values()
                .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
                .map(|pet| pet.level)
                .max()
                .unwrap_or(0),
        );
        #[cfg(not(test))]
        None
    }
    /// C++ `BattlePetMgr::ChangeBattlePetQuality`. `BattlePet::CalculateStats`
    /// may return early when breed-state DB2 rows are missing; that does not
    /// abort the quality change.
    #[cfg(test)]
    pub(crate) fn battle_pet_change_battle_pet_quality_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        quality: u8,
    ) -> RepresentedBattlePetQualityOutcomeLikeCpp {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self.represented_battle_pets_like_cpp.get(&pet_guid) else {
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
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before stats calculation");
        pet.quality = quality;
        crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }

        self.send_battle_pet_updates_like_cpp(&[pet_guid], false);
        RepresentedBattlePetQualityOutcomeLikeCpp::Changed
    }
    pub(crate) async fn battle_pet_change_quality_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        quality: u8,
    ) -> RepresentedBattlePetQualityOutcomeLikeCpp {
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self
                .battle_pet_change_battle_pet_quality_represented_like_cpp(pet_guid, quality);
            #[cfg(not(test))]
            return RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock;
        };
        if !attachment.has_lease_like_cpp() {
            return RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock;
        }
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        let Some(pet) = owner.pet_snapshot_like_cpp(pet_guid) else {
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
        let calculated =
            self.battle_pet_calculate_stats_like_cpp(pet.breed, pet.species, quality, pet.level);
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                pet.quality = quality;
                crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
            })
            .await
        {
            Ok(((), packet)) => {
                self.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![packet],
                    pet_added: false,
                });
                RepresentedBattlePetQualityOutcomeLikeCpp::Changed
            }
            Err(
                BattlePetMutationFailureLikeCpp::MissingAuthority
                | BattlePetMutationFailureLikeCpp::JournalLocked,
            ) => RepresentedBattlePetQualityOutcomeLikeCpp::NoJournalLock,
            Err(BattlePetMutationFailureLikeCpp::UnknownPet) => {
                RepresentedBattlePetQualityOutcomeLikeCpp::UnknownPet
            }
            Err(error) => {
                self.log_battle_pet_mutation_failure_like_cpp("change quality", pet_guid, &error);
                RepresentedBattlePetQualityOutcomeLikeCpp::UnknownPet
            }
        }
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
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetGrantLevelOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self.represented_battle_pets_like_cpp.get(&pet_guid) else {
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
            self.represented_battle_pet_level_criteria_like_cpp
                .push(RepresentedBattlePetLevelCriteriaLikeCpp { species, level });
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);

        let pet = self
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before stats calculation");
        pet.level = level;
        if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            pet.exp = 0;
        }
        crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
                crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
            })
            .await
        {
            Ok(((), packet)) => {
                #[cfg(test)]
                self.represented_battle_pet_level_criteria_like_cpp
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
                self.log_battle_pet_mutation_failure_like_cpp("grant level", pet_guid, &error);
                RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn set_represented_battle_pet_xp_per_level_like_cpp(
        &mut self,
        level: u16,
        xp_per_level: u16,
    ) {
        self.represented_battle_pet_xp_per_level_like_cpp
            .insert(level, xp_per_level);
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        &self.represented_battle_pet_level_criteria_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_active_level_criteria_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetLevelCriteriaLikeCpp] {
        &self.represented_battle_pet_active_level_criteria_like_cpp
    }
    /// C++ `BattlePetMgr::SendJournal` packet body builder.
    pub(crate) fn represented_battle_pet_journal_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::BattlePetJournal> {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return Some(
                attachment
                    .owner_like_cpp()
                    .journal_like_cpp(attachment.lease_id_like_cpp(), self.player_guid()),
            );
        }
        #[cfg(not(test))]
        return None;
        #[cfg(test)]
        {
            let player_guid = self.player_guid();
            let mut journal = wow_packet::packets::misc::BattlePetJournal {
                trap: 0,
                has_journal_lock: self.has_represented_battle_pet_journal_lock_like_cpp(),
                slots: Vec::with_capacity(BATTLE_PET_SLOT_COUNT_LIKE_CPP),
                pets: Vec::new(),
            };

            for (pet_guid, pet) in &self.represented_battle_pets_like_cpp {
                if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                    continue;
                }

                if pet
                    .owner_info
                    .is_some_and(|owner_info| Some(owner_info.guid) != player_guid)
                {
                    continue;
                }

                journal.pets.push(pet.packet_info_like_cpp(*pet_guid));
            }

            for slot in &self.represented_battle_pet_slots_like_cpp {
                let mut packet_slot = slot.packet_slot_like_cpp();
                if packet_slot.pet_guid
                    != wow_packet::packets::misc::empty_battle_pet_guid_like_cpp()
                    && !self
                        .represented_battle_pets_like_cpp
                        .contains_key(&packet_slot.pet_guid)
                {
                    packet_slot.pet_guid =
                        wow_packet::packets::misc::empty_battle_pet_guid_like_cpp();
                }
                journal.slots.push(packet_slot);
            }

            Some(journal)
        }
    }
}
