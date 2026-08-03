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
//! receipt identity), records completion before publishing, and compensates
//! terminal failures exactly once. Login recovery converges any interrupted
//! command; no in-memory state decides whether a charge, pet or refund
//! already happened.
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
use std::pin::Pin;
use std::sync::Arc;

use rand::RngCore;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};
use wow_core::ObjectGuid;
use wow_data::battle_pet_selection::{
    BattlePetTrainerSelectionLikeCpp, select_battle_pet_trainer_pet_like_cpp,
};
use wow_database::{CharStatements, CharacterDatabase, SqlResult, SqlTransaction};
use wow_packet::packets::misc::BattlePetJournalPet;
use wow_packet::packets::trainer::{LearnedSpells, TrainerBuyFailed};

use crate::battle_pet_account::{
    BattlePetAccountOwnerLikeCpp, BattlePetAddFailureLikeCpp, BattlePetAddOutcomeLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp,
};
use crate::session::{ExclusivePlayerMoneyPersistenceLikeCpp, WorldSession};
use crate::trainer_offer::PreparedBattlePetTrainerOfferLikeCpp;

/// Bounded login-recovery batch: at most this many unconverged commands are
/// resumed per character login; the remainder converges on later logins.
pub(crate) const BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP: u32 = 8;

/// Bounded synchronous retry for retryable store transitions. The retry is
/// always the identical transition against durable state, never a new
/// purchase attempt, so replaying it cannot double-charge or double-grant.
pub(crate) const BATTLE_PET_PURCHASE_MAX_ATTEMPTS_LIKE_CPP: u32 = 3;

/// Bounded linear backoff base (ms) between retryable-transition attempts:
/// 0, 25, 50 ms — well under one session tick budget in aggregate.
pub(crate) const BATTLE_PET_PURCHASE_RETRY_BACKOFF_MS_LIKE_CPP: u64 = 25;

pub(crate) type BattlePetPurchaseFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Durable saga states (`character_battle_pet_purchase.status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseStatusLikeCpp {
    /// Money charged and command durable; pet not yet confirmed durable.
    PendingApplication,
    /// Terminal success: the Login DB receipt confirmed the durable pet and
    /// the guarded flip committed before any publication.
    Completed,
    /// Terminal-failure decision durable; the refund is still owed.
    CompensationPending,
    /// Terminal: the refund and this flip committed in one transaction, so
    /// the refund ran exactly once.
    Compensated,
    /// Terminal: the refund is impossible automatically (the character row
    /// is gone); operator attention, never silently retried.
    TerminalFailure,
}

impl BattlePetPurchaseStatusLikeCpp {
    pub(crate) fn as_u8_like_cpp(self) -> u8 {
        match self {
            Self::PendingApplication => 0,
            Self::Completed => 1,
            Self::CompensationPending => 2,
            Self::Compensated => 3,
            Self::TerminalFailure => 4,
        }
    }

    pub(crate) fn from_u8_like_cpp(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::PendingApplication),
            1 => Some(Self::Completed),
            2 => Some(Self::CompensationPending),
            3 => Some(Self::Compensated),
            4 => Some(Self::TerminalFailure),
            _ => None,
        }
    }

    /// `Completed` is recorded only after the durable pet exists, so both
    /// terminal-success and terminal-compensation states are settled.
    pub(crate) fn is_terminal_like_cpp(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Compensated | Self::TerminalFailure
        )
    }
}

/// One durable purchase command. The payload columns are the stable inputs
/// selected at admission so an interrupted command can resume without
/// re-rolling display, breed or quality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattlePetPurchaseCommandLikeCpp {
    pub request_key: [u8; 16],
    pub character_guid: u64,
    pub account_id: u32,
    pub trainer_id: u32,
    pub spell_id: u32,
    pub species: u32,
    pub breed: u16,
    pub quality: u8,
    pub display_id: u32,
    pub level: u16,
    pub price: u32,
    pub money_before: u64,
    pub money_after: u64,
    pub status: BattlePetPurchaseStatusLikeCpp,
    pub failure_reason: Option<String>,
}

/// Store failure vocabulary shared by the production and fake stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseStoreErrorLikeCpp {
    /// The transition provably did not commit and may be retried as-is.
    Retryable(String),
    /// The transition provably did not commit and retrying this transition
    /// cannot succeed (a guarded precondition was violated).
    Terminal(String),
    /// A COMMIT reply was lost and reconciliation could not prove the
    /// outcome; the session must be quarantined rather than guess.
    Indeterminate(String),
}

/// T1 result: the money deduction and the pending command commit atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseChargeOutcomeLikeCpp {
    Charged,
    /// Definitely no charge and no command (the guarded money precondition
    /// no longer held, or the transaction provably rolled back).
    RolledBack,
}

/// Guarded single-row status transition result (T3/T4/T6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseMarkOutcomeLikeCpp {
    Applied,
    /// The row was already in the target state: an idempotent replay.
    AlreadyApplied,
    /// The row is `Completed`: a concurrent driver finished the purchase, so
    /// a pending compensation decision must be dropped, never refunded.
    ConflictedCompleted,
    /// The row is `Compensated` or `TerminalFailure` while a completion was
    /// requested; unreachable while the #160 fence serializes drivers, so it
    /// is logged loudly instead of silently accepted.
    ConflictedCompensated,
}

/// T5 result: the refund and the `Compensated` flip commit atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseCompensationOutcomeLikeCpp {
    Compensated,
    /// Replay: the row was already `Compensated`; the refund cannot run twice.
    AlreadyCompensated,
    /// The command is `Completed` (the pet exists): refunding is forbidden.
    ConflictedCompleted,
    /// The character row is gone, so the refund can never apply; the caller
    /// records `TerminalFailure` instead of retrying forever.
    CharacterMissing,
}

/// Reconcile a failed T1 commit by re-reading the durable command row. The
/// row can only exist when the charge transaction committed, because the
/// money deduction and the insert share one transaction.
pub(crate) fn reconcile_battle_pet_purchase_charge_like_cpp(
    row: Option<&BattlePetPurchaseCommandLikeCpp>,
    request_key: [u8; 16],
) -> BattlePetPurchaseChargeOutcomeLikeCpp {
    match row {
        Some(row) if row.request_key == request_key => {
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        }
        _ => BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack,
    }
}

/// Character DB seam for the saga. Every method maps to exactly one durable
/// transition of the state model; handlers never carry SQL of their own.
pub(crate) trait BattlePetPurchaseStoreLikeCpp: Send + Sync {
    /// T1: deduct the guarded money and insert the `PendingApplication`
    /// command in one transaction.
    fn charge_and_insert_command<'a>(
        &'a self,
        command: BattlePetPurchaseCommandLikeCpp,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// Read one command by its globally unique request key.
    fn load_command<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<Option<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// Read every unconverged (`PendingApplication`/`CompensationPending`)
    /// command of one character, oldest first, bounded by `limit`.
    fn load_pending_commands<'a>(
        &'a self,
        character_guid: u64,
        limit: u32,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T3: flip `PendingApplication`/`CompensationPending` → `Completed`.
    /// The wider source guard also closes a recorded compensation decision
    /// once the pet is known to be durable (receipt re-check).
    fn mark_completed<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T4: record the terminal-failure decision
    /// (`PendingApplication` → `CompensationPending`).
    fn mark_compensation_pending<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T5: refund the price and flip to `Compensated` in one transaction.
    fn compensate<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T6: record an automatically unrecoverable command
    /// (`CompensationPending` → `TerminalFailure`); best effort.
    fn mark_terminal_failure<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>>;
}

/// Production Character DB store. All SQL lives in `CharStatements`; the
/// transaction shapes reuse the issue #159 commit-outcome vocabulary.
pub(crate) struct CharacterBattlePetPurchaseStoreLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl CharacterBattlePetPurchaseStoreLikeCpp {
    pub(crate) fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }

    fn read_command_row_like_cpp(
        result: &SqlResult,
    ) -> Result<BattlePetPurchaseCommandLikeCpp, BattlePetPurchaseStoreErrorLikeCpp> {
        let key_bytes: Vec<u8> = result.try_read(0).unwrap_or_default();
        let request_key: [u8; 16] = key_bytes.try_into().map_err(|_| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase request_key is not 16 bytes".to_string(),
            )
        })?;
        let status_raw: u8 = result.try_read(13).ok_or_else(|| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase row lacks a status".to_string(),
            )
        })?;
        let status =
            BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(status_raw).ok_or_else(|| {
                BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                    "battle-pet purchase row has unknown status {status_raw}"
                ))
            })?;
        Ok(BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: result.try_read(1).unwrap_or_default(),
            account_id: result.try_read(2).unwrap_or_default(),
            trainer_id: result.try_read(3).unwrap_or_default(),
            spell_id: result.try_read(4).unwrap_or_default(),
            species: result.try_read(5).unwrap_or_default(),
            breed: result.try_read(6).unwrap_or_default(),
            quality: result.try_read(7).unwrap_or_default(),
            display_id: result.try_read(8).unwrap_or_default(),
            level: result.try_read(9).unwrap_or_default(),
            price: result.try_read(10).unwrap_or_default(),
            money_before: result.try_read(11).unwrap_or_default(),
            money_after: result.try_read(12).unwrap_or_default(),
            status,
            failure_reason: result.try_read(14),
        })
    }

    async fn load_command_impl(
        &self,
        request_key: [u8; 16],
    ) -> Result<Option<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp> {
        let mut statement = self
            .character_db
            .prepare(CharStatements::SEL_BATTLE_PET_PURCHASE_BY_KEY);
        statement.set_bytes(0, request_key.to_vec());
        let result = self.character_db.query(&statement).await.map_err(|error| {
            BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                "battle-pet purchase read failed: {error}"
            ))
        })?;
        if result.is_empty() {
            return Ok(None);
        }
        Self::read_command_row_like_cpp(&result).map(Some)
    }

    fn mark_statement_like_cpp(
        &self,
        statement_def: CharStatements,
        request_key: [u8; 16],
        reason: Option<&'static str>,
    ) -> SqlTransaction {
        let mut statement = self.character_db.prepare(statement_def);
        let mut index = 0;
        if let Some(reason) = reason {
            statement.set_string(index, reason);
            index += 1;
        }
        statement.set_bytes(index, request_key.to_vec());
        let mut transaction = SqlTransaction::new();
        transaction.append_expect_rows_affected(statement, 1);
        transaction
    }

    fn reconcile_mark_like_cpp(
        row: Option<&BattlePetPurchaseCommandLikeCpp>,
        pending_source: BattlePetPurchaseStatusLikeCpp,
        target: BattlePetPurchaseStatusLikeCpp,
    ) -> Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp> {
        let Some(row) = row else {
            return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase command disappeared during a status transition".to_string(),
            ));
        };
        Ok(match row.status {
            status if status == target => BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
            BattlePetPurchaseStatusLikeCpp::Completed => {
                BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted
            }
            BattlePetPurchaseStatusLikeCpp::Compensated
            | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
                BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
            }
            status if status == pending_source => {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                    "battle-pet purchase status transition did not commit".to_string(),
                ));
            }
            status => {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                    "battle-pet purchase status transition from {status:?} is not owned here"
                )));
            }
        })
    }
}

impl BattlePetPurchaseStoreLikeCpp for CharacterBattlePetPurchaseStoreLikeCpp {
    fn charge_and_insert_command<'a>(
        &'a self,
        command: BattlePetPurchaseCommandLikeCpp,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut money = self
                .character_db
                .prepare(CharStatements::UPD_CHARACTER_MONEY_GUARDED);
            money.set_u64(0, command.money_after);
            money.set_u64(1, command.character_guid);
            money.set_u64(2, command.money_before);
            let mut insert = self
                .character_db
                .prepare(CharStatements::INS_BATTLE_PET_PURCHASE);
            insert.set_bytes(0, command.request_key.to_vec());
            insert.set_u64(1, command.character_guid);
            insert.set_u32(2, command.account_id);
            insert.set_u32(3, command.trainer_id);
            insert.set_u32(4, command.spell_id);
            insert.set_u32(5, command.species);
            insert.set_u16(6, command.breed);
            insert.set_u8(7, command.quality);
            insert.set_u32(8, command.display_id);
            insert.set_u16(9, command.level);
            insert.set_u32(10, command.price);
            insert.set_u64(11, command.money_before);
            insert.set_u64(12, command.money_after);
            insert.set_u8(
                13,
                BattlePetPurchaseStatusLikeCpp::PendingApplication.as_u8_like_cpp(),
            );
            let mut transaction = SqlTransaction::new();
            transaction.append_expect_rows_affected(money, 1);
            transaction.append_expect_rows_affected(insert, 1);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged),
                Err(error) => {
                    // Whether the COMMIT reply was lost or the transaction
                    // definitely rolled back, only the durable row can say
                    // if the charge exists; it is the same transaction.
                    let reconciled = self.load_command_impl(command.request_key).await;
                    match reconciled {
                        Ok(row) => Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                            row.as_ref(),
                            command.request_key,
                        )),
                        Err(_) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                            "battle-pet purchase charge COMMIT outcome is unknown and the command row cannot be read: {error}"
                        ))),
                    }
                }
            }
        })
    }

    fn load_command<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<Option<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move { self.load_command_impl(request_key).await })
    }

    fn load_pending_commands<'a>(
        &'a self,
        character_guid: u64,
        limit: u32,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut statement = self
                .character_db
                .prepare(CharStatements::SEL_BATTLE_PET_PURCHASE_PENDING);
            statement.set_u64(0, character_guid);
            statement.set_u32(1, limit);
            let mut result = self.character_db.query(&statement).await.map_err(|error| {
                BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                    "battle-pet purchase recovery scan failed: {error}"
                ))
            })?;
            let mut commands = Vec::new();
            if !result.is_empty() {
                loop {
                    commands.push(Self::read_command_row_like_cpp(&result)?);
                    if !result.next_row() {
                        break;
                    }
                }
            }
            Ok(commands)
        })
    }

    fn mark_completed<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_COMPLETED,
                request_key,
                None,
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(row) => Self::reconcile_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::Completed,
                    ),
                    Err(error) => Err(error),
                },
            }
        })
    }

    fn mark_compensation_pending<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATION_PENDING,
                request_key,
                Some(reason),
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(row) => Self::reconcile_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::CompensationPending,
                    ),
                    Err(error) => Err(error),
                },
            }
        })
    }

    fn compensate<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let command = match self.load_command_impl(request_key).await? {
                Some(command) => command,
                None => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "battle-pet purchase command to compensate does not exist".to_string(),
                    ));
                }
            };
            match command.status {
                BattlePetPurchaseStatusLikeCpp::Compensated => {
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated);
                }
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted);
                }
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {}
                status => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                        "battle-pet purchase compensation from {status:?} is not owned here"
                    )));
                }
            }
            let mut refund = self
                .character_db
                .prepare(CharStatements::UPD_CHARACTER_MONEY_REFUND);
            refund.set_u32(0, command.price);
            refund.set_u64(1, wow_entities::MAX_MONEY_AMOUNT);
            refund.set_u64(2, command.character_guid);
            let mut flip = self
                .character_db
                .prepare(CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATED);
            flip.set_bytes(0, request_key.to_vec());
            let mut transaction = SqlTransaction::new();
            transaction.append_expect_rows_affected(refund, 1);
            transaction.append_expect_rows_affected(flip, 1);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated),
                Err(error) => {
                    // The pre-read established `CompensationPending`. A lost
                    // COMMIT reply followed by a `Compensated` read means this
                    // transaction's own refund committed; a definite rollback
                    // followed by `Compensated` means another driver already
                    // compensated. The caller must only restore runtime money
                    // for its own refund.
                    let own_commit_maybe = error.is_commit_outcome_unknown_like_cpp();
                    let row = self.load_command_impl(request_key).await?;
                    let Some(row) = row else {
                        return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "battle-pet purchase command disappeared during compensation"
                                .to_string(),
                        ));
                    };
                    match row.status {
                        BattlePetPurchaseStatusLikeCpp::Compensated => Ok(if own_commit_maybe {
                            BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated
                        } else {
                            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated
                        }),
                        BattlePetPurchaseStatusLikeCpp::Completed => {
                            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted)
                        }
                        BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                            // Still owed: either the transaction provably
                            // rolled back or its reply was lost pre-commit.
                            // A missing character row is the one cause that
                            // can never converge and becomes TerminalFailure.
                            let mut probe = self.character_db.prepare(CharStatements::cpp(
                                "SELECT money FROM characters WHERE guid = ?",
                            ));
                            probe.set_u64(0, command.character_guid);
                            let probe = self.character_db.query(&probe).await.map_err(|error| {
                                BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                                    "battle-pet purchase compensation character probe failed: {error}"
                                ))
                            })?;
                            if probe.is_empty() {
                                Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing)
                            } else {
                                Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                                    "battle-pet purchase compensation did not commit".to_string(),
                                ))
                            }
                        }
                        status => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                            "battle-pet purchase compensation observed unexpected {status:?}"
                        ))),
                    }
                }
            }
        })
    }

    fn mark_terminal_failure<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>> {
        Box::pin(async move {
            let transaction = self.mark_statement_like_cpp(
                CharStatements::UPD_BATTLE_PET_PURCHASE_TERMINAL_FAILURE,
                request_key,
                Some(reason),
            );
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => {
                    info!(
                        target: "battle_pet_purchase",
                        reason,
                        "Battle-pet purchase recorded as terminal failure"
                    );
                    Ok(())
                }
                Err(error) => {
                    let reconciled = self.load_command_impl(request_key).await;
                    match reconciled {
                        Ok(Some(row))
                            if row.status == BattlePetPurchaseStatusLikeCpp::TerminalFailure =>
                        {
                            Ok(())
                        }
                        _ => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                            "battle-pet purchase terminal-failure mark did not commit: {error}"
                        ))),
                    }
                }
            }
        })
    }
}

// ── Saga executor (live purchase + login recovery) ────────────────────────

/// Structured admission-time failure of a battle-pet purchase (issue #161).
/// C++ sends no packet for the capacity case (`Trainer.cpp:102-106`,
/// "Don't send any error to client (intended)") and has no journal-lock
/// case at all, so the wire stays silent while the typed result keeps the
/// failure observable to tests, diagnostics and the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseAdmissionFailureLikeCpp {
    /// No #160 attachment/owner for this account: the journal cannot be
    /// durable, so the purchase fails closed.
    NoJournalAuthority,
    /// The journal lease could not be acquired at admission.
    JournalLocked,
    /// The per-species account capacity was already reached (C++
    /// `HasMaxPetCount`).
    Capacity,
    /// No Character DB saga store or no player identity is available.
    StoreUnavailable,
    /// The confirmed species cannot be materialized (no DB2 species row or
    /// no selection store).
    SelectionUnavailable,
}

/// Terminal outcome of one live purchase execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetPurchaseExecutionLikeCpp {
    /// Pet durable, command `Completed`; `published` is true exactly when
    /// this execution sent the one `SMSG_BATTLE_PET_UPDATES` petAdded
    /// (an `Added` outcome). A replayed receipt completes silently.
    Purchased {
        pet_guid: ObjectGuid,
        published: bool,
    },
    /// Admission refused before any charge; wire-silent typed result.
    Unavailable(BattlePetPurchaseAdmissionFailureLikeCpp),
    /// C++ `FailReason::NotEnoughMoney`; `SMSG_TRAINER_BUY_FAILED` reason 1.
    InsufficientMoney,
    /// The charge transaction provably rolled back: no charge, no command.
    ChargeDeclined,
    /// The charge COMMIT could not be reconciled; the session was
    /// quarantined (kick) exactly like the #159 money boundary.
    ChargeIndeterminate,
    /// A retryable step exhausted its bounded attempts; the command stays
    /// `PendingApplication` and login recovery resumes it.
    RetryableDeferred,
    /// A terminal apply failure was recorded and refunded exactly once.
    Compensated,
    /// The terminal-failure decision is durable but the refund did not
    /// converge; the command stays `CompensationPending` for recovery.
    CompensationDeferred,
    /// The command reached `TerminalFailure`: operator attention, no
    /// automatic retry, no silent money loss.
    TerminalFailure,
    /// A concurrent driver already completed the command (its publication,
    /// if any, is not repeated here).
    CompletedElsewhere,
}

/// Login-recovery summary for diagnostics and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattlePetPurchaseRecoveryLikeCpp {
    pub applied: u32,
    pub compensated: u32,
    pub deferred: u32,
    pub terminal_failures: u32,
}

/// `BattlePetAddFailureLikeCpp` classes that may succeed on a later attempt
/// (transient DB trouble, lease churn, GUID collision with a fresh counter).
fn battle_pet_add_failure_is_retryable_like_cpp(error: &BattlePetAddFailureLikeCpp) -> bool {
    matches!(
        error,
        BattlePetAddFailureLikeCpp::MissingAuthority
            | BattlePetAddFailureLikeCpp::JournalLocked
            | BattlePetAddFailureLikeCpp::Busy
            | BattlePetAddFailureLikeCpp::GuidCollision
            | BattlePetAddFailureLikeCpp::DatabaseFailure(_)
    )
}

/// Failure classes that can never succeed for this exact payload: the C++
/// per-species cap, an unleasant/unlearnable species, or a receipt-key
/// payload conflict.
fn battle_pet_add_failure_is_terminal_like_cpp(error: &BattlePetAddFailureLikeCpp) -> bool {
    matches!(
        error,
        BattlePetAddFailureLikeCpp::Capacity
            | BattlePetAddFailureLikeCpp::InvalidSpecies
            | BattlePetAddFailureLikeCpp::DuplicateRequest
    )
}

/// Bounded identical-transition retry with a bounded linear backoff. The
/// retried transition is always the same durable step, never a new purchase
/// attempt, so replaying it cannot double-charge or double-grant.
async fn retry_battle_pet_purchase_step_like_cpp<T, E, Fut, F>(
    mut step: F,
    is_retryable: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempt = 0_u32;
    loop {
        match step().await {
            Err(error)
                if is_retryable(&error)
                    && attempt + 1 < BATTLE_PET_PURCHASE_MAX_ATTEMPTS_LIKE_CPP =>
            {
                attempt += 1;
                sleep(Duration::from_millis(
                    BATTLE_PET_PURCHASE_RETRY_BACKOFF_MS_LIKE_CPP * u64::from(attempt),
                ))
                .await;
            }
            result => return result,
        }
    }
}

fn store_error_is_retryable_like_cpp(error: &BattlePetPurchaseStoreErrorLikeCpp) -> bool {
    matches!(error, BattlePetPurchaseStoreErrorLikeCpp::Retryable(_))
}

/// Whether a compensation publishes its restored money with a values-update
/// packet (in-world live path) or only stages the runtime value (login
/// recovery, where the initial object update carries it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattlePetPurchaseRefundPublicationLikeCpp {
    ValuesUpdatePacket,
    RuntimeOnly,
}

impl WorldSession {
    /// Issue #161 live purchase: revalidates the offer admission under the
    /// exclusive money guard, commits charge + durable command, applies the
    /// pet once through the #160 account owner, completes, then publishes in
    /// the C++ `Trainer::TeachSpell` battle-pet order (money update,
    /// `SMSG_BATTLE_PET_UPDATES`, dependent `SMSG_LEARNED_SPELLS`) with the
    /// trainer visual kits suppressed (`Trainer.cpp:108,121-125`).
    pub(crate) async fn execute_battle_pet_trainer_purchase_like_cpp(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        trainer_guid: ObjectGuid,
        trainer_id: u32,
        offer: PreparedBattlePetTrainerOfferLikeCpp,
    ) -> BattlePetPurchaseExecutionLikeCpp {
        let Some((owner, lease_id)) = self.battle_pet_account_owner_lease_like_cpp() else {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::NoJournalAuthority,
            );
        };
        let Some(store) = self.battle_pet_purchase_store_like_cpp() else {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::StoreUnavailable,
            );
        };
        let Some(player_guid) = self.player_guid() else {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::StoreUnavailable,
            );
        };
        let Some(species_entry) = self.battle_pet_species_entry_like_cpp(offer.species_id) else {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::SelectionUnavailable,
            );
        };

        // C++ checks the per-species account cap before charging and stays
        // silent (`Trainer.cpp:102-106`). The #160 owner rechecks it inside
        // its own transaction; this admission check only avoids creating a
        // command that would have to be compensated.
        if owner.has_max_pet_count_like_cpp(offer.species_id, Some(player_guid)) {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::Capacity,
            );
        }
        if !self.battle_pet_try_acquire_journal_lease_like_cpp().await {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked,
            );
        }
        let Some(selection) = self.battle_pet_trainer_selection_like_cpp(&species_entry) else {
            return BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::SelectionUnavailable,
            );
        };

        let old_money = self.player_gold_like_cpp();
        let price = u64::from(offer.effective_price);
        if old_money < price {
            // C++ `FailReason::NotEnoughMoney` (`Trainer.cpp:113-117`).
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id: offer.source_spell_id as i32,
                reason: 1,
            });
            return BattlePetPurchaseExecutionLikeCpp::InsufficientMoney;
        }
        let new_money = old_money - price;

        // T1: the guarded money deduction and the pending command commit in
        // one Character DB transaction; the request key is also the #160
        // Login DB receipt identity.
        let mut request_key = [0_u8; 16];
        rand::thread_rng().fill_bytes(&mut request_key);
        let command = BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: player_guid.counter() as u64,
            account_id: self.account_id,
            trainer_id,
            spell_id: offer.source_spell_id,
            species: selection.species,
            breed: selection.breed,
            quality: selection.quality,
            display_id: selection.display_id,
            level: selection.level,
            price: offer.effective_price,
            money_before: old_money,
            money_after: new_money,
            status: BattlePetPurchaseStatusLikeCpp::PendingApplication,
            failure_reason: None,
        };
        let charge = retry_battle_pet_purchase_step_like_cpp(
            || store.charge_and_insert_command(command.clone()),
            store_error_is_retryable_like_cpp,
        )
        .await;
        match charge {
            Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged) => {}
            Ok(BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack) => {
                return BattlePetPurchaseExecutionLikeCpp::ChargeDeclined;
            }
            Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(error)) => {
                warn!(
                    account = self.account_id,
                    error,
                    "Battle-pet purchase charge COMMIT outcome is unknown; quarantined the session"
                );
                self.quarantine_player_money_persistence_like_cpp(
                    "battle-pet purchase charge COMMIT outcome is unknown; relog required",
                );
                return BattlePetPurchaseExecutionLikeCpp::ChargeIndeterminate;
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase charge did not commit"
                );
                return BattlePetPurchaseExecutionLikeCpp::ChargeDeclined;
            }
        }

        // Publish the durable charge under the guard (C++ `ModifyMoney`),
        // then release it before draining criteria, matching the #159 order.
        self.stage_player_money_change_like_cpp(old_money, new_money);
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
        drop(money_persistence);
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;

        // T2: apply once through the #160 owner; its Login DB transaction
        // revalidates fence, lease and per-species capacity and writes pet +
        // receipt together, so a replay returns the original pet.
        let request = BattlePetAddRequestLikeCpp {
            request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
            species: selection.species,
            display_id: selection.display_id,
            breed: selection.breed,
            quality: selection.quality,
            level: selection.level,
            owner_guid: Some(player_guid),
        };
        let applied = retry_battle_pet_purchase_step_like_cpp(
            || owner.try_add_pet_like_cpp(lease_id, request.clone()),
            battle_pet_add_failure_is_retryable_like_cpp,
        )
        .await;
        match applied {
            Ok(outcome) => {
                let (pet, published) = match outcome {
                    BattlePetAddOutcomeLikeCpp::Added(pet) => (pet, true),
                    BattlePetAddOutcomeLikeCpp::Replayed(pet) => (pet, false),
                };
                // T3: completion is recorded before any publication.
                if !self
                    .complete_battle_pet_purchase_like_cpp(&store, request_key)
                    .await
                {
                    // The pet is durable; recovery finishes the command and
                    // the publication stays silent (at-most-once).
                    return BattlePetPurchaseExecutionLikeCpp::RetryableDeferred;
                }
                let pet_guid = pet.guid;
                if published {
                    self.publish_battle_pet_trainer_purchase_like_cpp(pet, offer.source_spell_id);
                }
                BattlePetPurchaseExecutionLikeCpp::Purchased {
                    pet_guid,
                    published,
                }
            }
            Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
                self.compensate_battle_pet_purchase_like_cpp(
                    &owner,
                    &store,
                    &command,
                    BattlePetPurchaseRefundPublicationLikeCpp::ValuesUpdatePacket,
                )
                .await
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase apply deferred after bounded retries"
                );
                BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
            }
        }
    }

    /// Login recovery: resume every unconverged durable command of this
    /// character, oldest first, bounded by
    /// `BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP`. Runs inline
    /// during the login burst (no background task) and is cancellation-safe:
    /// every step is either a committed transition or leaves a resumable
    /// durable state for the next login.
    pub(crate) async fn recover_battle_pet_trainer_purchases_like_cpp(
        &mut self,
    ) -> Option<BattlePetPurchaseRecoveryLikeCpp> {
        let (owner, lease_id) = self.battle_pet_account_owner_lease_like_cpp()?;
        let store = self.battle_pet_purchase_store_like_cpp()?;
        let player_guid = self.player_guid()?;
        let commands = match store
            .load_pending_commands(
                player_guid.counter() as u64,
                BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP,
            )
            .await
        {
            Ok(commands) => commands,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase recovery scan failed; will retry on next login"
                );
                return None;
            }
        };
        if commands.is_empty() {
            return Some(BattlePetPurchaseRecoveryLikeCpp {
                applied: 0,
                compensated: 0,
                deferred: 0,
                terminal_failures: 0,
            });
        }

        let mut summary = BattlePetPurchaseRecoveryLikeCpp {
            applied: 0,
            compensated: 0,
            deferred: 0,
            terminal_failures: 0,
        };
        let full_batch =
            commands.len() == BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP as usize;
        for command in commands {
            match command.status {
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                    match self
                        .compensate_battle_pet_purchase_like_cpp(
                            &owner,
                            &store,
                            &command,
                            BattlePetPurchaseRefundPublicationLikeCpp::RuntimeOnly,
                        )
                        .await
                    {
                        BattlePetPurchaseExecutionLikeCpp::Compensated => {
                            summary.compensated += 1;
                        }
                        BattlePetPurchaseExecutionLikeCpp::TerminalFailure => {
                            summary.terminal_failures += 1;
                        }
                        _ => {
                            summary.deferred += 1;
                            break;
                        }
                    }
                }
                BattlePetPurchaseStatusLikeCpp::PendingApplication => {
                    if !self.battle_pet_try_acquire_journal_lease_like_cpp().await {
                        summary.deferred += 1;
                        break;
                    }
                    let request = BattlePetAddRequestLikeCpp {
                        request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(command.request_key),
                        species: command.species,
                        display_id: command.display_id,
                        breed: command.breed,
                        quality: command.quality,
                        level: command.level,
                        owner_guid: Some(player_guid),
                    };
                    // One attempt per login per command: a retryable failure
                    // stops the batch and resumes on the next login, keeping
                    // recovery bounded.
                    match owner.try_add_pet_like_cpp(lease_id, request).await {
                        Ok(outcome) => {
                            let (pet, publish) = match outcome {
                                BattlePetAddOutcomeLikeCpp::Added(pet) => (pet, true),
                                BattlePetAddOutcomeLikeCpp::Replayed(pet) => (pet, false),
                            };
                            if !self
                                .complete_battle_pet_purchase_like_cpp(&store, command.request_key)
                                .await
                            {
                                summary.deferred += 1;
                                continue;
                            }
                            if publish {
                                // The recovery publication: the pet became
                                // durable during this recovery, so the one
                                // allowed petAdded update is sent now.
                                self.publish_battle_pet_trainer_purchase_like_cpp(
                                    pet,
                                    command.spell_id,
                                );
                            }
                            summary.applied += 1;
                        }
                        Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
                            match self
                                .compensate_battle_pet_purchase_like_cpp(
                                    &owner,
                                    &store,
                                    &command,
                                    BattlePetPurchaseRefundPublicationLikeCpp::RuntimeOnly,
                                )
                                .await
                            {
                                BattlePetPurchaseExecutionLikeCpp::Compensated => {
                                    summary.compensated += 1;
                                }
                                BattlePetPurchaseExecutionLikeCpp::TerminalFailure => {
                                    summary.terminal_failures += 1;
                                }
                                _ => {
                                    summary.deferred += 1;
                                    break;
                                }
                            }
                        }
                        Err(error) => {
                            warn!(
                                account = self.account_id,
                                ?error,
                                "Battle-pet purchase recovery deferred a command"
                            );
                            summary.deferred += 1;
                            break;
                        }
                    }
                }
                status => {
                    warn!(
                        account = self.account_id,
                        ?status,
                        "Battle-pet purchase recovery scanned an unexpected terminal command"
                    );
                }
            }
        }
        if full_batch {
            warn!(
                account = self.account_id,
                limit = BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP,
                "Battle-pet purchase recovery batch is full; remaining commands resume on later logins"
            );
        }
        Some(summary)
    }

    /// The C++ `AddPet` materialization inputs for one admission, frozen
    /// into the durable command so recovery never re-rolls.
    fn battle_pet_trainer_selection_like_cpp(
        &self,
        species_entry: &wow_data::BattlePetSpeciesEntry,
    ) -> Option<BattlePetTrainerSelectionLikeCpp> {
        #[cfg(test)]
        if let Some(selection) = self.battle_pet_purchase_selection_override_like_cpp() {
            return Some(selection);
        }
        let store = self.battle_pet_selection_store_like_cpp()?;
        let template = self
            .creature_template_lifecycle_store_like_cpp()
            .and_then(|templates| {
                u32::try_from(species_entry.creature_id)
                    .ok()
                    .and_then(|entry| templates.get(entry))
            });
        let mut breed_random = rand::thread_rng();
        let mut display_random = rand::thread_rng();
        Some(select_battle_pet_trainer_pet_like_cpp(
            store,
            species_entry,
            template,
            &mut breed_random,
            &mut display_random,
        ))
    }

    /// T3 with bounded retries. False means the pet is durable but the
    /// completion did not converge; recovery finishes it silently.
    async fn complete_battle_pet_purchase_like_cpp(
        &mut self,
        store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
        request_key: [u8; 16],
    ) -> bool {
        match retry_battle_pet_purchase_step_like_cpp(
            || store.mark_completed(request_key),
            store_error_is_retryable_like_cpp,
        )
        .await
        {
            Ok(
                BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
            ) => true,
            Ok(conflict) => {
                warn!(
                    account = self.account_id,
                    ?conflict,
                    "Battle-pet purchase completion observed a conflicting terminal state"
                );
                false
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase completion deferred"
                );
                false
            }
        }
    }

    /// T4+T5: record the terminal-failure decision and refund exactly once.
    /// The receipt re-check before refunding closes the residual race where
    /// a concurrent driver made the pet durable after all: a durable receipt
    /// forbids the refund and completes the command instead.
    async fn compensate_battle_pet_purchase_like_cpp(
        &mut self,
        owner: &Arc<BattlePetAccountOwnerLikeCpp>,
        store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
        command: &BattlePetPurchaseCommandLikeCpp,
        refund_publication: BattlePetPurchaseRefundPublicationLikeCpp,
    ) -> BattlePetPurchaseExecutionLikeCpp {
        // T4: persist the decision before touching money so recovery never
        // re-applies a command whose compensation was already decided.
        let decision = retry_battle_pet_purchase_step_like_cpp(
            || {
                store.mark_compensation_pending(
                    command.request_key,
                    "battle-pet purchase apply failed terminally",
                )
            },
            store_error_is_retryable_like_cpp,
        )
        .await;
        match decision {
            Ok(
                BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied,
            ) => {}
            Ok(BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted) => {
                return BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere;
            }
            Ok(BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated) => {
                return BattlePetPurchaseExecutionLikeCpp::Compensated;
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation decision could not be recorded; recovery re-derives it"
                );
                return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
            }
        }

        // Receipt re-check: a durable pet forbids the refund.
        match owner
            .add_request_committed_like_cpp(BattlePetAddRequestKeyLikeCpp::from_bytes(
                command.request_key,
            ))
            .await
        {
            Ok(true) => {
                let _ = self
                    .complete_battle_pet_purchase_like_cpp(store, command.request_key)
                    .await;
                return BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere;
            }
            Ok(false) => {}
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation cannot prove the receipt absent; refund deferred"
                );
                return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
            }
        }

        // T5: refund and flip in one Character DB transaction, under the
        // same per-character money exclusion as the charge.
        let Some(refund_guard) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
        };
        let compensated = retry_battle_pet_purchase_step_like_cpp(
            || store.compensate(command.request_key),
            store_error_is_retryable_like_cpp,
        )
        .await;
        match compensated {
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated) => {
                // Only this call's own refund restores runtime money; an
                // earlier compensator already published its own restore.
                let current = self.player_gold_like_cpp();
                let restored = current
                    .saturating_add(u64::from(command.price))
                    .min(wow_entities::MAX_MONEY_AMOUNT);
                self.stage_player_money_change_like_cpp(current, restored);
                if refund_publication
                    == BattlePetPurchaseRefundPublicationLikeCpp::ValuesUpdatePacket
                    && current != restored
                {
                    self.send_player_values_update_from_entity_bridge(
                        &[],
                        &[],
                        &[],
                        &[],
                        Some(restored),
                    );
                }
                drop(refund_guard);
                self.drain_represented_quest_objective_progress_like_cpp()
                    .await;
                BattlePetPurchaseExecutionLikeCpp::Compensated
            }
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated) => {
                drop(refund_guard);
                BattlePetPurchaseExecutionLikeCpp::Compensated
            }
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted) => {
                drop(refund_guard);
                BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere
            }
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing) => {
                drop(refund_guard);
                let marked = retry_battle_pet_purchase_step_like_cpp(
                    || {
                        store.mark_terminal_failure(
                            command.request_key,
                            "battle-pet purchase refund impossible: character row missing",
                        )
                    },
                    store_error_is_retryable_like_cpp,
                )
                .await;
                if let Err(error) = marked {
                    warn!(
                        account = self.account_id,
                        ?error,
                        "Battle-pet purchase terminal-failure mark did not commit"
                    );
                    return BattlePetPurchaseExecutionLikeCpp::CompensationDeferred;
                }
                BattlePetPurchaseExecutionLikeCpp::TerminalFailure
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase compensation did not converge"
                );
                drop(refund_guard);
                BattlePetPurchaseExecutionLikeCpp::CompensationDeferred
            }
        }
    }

    /// The one allowed success publication, in the C++ battle-pet
    /// `TeachSpell` order with trainer visuals suppressed: the petAdded
    /// journal update with its criteria hooks, then the dependent runtime
    /// spell learn (`Player::LearnSpell(dependent=true)` — runtime-only,
    /// never a `character_spell` row) and its `SMSG_LEARNED_SPELLS`.
    fn publish_battle_pet_trainer_purchase_like_cpp(
        &mut self,
        pet: BattlePetJournalPet,
        spell_id: u32,
    ) {
        self.publish_battle_pet_trainer_purchase_add_like_cpp(pet.clone(), pet.species);
        self.learn_dependent_known_spell_like_cpp(spell_id as i32);
        self.send_packet(&LearnedSpells::single(spell_id as i32));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::*;

    /// In-memory Character DB with the same transition guards as the
    /// production SQL. Fault flags model the crash boundaries: a raw commit
    /// that fails before applying (`fail_*_pre_commit`), and a raw commit
    /// that applies but loses the reply (`lose_*_reply`), which must drive
    /// the shared reconcile path.
    struct FakeBattlePetPurchaseStoreLikeCpp {
        inner: Mutex<FakeStoreInnerLikeCpp>,
        fail_next_charge_pre_commit: AtomicBool,
        lose_next_charge_reply: AtomicBool,
        fail_next_compensate_pre_commit: AtomicBool,
        lose_next_compensate_reply: AtomicBool,
        fail_next_mark: AtomicBool,
        charge_attempts: AtomicUsize,
        compensate_attempts: AtomicUsize,
        money_mutations: AtomicUsize,
    }

    #[derive(Default)]
    struct FakeStoreInnerLikeCpp {
        commands: BTreeMap<[u8; 16], BattlePetPurchaseCommandLikeCpp>,
        money: BTreeMap<u64, u64>,
    }

    impl FakeBattlePetPurchaseStoreLikeCpp {
        fn new() -> Self {
            Self {
                inner: Mutex::new(FakeStoreInnerLikeCpp::default()),
                fail_next_charge_pre_commit: AtomicBool::new(false),
                lose_next_charge_reply: AtomicBool::new(false),
                fail_next_compensate_pre_commit: AtomicBool::new(false),
                lose_next_compensate_reply: AtomicBool::new(false),
                fail_next_mark: AtomicBool::new(false),
                charge_attempts: AtomicUsize::new(0),
                compensate_attempts: AtomicUsize::new(0),
                money_mutations: AtomicUsize::new(0),
            }
        }

        fn with_money(self, guid: u64, money: u64) -> Self {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .insert(guid, money);
            self
        }

        fn money(&self, guid: u64) -> Option<u64> {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .get(&guid)
                .copied()
        }

        fn command(&self, request_key: [u8; 16]) -> Option<BattlePetPurchaseCommandLikeCpp> {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .commands
                .get(&request_key)
                .cloned()
        }

        fn seed_command(&self, command: BattlePetPurchaseCommandLikeCpp) {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .commands
                .insert(command.request_key, command);
        }

        fn money_mutations(&self) -> usize {
            self.money_mutations.load(Ordering::SeqCst)
        }
    }

    fn test_command(request_key: [u8; 16], guid: u64) -> BattlePetPurchaseCommandLikeCpp {
        BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: guid,
            account_id: 7,
            trainer_id: 11,
            spell_id: 12345,
            species: 42,
            breed: 3,
            quality: 0,
            display_id: 999,
            level: 1,
            price: 250,
            money_before: 1_000,
            money_after: 750,
            status: BattlePetPurchaseStatusLikeCpp::PendingApplication,
            failure_reason: None,
        }
    }

    impl BattlePetPurchaseStoreLikeCpp for FakeBattlePetPurchaseStoreLikeCpp {
        fn charge_and_insert_command<'a>(
            &'a self,
            command: BattlePetPurchaseCommandLikeCpp,
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                self.charge_attempts.fetch_add(1, Ordering::SeqCst);
                if self
                    .fail_next_charge_pre_commit
                    .swap(false, Ordering::SeqCst)
                {
                    return Ok(BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
                }
                let applied = {
                    let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                    let guard_ok = inner.money.get(&command.character_guid).copied()
                        == Some(command.money_before);
                    let key_free = !inner.commands.contains_key(&command.request_key);
                    if guard_ok && key_free {
                        inner
                            .money
                            .insert(command.character_guid, command.money_after);
                        inner.commands.insert(command.request_key, command.clone());
                        self.money_mutations.fetch_add(1, Ordering::SeqCst);
                        true
                    } else {
                        false
                    }
                };
                if applied && self.lose_next_charge_reply.swap(false, Ordering::SeqCst) {
                    // The commit happened; the reply was lost. The shared
                    // reconcile must attribute it by the durable row.
                    let row = self.command(command.request_key);
                    return Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                        row.as_ref(),
                        command.request_key,
                    ));
                }
                if applied {
                    Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged)
                } else {
                    // Guard or unique-key failure: the raw transaction rolls
                    // back, then reconcile-by-row decides (a same-token retry
                    // finds its own earlier commit and must not charge again).
                    let row = self.command(command.request_key);
                    Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                        row.as_ref(),
                        command.request_key,
                    ))
                }
            })
        }

        fn load_command<'a>(
            &'a self,
            request_key: [u8; 16],
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<Option<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move { Ok(self.command(request_key)) })
        }

        fn load_pending_commands<'a>(
            &'a self,
            character_guid: u64,
            limit: u32,
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                let inner = self.inner.lock().expect("fake purchase store poisoned");
                Ok(inner
                    .commands
                    .values()
                    .filter(|command| {
                        command.character_guid == character_guid
                            && matches!(
                                command.status,
                                BattlePetPurchaseStatusLikeCpp::PendingApplication
                                    | BattlePetPurchaseStatusLikeCpp::CompensationPending
                            )
                            && !command.status.is_terminal_like_cpp()
                    })
                    .take(limit as usize)
                    .cloned()
                    .collect())
            })
        }

        fn mark_completed<'a>(
            &'a self,
            request_key: [u8; 16],
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                if self.fail_next_mark.swap(false, Ordering::SeqCst) {
                    let row = self.command(request_key);
                    return CharacterBattlePetPurchaseStoreLikeCpp::reconcile_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::Completed,
                    );
                }
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                let Some(row) = inner.commands.get_mut(&request_key) else {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    ));
                };
                Ok(match row.status {
                    BattlePetPurchaseStatusLikeCpp::PendingApplication
                    | BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                        row.status = BattlePetPurchaseStatusLikeCpp::Completed;
                        row.failure_reason = None;
                        BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                    }
                    BattlePetPurchaseStatusLikeCpp::Completed => {
                        BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                    }
                    BattlePetPurchaseStatusLikeCpp::Compensated
                    | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
                        BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
                    }
                })
            })
        }

        fn mark_compensation_pending<'a>(
            &'a self,
            request_key: [u8; 16],
            reason: &'static str,
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                if self.fail_next_mark.swap(false, Ordering::SeqCst) {
                    let row = self.command(request_key);
                    return CharacterBattlePetPurchaseStoreLikeCpp::reconcile_mark_like_cpp(
                        row.as_ref(),
                        BattlePetPurchaseStatusLikeCpp::PendingApplication,
                        BattlePetPurchaseStatusLikeCpp::CompensationPending,
                    );
                }
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                let Some(row) = inner.commands.get_mut(&request_key) else {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    ));
                };
                Ok(match row.status {
                    BattlePetPurchaseStatusLikeCpp::PendingApplication => {
                        row.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
                        row.failure_reason = Some(reason.to_string());
                        BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                    }
                    BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                        BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                    }
                    BattlePetPurchaseStatusLikeCpp::Completed => {
                        BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted
                    }
                    BattlePetPurchaseStatusLikeCpp::Compensated
                    | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
                        BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
                    }
                })
            })
        }

        fn compensate<'a>(
            &'a self,
            request_key: [u8; 16],
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                self.compensate_attempts.fetch_add(1, Ordering::SeqCst);
                let command = match self.command(request_key) {
                    Some(command) => command,
                    None => {
                        return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "missing command".to_string(),
                        ));
                    }
                };
                match command.status {
                    BattlePetPurchaseStatusLikeCpp::Compensated => {
                        return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated);
                    }
                    BattlePetPurchaseStatusLikeCpp::Completed => {
                        return Ok(
                            BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted,
                        );
                    }
                    BattlePetPurchaseStatusLikeCpp::CompensationPending => {}
                    status => {
                        return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                            "compensation from {status:?}"
                        )));
                    }
                }
                if self
                    .fail_next_compensate_pre_commit
                    .swap(false, Ordering::SeqCst)
                {
                    let character_present = self
                        .inner
                        .lock()
                        .expect("fake purchase store poisoned")
                        .money
                        .contains_key(&command.character_guid);
                    return if character_present {
                        Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                            "injected pre-commit compensation failure".to_string(),
                        ))
                    } else {
                        Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing)
                    };
                }
                let applied = {
                    let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                    match inner.money.get_mut(&command.character_guid) {
                        Some(money) => {
                            *money = (*money + u64::from(command.price))
                                .min(wow_entities::MAX_MONEY_AMOUNT);
                            self.money_mutations.fetch_add(1, Ordering::SeqCst);
                            inner
                                .commands
                                .get_mut(&request_key)
                                .expect("seeded command")
                                .status = BattlePetPurchaseStatusLikeCpp::Compensated;
                            true
                        }
                        None => false,
                    }
                };
                if !applied {
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing);
                }
                if self
                    .lose_next_compensate_reply
                    .swap(false, Ordering::SeqCst)
                {
                    // Reply lost after this call's own refund committed; the
                    // status read is decisive and attributes the refund to
                    // this call.
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated);
                }
                Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated)
            })
        }

        fn mark_terminal_failure<'a>(
            &'a self,
            request_key: [u8; 16],
            reason: &'static str,
        ) -> BattlePetPurchaseFuture<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>> {
            Box::pin(async move {
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                let Some(row) = inner.commands.get_mut(&request_key) else {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    ));
                };
                if row.status == BattlePetPurchaseStatusLikeCpp::CompensationPending {
                    row.status = BattlePetPurchaseStatusLikeCpp::TerminalFailure;
                    row.failure_reason = Some(reason.to_string());
                }
                Ok(())
            })
        }
    }

    #[test]
    fn status_codes_roundtrip_and_terminal_set_is_closed() {
        for (code, status, terminal) in [
            (0, BattlePetPurchaseStatusLikeCpp::PendingApplication, false),
            (1, BattlePetPurchaseStatusLikeCpp::Completed, true),
            (
                2,
                BattlePetPurchaseStatusLikeCpp::CompensationPending,
                false,
            ),
            (3, BattlePetPurchaseStatusLikeCpp::Compensated, true),
            (4, BattlePetPurchaseStatusLikeCpp::TerminalFailure, true),
        ] {
            assert_eq!(status.as_u8_like_cpp(), code);
            assert_eq!(
                BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(code),
                Some(status)
            );
            assert_eq!(status.is_terminal_like_cpp(), terminal);
        }
        assert_eq!(BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(5), None);
    }

    #[test]
    fn durable_saga_schema_is_pinned_to_the_migration_file() {
        let migration = include_str!(
            "../../../sql/updates/characters/wotlk_classic/2026_08_03_00_characters.sql"
        );
        for needle in [
            "CREATE TABLE IF NOT EXISTS `character_battle_pet_purchase`",
            "`request_key` binary(16) NOT NULL",
            "`guid` bigint unsigned NOT NULL",
            "`status` tinyint unsigned NOT NULL",
            "`money_before` bigint unsigned NOT NULL",
            "`money_after` bigint unsigned NOT NULL",
            "PRIMARY KEY (`request_key`)",
            "KEY `idx_guid_status` (`guid`,`status`)",
        ] {
            assert!(
                migration.contains(needle),
                "battle-pet purchase migration must contain {needle}"
            );
        }
    }

    #[tokio::test]
    async fn charge_commits_money_and_command_exactly_once_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        let command = test_command([1; 16], 9);
        let charged = store
            .charge_and_insert_command(command.clone())
            .await
            .expect("charge must succeed");
        assert_eq!(charged, BattlePetPurchaseChargeOutcomeLikeCpp::Charged);
        assert_eq!(store.money(9), Some(750));
        assert_eq!(store.command([1; 16]), Some(command));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn same_token_charge_replay_is_attributed_without_a_second_charge_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        let command = test_command([2; 16], 9);
        assert_eq!(
            store
                .charge_and_insert_command(command.clone())
                .await
                .expect("first charge"),
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        );
        // A raw same-token retry fails its guarded statements but the
        // reconcile must find the earlier commit instead of charging again.
        assert_eq!(
            store
                .charge_and_insert_command(command)
                .await
                .expect("replayed charge"),
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        );
        assert_eq!(store.money(9), Some(750));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn lost_charge_reply_reconciles_to_charged_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        store.lose_next_charge_reply.store(true, Ordering::SeqCst);
        let charged = store
            .charge_and_insert_command(test_command([3; 16], 9))
            .await
            .expect("lost reply must reconcile through the durable row");
        assert_eq!(charged, BattlePetPurchaseChargeOutcomeLikeCpp::Charged);
        assert_eq!(store.money(9), Some(750));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn failed_charge_leaves_no_money_and_no_command_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        store
            .fail_next_charge_pre_commit
            .store(true, Ordering::SeqCst);
        let outcome = store
            .charge_and_insert_command(test_command([4; 16], 9))
            .await
            .expect("pre-commit failure");
        assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.command([4; 16]), None);
        assert_eq!(store.money_mutations(), 0);
    }

    #[tokio::test]
    async fn guarded_charge_fails_closed_when_money_moved_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 2_000);
        let outcome = store
            .charge_and_insert_command(test_command([5; 16], 9))
            .await
            .expect("guarded charge");
        assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
        assert_eq!(store.money(9), Some(2_000));
        assert_eq!(store.command([5; 16]), None);
    }

    #[tokio::test]
    async fn compensation_refunds_exactly_once_across_replays_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
        let mut command = test_command([6; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        store.seed_command(command);
        assert_eq!(
            store.compensate([6; 16]).await.expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
        assert_eq!(
            store
                .compensate([6; 16])
                .await
                .expect("replayed compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn lost_compensation_reply_still_refunds_exactly_once_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
        let mut command = test_command([7; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        store.seed_command(command);
        store
            .lose_next_compensate_reply
            .store(true, Ordering::SeqCst);
        // The lost reply belongs to this call's own committed refund.
        assert_eq!(
            store.compensate([7; 16]).await.expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
        // A later replay sees the durable flip and does not refund again.
        assert_eq!(
            store
                .compensate([7; 16])
                .await
                .expect("replayed compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn compensation_never_refunds_a_completed_command_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
        let mut command = test_command([8; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::Completed;
        store.seed_command(command);
        assert_eq!(
            store.compensate([8; 16]).await.expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted
        );
        assert_eq!(store.money(9), Some(750));
        assert_eq!(store.money_mutations(), 0);
    }

    #[tokio::test]
    async fn missing_character_compensation_is_terminal_not_retried_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new();
        let mut command = test_command([9; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        store.seed_command(command);
        assert_eq!(
            store.compensate([9; 16]).await.expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing
        );
    }

    #[tokio::test]
    async fn pending_scan_returns_only_unconverged_commands_bounded_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new();
        for (index, status) in [
            BattlePetPurchaseStatusLikeCpp::PendingApplication,
            BattlePetPurchaseStatusLikeCpp::CompensationPending,
            BattlePetPurchaseStatusLikeCpp::Completed,
            BattlePetPurchaseStatusLikeCpp::Compensated,
            BattlePetPurchaseStatusLikeCpp::TerminalFailure,
        ]
        .into_iter()
        .enumerate()
        {
            let mut command = test_command([index as u8 + 10; 16], 9);
            command.status = status;
            store.seed_command(command);
        }
        let pending = store
            .load_pending_commands(9, BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP)
            .await
            .expect("scan");
        assert_eq!(pending.len(), 2);
        assert!(
            pending
                .iter()
                .all(|command| !command.status.is_terminal_like_cpp())
        );
        let bounded = store.load_pending_commands(9, 1).await.expect("scan");
        assert_eq!(bounded.len(), 1);
    }
}
