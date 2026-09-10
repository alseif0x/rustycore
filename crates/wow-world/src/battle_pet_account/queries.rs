//! Queries operations of battle_pet_account.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl BattlePetAccountOwnerLikeCpp {
    /// Process-owned DB2 species row used by account-journal operations.
    ///
    /// C++ battle-pet flows resolve this through the global DB2 manager while
    /// mutating one account journal. Keeping the lookup on the account owner
    /// avoids making `WorldSession` a second catalog owner.
    pub(crate) fn species_entry_like_cpp(
        &self,
        species: u32,
    ) -> Option<wow_data::BattlePetSpeciesEntry> {
        self.species_store.get(species).cloned()
    }

    pub(crate) fn species_has_flag_like_cpp(&self, species: u32, flag: i32) -> bool {
        self.species_store
            .get(species)
            .is_some_and(|entry| entry.has_flag_like_cpp(flag))
    }

    pub(crate) fn xp_per_level_like_cpp(&self, level: u16) -> Option<u16> {
        self.xp_game_table.xp_per_level_like_cpp(level)
    }

    pub(crate) fn calculate_stats_like_cpp(
        &self,
        breed: u16,
        species: u32,
        quality: u8,
        level: u16,
    ) -> Option<crate::session::RepresentedBattlePetCalculatedStatsLikeCpp> {
        let stats = calculate_battle_pet_stats_like_cpp(
            breed,
            species,
            quality,
            level,
            &self.breed_state_store,
            &self.species_state_store,
            &self.breed_quality_store,
        )?;
        Some(crate::session::RepresentedBattlePetCalculatedStatsLikeCpp {
            max_health: stats.max_health,
            power: stats.power,
            speed: stats.speed,
        })
    }

    pub(crate) fn journal_like_cpp(
        &self,
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: Option<ObjectGuid>,
    ) -> BattlePetJournal {
        let has_journal_lock = self.has_lease_like_cpp(lease_id);
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let mut pets: Vec<_> = state
            .pets
            .iter()
            .filter(|(_, pet)| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .filter(|(_, pet)| {
                pet.owner_info
                    .is_none_or(|owner| Some(owner.guid) == player_guid)
            })
            .map(|(guid, pet)| pet.packet_info_like_cpp(*guid))
            .collect();
        pets.sort_by_key(|pet| pet.guid.counter());
        BattlePetJournal {
            trap: 0,
            has_journal_lock,
            slots: state
                .slots
                .iter()
                .map(|slot| {
                    let mut packet = slot.packet_slot_like_cpp();
                    if !packet.pet_guid.is_empty() && !state.pets.contains_key(&packet.pet_guid) {
                        packet.pet_guid = empty_battle_pet_guid_like_cpp();
                    }
                    packet
                })
                .collect(),
            pets,
        }
    }

    pub(crate) fn pet_snapshot_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .get(&pet_guid)
            .cloned()
    }

    pub(crate) fn max_pet_level_like_cpp(&self) -> u16 {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .values()
            .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .map(|pet| pet.level)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn has_max_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> bool {
        let Some(entry) = self.species_store.get(species) else {
            return false;
        };
        let max = if entry.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
        {
            1
        } else {
            DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
        };
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        count_species_like_cpp(&state, species, owner_guid, entry.flags) >= max
    }

    pub(crate) fn pet_count_like_cpp(&self, species: u32, owner_guid: Option<ObjectGuid>) -> u8 {
        let Some(entry) = self.species_store.get(species) else {
            return 0;
        };
        count_species_like_cpp(
            &self
                .state
                .lock()
                .expect("battle-pet account state poisoned"),
            species,
            owner_guid,
            entry.flags,
        )
    }

    pub(crate) fn unique_species_count_like_cpp(&self) -> u32 {
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        u32::try_from(
            state
                .pets
                .values()
                .map(|pet| pet.species)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX)
    }
}
