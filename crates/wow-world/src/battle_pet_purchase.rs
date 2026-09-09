// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable saga for battle-pet trainer purchases (issue #161).
//!
//! C++ anchors: `Trainer::TeachSpell`
//! (`/home/server/woltk-trinity-legacy/src/server/game/Entities/Creature/Trainer.cpp:79-147`),
//! `BattlePetMgr::{AddPet,SaveToDB}`
//! (`/home/server/woltk-trinity-legacy/src/server/game/BattlePets/BattlePetMgr.cpp:331-490`),
//! and the Character-first / Login-second commit order in `Player::SaveToDB`
//! (`/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp:19336-19344`).
//!
//! The legacy server charges money in memory at buy time and persists both
//! databases only at the next save, committing Character DB first and Login
//! DB second; a crash between the two commits keeps the charge and loses the
//! pet, and `BattlePetMgr::SaveToDB` clears `SaveInfo` when statements are
//! appended (`BattlePetMgr.cpp:377`), so the loss is silent and permanent.
//! No portable SQL transaction spans the two pools, so this module records a
//! durable purchase command in the same Character DB transaction that deducts
//! the money, applies it exactly once through the issue #160 account owner
//! (the command `request_key` is the Login DB `battle_pet_add_requests`
//! receipt identity), records completion after publishing the success update,
//! and compensates terminal failures exactly once. The durable `published`
//! marker is written after packets are queued. A crash can therefore cause an
//! idempotent re-send, but cannot permanently lose the only notification;
//! exactly-once network delivery is impossible without a client ACK. A
//! `Completed` row with a clear marker is the recovery-publication signal.
//! Login recovery converges any
//! interrupted command; no in-memory state decides whether a charge, pet or
//! refund already happened.
//!
//! State model (`character_battle_pet_purchase.status`): `PendingApplication`
//! (0), `Completed` (1), `CompensationPending` (2), `Compensated` (3),
//! `TerminalFailure` (4). The reference model's `PetApplied` state is
//! deliberately derived rather than persisted: the #160 Login DB receipt is
//! itself the durable "pet applied" fact and recovery re-derives it by
//! receipt lookup, so a redundant Character DB state could only disagree
//! with the authority. The full transition table lives in
//! `docs/migration/battlepets.md` (2026-08-03, #161).

use std::future::Future;
use std::sync::Arc;

use rand::RngCore;
use tokio::time::{Duration, sleep};
use tracing::warn;
use wow_core::ObjectGuid;
use wow_data::battle_pet_selection::{
    BattlePetTrainerSelectionLikeCpp, select_battle_pet_trainer_pet_like_cpp,
};
use wow_packet::packets::misc::BattlePetJournalPet;
use wow_packet::packets::trainer::{LearnedSpells, TrainerBuyFailed};
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

use crate::battle_pet_account::{
    BattlePetAccountOwnerLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp, BattlePetLeaseIdLikeCpp,
};
use crate::session::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp, WorldSession,
};
use crate::trainer_offer::PreparedBattlePetTrainerOfferLikeCpp;

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

#[cfg(test)]
#[path = "battle_pet_purchase/executor_tests/mod.rs"]
mod executor_tests;
#[cfg(test)]
#[path = "battle_pet_purchase/tests/mod.rs"]
pub(crate) mod tests;

// ── Saga executor (live purchase + login recovery) ────────────────────────
