// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable battle-pet operations of the World session (#1363 slice 7b): the
//! C++ `BattlePetMgr::AddPet`, `GrantBattlePetExperience` and
//! `GrantBattlePetLevel` replacements over the account-scoped owner, the
//! uncage effect with its `Player::DestroyItem` phase, and the journal helpers
//! that the World `PetsCx`/`PetsCxRef` facade used to own.
//!
//! The bodies borrow the session through [`BattlePetSessionHostLikeCpp`]. A
//! former `#[cfg(test)]` fixture step runs only when the host reports the World
//! test fixtures, so it keeps meaning the World test build.

use std::future::Future;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
use wow_entities::{Item, PlayerInventoryItem as InventoryItem};
use wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp;
use wow_world_core::session::{
    HubMut, HubRef, RepresentedBattlePetCalculatedStatsLikeCpp, RepresentedBattlePetDataLikeCpp,
    SessionHubHostLikeCpp,
};
use wow_world_lifecycle::SessionLifecycleState;

use crate::InventoryState;

mod durable;
mod journal;
mod uncage;

pub use durable::{
    battle_pet_grant_experience_durable_like_cpp, battle_pet_grant_level_durable_like_cpp,
    battle_pet_try_add_pet_durable_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use journal::battle_pet_count_like_cpp;
pub use journal::{
    battle_pet_account_owner_lease_like_cpp, battle_pet_add_request_committed_like_cpp,
    battle_pet_calculate_stats_like_cpp, battle_pet_has_max_pet_count_like_cpp,
    battle_pet_max_pet_level_like_cpp, battle_pet_purchase_store_like_cpp,
    battle_pet_species_entry_like_cpp, battle_pet_species_has_flag_like_cpp,
    battle_pet_try_acquire_journal_lease_like_cpp, battle_pet_xp_per_level_like_cpp,
    record_battle_pet_trainer_purchase_criteria_like_cpp,
};
pub use uncage::{
    apply_grant_battle_pet_experience_effect_like_cpp,
    apply_grant_battle_pet_level_effect_like_cpp, apply_uncage_battle_pet_effect_like_cpp,
    destroy_uncaged_battle_pet_item_durable_like_cpp,
};

pub const BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP: u32 = 222;
pub const MAX_BATTLE_PET_LEVEL_LIKE_CPP: u16 = 25;

/// The World operations the durable battle-pet bodies still need.
pub trait BattlePetSessionHostLikeCpp: SessionHubHostLikeCpp {
    fn lifecycle(&self) -> &SessionLifecycleState;
    fn split_lifecycle_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>);
    fn split_inventory_ref(&self) -> (&InventoryState, HubRef<'_>);
    fn uncage_item_state_like_cpp(
        &self,
        player_db_guid: u64,
        item_db_guid: u64,
    ) -> impl Future<Output = PlayerUncageItemStateLoadOutcomeLikeCpp> + Send;
    fn destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item: InventoryItem,
        runtime_item: Option<Item>,
        expected_owner_db_guid: Option<u64>,
        context: &str,
    ) -> impl Future<Output = bool> + Send;
    /// The World test build's represented `BattlePetMgr::AddPet`; `None`
    /// everywhere else.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn battle_pet_add_pet_fixture_like_cpp(
        &mut self,
        _species: u32,
        _display_id: u32,
        _breed: u16,
        _quality: u8,
        _level: u16,
    ) -> Option<Option<ObjectGuid>> {
        None
    }
    /// The World test build's represented `GrantBattlePetExperience`.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn battle_pet_grant_experience_fixture_like_cpp(
        &mut self,
        _pet_guid: ObjectGuid,
        _xp: u16,
        _xp_source: RepresentedBattlePetXpSourceLikeCpp,
        _pet_battle_xp_multiplier: f32,
    ) -> Option<RepresentedBattlePetGrantExperienceOutcomeLikeCpp> {
        None
    }
    /// The World test build's represented `GrantBattlePetLevel`.
    #[cfg(any(test, feature = "test-fixtures"))]
    fn battle_pet_grant_level_fixture_like_cpp(
        &mut self,
        _pet_guid: ObjectGuid,
        _granted_levels: u16,
    ) -> Option<RepresentedBattlePetGrantLevelOutcomeLikeCpp> {
        None
    }
}

pub fn apply_battle_pet_calculated_stats_like_cpp(
    pet: &mut RepresentedBattlePetDataLikeCpp,
    calculated_stats: Option<RepresentedBattlePetCalculatedStatsLikeCpp>,
) {
    if let Some(calculated_stats) = calculated_stats {
        pet.max_health = calculated_stats.max_health;
        pet.power = calculated_stats.power;
        pet.speed = calculated_stats.speed;
    }
    pet.health = pet.max_health;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlePetXpSourceLikeCpp {
    PetBattle,
    SpellEffect,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlePetQualityOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    QualityAboveRare,
    CantBattle,
    NotUpgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlePetGrantLevelOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    CantBattle,
    AlreadyMaxLevel,
    NoGrantedLevels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    InvalidXpOrSource,
    CantBattle,
    AlreadyMaxLevel,
    MissingXpRow,
}
