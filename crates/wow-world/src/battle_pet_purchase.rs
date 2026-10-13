// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable saga for battle-pet trainer purchases (issue #161): the saga moved
//! to `wow_world_inventory::battle_pet_purchase` (#1363 slice 7a) behind
//! `BattlePetPurchaseHostLikeCpp` (adapter [`host`]). The test entry points and
//! the names the executor tests read through `super::*` stay here.

#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use wow_core::ObjectGuid;
#[cfg(test)]
use wow_data::battle_pet_selection::BattlePetTrainerSelectionLikeCpp;
#[cfg(test)]
use wow_packet::packets::trainer::{LearnedSpells, TrainerBuyFailed};
#[cfg(test)]
use wow_persistence::{
    BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseCommandLikeCpp,
    BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseMarkOutcomeLikeCpp,
    BattlePetPurchasePersistencePortLikeCpp as BattlePetPurchaseStoreLikeCpp,
    BattlePetPurchaseStatusLikeCpp, BattlePetPurchaseStoreErrorLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    BattlePetPurchaseCommitFenceLikeCpp, PersistenceFutureLikeCpp as BattlePetPurchaseFuture,
    reconcile_battle_pet_purchase_charge_like_cpp, reconcile_battle_pet_purchase_mark_like_cpp,
};
#[cfg(test)]
use wow_world_inventory::{
    BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP, BattlePetPurchaseAdmissionFailureLikeCpp,
    BattlePetPurchaseExecutionLikeCpp, BattlePetPurchaseRecoveryLikeCpp,
};

#[cfg(test)]
use crate::battle_pet_account::{BattlePetAccountOwnerLikeCpp, BattlePetAddRequestKeyLikeCpp};
#[cfg(test)]
use crate::session::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp, WorldSession,
};
#[cfg(test)]
use wow_world_application::PreparedBattlePetTrainerOfferLikeCpp;

mod host;
#[cfg(test)]
mod ops_1;

#[cfg(test)]
#[path = "../unit_tests/battle_pet_purchase/executor_tests/mod.rs"]
mod executor_tests;
#[cfg(test)]
#[path = "../unit_tests/battle_pet_purchase/tests/mod.rs"]
pub(crate) mod tests;
