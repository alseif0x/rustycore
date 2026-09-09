//! Battle-pet purchase executor tests.
//!
//! Separated from the battle_pet_purchase.rs root under #656.

use super::*;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration as StdDuration;

use tokio::sync::Notify;
use wow_core::guid::HighGuid;
use wow_core::{ObjectGuid, Position};
use wow_data::{
    BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP, BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
    BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP, BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
    BattlePetBreedQualityEntry, BattlePetBreedQualityStore, BattlePetBreedStateEntry,
    BattlePetBreedStateStore, BattlePetSpeciesEntry, BattlePetSpeciesStateEntry,
    BattlePetSpeciesStateStore, BattlePetSpeciesStore,
};
use wow_packet::{ServerPacket, WorldPacket};

use super::tests::{FakeBattlePetPurchaseStoreLikeCpp, test_money_commit_fence_like_cpp};
use super::*;
use crate::battle_pet_account::{
    BattlePetAccountRegistryLikeCpp, BattlePetPersistenceErrorLikeCpp, BattlePetPersistenceLikeCpp,
    BattlePetProcessLeaseLikeCpp, DurableBattlePetAddLikeCpp, DurableBattlePetAddReceiptLikeCpp,
    DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp, LoadedBattlePetAccountLikeCpp,
    PersistBattlePetAddOutcomeLikeCpp,
};
use crate::session::SessionPlayerController;

// ── Handler-level fixtures (CMSG_TRAINER_BUY_SPELL end to end) ────

// ── Castable (wrapper) spells that still carry a battle-pet species ──

mod fixtures;
#[allow(unused_imports)]
pub(crate) use fixtures::*;

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
