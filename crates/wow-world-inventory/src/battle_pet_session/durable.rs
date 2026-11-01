// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable account-scoped `BattlePetMgr` mutations (#1363 slice 7b): the
//! owner commits the Login DB row first, and the session publishes only the
//! committed packet.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_world_core::battle_pet_account::{
    BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp, BattlePetAddRequestKeyLikeCpp,
    BattlePetAddRequestLikeCpp, BattlePetMutationFailureLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::RepresentedBattlePetLevelCriteriaLikeCpp;

use super::journal::{
    battle_pet_calculate_stats_like_cpp, battle_pet_species_has_flag_like_cpp,
    battle_pet_xp_per_level_like_cpp,
};
use super::{
    BattlePetSessionHostLikeCpp, MAX_BATTLE_PET_LEVEL_LIKE_CPP,
    RepresentedBattlePetGrantExperienceOutcomeLikeCpp,
    RepresentedBattlePetGrantLevelOutcomeLikeCpp, RepresentedBattlePetXpSourceLikeCpp,
    apply_battle_pet_calculated_stats_like_cpp,
};

/// Durable account-scoped `BattlePetMgr::AddPet` replacement. Unlike the
/// target C++ split `HasMaxPetCount`/`AddPet` sequence, the owner rechecks
/// lease and capacity while reserving the insert. Publication happens only
/// after the Login DB pet row and idempotency receipt commit together.
pub async fn battle_pet_try_add_pet_durable_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    request_key: [u8; 16],
    species: u32,
    display_id: u32,
    breed: u16,
    quality: u8,
    level: u16,
) -> Result<ObjectGuid, BattlePetAddFailureLikeCpp> {
    let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() else {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(added) =
            host.battle_pet_add_pet_fixture_like_cpp(species, display_id, breed, quality, level)
        {
            return added.ok_or(BattlePetAddFailureLikeCpp::InvalidSpecies);
        }
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
                owner_guid: host.player_guid(),
            },
        )
        .await?;
    match outcome {
        BattlePetAddOutcomeLikeCpp::Added(pet) => {
            let guid = pet.guid;
            host.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![pet],
                pet_added: true,
            });
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_unique_owned_criteria_like_cpp = fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_unique_owned_criteria_like_cpp
                    .saturating_add(1);
                fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_learned_new_pet_criteria_like_cpp
                    .push(species);
            }
            Ok(guid)
        }
        BattlePetAddOutcomeLikeCpp::Replayed(pet) => Ok(pet.guid),
    }
}

pub async fn battle_pet_grant_experience_durable_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    pet_guid: ObjectGuid,
    xp: u16,
    xp_source: RepresentedBattlePetXpSourceLikeCpp,
    pet_battle_xp_multiplier: f32,
) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
    let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() else {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(outcome) = host.battle_pet_grant_experience_fixture_like_cpp(
            pet_guid,
            xp,
            xp_source,
            pet_battle_xp_multiplier,
        ) {
            return outcome;
        }
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
    if battle_pet_species_has_flag_like_cpp(
        host,
        pet.species,
        wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
    ) {
        return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::CantBattle;
    }
    if pet.level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
        return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::AlreadyMaxLevel;
    }
    let Some(mut next_level_xp) = battle_pet_xp_per_level_like_cpp(host, pet.level) else {
        return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
    };
    let mut level = pet.level;
    let mut total_xp = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
        (f32::from(xp) * pet_battle_xp_multiplier) as u16
    } else {
        xp
    };
    total_xp = total_xp.saturating_add(pet.exp);
    #[cfg(any(test, feature = "test-fixtures"))]
    let mut criteria = Vec::new();
    while total_xp >= next_level_xp && level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
        total_xp = total_xp.saturating_sub(next_level_xp);
        level += 1;
        let Some(row_xp) = battle_pet_xp_per_level_like_cpp(host, level) else {
            return RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow;
        };
        next_level_xp = row_xp;
        #[cfg(any(test, feature = "test-fixtures"))]
        criteria.push(RepresentedBattlePetLevelCriteriaLikeCpp {
            species: pet.species,
            level,
        });
    }
    let calculated =
        battle_pet_calculate_stats_like_cpp(host, pet.breed, pet.species, pet.quality, level);
    let persisted_exp = if level < MAX_BATTLE_PET_LEVEL_LIKE_CPP {
        total_xp
    } else {
        0
    };
    match owner
        .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
            pet.level = level;
            pet.exp = persisted_exp;
            apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
        })
        .await
    {
        Ok(((), packet)) => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_level_criteria_like_cpp
                    .extend(criteria.iter().copied());
                if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
                    fixtures
                        .pets
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_active_level_criteria_like_cpp
                        .extend(criteria);
                }
            }
            host.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
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
            host.hub_ref().log_battle_pet_mutation_failure_like_cpp(
                "grant experience",
                pet_guid,
                &error,
            );
            RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet
        }
    }
}

pub async fn battle_pet_grant_level_durable_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    pet_guid: ObjectGuid,
    granted_levels: u16,
) -> RepresentedBattlePetGrantLevelOutcomeLikeCpp {
    let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() else {
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(outcome) =
            host.battle_pet_grant_level_fixture_like_cpp(pet_guid, granted_levels)
        {
            return outcome;
        }
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
    if battle_pet_species_has_flag_like_cpp(
        host,
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
    #[cfg(any(test, feature = "test-fixtures"))]
    let criteria: Vec<_> = ((pet.level + 1)..=level)
        .map(|level| RepresentedBattlePetLevelCriteriaLikeCpp {
            species: pet.species,
            level,
        })
        .collect();
    let calculated =
        battle_pet_calculate_stats_like_cpp(host, pet.breed, pet.species, pet.quality, level);
    match owner
        .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
            pet.level = level;
            if level >= MAX_BATTLE_PET_LEVEL_LIKE_CPP {
                pet.exp = 0;
            }
            apply_battle_pet_calculated_stats_like_cpp(pet, calculated);
        })
        .await
    {
        Ok(((), packet)) => {
            #[cfg(any(test, feature = "test-fixtures"))]
            if let Some(fixtures) = host.test_fixtures_like_cpp() {
                fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_level_criteria_like_cpp
                    .extend(criteria);
            }
            host.send_packet(&wow_packet::packets::misc::BattlePetUpdates {
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
            host.hub_ref().log_battle_pet_mutation_failure_like_cpp(
                "grant level",
                pet_guid,
                &error,
            );
            RepresentedBattlePetGrantLevelOutcomeLikeCpp::UnknownPet
        }
    }
}
