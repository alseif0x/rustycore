// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only battle-pet bodies and method entry points (#1363 slice 7b).
//!
//! The durable battle-pet operations and the journal helpers moved to
//! `wow_world_inventory::battle_pet_session`. The represented
//! (fixture-backed) `BattlePetMgr` bodies below were `#[cfg(test)]` methods
//! nested in the production battle-pet impls; they are the World test build's
//! fallback that the moved durable operations reach through the
//! `battle_pet_*_fixture_like_cpp` host hooks. The method entry points forward
//! to the moved free functions so the scenarios keep their method syntax.
//! Mounted from `handlers/mod.rs` outside the session hotspot tree and marked
//! `#![cfg(test)]`, so the classifier counts it as test code.

#![cfg(test)]

use tracing::warn;
use wow_core::ObjectGuid;
use wow_packet::packets::misc::CageBattlePet;

use crate::session::{
    BATTLE_PET_BREED_QUALITY_RARE_LIKE_CPP, BATTLE_PET_CAGE_ITEM_ID_LIKE_CPP,
    MAX_BATTLE_PET_LEVEL_LIKE_CPP, RepresentedAuraEffectLikeCpp,
    RepresentedBattlePetCageItemLikeCpp, RepresentedBattlePetCageOutcomeLikeCpp,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetGrantExperienceOutcomeLikeCpp,
    RepresentedBattlePetGrantLevelOutcomeLikeCpp, RepresentedBattlePetLevelCriteriaLikeCpp,
    RepresentedBattlePetQualityOutcomeLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
    RepresentedBattlePetXpSourceLikeCpp, WorldSession,
};

impl WorldSession {
    pub(crate) fn battle_pet_max_pet_level_like_cpp(&self) -> Option<u16> {
        wow_world_inventory::battle_pet_max_pet_level_like_cpp(self)
    }
    pub(crate) fn battle_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> u8 {
        wow_world_inventory::battle_pet_count_like_cpp(
            &self.lifecycle,
            &self.fixtures,
            species,
            owner_guid,
        )
    }

    /// CMSG_CAGE_BATTLE_PET — represented cage body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::CageBattlePet`. The manager then performs the journal,
    /// species, slot, health, inventory, item-store, remove, deleted-packet,
    /// and summoned-companion gates. The archived opcode id is still the
    /// unresolved `0xBADD` placeholder, so this method remains intentionally
    /// unregistered for production dispatch. Until the real inventory path is
    /// wired, this represented body exercises the successful inventory seam.

    pub(crate) async fn handle_cage_battle_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match CageBattlePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "CageBattlePet parse failed: {error}"
                );
                return;
            }
        };

        let _ = self.battle_pet_cage_battle_pet_represented_like_cpp(request.pet_guid, true, true);
    }

    /// C++ `BattlePetMgr::CageBattlePet`, represented at the battle-pet state
    /// boundary. Inventory placement is still external: the caller must pass
    /// the already-resolved `CanStoreNewItem`/`StoreNewItem` outcomes.
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
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
            .cloned()
        else {
            return RepresentedBattlePetCageOutcomeLikeCpp::UnknownPet;
        };

        if wow_world_inventory::battle_pet_species_has_flag_like_cpp(
            self,
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_NOT_TRADABLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetCageOutcomeLikeCpp::NotTradable;
        }

        if self
            .fixtures
            .pets
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
        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_cage_items_like_cpp
            .push(cage_item);
        #[cfg(not(test))]
        let _ = cage_item;

        let _ = self.battle_pet_remove_pet_like_cpp(pet_guid);
        self.send_packet(&wow_packet::packets::misc::BattlePetDeleted { pet_guid });

        if crate::session::hub_ref(self).represented_summoned_battle_pet_guid_like_cpp()
            == Some(pet_guid)
        {
            let _cleared = self.core.mutate_canonical_player_like_cpp(|player| {
                player.clear_battle_pet_data_like_cpp();
            });
            #[cfg(test)]
            if _cleared.is_none() {
                self.fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_summoned_battle_pet_guid_like_cpp = None;
            }
        }

        RepresentedBattlePetCageOutcomeLikeCpp::Caged(cage_item)
    }
    /// C++ `BattlePetMgr::AddPet`, represented without DB persistence and with
    /// a local GUID counter until `sObjectMgr->GetGenerator<HighGuid::BattlePet>()`
    /// is ported.
    pub(crate) fn battle_pet_add_pet_represented_like_cpp(
        &mut self,
        species: u32,
        display_id: u32,
        breed: u16,
        quality: u8,
        level: u16,
    ) -> Option<ObjectGuid> {
        let species_entry = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned()?;

        if !species_entry.has_flag_like_cpp(wow_data::BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP) {
            return None;
        }

        let calculated_stats = wow_world_inventory::battle_pet_calculate_stats_like_cpp(
            self, breed, species, quality, level,
        );
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

        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .insert(pet_guid, pet);
        self.send_battle_pet_updates_like_cpp(&[pet_guid], true);
        #[cfg(test)]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_unique_owned_criteria_like_cpp = self
                .fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_unique_owned_criteria_like_cpp
                .saturating_add(1);
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_learned_new_pet_criteria_like_cpp
                .push(species);
        }

        Some(pet_guid)
    }
    /// C++ `BattlePetMgr::GrantBattlePetExperience`, represented after
    /// external aura multiplier resolution.
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
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
        else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet;
        };

        if xp == 0 || xp_source == RepresentedBattlePetXpSourceLikeCpp::Invalid {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::InvalidXpOrSource;
        }

        if wow_world_inventory::battle_pet_species_has_flag_like_cpp(
            self,
            pet.species,
            wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
        ) {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::CantBattle;
        }

        let mut level = pet.level;
        if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::AlreadyMaxLevel;
        }

        let Some(mut next_level_xp) =
            wow_world_inventory::battle_pet_xp_per_level_like_cpp(self, level)
        else {
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

            let Some(row_xp) = wow_world_inventory::battle_pet_xp_per_level_like_cpp(self, level)
            else {
                return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
            };
            next_level_xp = row_xp;

            #[cfg(test)]
            {
                let criteria = RepresentedBattlePetLevelCriteriaLikeCpp { species, level };
                self.fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_level_criteria_like_cpp
                    .push(criteria);
                if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                    self.fixtures
                        .pets
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_active_level_criteria_like_cpp
                        .push(criteria);
                }
            }
        }

        let calculated_stats = wow_world_inventory::battle_pet_calculate_stats_like_cpp(
            self, breed, species, quality, level,
        );

        let pet = self
            .fixtures
            .pets
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
    /// C++ `BattlePetMgr::ChangeBattlePetQuality`. `BattlePet::CalculateStats`
    /// may return early when breed-state DB2 rows are missing; that does not
    /// abort the quality change.
    pub(crate) fn battle_pet_change_battle_pet_quality_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        quality: u8,
    ) -> RepresentedBattlePetQualityOutcomeLikeCpp {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
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

        if wow_world_inventory::battle_pet_species_has_flag_like_cpp(
            self,
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
        let calculated_stats = wow_world_inventory::battle_pet_calculate_stats_like_cpp(
            self, breed, species, quality, level,
        );

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
    pub(crate) fn battle_pet_grant_battle_pet_level_represented_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        granted_levels: u16,
    ) -> RepresentedBattlePetGrantLevelOutcomeLikeCpp {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
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

        if wow_world_inventory::battle_pet_species_has_flag_like_cpp(
            self,
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

        let calculated_stats = wow_world_inventory::battle_pet_calculate_stats_like_cpp(
            self, breed, species, quality, level,
        );

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
    /// C++ `BattlePetMgr::RemovePet`.
    pub(crate) fn battle_pet_remove_pet_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return false;
        }

        let Some(pet) = self
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get_mut(&pet_guid)
        else {
            return false;
        };

        pet.save_info = RepresentedBattlePetSaveInfoLikeCpp::Removed;
        true
    }

    /// C++ `BattlePetMgr::ModifyName`, represented without live summoned-creature
    /// timestamp propagation until the companion creature runtime exists.
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

        let Some(pet) = self
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
    pub(crate) fn battle_pet_grant_battle_pet_experience_with_owner_auras_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        xp: u16,
        xp_source: RepresentedBattlePetXpSourceLikeCpp,
    ) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
        let multiplier = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
            self.total_represented_aura_multiplier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModBattlePetXpPct,
            )
        } else {
            1.0
        };

        self.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid, xp, xp_source, multiplier,
        )
    }
}
