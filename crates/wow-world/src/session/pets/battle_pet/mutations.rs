//! Battle-pet represented mutations and durable journal operations.

use super::super::*;

impl WorldSession {
    /// Test/setup seam for represented `BattlePetMgr::_pets`.
    #[cfg(test)]
    pub(crate) fn add_represented_battle_pet_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        save_info: RepresentedBattlePetSaveInfoLikeCpp,
    ) {
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .insert(
                pet_guid,
                RepresentedBattlePetDataLikeCpp::minimal_like_cpp(flags, save_info),
            );
    }
    /// C++ `BattlePetMgr::ClearFanfare`.
    #[cfg(test)]
    pub(crate) fn battle_pet_clear_fanfare_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        pet.flags &= !BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }
        true
    }
    pub(crate) async fn battle_pet_clear_fanfare_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
    ) -> bool {
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self.battle_pet_clear_fanfare_like_cpp(pet_guid);
            #[cfg(not(test))]
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        match owner
            .try_mutate_pet_without_lease_like_cpp(pet_guid, |pet| {
                pet.flags &= !BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
            })
            .await
        {
            Ok(_) => true,
            Err(error) => {
                self.log_battle_pet_mutation_failure_like_cpp("clear fanfare", pet_guid, &error);
                false
            }
        }
    }
    /// C++ `BattlePetMgr::RemovePet`.
    #[cfg(test)]
    pub(crate) fn battle_pet_remove_pet_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return false;
        }

        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Removed;
        true
    }
    pub(crate) async fn battle_pet_remove_pet_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
    ) -> bool {
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self.battle_pet_remove_pet_like_cpp(pet_guid);
            #[cfg(not(test))]
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner.try_remove_pet_like_cpp(lease, pet_guid).await {
            Ok(()) => true,
            Err(error) => {
                self.log_battle_pet_mutation_failure_like_cpp("remove", pet_guid, &error);
                false
            }
        }
    }
    /// C++ `BattlePetMgr::CageBattlePet`, represented at the battle-pet state
    /// boundary. Inventory placement is still external: the caller must pass
    /// the already-resolved `CanStoreNewItem`/`StoreNewItem` outcomes.
    #[cfg(test)]
    pub(crate) fn battle_pet_cage_battle_pet_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        inventory_can_store: bool,
        item_stored: bool,
    ) -> RepresentedBattlePetCageOutcomeLikeCpp {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return RepresentedBattlePetCageOutcomeLikeCpp::NoJournalLock;
        }

        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
            .cloned()
        else {
            return RepresentedBattlePetCageOutcomeLikeCpp::UnknownPet;
        };

        if self.battle_pet_species_has_flag_like_cpp(
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetCageOutcomeLikeCpp::NotTradable;
        }

        if self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
            .iter()
            .any(|slot| slot.pet_guid == Some(pet_guid))
        {
            return RepresentedBattlePetCageOutcomeLikeCpp::InBattleSlot;
        }

        if pet.health < pet.max_health {
            return RepresentedBattlePetCageOutcomeLikeCpp::Damaged;
        }

        if !inventory_can_store {
            return RepresentedBattlePetCageOutcomeLikeCpp::InventoryUnavailable;
        }

        if !item_stored {
            return RepresentedBattlePetCageOutcomeLikeCpp::StoreFailed;
        }

        let cage_item = RepresentedBattlePetCageItemLikeCpp {
            item_id: BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
            species_id: pet.species,
            breed_data: u32::from(pet.breed) | (u32::from(pet.quality) << 24),
            level: pet.level,
            display_id: pet.display_id,
        };
        #[cfg(test)]
        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pet_cage_items_like_cpp
            .push(cage_item);
        #[cfg(not(test))]
        let _ = cage_item;

        let _ = self.battle_pet_remove_pet_like_cpp(pet_guid);
        self.send_packet(&wow_packet::packets::misc::BattlePetDeleted { pet_guid });

        if self.represented_summoned_battle_pet_guid_like_cpp() == Some(pet_guid) {
            let _cleared = self.mutate_canonical_player_like_cpp(|player| {
                player.clear_battle_pet_data_like_cpp();
            });
            #[cfg(test)]
            if _cleared.is_none() {
                self.battle_pet_test_fixture_like_cpp
                    .represented_summoned_battle_pet_guid_like_cpp = None;
            }
        }

        RepresentedBattlePetCageOutcomeLikeCpp::Caged(cage_item)
    }
    /// C++ `WorldSession::HandleBattlePetSetFlags` flag mutation.
    #[cfg(test)]
    pub(crate) fn battle_pet_set_flags_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> bool {
        let Some(pet) = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        if control_type == BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP {
            pet.flags |= flags;
        } else {
            pet.flags &= !flags;
        }

        if pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::New {
            pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Changed;
        }
        true
    }
    pub(crate) async fn battle_pet_set_flags_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> bool {
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self.battle_pet_set_flags_like_cpp(pet_guid, flags, control_type);
            #[cfg(not(test))]
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                if control_type == BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP {
                    pet.flags |= flags;
                } else {
                    pet.flags &= !flags;
                }
            })
            .await
        {
            Ok(_) => true,
            Err(error) => {
                self.log_battle_pet_mutation_failure_like_cpp("set flags", pet_guid, &error);
                false
            }
        }
    }
    pub(in crate::session) fn log_battle_pet_mutation_failure_like_cpp(
        &self,
        operation: &'static str,
        pet_guid: ObjectGuid,
        error: &BattlePetMutationFailureLikeCpp,
    ) {
        if matches!(
            error,
            BattlePetMutationFailureLikeCpp::MissingAuthority
                | BattlePetMutationFailureLikeCpp::JournalLocked
                | BattlePetMutationFailureLikeCpp::UnknownPet
        ) {
            return;
        }
        warn!(
            account = self.account_id,
            ?pet_guid,
            ?error,
            operation,
            "Durable battle-pet mutation failed"
        );
    }
    /// C++ `BattlePetMgr::GetPetCount`.
    #[cfg(test)]
    pub(crate) fn battle_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> u8 {
        if let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .pet_count_like_cpp(species, owner_guid);
        }
        let species_flags = self
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .map(|entry| entry.flags)
            .unwrap_or(0);
        let not_account_wide =
            species_flags & wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP != 0;

        let count = self
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .values()
            .filter(|pet| pet.species == species)
            .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .filter(|pet| {
                if not_account_wide {
                    if let (Some(owner_guid), Some(owner_info)) = (owner_guid, pet.owner_info) {
                        return owner_info.guid == owner_guid;
                    }
                }
                true
            })
            .count();

        u8::try_from(count).unwrap_or(u8::MAX)
    }
    /// C++ `BattlePetMgr::HasMaxPetCount`.
    pub(crate) fn battle_pet_has_max_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> Option<bool> {
        if let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp {
            return Some(
                attachment
                    .owner_like_cpp()
                    .has_max_pet_count_like_cpp(species, owner_guid),
            );
        }
        #[cfg(test)]
        {
            let Some(species_entry) = self
                .battle_pet_test_fixture_like_cpp
                .battle_pet_species_store
                .as_ref()
                .and_then(|store| store.get(species))
            else {
                return Some(false);
            };

            let max_pets_per_species = if species_entry
                .has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
            {
                1
            } else {
                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
            };

            return Some(
                self.battle_pet_count_like_cpp(species, owner_guid) >= max_pets_per_species,
            );
        }
        #[cfg(not(test))]
        None
    }
    /// C++ `BattlePetMgr::AddPet`, represented without DB persistence and with
    /// a local GUID counter until `sObjectMgr->GetGenerator<HighGuid::BattlePet>()`
    /// is ported.
    #[cfg(test)]
    pub(crate) fn battle_pet_add_pet_represented_like_cpp(
        &mut self,
        species: u32,
        display_id: u32,
        breed: u16,
        quality: u8,
        level: u16,
    ) -> Option<ObjectGuid> {
        let species_entry = self
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned()?;

        if !species_entry.has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP) {
            return None;
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);
        let pet_guid = crate::session::next_represented_battle_pet_guid_like_cpp();
        let owner_info = if species_entry
            .has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
        {
            self.player_guid().map(
                |guid| wow_packet::packets::misc::BattlePetJournalPetOwnerInfo {
                    guid,
                    player_virtual_realm: 1,
                    player_native_realm: 1,
                },
            )
        } else {
            None
        };

        let mut pet = RepresentedBattlePetDataLikeCpp {
            species,
            creature_id: u32::try_from(species_entry.creature_id).unwrap_or_default(),
            display_id,
            breed,
            level,
            exp: 0,
            flags: 0,
            power: 0,
            health: 0,
            max_health: 0,
            speed: 0,
            quality,
            owner_info,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::New,
        };
        crate::session::apply_battle_pet_calculated_stats_like_cpp(&mut pet, calculated_stats);

        self.battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .insert(pet_guid, pet);
        self.send_battle_pet_updates_like_cpp(&[pet_guid], true);
        #[cfg(test)]
        {
            self.battle_pet_test_fixture_like_cpp
                .represented_battle_pet_unique_owned_criteria_like_cpp = self
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_unique_owned_criteria_like_cpp
                .saturating_add(1);
            self.battle_pet_test_fixture_like_cpp
                .represented_battle_pet_learned_new_pet_criteria_like_cpp
                .push(species);
        }

        Some(pet_guid)
    }
    /// Durable account-scoped `BattlePetMgr::AddPet` replacement. Unlike the
    /// target C++ split `HasMaxPetCount`/`AddPet` sequence, the owner rechecks
    /// lease and capacity while reserving the insert. Publication happens only
    /// after the Login DB pet row and idempotency receipt commit together.
    pub(crate) async fn battle_pet_try_add_pet_durable_like_cpp(
        &mut self,
        request_key: [u8; 16],
        species: u32,
        display_id: u32,
        breed: u16,
        quality: u8,
        level: u16,
    ) -> Result<ObjectGuid, BattlePetAddFailureLikeCpp> {
        let Some(attachment) = &self.lifecycle.battle_pet_account_attachment_like_cpp else {
            #[cfg(test)]
            return self
                .battle_pet_add_pet_represented_like_cpp(species, display_id, breed, quality, level)
                .ok_or(BattlePetAddFailureLikeCpp::InvalidSpecies);
            #[cfg(not(test))]
            return Err(BattlePetAddFailureLikeCpp::MissingAuthority);
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease_id = attachment.lease_id_like_cpp();
        let outcome = owner
            .try_add_pet_like_cpp(
                lease_id,
                BattlePetAddRequestLikeCpp {
                    request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
                    species,
                    display_id,
                    breed,
                    quality,
                    level,
                    owner_guid: self.player_guid(),
                },
            )
            .await?;
        match outcome {
            BattlePetAddOutcomeLikeCpp::Added(pet) => {
                let guid = pet.guid;
                self.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
                    pets: vec![pet],
                    pet_added: true,
                });
                #[cfg(test)]
                {
                    self.battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_unique_owned_criteria_like_cpp = self
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_unique_owned_criteria_like_cpp
                        .saturating_add(1);
                    self.battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_learned_new_pet_criteria_like_cpp
                        .push(species);
                }
                Ok(guid)
            }
            BattlePetAddOutcomeLikeCpp::Replayed(pet) => Ok(pet.guid),
        }
    }
}
