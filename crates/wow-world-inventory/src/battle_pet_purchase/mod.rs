// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable saga for battle-pet trainer purchases (issue #161; moved from the
//! World `battle_pet_purchase.rs` under #1363 slice 7a).
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

//!
//! The saga runs on the Player's session. Each [`BattlePetPurchaseHostLikeCpp`]
//! method is the same-named `WorldSession` operation, called at the exact
//! point the body called it before the move: the battle-pet journal reads and
//! lease, the purchase criteria, money staging and publication, the quest
//! objective drain and the dependent spell learn stay with the host.

use std::future::Future;
use std::sync::Arc;

use rand::RngCore;
use tokio::time::{Duration, sleep};
use tracing::warn;
use wow_core::ObjectGuid;
use wow_packet::packets::misc::BattlePetJournalPet;
use wow_packet::packets::trainer::{LearnedSpells, TrainerBuyFailed};
use wow_persistence::{
    BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseCommandLikeCpp,
    BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseMarkOutcomeLikeCpp,
    BattlePetPurchasePersistencePortLikeCpp as BattlePetPurchaseStoreLikeCpp,
    BattlePetPurchaseStatusLikeCpp, BattlePetPurchaseStoreErrorLikeCpp,
};
use wow_world_core::battle_pet_account::{
    BattlePetAccountOwnerLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp, BattlePetLeaseIdLikeCpp,
};
use wow_world_core::session::{HubMut, SessionHubHostLikeCpp};
use wow_world_lifecycle::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp,
    SessionLifecycleState,
};

mod ops_1;
mod ops_2;
mod state;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use ops_2::*;
#[allow(unused_imports)]
pub use state::*;

/// C++ `Trainer::TeachSpell` reaches `BattlePetMgr::AddPet` for a confirmed
/// species only when the trainer spell is not castable. The live purchase
/// saga remains a separate operation after this pure offer decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedBattlePetTrainerOfferLikeCpp {
    pub source_spell_id: u32,
    pub effective_price: u32,
    pub species_id: u32,
}

pub trait BattlePetPurchaseHostLikeCpp: SessionHubHostLikeCpp {
    fn lifecycle(&self) -> &SessionLifecycleState;
    fn split_lifecycle_mut(&mut self) -> (&mut SessionLifecycleState, HubMut<'_>);
    fn battle_pet_account_owner_lease_like_cpp(
        &self,
    ) -> Option<(Arc<BattlePetAccountOwnerLikeCpp>, BattlePetLeaseIdLikeCpp)>;
    fn battle_pet_purchase_store_like_cpp(&self) -> Option<Arc<dyn BattlePetPurchaseStoreLikeCpp>>;
    fn battle_pet_species_entry_like_cpp(
        &self,
        species: u32,
    ) -> Option<wow_data::BattlePetSpeciesEntry>;
    fn battle_pet_try_acquire_journal_lease_like_cpp(&self) -> impl Future<Output = bool> + Send;
    fn record_battle_pet_trainer_purchase_criteria_like_cpp(&mut self, species: u32);
    fn resolved_player_money_like_cpp(&self) -> Option<u64>;
    fn stage_player_money_change_like_cpp(&mut self, old_money: u64, new_money: u64) -> bool;
    fn send_player_values_update_from_entity_bridge(
        &self,
        inv_slot_changes: &[(u8, ObjectGuid)],
        visible_item_changes: &[(u8, i32, u16, u16)],
        virtual_item_changes: &[(u8, i32, u16, u16)],
        buyback_changes: &[(u8, u32, i64)],
        coinage: Option<u64>,
    ) -> bool;
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;
    fn learn_dependent_known_spell_like_cpp(&mut self, spell_id: i32);
    fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send;
}
