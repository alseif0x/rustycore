//! Represented battle-pet state at the Session boundary.
//!
//! Moved out of the Session root under #607. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Attach this session to the one canonical journal owner for its
    /// Battle.net account. The attachment releases any held journal lease on
    /// drop, matching C++ `WorldSession` teardown.
    pub fn set_battle_pet_account_attachment_like_cpp(
        &mut self,
        attachment: BattlePetAccountAttachmentLikeCpp,
    ) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.battle_pet_account_attachment_like_cpp = Some(attachment);
    }
    /// The #160 account owner and this session's journal lease id, when the
    /// canonical journal is attached (issue #161 purchase saga).
    pub(crate) fn battle_pet_account_owner_lease_like_cpp(
        &self,
    ) -> Option<(
        Arc<crate::battle_pet_account::BattlePetAccountOwnerLikeCpp>,
        crate::battle_pet_account::BattlePetLeaseIdLikeCpp,
    )> {
        self.battle_pet_account_attachment_like_cpp
            .as_ref()
            .map(|attachment| {
                (
                    Arc::clone(attachment.owner_like_cpp()),
                    attachment.lease_id_like_cpp(),
                )
            })
    }
    /// Set the world-DB battle-pet breed/quality selection store (#161).
    #[cfg(test)]
    pub fn set_battle_pet_selection_store_like_cpp(
        &mut self,
        store: Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    ) {
        self.battle_pet_selection_store_like_cpp = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_selection_store_like_cpp(
        &self,
    ) -> Option<&Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>> {
        self.battle_pet_selection_store_like_cpp.as_ref()
    }
    pub(crate) fn battle_pet_purchase_store_like_cpp(
        &self,
    ) -> Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>> {
        self.persistence_ports_like_cpp
            .player
            .battle_pet_purchase
            .as_ref()
            .map(Arc::clone)
    }
    #[cfg(test)]
    pub(crate) fn set_battle_pet_purchase_selection_override_like_cpp(
        &mut self,
        selection: Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp>,
    ) {
        self.battle_pet_purchase_selection_override_like_cpp = selection;
    }
    #[cfg(test)]
    pub(crate) fn battle_pet_purchase_selection_override_like_cpp(
        &self,
    ) -> Option<wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp> {
        self.battle_pet_purchase_selection_override_like_cpp
    }
    #[cfg(test)]
    pub fn set_battle_pet_breed_state_store(&mut self, store: Arc<BattlePetBreedStateStore>) {
        self.battle_pet_breed_state_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_store(&mut self, store: Arc<BattlePetSpeciesStore>) {
        self.battle_pet_species_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_species_state_store(&mut self, store: Arc<BattlePetSpeciesStateStore>) {
        self.battle_pet_species_state_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_battle_pet_xp_game_table(&mut self, table: Arc<BattlePetXpGameTableLikeCpp>) {
        self.battle_pet_xp_game_table = Some(table);
    }
    pub(crate) fn battle_pet_calculate_stats_like_cpp(
        &self,
        breed: u16,
        species: u32,
        quality: u8,
        level: u16,
    ) -> Option<RepresentedBattlePetCalculatedStatsLikeCpp> {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .calculate_stats_like_cpp(breed, species, quality, level);
        }
        #[cfg(test)]
        {
            let stats = calculate_battle_pet_stats_like_cpp(
                breed,
                species,
                quality,
                level,
                self.battle_pet_breed_state_store.as_ref()?,
                self.battle_pet_species_state_store.as_ref()?,
                self.battle_pet_breed_quality_store.as_ref()?,
            )?;

            Some(RepresentedBattlePetCalculatedStatsLikeCpp {
                max_health: stats.max_health,
                power: stats.power,
                speed: stats.speed,
            })
        }
        #[cfg(not(test))]
        None
    }
    pub(in crate::session) fn battle_pet_species_has_flag_like_cpp(
        &self,
        species: u32,
        flag: i32,
    ) -> bool {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .species_has_flag_like_cpp(species, flag);
        }
        #[cfg(test)]
        return self
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .is_some_and(|entry| entry.has_flag_like_cpp(flag));
        #[cfg(not(test))]
        false
    }
    /// Test/setup seam for represented `BattlePetMgr::_pets`.
    #[cfg(test)]
    pub(crate) fn add_represented_battle_pet_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        save_info: RepresentedBattlePetSaveInfoLikeCpp,
    ) {
        self.represented_battle_pets_like_cpp.insert(
            pet_guid,
            RepresentedBattlePetDataLikeCpp::minimal_like_cpp(flags, save_info),
        );
    }
    /// C++ `BattlePetMgr::ClearFanfare`.
    #[cfg(test)]
    pub(crate) fn battle_pet_clear_fanfare_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self.represented_battle_pets_like_cpp.get_mut(&pet_guid) else {
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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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

        let Some(pet) = self.represented_battle_pets_like_cpp.get_mut(&pet_guid) else {
            return false;
        };

        pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Removed;
        true
    }
    pub(crate) async fn battle_pet_remove_pet_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
    ) -> bool {
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
        self.represented_battle_pet_cage_items_like_cpp
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
                self.represented_summoned_battle_pet_guid_like_cpp = None;
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
        let Some(pet) = self.represented_battle_pets_like_cpp.get_mut(&pet_guid) else {
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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment
                .owner_like_cpp()
                .pet_count_like_cpp(species, owner_guid);
        }
        let species_flags = self
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .map(|entry| entry.flags)
            .unwrap_or(0);
        let not_account_wide =
            species_flags & wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP != 0;

        let count = self
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
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return Some(
                attachment
                    .owner_like_cpp()
                    .has_max_pet_count_like_cpp(species, owner_guid),
            );
        }
        #[cfg(test)]
        {
            let Some(species_entry) = self
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
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned()?;

        if !species_entry.has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP) {
            return None;
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);
        let pet_guid = crate::session_rules::next_represented_battle_pet_guid_like_cpp();
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
        crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(
            &mut pet,
            calculated_stats,
        );

        self.represented_battle_pets_like_cpp.insert(pet_guid, pet);
        self.send_battle_pet_updates_like_cpp(&[pet_guid], true);
        #[cfg(test)]
        {
            self.represented_battle_pet_unique_owned_criteria_like_cpp = self
                .represented_battle_pet_unique_owned_criteria_like_cpp
                .saturating_add(1);
            self.represented_battle_pet_learned_new_pet_criteria_like_cpp
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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
                    self.represented_battle_pet_unique_owned_criteria_like_cpp = self
                        .represented_battle_pet_unique_owned_criteria_like_cpp
                        .saturating_add(1);
                    self.represented_battle_pet_learned_new_pet_criteria_like_cpp
                        .push(species);
                }
                Ok(guid)
            }
            BattlePetAddOutcomeLikeCpp::Replayed(pet) => Ok(pet.guid),
        }
    }
    /// Record the C++ `BattlePetMgr::AddPet` criteria hooks from durable
    /// current state. Both C++ criteria are set-like (`UniquePetsOwned` uses
    /// the current unique-species count and `LearnedNewPet` sets one species
    /// to 1), so receipt recovery and packet re-sends are idempotent.
    pub(crate) fn record_battle_pet_trainer_purchase_criteria_like_cpp(&mut self, species: u32) {
        #[cfg(not(test))]
        let _ = species;
        #[cfg(test)]
        {
            self.represented_battle_pet_unique_owned_criteria_like_cpp = self
                .battle_pet_account_attachment_like_cpp
                .as_ref()
                .map(|attachment| attachment.owner_like_cpp().unique_species_count_like_cpp())
                .unwrap_or_else(|| {
                    u32::try_from(
                        self.represented_battle_pets_like_cpp
                            .values()
                            .map(|pet| pet.species)
                            .collect::<BTreeSet<_>>()
                            .len(),
                    )
                    .unwrap_or(u32::MAX)
                });
            if !self
                .represented_battle_pet_learned_new_pet_criteria_like_cpp
                .contains(&species)
            {
                self.represented_battle_pet_learned_new_pet_criteria_like_cpp
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

        for (pet_guid, pet) in &mut self.represented_battle_pets_like_cpp {
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

        let Some(pet) = self.represented_battle_pets_like_cpp.get(&pet_guid) else {
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
                self.represented_battle_pet_level_criteria_like_cpp
                    .push(criteria);
                if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                    self.represented_battle_pet_active_level_criteria_like_cpp
                        .push(criteria);
                }
            }
        }

        let calculated_stats =
            self.battle_pet_calculate_stats_like_cpp(breed, species, quality, level);

        let pet = self
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
            .expect("pet was checked before XP calculation");
        pet.level = level;
        pet.exp = if level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            total_xp
        } else {
            0
        };
        crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated_stats);

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
        let Some(attachment) = &self.battle_pet_account_attachment_like_cpp else {
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
                crate::session_rules::apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
            })
            .await
        {
            Ok(((), packet)) => {
                #[cfg(test)]
                {
                    self.represented_battle_pet_level_criteria_like_cpp
                        .extend(criteria.iter().copied());
                    if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                        self.represented_battle_pet_active_level_criteria_like_cpp
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
        self.represented_battle_pet_unique_owned_criteria_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_learned_new_pet_criteria_like_cpp(&self) -> &[u32] {
        &self.represented_battle_pet_learned_new_pet_criteria_like_cpp
    }
    /// C++ `WorldSession::HandleBattlePetSummon` represented toggle.
    ///
    /// `BattlePetMgr::SummonPet` silently ignores unknown pets before casting
    /// the summon spell; `DismissPet` clears the active summoned companion.
    pub(crate) fn battle_pet_summon_toggle_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        if self.represented_summoned_battle_pet_guid_like_cpp() == Some(pet_guid) {
            return self.set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        if self.represented_battle_pet_like_cpp(pet_guid).is_none() {
            return false;
        }

        self.set_represented_summoned_battle_pet_guid_like_cpp(Some(pet_guid))
    }
    pub(crate) fn represented_summoned_battle_pet_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) =
            self.with_owned_player_like_cpp(Player::summoned_battle_pet_guid_like_cpp)
        {
            return guid;
        }
        #[cfg(test)]
        {
            self.represented_summoned_battle_pet_guid_like_cpp
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    pub(in crate::session) fn set_represented_summoned_battle_pet_guid_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
    ) -> bool {
        if self
            .with_owned_player_mut_like_cpp(|player| {
                player.set_summoned_battle_pet_guid_like_cpp(
                    pet_guid.unwrap_or(wow_core::ObjectGuid::EMPTY),
                );
            })
            .is_some()
        {
            return true;
        }
        #[cfg(test)]
        {
            self.represented_summoned_battle_pet_guid_like_cpp = pet_guid;
            true
        }
        #[cfg(not(test))]
        {
            false
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_battle_pet_cage_items_like_cpp(
        &self,
    ) -> &[RepresentedBattlePetCageItemLikeCpp] {
        &self.represented_battle_pet_cage_items_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn set_represented_battle_pet_query_companion_like_cpp(
        &mut self,
        unit_guid: ObjectGuid,
        companion: RepresentedBattlePetQueryCompanionLikeCpp,
    ) {
        self.represented_battle_pet_query_companions_like_cpp
            .insert(unit_guid, companion);
    }
    pub(crate) fn represented_battle_pet_query_companion_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetQueryCompanionLikeCpp> {
        if let Some(manager) = self.canonical_map_manager.as_ref()
            && let Ok(manager) = manager.lock()
        {
            let mut snapshot = None;
            manager.do_for_all_maps(|managed| {
                if snapshot.is_some() {
                    return;
                }
                let Some(companion) = managed.map().with_creature_like_cpp(unit_guid, |creature| {
                    let unit = creature.unit();
                    RepresentedBattlePetQueryCompanionLikeCpp {
                        creature_id: i32::try_from(creature.entry()).unwrap_or(i32::MAX),
                        name_timestamp: i64::from(
                            unit.battle_pet_companion_name_timestamp_like_cpp(),
                        ),
                        is_summon: creature.is_summon_like_cpp(),
                        owner_is_player: unit
                            .subsystems()
                            .control
                            .owner_guid
                            .is_some_and(|guid| guid.is_player()),
                        battle_pet_companion_guid: unit.battle_pet_companion_guid_like_cpp(),
                    }
                }) else {
                    return;
                };
                snapshot = Some(companion);
            });
            if snapshot.is_some() {
                return snapshot;
            }
        }
        #[cfg(test)]
        {
            self.represented_battle_pet_query_companions_like_cpp
                .get(&unit_guid)
                .copied()
        }
        #[cfg(not(test))]
        {
            None
        }
    }
    pub(crate) fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            return attachment.owner_like_cpp().pet_snapshot_like_cpp(pet_guid);
        }
        #[cfg(test)]
        return self
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
            .cloned();
        #[cfg(not(test))]
        None
    }
}
