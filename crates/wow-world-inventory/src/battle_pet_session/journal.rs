// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle-pet journal helpers over the account-scoped owner (#1363 slice 7b):
//! the former World `PetsCx`/`PetsCxRef` battle-pet methods, the stats and
//! species-flag reads and the add-request receipt query.

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::BTreeSet;
use std::sync::Arc;

use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_data::calculate_battle_pet_stats_like_cpp;
use wow_world_core::battle_pet_account::{
    BattlePetAccountOwnerLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddRequestKeyLikeCpp,
    BattlePetLeaseIdLikeCpp,
};
use wow_world_core::session::RepresentedBattlePetCalculatedStatsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::state::SessionFixtures;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP, RepresentedBattlePetSaveInfoLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_lifecycle::SessionLifecycleState;

use super::BattlePetSessionHostLikeCpp;

/// The #160 account owner and this session's journal lease id, when the
/// canonical journal is attached (issue #161 purchase saga).
pub fn battle_pet_account_owner_lease_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
) -> Option<(Arc<BattlePetAccountOwnerLikeCpp>, BattlePetLeaseIdLikeCpp)> {
    host.lifecycle()
        .battle_pet_account_attachment_like_cpp()
        .map(|attachment| {
            (
                Arc::clone(attachment.owner_like_cpp()),
                attachment.lease_id_like_cpp(),
            )
        })
}

pub fn battle_pet_purchase_store_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
) -> Option<Arc<dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp>> {
    host.lifecycle()
        .battle_pet_purchase_persistence_port_like_cpp()
        .map(Arc::clone)
}

/// C++ `BattlePetMgr::GetPetCount`.
#[cfg(any(test, feature = "test-fixtures"))]
pub fn battle_pet_count_like_cpp(
    lifecycle: &SessionLifecycleState,
    fixtures: &SessionFixtures,
    species: u32,
    owner_guid: Option<ObjectGuid>,
) -> u8 {
    if let Some(attachment) = lifecycle.battle_pet_account_attachment_like_cpp() {
        return attachment
            .owner_like_cpp()
            .pet_count_like_cpp(species, owner_guid);
    }
    let species_flags = fixtures
        .pets
        .battle_pet_test_fixture_like_cpp
        .battle_pet_species_store
        .as_ref()
        .and_then(|store| store.get(species))
        .map(|entry| entry.flags)
        .unwrap_or(0);
    let not_account_wide =
        species_flags & wow_data::BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP != 0;

    let count = fixtures
        .pets
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
pub fn battle_pet_has_max_pet_count_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    species: u32,
    owner_guid: Option<ObjectGuid>,
) -> Option<bool> {
    if let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() {
        return Some(
            attachment
                .owner_like_cpp()
                .has_max_pet_count_like_cpp(species, owner_guid),
        );
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        let Some(species_entry) = fixtures
            .pets
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
            battle_pet_count_like_cpp(host.lifecycle(), fixtures, species, owner_guid)
                >= max_pets_per_species,
        );
    }
    None
}

/// Acquire this session's journal lease through the #160 attachment
/// (issue #161 admission/recovery).
pub async fn battle_pet_try_acquire_journal_lease_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
) -> bool {
    let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() else {
        return false;
    };
    attachment.try_acquire_lease_like_cpp().await
}

/// One cloned DB2 species row for admission-time materialization (#161).
pub fn battle_pet_species_entry_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    species: u32,
) -> Option<wow_data::BattlePetSpeciesEntry> {
    if let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() {
        return attachment.owner_like_cpp().species_entry_like_cpp(species);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        return fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .cloned();
    }
    None
}

pub fn battle_pet_xp_per_level_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    level: u16,
) -> Option<u16> {
    let canonical = host
        .lifecycle()
        .battle_pet_account_attachment_like_cpp()
        .and_then(|attachment| attachment.owner_like_cpp().xp_per_level_like_cpp(level));
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        return canonical.or_else(|| {
            fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .battle_pet_xp_game_table
                .as_ref()
                .and_then(|table| table.xp_per_level_like_cpp(level))
                .or_else(|| {
                    fixtures
                        .pets
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pet_xp_per_level_like_cpp
                        .get(&level)
                        .copied()
                })
        });
    }
    canonical
}

/// C++ `BattlePetMgr::GetMaxPetLevel`.
pub fn battle_pet_max_pet_level_like_cpp<H: BattlePetSessionHostLikeCpp>(host: &H) -> Option<u16> {
    if let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() {
        return Some(attachment.owner_like_cpp().max_pet_level_like_cpp());
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        return Some(
            fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pets_like_cpp
                .values()
                .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
                .map(|pet| pet.level)
                .max()
                .unwrap_or(0),
        );
    }
    None
}

pub fn battle_pet_calculate_stats_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    breed: u16,
    species: u32,
    quality: u8,
    level: u16,
) -> Option<RepresentedBattlePetCalculatedStatsLikeCpp> {
    if let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() {
        return attachment
            .owner_like_cpp()
            .calculate_stats_like_cpp(breed, species, quality, level);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        let stats = calculate_battle_pet_stats_like_cpp(
            breed,
            species,
            quality,
            level,
            fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .battle_pet_breed_state_store
                .as_ref()?,
            fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .battle_pet_species_state_store
                .as_ref()?,
            fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .battle_pet_breed_quality_store
                .as_ref()?,
        )?;

        return Some(RepresentedBattlePetCalculatedStatsLikeCpp {
            max_health: stats.max_health,
            power: stats.power,
            speed: stats.speed,
        });
    }
    None
}

pub fn battle_pet_species_has_flag_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    species: u32,
    flag: i32,
) -> bool {
    if let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() {
        return attachment
            .owner_like_cpp()
            .species_has_flag_like_cpp(species, flag);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_ref_like_cpp() {
        return fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .battle_pet_species_store
            .as_ref()
            .and_then(|store| store.get(species))
            .is_some_and(|entry| entry.has_flag_like_cpp(flag));
    }
    false
}

/// Record the C++ `BattlePetMgr::AddPet` criteria hooks from durable
/// current state. Both C++ criteria are set-like (`UniquePetsOwned` uses
/// the current unique-species count and `LearnedNewPet` sets one species
/// to 1), so receipt recovery and packet re-sends are idempotent.
pub fn record_battle_pet_trainer_purchase_criteria_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    species: u32,
) {
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (host, species);
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_like_cpp().is_some() {
        let (lifecycle, hub) = host.split_lifecycle_mut();
        hub.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_unique_owned_criteria_like_cpp = lifecycle
            .battle_pet_account_attachment_like_cpp()
            .map(|attachment| attachment.owner_like_cpp().unique_species_count_like_cpp())
            .unwrap_or_else(|| {
                u32::try_from(
                    hub.fixtures
                        .pets
                        .battle_pet_test_fixture_like_cpp
                        .represented_battle_pets_like_cpp
                        .values()
                        .map(|pet| pet.species)
                        .collect::<BTreeSet<_>>()
                        .len(),
                )
                .unwrap_or(u32::MAX)
            });
        if !hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_learned_new_pet_criteria_like_cpp
            .contains(&species)
        {
            hub.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_learned_new_pet_criteria_like_cpp
                .push(species);
        }
    }
}

pub async fn battle_pet_add_request_committed_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &H,
    request_key: BattlePetAddRequestKeyLikeCpp,
) -> Result<bool, BattlePetAddFailureLikeCpp> {
    let Some(attachment) = host.lifecycle().battle_pet_account_attachment_like_cpp() else {
        return Ok(false);
    };
    attachment
        .owner_like_cpp()
        .add_request_committed_like_cpp(request_key)
        .await
}
