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
    BattlePetAddRequestKeyLikeCpp, BattlePetAddRequestLikeCpp, BattlePetLeaseIdLikeCpp,
};
use crate::session::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp, WorldSession,
};
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
    /// Whether success delivery was recorded after the durable pet existed
    /// and packets were queued. Recovery re-sends while it is clear.
    pub published: bool,
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
    Compensated {
        durable_money: u64,
    },
    /// Replay: the row was already `Compensated`; the refund cannot run twice.
    AlreadyCompensated {
        durable_money: u64,
    },
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
    expected: &BattlePetPurchaseCommandLikeCpp,
) -> BattlePetPurchaseChargeOutcomeLikeCpp {
    match row {
        // A random request-key collision is not a replay of this command.
        // Compare every immutable input before treating a durable row as
        // proof that this transaction's guarded charge committed.
        Some(row)
            if row.request_key == expected.request_key
                && row.character_guid == expected.character_guid
                && row.account_id == expected.account_id
                && row.trainer_id == expected.trainer_id
                && row.spell_id == expected.spell_id
                && row.species == expected.species
                && row.breed == expected.breed
                && row.quality == expected.quality
                && row.display_id == expected.display_id
                && row.level == expected.level
                && row.price == expected.price
                && row.money_before == expected.money_before
                && row.money_after == expected.money_after =>
        {
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
        cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
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

    /// Record success delivery (`published` 0 → 1) after packets are queued.
    /// A clear marker means recovery must re-send because delivery cannot be
    /// proven.
    fn mark_published<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    >;

    /// T3: flip `PendingApplication`/`CompensationPending` → `Completed`.
    /// The wider source guard also closes a recorded compensation decision
    /// once the pet is known to be durable (receipt re-check). The
    /// completion does not imply that the delivery marker committed.
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
        cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
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
        macro_rules! required {
            ($index:expr, $ty:ty, $name:literal) => {
                result.try_read::<$ty>($index).ok_or_else(|| {
                    BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        concat!("battle-pet purchase row cannot decode ", $name).to_string(),
                    )
                })?
            };
        }

        let key_bytes: Vec<u8> = required!(0, Vec<u8>, "request_key");
        let request_key: [u8; 16] = key_bytes.try_into().map_err(|_| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase request_key is not 16 bytes".to_string(),
            )
        })?;
        let status_raw: u8 = required!(13, u8, "status");
        let status =
            BattlePetPurchaseStatusLikeCpp::from_u8_like_cpp(status_raw).ok_or_else(|| {
                BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                    "battle-pet purchase row has unknown status {status_raw}"
                ))
            })?;
        Ok(BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: required!(1, u64, "guid"),
            account_id: required!(2, u32, "account_id"),
            trainer_id: required!(3, u32, "trainer_id"),
            spell_id: required!(4, u32, "spell_id"),
            species: required!(5, u32, "species"),
            breed: required!(6, u16, "breed"),
            quality: required!(7, u8, "quality"),
            display_id: required!(8, u32, "display_id"),
            level: required!(9, u16, "level"),
            price: required!(10, u32, "price"),
            money_before: required!(11, u64, "money_before"),
            money_after: required!(12, u64, "money_after"),
            status,
            published: required!(15, u8, "published") != 0,
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

    async fn load_character_money_impl(
        &self,
        character_guid: u64,
    ) -> Result<Option<u64>, BattlePetPurchaseStoreErrorLikeCpp> {
        let mut statement = self.character_db.prepare(CharStatements::cpp(
            "CHAR_SEL_CHARACTER_MONEY",
            "SELECT money FROM characters WHERE guid = ?",
        ));
        statement.set_u64(0, character_guid);
        let result = self.character_db.query(&statement).await.map_err(|error| {
            BattlePetPurchaseStoreErrorLikeCpp::Retryable(format!(
                "battle-pet purchase character money read failed: {error}"
            ))
        })?;
        if result.is_empty() {
            return Ok(None);
        }
        result.try_read::<u64>(0).map(Some).ok_or_else(|| {
            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                "battle-pet purchase character row cannot decode money".to_string(),
            )
        })
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
        mut cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            // C++ `HasEnoughMoney(0)` always passes and `ModifyMoney(-0)` is
            // a no-op; MySQL reports zero changed rows for a no-op UPDATE,
            // so the guarded money statement only exists for a nonzero
            // price, with the command row as the zero-price commit marker.
            let money = (command.money_after != command.money_before).then(|| {
                let mut money = self
                    .character_db
                    .prepare(CharStatements::UPD_CHARACTER_MONEY_GUARDED);
                money.set_u64(0, command.money_after);
                money.set_u64(1, command.character_guid);
                money.set_u64(2, command.money_before);
                money
            });
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
            if let Some(money) = money {
                transaction.append_expect_rows_affected(money, 1);
            }
            transaction.append_expect_rows_affected(insert, 1);
            cancellation_fence.arm_like_cpp();
            let outcome = match transaction
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
                            &command,
                        )),
                        Err(_) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                            "battle-pet purchase charge COMMIT outcome is unknown and the command row cannot be read: {error}"
                        ))),
                    }
                }
            };
            cancellation_fence.disarm_like_cpp();
            outcome
        })
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

    fn mark_published<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let mut statement = self
                .character_db
                .prepare(CharStatements::UPD_BATTLE_PET_PURCHASE_PUBLISHED);
            statement.set_bytes(0, request_key.to_vec());
            let mut transaction = SqlTransaction::new();
            transaction.append_expect_rows_affected(statement, 1);
            match transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await
            {
                Ok(()) => Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied),
                Err(_) => match self.load_command_impl(request_key).await {
                    Ok(Some(row)) if row.published => {
                        Ok(BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
                    }
                    Ok(Some(row))
                        if row.status.is_terminal_like_cpp()
                            && row.status != BattlePetPurchaseStatusLikeCpp::Completed =>
                    {
                        Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                            "battle-pet purchase publication mark on terminal {:?}",
                            row.status
                        )))
                    }
                    Ok(Some(_)) => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                        "battle-pet purchase publication mark did not commit".to_string(),
                    )),
                    Ok(None) => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "battle-pet purchase command disappeared during publication mark"
                            .to_string(),
                    )),
                    Err(error) => Err(error),
                },
            }
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
        mut cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
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
                    let durable_money = self
                        .load_character_money_impl(command.character_guid)
                        .await?
                        .ok_or_else(|| {
                            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                "compensated battle-pet purchase has no character row".to_string(),
                            )
                        })?;
                    return Ok(
                        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                            durable_money,
                        },
                    );
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
            // A zero-price command has no money to refund; the status flip
            // alone is the exactly-once marker and a no-op refund UPDATE
            // would report zero rows and roll the transaction back.
            let refund = (command.price != 0).then(|| {
                let mut refund = self
                    .character_db
                    .prepare(CharStatements::UPD_CHARACTER_MONEY_REFUND);
                refund.set_u32(0, command.price);
                refund.set_u64(1, wow_entities::MAX_MONEY_AMOUNT);
                refund.set_u64(2, command.character_guid);
                refund
            });
            let mut flip = self
                .character_db
                .prepare(CharStatements::UPD_BATTLE_PET_PURCHASE_COMPENSATED);
            flip.set_bytes(0, request_key.to_vec());
            let mut transaction = SqlTransaction::new();
            if let Some(refund) = refund {
                transaction.append_expect_rows_affected(refund, 1);
            }
            transaction.append_expect_rows_affected(flip, 1);
            cancellation_fence.arm_like_cpp();
            let commit_result = transaction
                .commit_with_outcome_like_cpp(self.character_db.pool())
                .await;
            let outcome = match commit_result {
                Ok(()) => match self.load_character_money_impl(command.character_guid).await {
                    Ok(Some(durable_money)) => {
                        Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                            durable_money,
                        })
                    }
                    Ok(None) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                        "battle-pet purchase refund committed but character row disappeared"
                            .to_string(),
                    )),
                    Err(error) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                        "battle-pet purchase refund committed but durable money cannot be read: {error:?}"
                    ))),
                },
                Err(error) => {
                    // Never attribute an ambiguous COMMIT to this driver from
                    // status alone: another driver may have compensated after
                    // our connection failed. Reconcile to the durable absolute
                    // money value, which is safe for either attribution.
                    match self.load_command_impl(request_key).await {
                        Ok(Some(row)) => {
                            match row.status {
                                BattlePetPurchaseStatusLikeCpp::Compensated => match self
                                    .load_character_money_impl(command.character_guid)
                                    .await
                                {
                                    Ok(Some(durable_money)) => Ok(
                                        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                                            durable_money,
                                        },
                                    ),
                                    Ok(None) => Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                                        "compensated battle-pet purchase has no character row"
                                            .to_string(),
                                    )),
                                    Err(read_error) => {
                                        Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                                            format!(
                                                "battle-pet purchase is durably compensated but durable money cannot be read: {read_error:?}"
                                            ),
                                        ))
                                    }
                                },
                                BattlePetPurchaseStatusLikeCpp::Completed => Ok(
                                    BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted,
                                ),
                                BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                                    // Still owed: either the transaction provably
                                    // rolled back or its reply was lost pre-commit.
                                    // A missing character row is the one cause that
                                    // can never converge and becomes TerminalFailure.
                                    match self
                                        .load_character_money_impl(command.character_guid)
                                        .await
                                    {
                                        Ok(None) => Ok(
                                            BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing,
                                        ),
                                        Ok(Some(_)) => {
                                            Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                                                "battle-pet purchase compensation did not commit"
                                                    .to_string(),
                                            ))
                                        }
                                        Err(read_error) => Err(
                                            if error.is_commit_outcome_unknown_like_cpp() {
                                                BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                                                    format!(
                                                        "battle-pet purchase refund COMMIT is unknown and cannot be reconciled: {read_error:?}"
                                                    ),
                                                )
                                            } else {
                                                read_error
                                            },
                                        ),
                                    }
                                }
                                status => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                    format!(
                                        "battle-pet purchase compensation observed unexpected {status:?}"
                                    ),
                                )),
                            }
                        }
                        Ok(None) => Err(if error.is_commit_outcome_unknown_like_cpp() {
                            BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                                "battle-pet purchase refund COMMIT is unknown and command disappeared"
                                    .to_string(),
                            )
                        } else {
                            BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                "battle-pet purchase command disappeared during compensation"
                                    .to_string(),
                            )
                        }),
                        Err(read_error) => Err(if error.is_commit_outcome_unknown_like_cpp() {
                            BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(format!(
                                "battle-pet purchase refund COMMIT is unknown and command cannot be read: {read_error:?}"
                            ))
                        } else {
                            read_error
                        }),
                    }
                }
            };
            // Once a refund is known to have committed, failure to obtain the
            // absolute durable balance must keep the fence armed. Dropping an
            // armed fence marks money persistence indeterminate so a stale
            // runtime balance cannot overwrite the refund during autosave.
            if !matches!(
                outcome,
                Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(_))
            ) {
                cancellation_fence.disarm_like_cpp();
            }
            outcome
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
    /// pet once through the #160 account owner, publishes, records delivery,
    /// then completes in
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
        // The receipt/owner authority is the Battle.net account (#160), not
        // the game account; persist that identity for recovery binding.
        let command = BattlePetPurchaseCommandLikeCpp {
            request_key,
            character_guid: player_guid.counter() as u64,
            account_id: self.battlenet_account_id(),
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
            published: false,
            failure_reason: None,
        };
        let money_tracker = self.durable_loot_money_persistence_tracker_like_cpp();
        let charge = retry_battle_pet_purchase_step_like_cpp(
            || {
                store.charge_and_insert_command(
                    command.clone(),
                    PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::clone(
                        &money_tracker,
                    )),
                )
            },
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
                let pet = match outcome {
                    BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                    BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                };
                let pet_guid = pet.guid;
                // The durable pet exists before any success packet. Emit first
                // and record delivery second so a crash can cause an idempotent
                // recovery re-send but can never lose the only notification.
                // Exactly-once transport is impossible without a client ACK;
                // pet creation, charge and compensation remain exactly-once.
                let enqueued =
                    self.publish_battle_pet_trainer_purchase_like_cpp(pet, offer.source_spell_id);
                let published = if enqueued {
                    self.record_battle_pet_purchase_publication_like_cpp(&store, request_key)
                        .await
                } else {
                    false
                };
                if !self
                    .complete_battle_pet_purchase_like_cpp(&store, request_key)
                    .await
                {
                    // The pet is durable; recovery completes the command and
                    // emits only if the claim never committed.
                    return BattlePetPurchaseExecutionLikeCpp::RetryableDeferred;
                }
                BattlePetPurchaseExecutionLikeCpp::Purchased {
                    pet_guid,
                    published,
                }
            }
            Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
                self.compensate_battle_pet_purchase_like_cpp(
                    &owner,
                    lease_id,
                    player_guid,
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
            // A character transferred to another Battle.net account mid-
            // purchase must never apply or publish into the new account:
            // the pet/receipt authority is the original account, so the
            // command is compensated (money travels with the character) or
            // its marker is closed without emitting.
            let account_mismatch = command.account_id != self.battlenet_account_id();
            if account_mismatch && command.status == BattlePetPurchaseStatusLikeCpp::Completed {
                if !command.published {
                    warn!(
                        account = self.account_id,
                        command_account = command.account_id,
                        "Closing the publication marker of a battle-pet purchase owned by another account"
                    );
                    self.record_battle_pet_purchase_publication_like_cpp(
                        &store,
                        command.request_key,
                    )
                    .await;
                    summary.applied += 1;
                }
                continue;
            }
            if account_mismatch
                && command.status == BattlePetPurchaseStatusLikeCpp::PendingApplication
            {
                let decision = retry_battle_pet_purchase_step_like_cpp(
                    || {
                        store.mark_compensation_pending(
                            command.request_key,
                            "battle-pet purchase account changed before application",
                        )
                    },
                    store_error_is_retryable_like_cpp,
                )
                .await;
                if matches!(
                    decision,
                    Ok(BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                        | BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
                ) {
                    match self
                        .compensate_battle_pet_purchase_like_cpp(
                            &owner,
                            lease_id,
                            player_guid,
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
                        BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                            // The original account already owns the durable
                            // pet: completed, no refund.
                            summary.applied += 1;
                        }
                        _ => {
                            summary.deferred += 1;
                            break;
                        }
                    }
                } else {
                    summary.deferred += 1;
                    break;
                }
                continue;
            }
            match command.status {
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                    match self
                        .compensate_battle_pet_purchase_like_cpp(
                            &owner,
                            lease_id,
                            player_guid,
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
                        BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                            // The receipt re-check proved the pet durable:
                            // the command completed instead of refunding.
                            summary.applied += 1;
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
                            let pet = match outcome {
                                BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                                BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                            };
                            if !command.published {
                                let enqueued = self.publish_battle_pet_trainer_purchase_like_cpp(
                                    pet,
                                    command.spell_id,
                                );
                                if enqueued {
                                    self.record_battle_pet_purchase_publication_like_cpp(
                                        &store,
                                        command.request_key,
                                    )
                                    .await;
                                }
                            }
                            if !self
                                .complete_battle_pet_purchase_like_cpp(&store, command.request_key)
                                .await
                            {
                                summary.deferred += 1;
                                continue;
                            }
                            summary.applied += 1;
                        }
                        Err(error) if battle_pet_add_failure_is_terminal_like_cpp(&error) => {
                            match self
                                .compensate_battle_pet_purchase_like_cpp(
                                    &owner,
                                    lease_id,
                                    player_guid,
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
                                BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere => {
                                    // The receipt re-check proved the pet
                                    // durable: completed instead of refunding.
                                    summary.applied += 1;
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
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    // Completed but never recorded as published: replay the
                    // receipt and enqueue its packets before recording the
                    // marker. The recovery scan selects these rows deliberately;
                    // a prior enqueue followed by a crash may be re-sent.
                    if command.published {
                        continue;
                    }
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
                    match owner.try_add_pet_like_cpp(lease_id, request).await {
                        Ok(outcome) => {
                            let pet = match outcome {
                                BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                                BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                            };
                            let enqueued = self.publish_battle_pet_trainer_purchase_like_cpp(
                                pet,
                                command.spell_id,
                            );
                            if enqueued {
                                self.record_battle_pet_purchase_publication_like_cpp(
                                    &store,
                                    command.request_key,
                                )
                                .await;
                            }
                            summary.applied += 1;
                        }
                        Err(BattlePetAddFailureLikeCpp::DuplicateRequest) => {
                            // The receipt exists but its pet is gone (deleted
                            // by another session before recovery): the charge
                            // stands and nothing can be published, so close
                            // the marker instead of blocking the batch behind
                            // an unresolvable row on every login.
                            self.record_battle_pet_purchase_publication_like_cpp(
                                &store,
                                command.request_key,
                            )
                            .await;
                            summary.applied += 1;
                        }
                        Err(error) => {
                            warn!(
                                account = self.account_id,
                                ?error,
                                "Battle-pet purchase recovery could not resolve the publication packet"
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

    /// Record a success publication after the packets were queued. A failed
    /// marker leaves the command selected for a recovery re-send. Enqueue
    /// attempts may therefore repeat and avoid consuming the only recovery
    /// signal before emission; actual delivery remains best-effort without a
    /// client acknowledgement.
    async fn record_battle_pet_purchase_publication_like_cpp(
        &mut self,
        store: &Arc<dyn BattlePetPurchaseStoreLikeCpp>,
        request_key: [u8; 16],
    ) -> bool {
        match retry_battle_pet_purchase_step_like_cpp(
            || store.mark_published(request_key),
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
                    "Battle-pet purchase publication marker observed a conflicting terminal state"
                );
                false
            }
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "Battle-pet purchase publication marker did not commit"
                );
                false
            }
        }
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
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: ObjectGuid,
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

        // Receipt re-check: a durable pet forbids the refund; the command
        // completes instead. An account-mismatched command is probed under
        // its ORIGINAL account, the only receipt authority: a durable pet
        // there still forbids the refund (the pet travels nowhere, the
        // charge stays), but nothing is ever published into the new
        // account. When this account owns the receipt and the pet resolves
        // in-process, publish first if no publication was ever recorded;
        // when the owner cannot resolve the packet (its in-memory journal
        // lost the pet with a failed insert reply), completion proceeds
        // without the marker and login recovery finishes the publication.
        let account_mismatch = command.account_id != self.battlenet_account_id();
        let receipt_probe = if account_mismatch {
            // The unfenced snapshot could be falsified by a still-flying
            // detached insert from the original account; probe through the
            // original account's process fence instead and defer while its
            // authority is held elsewhere.
            match owner
                .receipt_probe_for_account_fenced_like_cpp(
                    command.account_id,
                    BattlePetAddRequestKeyLikeCpp::from_bytes(command.request_key),
                )
                .await
            {
                Ok(crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::Committed) => {
                    Ok(true)
                }
                Ok(crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::Absent) => {
                    Ok(false)
                }
                Ok(
                    crate::battle_pet_account::BattlePetFencedReceiptProbeLikeCpp::AuthorityUnavailable,
                ) => Err(BattlePetAddFailureLikeCpp::MissingAuthority),
                Err(error) => Err(error),
            }
        } else {
            owner
                .add_request_committed_like_cpp(BattlePetAddRequestKeyLikeCpp::from_bytes(
                    command.request_key,
                ))
                .await
        };
        match receipt_probe {
            Ok(true) => {
                if !command.published && account_mismatch {
                    // The pet is durable in the original account's journal;
                    // just close the marker so recovery stops selecting the
                    // row, without emitting anything here.
                    self.record_battle_pet_purchase_publication_like_cpp(
                        store,
                        command.request_key,
                    )
                    .await;
                }
                if !command.published
                    && !account_mismatch
                    && self.battle_pet_try_acquire_journal_lease_like_cpp().await
                {
                    let replay = owner
                        .try_add_pet_like_cpp(
                            lease_id,
                            BattlePetAddRequestLikeCpp {
                                request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(
                                    command.request_key,
                                ),
                                species: command.species,
                                display_id: command.display_id,
                                breed: command.breed,
                                quality: command.quality,
                                level: command.level,
                                owner_guid: Some(player_guid),
                            },
                        )
                        .await;
                    if let Ok(outcome) = replay {
                        let pet = match outcome {
                            BattlePetAddOutcomeLikeCpp::Added(pet) => pet,
                            BattlePetAddOutcomeLikeCpp::Replayed(pet) => pet,
                        };
                        let enqueued = self
                            .publish_battle_pet_trainer_purchase_like_cpp(pet, command.spell_id);
                        if enqueued {
                            self.record_battle_pet_purchase_publication_like_cpp(
                                store,
                                command.request_key,
                            )
                            .await;
                        }
                    }
                }
                let _ = self
                    .complete_battle_pet_purchase_like_cpp(store, command.request_key)
                    .await;
                return BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere;
            }
            Ok(false) => {}
            Err(error) => {
                // Absence cannot be proven; refunding blind is forbidden.
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
        let money_tracker = self.durable_loot_money_persistence_tracker_like_cpp();
        let compensated = retry_battle_pet_purchase_step_like_cpp(
            || {
                store.compensate(
                    command.request_key,
                    PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::clone(
                        &money_tracker,
                    )),
                )
            },
            store_error_is_retryable_like_cpp,
        )
        .await;
        match compensated {
            Ok(
                BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated { durable_money }
                | BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated { durable_money },
            ) => {
                // Reconcile to the absolute durable value. Attribution based
                // on a lost COMMIT reply is racy when another driver can finish
                // compensation before the status re-read.
                let current = self.player_gold_like_cpp();
                let restored = durable_money.min(wow_entities::MAX_MONEY_AMOUNT);
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
            Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(error)) => {
                warn!(
                    account = self.account_id,
                    error,
                    "Battle-pet purchase refund COMMIT outcome is unknown; quarantined the session"
                );
                drop(refund_guard);
                self.quarantine_player_money_persistence_like_cpp(
                    "battle-pet purchase refund COMMIT outcome is unknown; relog required",
                );
                BattlePetPurchaseExecutionLikeCpp::CompensationDeferred
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
    ) -> bool {
        let species = pet.species;
        let journal_enqueued = self.publish_battle_pet_trainer_purchase_add_like_cpp(pet.clone());
        // C++ `BattlePetMgr::AddPet`: SendUpdates first, then the two set-like
        // criteria hooks. Their represented bridge derives current durable
        // state, making receipt and packet-publication recovery idempotent.
        self.record_battle_pet_trainer_purchase_criteria_like_cpp(species);
        self.learn_dependent_known_spell_like_cpp(spell_id as i32);
        let learned_enqueued = self
            .send_tx()
            .send(wow_packet::ServerPacket::to_bytes(&LearnedSpells::single(
                spell_id as i32,
            )))
            .is_ok();
        if !learned_enqueued {
            warn!("Send channel closed for account {}", self.account_id);
        }
        // Where this lands relative to the commit is the crash window, so the
        // trace has to see it. Without the hook, moving the publication before
        // the commit -- or dropping it -- produced an identical trace.
        wow_database::persistence_trace::record_publication("battle_pet_trainer_purchase.client");
        journal_enqueued && learned_enqueued
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use tokio::sync::Notify;

    use super::*;

    /// In-memory Character DB with the same transition guards as the
    /// production SQL. Fault flags model the crash boundaries: a raw commit
    /// that fails before applying (`fail_*_pre_commit`), and a raw commit
    /// that applies but loses the reply (`lose_*_reply`), which must drive
    /// the shared reconcile path. Blocking gates (`block_*`) pause a step
    /// mid-flight so cancellation tests can abort the saga future exactly
    /// before or after the durable apply.
    pub(crate) struct FakeBattlePetPurchaseStoreLikeCpp {
        inner: Mutex<FakeStoreInnerLikeCpp>,
        pub(crate) fail_next_charge_pre_commit: AtomicBool,
        pub(crate) lose_next_charge_reply: AtomicBool,
        pub(crate) fail_next_compensate_pre_commit: AtomicBool,
        pub(crate) lose_next_compensate_reply: AtomicBool,
        pub(crate) fail_next_compensate_post_apply_read: AtomicBool,
        pub(crate) fail_next_mark: AtomicBool,
        pub(crate) fail_marks_remaining: AtomicUsize,
        pub(crate) block_next_charge_pre_apply: AtomicBool,
        pub(crate) block_next_charge_post_apply: AtomicBool,
        pub(crate) block_next_compensate_pre_apply: AtomicBool,
        pub(crate) block_next_compensate_post_apply: AtomicBool,
        pub(crate) gate_started: Notify,
        pub(crate) allow_gate: Notify,
        pub(crate) charge_attempts: AtomicUsize,
        pub(crate) compensate_attempts: AtomicUsize,
        money_mutations: AtomicUsize,
    }

    #[derive(Default)]
    struct FakeStoreInnerLikeCpp {
        commands: BTreeMap<[u8; 16], BattlePetPurchaseCommandLikeCpp>,
        money: BTreeMap<u64, u64>,
    }

    impl FakeBattlePetPurchaseStoreLikeCpp {
        pub(crate) fn new() -> Self {
            Self {
                inner: Mutex::new(FakeStoreInnerLikeCpp::default()),
                fail_next_charge_pre_commit: AtomicBool::new(false),
                lose_next_charge_reply: AtomicBool::new(false),
                fail_next_compensate_pre_commit: AtomicBool::new(false),
                lose_next_compensate_reply: AtomicBool::new(false),
                fail_next_compensate_post_apply_read: AtomicBool::new(false),
                fail_next_mark: AtomicBool::new(false),
                fail_marks_remaining: AtomicUsize::new(0),
                block_next_charge_pre_apply: AtomicBool::new(false),
                block_next_charge_post_apply: AtomicBool::new(false),
                block_next_compensate_pre_apply: AtomicBool::new(false),
                block_next_compensate_post_apply: AtomicBool::new(false),
                gate_started: Notify::new(),
                allow_gate: Notify::new(),
                charge_attempts: AtomicUsize::new(0),
                compensate_attempts: AtomicUsize::new(0),
                money_mutations: AtomicUsize::new(0),
            }
        }

        pub(crate) fn with_money(self, guid: u64, money: u64) -> Self {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .insert(guid, money);
            self
        }

        pub(crate) fn money(&self, guid: u64) -> Option<u64> {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .get(&guid)
                .copied()
        }

        pub(crate) fn command(
            &self,
            request_key: [u8; 16],
        ) -> Option<BattlePetPurchaseCommandLikeCpp> {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .commands
                .get(&request_key)
                .cloned()
        }

        pub(crate) fn seed_command(&self, command: BattlePetPurchaseCommandLikeCpp) {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .commands
                .insert(command.request_key, command);
        }

        pub(crate) fn seed_money_like_cpp(&self, guid: u64, money: u64) {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .insert(guid, money);
        }

        pub(crate) fn remove_money_row_for_test_like_cpp(&self, guid: u64) {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .money
                .remove(&guid);
        }

        pub(crate) fn commands_snapshot(&self) -> Vec<BattlePetPurchaseCommandLikeCpp> {
            self.inner
                .lock()
                .expect("fake purchase store poisoned")
                .commands
                .values()
                .cloned()
                .collect()
        }

        pub(crate) fn money_mutations(&self) -> usize {
            self.money_mutations.load(Ordering::SeqCst)
        }

        fn fail_mark_now_like_cpp(&self) -> bool {
            if self.fail_next_mark.swap(false, Ordering::SeqCst) {
                return true;
            }
            let remaining = self.fail_marks_remaining.load(Ordering::SeqCst);
            remaining > 0
                && self
                    .fail_marks_remaining
                    .compare_exchange(remaining, remaining - 1, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
        }
    }

    pub(crate) fn test_command(
        request_key: [u8; 16],
        guid: u64,
    ) -> BattlePetPurchaseCommandLikeCpp {
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
            published: false,
            failure_reason: None,
        }
    }

    pub(crate) fn test_money_commit_fence_like_cpp() -> PlayerMoneyCommitCancellationFenceLikeCpp {
        PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::new(
            wow_network::DurableLootMoneyPersistenceTrackerLikeCpp::default(),
        ))
    }

    impl BattlePetPurchaseStoreLikeCpp for FakeBattlePetPurchaseStoreLikeCpp {
        fn charge_and_insert_command<'a>(
            &'a self,
            command: BattlePetPurchaseCommandLikeCpp,
            mut cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                self.charge_attempts.fetch_add(1, Ordering::SeqCst);
                if self
                    .block_next_charge_pre_apply
                    .swap(false, Ordering::SeqCst)
                {
                    self.gate_started.notify_one();
                    self.allow_gate.notified().await;
                }
                if self
                    .fail_next_charge_pre_commit
                    .swap(false, Ordering::SeqCst)
                {
                    return Ok(BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
                }
                cancellation_fence.arm_like_cpp();
                let applied = {
                    let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                    let guard_ok = inner.money.get(&command.character_guid).copied()
                        == Some(command.money_before);
                    let key_free = !inner.commands.contains_key(&command.request_key);
                    if guard_ok && key_free {
                        if command.money_after != command.money_before {
                            inner
                                .money
                                .insert(command.character_guid, command.money_after);
                            self.money_mutations.fetch_add(1, Ordering::SeqCst);
                        }
                        inner.commands.insert(command.request_key, command.clone());
                        true
                    } else {
                        false
                    }
                };
                if applied
                    && self
                        .block_next_charge_post_apply
                        .swap(false, Ordering::SeqCst)
                {
                    self.gate_started.notify_one();
                    self.allow_gate.notified().await;
                }
                let outcome =
                    if applied && self.lose_next_charge_reply.swap(false, Ordering::SeqCst) {
                        // The commit happened; the reply was lost. The shared
                        // reconcile must attribute it by the durable row.
                        let row = self.command(command.request_key);
                        Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                            row.as_ref(),
                            &command,
                        ))
                    } else if applied {
                        Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged)
                    } else {
                        // Guard or unique-key failure: the raw transaction rolls
                        // back, then reconcile-by-row decides (a same-token retry
                        // finds its own earlier commit and must not charge again).
                        let row = self.command(command.request_key);
                        Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                            row.as_ref(),
                            &command,
                        ))
                    };
                cancellation_fence.disarm_like_cpp();
                outcome
            })
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
                            && (matches!(
                                command.status,
                                BattlePetPurchaseStatusLikeCpp::PendingApplication
                                    | BattlePetPurchaseStatusLikeCpp::CompensationPending
                            ) || (command.status == BattlePetPurchaseStatusLikeCpp::Completed
                                && !command.published))
                    })
                    .take(limit as usize)
                    .cloned()
                    .collect())
            })
        }

        fn mark_published<'a>(
            &'a self,
            request_key: [u8; 16],
        ) -> BattlePetPurchaseFuture<
            'a,
            Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
        > {
            Box::pin(async move {
                if self.fail_mark_now_like_cpp() {
                    let row = self.command(request_key);
                    return match row {
                        Some(row) if row.published => {
                            Ok(BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
                        }
                        Some(row) if row.status.is_terminal_like_cpp() => {
                            Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                "publication mark on terminal command".to_string(),
                            ))
                        }
                        Some(_) => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                            "publication mark did not commit".to_string(),
                        )),
                        None => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "missing command".to_string(),
                        )),
                    };
                }
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                let Some(row) = inner.commands.get_mut(&request_key) else {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    ));
                };
                Ok(match row.status {
                    BattlePetPurchaseStatusLikeCpp::PendingApplication
                    | BattlePetPurchaseStatusLikeCpp::Completed
                    | BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                        if row.published {
                            BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                        } else {
                            row.published = true;
                            BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                        }
                    }
                    _ => BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated,
                })
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
                if self.fail_mark_now_like_cpp() {
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
                if self.fail_mark_now_like_cpp() {
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
            mut cancellation_fence: PlayerMoneyCommitCancellationFenceLikeCpp,
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
                        let durable_money =
                            self.money(command.character_guid).ok_or_else(|| {
                                BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                                    "compensated command has no money row".to_string(),
                                )
                            })?;
                        return Ok(
                            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                                durable_money,
                            },
                        );
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
                    .block_next_compensate_pre_apply
                    .swap(false, Ordering::SeqCst)
                {
                    self.gate_started.notify_one();
                    self.allow_gate.notified().await;
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
                cancellation_fence.arm_like_cpp();
                let applied = {
                    let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                    match inner.money.get_mut(&command.character_guid) {
                        Some(money) => {
                            if command.price != 0 {
                                *money = (*money + u64::from(command.price))
                                    .min(wow_entities::MAX_MONEY_AMOUNT);
                                self.money_mutations.fetch_add(1, Ordering::SeqCst);
                            }
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
                    cancellation_fence.disarm_like_cpp();
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing);
                }
                if self
                    .block_next_compensate_post_apply
                    .swap(false, Ordering::SeqCst)
                {
                    self.gate_started.notify_one();
                    self.allow_gate.notified().await;
                }
                if self
                    .fail_next_compensate_post_apply_read
                    .swap(false, Ordering::SeqCst)
                {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                        "injected durable money read failure after committed refund".to_string(),
                    ));
                }
                if self
                    .lose_next_compensate_reply
                    .swap(false, Ordering::SeqCst)
                {
                    // Reply lost after this call's own refund committed; the
                    // status read is decisive and attributes the refund to
                    // this call.
                    let durable_money = self.money(command.character_guid).expect("money row");
                    cancellation_fence.disarm_like_cpp();
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                        durable_money,
                    });
                }
                let durable_money = self.money(command.character_guid).expect("money row");
                cancellation_fence.disarm_like_cpp();
                Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated { durable_money })
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
            "`published` tinyint unsigned NOT NULL DEFAULT 0",
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
            .charge_and_insert_command(command.clone(), test_money_commit_fence_like_cpp())
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
                .charge_and_insert_command(command.clone(), test_money_commit_fence_like_cpp())
                .await
                .expect("first charge"),
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        );
        // A raw same-token retry fails its guarded statements but the
        // reconcile must find the earlier commit instead of charging again.
        assert_eq!(
            store
                .charge_and_insert_command(command, test_money_commit_fence_like_cpp())
                .await
                .expect("replayed charge"),
            BattlePetPurchaseChargeOutcomeLikeCpp::Charged
        );
        assert_eq!(store.money(9), Some(750));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn colliding_token_with_different_payload_is_not_attributed_as_a_charge_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        let existing = test_command([21; 16], 9);
        store.seed_command(existing);

        let mut colliding = test_command([21; 16], 9);
        colliding.species += 1;
        let outcome = store
            .charge_and_insert_command(colliding, test_money_commit_fence_like_cpp())
            .await
            .expect("a key collision is a definite rollback");

        assert_eq!(outcome, BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 0);
    }

    #[tokio::test]
    async fn lost_charge_reply_reconciles_to_charged_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 1_000);
        store.lose_next_charge_reply.store(true, Ordering::SeqCst);
        let charged = store
            .charge_and_insert_command(test_command([3; 16], 9), test_money_commit_fence_like_cpp())
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
            .charge_and_insert_command(test_command([4; 16], 9), test_money_commit_fence_like_cpp())
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
            .charge_and_insert_command(test_command([5; 16], 9), test_money_commit_fence_like_cpp())
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
            store
                .compensate([6; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                durable_money: 1_000
            }
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
        assert_eq!(
            store
                .compensate([6; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("replayed compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                durable_money: 1_000
            }
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
            store
                .compensate([7; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                durable_money: 1_000
            }
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
        // A later replay sees the durable flip and does not refund again.
        assert_eq!(
            store
                .compensate([7; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("replayed compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                durable_money: 1_000
            }
        );
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn committed_refund_with_unreadable_money_keeps_persistence_quarantined_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
        let mut command = test_command([10; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        store.seed_command(command);
        store
            .fail_next_compensate_post_apply_read
            .store(true, Ordering::SeqCst);
        let money_tracker =
            Arc::new(wow_network::DurableLootMoneyPersistenceTrackerLikeCpp::default());
        let outcome = store
            .compensate(
                [10; 16],
                PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::clone(
                    &money_tracker,
                )),
            )
            .await;
        assert!(matches!(
            outcome,
            Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(_))
        ));
        assert_eq!(store.money(9), Some(1_000));
        assert_eq!(store.money_mutations(), 1);
        assert_eq!(
            store.command([10; 16]).expect("command").status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert!(money_tracker.is_indeterminate_like_cpp());
    }

    #[tokio::test]
    async fn compensation_never_refunds_a_completed_command_like_cpp() {
        let store = FakeBattlePetPurchaseStoreLikeCpp::new().with_money(9, 750);
        let mut command = test_command([8; 16], 9);
        command.status = BattlePetPurchaseStatusLikeCpp::Completed;
        store.seed_command(command);
        assert_eq!(
            store
                .compensate([8; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("compensation"),
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
            store
                .compensate([9; 16], test_money_commit_fence_like_cpp())
                .await
                .expect("compensation"),
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
            // A Completed row is converged only once its publication was
            // recorded; this one is fully converged.
            command.published = true;
            store.seed_command(command);
        }
        // A Completed row that was never published is still owed its
        // publication and must be scanned.
        let mut owed = test_command([99; 16], 9);
        owed.status = BattlePetPurchaseStatusLikeCpp::Completed;
        owed.published = false;
        store.seed_command(owed);
        let pending = store
            .load_pending_commands(9, BATTLE_PET_PURCHASE_RECOVERY_BATCH_LIMIT_LIKE_CPP)
            .await
            .expect("scan");
        assert_eq!(pending.len(), 3);
        assert!(
            pending
                .iter()
                .all(|command| !command.status.is_terminal_like_cpp()
                    || command.status == BattlePetPurchaseStatusLikeCpp::Completed)
        );
        let bounded = store.load_pending_commands(9, 1).await.expect("scan");
        assert_eq!(bounded.len(), 1);
    }
}

/// Executor fixtures: an in-memory Login DB persistence for the #160 owner,
/// saga sessions over flume channels, and the fault-injection/concurrency/
/// cancellation/drain matrix of issue #161.
#[cfg(test)]
mod executor_tests {
    use std::collections::HashMap;
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
        BattlePetAccountRegistryLikeCpp, BattlePetPersistenceErrorLikeCpp,
        BattlePetPersistenceLikeCpp, BattlePetProcessLeaseLikeCpp, DurableBattlePetAddLikeCpp,
        DurableBattlePetAddReceiptLikeCpp, DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp,
        LoadedBattlePetAccountLikeCpp, PersistBattlePetAddOutcomeLikeCpp,
    };
    use crate::session::SessionPlayerController;

    const PLAYER_COUNTER: i64 = 42;
    const OTHER_PLAYER_COUNTER: i64 = 43;
    const ACCOUNT_ID: u32 = 1;
    const TRAINER_ID: u32 = 7;
    const SAGA_SPELL_ID: u32 = 54_330;
    const SAGA_SPECIES: u32 = 11;
    const LEGACY_UNIQUE_SPECIES: u32 = 12;
    const SAGA_PRICE: u32 = 250;
    const SAGA_MONEY: u64 = 1_000;
    const REALM_ID: u16 = 7;
    const VIRTUAL_REALM: u32 = 0x0102_0007;

    type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    #[derive(Default)]
    struct FakeSagaPersistenceStateLikeCpp {
        pets: Vec<DurableBattlePetRowLikeCpp>,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
        receipts: HashMap<BattlePetAddRequestKeyLikeCpp, (u32, DurableBattlePetRowLikeCpp)>,
    }

    /// In-memory Login DB with the #160 persistence contract (receipt
    /// replay, capacity, fence validation) plus the fault gates the saga
    /// matrix needs: pre-commit insert failure, lost insert reply and a
    /// blocking insert for cancellation/drain tests.
    #[derive(Default)]
    struct FakeSagaPersistenceLikeCpp {
        state: StdMutex<FakeSagaPersistenceStateLikeCpp>,
        process_lease: Arc<AtomicBool>,
        current_fence: Arc<AtomicU64>,
        next_guid: AtomicU64,
        insert_calls: AtomicUsize,
        fail_next_insert: AtomicBool,
        lose_next_insert_reply: AtomicBool,
        reconcile_next_insert_after_commit: AtomicBool,
        block_next_insert: AtomicBool,
        insert_started: Notify,
        allow_insert: Notify,
    }

    struct FakeSagaLeaseGuardLikeCpp {
        held: Arc<AtomicBool>,
        fence: u64,
    }

    impl BattlePetProcessLeaseLikeCpp for FakeSagaLeaseGuardLikeCpp {
        fn is_valid_like_cpp(&self) -> bool {
            self.held.load(Ordering::Acquire)
        }

        fn fence_like_cpp(&self) -> u64 {
            self.fence
        }
    }

    impl Drop for FakeSagaLeaseGuardLikeCpp {
        fn drop(&mut self) {
            self.held.store(false, Ordering::Release);
        }
    }

    impl FakeSagaPersistenceLikeCpp {
        fn with_seeded_pets(pets: Vec<DurableBattlePetRowLikeCpp>) -> Self {
            let persistence = Self::default();
            persistence.next_guid.store(
                pets.iter().map(|pet| pet.guid_counter).max().unwrap_or(0) + 10,
                Ordering::Release,
            );
            persistence
                .state
                .lock()
                .expect("fake saga persistence poisoned")
                .pets = pets;
            persistence
        }

        fn pet_count(&self) -> usize {
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .pets
                .len()
        }

        fn species_count(&self, species: u32) -> usize {
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .pets
                .iter()
                .filter(|pet| pet.species == species)
                .count()
        }

        fn receipt_count(&self) -> usize {
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .receipts
                .len()
        }

        fn receipt(&self, request_key: [u8; 16]) -> Option<DurableBattlePetRowLikeCpp> {
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .receipts
                .get(&BattlePetAddRequestKeyLikeCpp::from_bytes(request_key))
                .map(|(_, pet)| pet.clone())
        }

        /// Simulate another process winning the named lock: the current
        /// guard dies and the next acquisition observes a higher fence.
        fn simulate_process_takeover_like_cpp(&self) {
            self.process_lease.store(false, Ordering::Release);
            self.current_fence.fetch_add(1, Ordering::AcqRel);
        }
    }

    fn request_matches_row_like_cpp(
        pet: &DurableBattlePetRowLikeCpp,
        existing: &DurableBattlePetRowLikeCpp,
    ) -> bool {
        pet.species == existing.species
            && pet.breed == existing.breed
            && pet.display_id == existing.display_id
            && pet.quality == existing.quality
            && pet.level == existing.level
            && pet.owner_guid_counter == existing.owner_guid_counter
    }

    impl BattlePetPersistenceLikeCpp for FakeSagaPersistenceLikeCpp {
        fn try_acquire_process_lease<'a>(
            &'a self,
            _account_id: u32,
        ) -> BoxFuture<
            'a,
            Result<Option<Box<dyn BattlePetProcessLeaseLikeCpp>>, BattlePetPersistenceErrorLikeCpp>,
        > {
            Box::pin(async move {
                Ok(self
                    .process_lease
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                    .then(|| {
                        let fence = self.current_fence.fetch_add(1, Ordering::AcqRel) + 1;
                        Box::new(FakeSagaLeaseGuardLikeCpp {
                            held: Arc::clone(&self.process_lease),
                            fence,
                        }) as Box<dyn BattlePetProcessLeaseLikeCpp>
                    }))
            })
        }

        fn load_account<'a>(
            &'a self,
            _account_id: u32,
            _realm_id: u16,
        ) -> BoxFuture<'a, Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>>
        {
            Box::pin(async move {
                let state = self.state.lock().expect("fake saga persistence poisoned");
                Ok(LoadedBattlePetAccountLikeCpp {
                    pets: state.pets.clone(),
                    slots: state.slots.clone(),
                })
            })
        }

        fn allocate_guid_counter_like_cpp(
            &self,
        ) -> BoxFuture<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move { Ok(self.next_guid.fetch_add(1, Ordering::AcqRel)) })
        }

        fn insert_pet_idempotently<'a>(
            &'a self,
            request: DurableBattlePetAddLikeCpp,
        ) -> BoxFuture<
            'a,
            Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
        > {
            Box::pin(async move {
                self.insert_calls.fetch_add(1, Ordering::Relaxed);
                tokio::task::yield_now().await;
                if self.block_next_insert.swap(false, Ordering::AcqRel) {
                    self.insert_started.notify_one();
                    self.allow_insert.notified().await;
                }
                if self.current_fence.load(Ordering::Acquire) != request.fence {
                    return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
                }
                if self.fail_next_insert.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected insert failure".to_string(),
                    ));
                }
                let applied = {
                    let mut state = self.state.lock().expect("fake saga persistence poisoned");
                    if let Some((receipt_account_id, existing)) =
                        state.receipts.get(&request.request_key).cloned()
                    {
                        let still_present = state
                            .pets
                            .iter()
                            .any(|pet| pet.guid_counter == existing.guid_counter);
                        return if receipt_account_id == request.account_id
                            && request_matches_row_like_cpp(&request.pet, &existing)
                        {
                            Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                                pet: existing,
                                still_present,
                            })
                        } else {
                            Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                        };
                    }
                    let scoped_count = state
                        .pets
                        .iter()
                        .filter(|pet| pet.species == request.pet.species)
                        .filter(|pet| pet.owner_guid_counter == request.pet.owner_guid_counter)
                        .count();
                    if scoped_count >= usize::from(request.max_per_scope) {
                        return Err(BattlePetPersistenceErrorLikeCpp::Capacity);
                    }
                    if state
                        .pets
                        .iter()
                        .any(|pet| pet.guid_counter == request.pet.guid_counter)
                    {
                        return Err(BattlePetPersistenceErrorLikeCpp::GuidCollision);
                    }
                    state.pets.push(request.pet.clone());
                    state
                        .receipts
                        .insert(request.request_key, (request.account_id, request.pet));
                    true
                };
                let _ = applied;
                if self.lose_next_insert_reply.swap(false, Ordering::AcqRel) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "injected lost insert reply".to_string(),
                    ));
                }
                if self
                    .reconcile_next_insert_after_commit
                    .swap(false, Ordering::AcqRel)
                {
                    // Production reconciles its own lost COMMIT reply through
                    // the receipt as `Inserted`, preserving the fact that this
                    // invocation owns the one C++ new-pet criteria update.
                    return Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted);
                }
                Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
            })
        }

        fn lookup_add_request<'a>(
            &'a self,
            account_id: u32,
            request_key: BattlePetAddRequestKeyLikeCpp,
        ) -> BoxFuture<
            'a,
            Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
        > {
            Box::pin(async move {
                let state = self.state.lock().expect("fake saga persistence poisoned");
                let Some((receipt_account_id, pet)) = state.receipts.get(&request_key).cloned()
                else {
                    return Ok(None);
                };
                if receipt_account_id != account_id {
                    return Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest);
                }
                Ok(Some(DurableBattlePetAddReceiptLikeCpp {
                    account_id: receipt_account_id,
                    requested_pet: pet.clone(),
                    current_pet: state
                        .pets
                        .iter()
                        .find(|existing| existing.guid_counter == pet.guid_counter)
                        .cloned(),
                }))
            })
        }

        fn update_pet<'a>(
            &'a self,
            _account_id: u32,
            fence: u64,
            pet: DurableBattlePetRowLikeCpp,
        ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.current_fence.load(Ordering::Acquire) != fence {
                    return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
                }
                let mut state = self.state.lock().expect("fake saga persistence poisoned");
                let Some(existing) = state
                    .pets
                    .iter_mut()
                    .find(|existing| existing.guid_counter == pet.guid_counter)
                else {
                    return Err(BattlePetPersistenceErrorLikeCpp::Database(
                        "unknown fake pet".to_string(),
                    ));
                };
                *existing = pet;
                Ok(())
            })
        }

        fn delete_pet<'a>(
            &'a self,
            _account_id: u32,
            fence: u64,
            pet_guid_counter: u64,
            _slots: Vec<DurableBattlePetSlotLikeCpp>,
        ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.current_fence.load(Ordering::Acquire) != fence {
                    return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
                }
                self.state
                    .lock()
                    .expect("fake saga persistence poisoned")
                    .pets
                    .retain(|pet| pet.guid_counter != pet_guid_counter);
                Ok(())
            })
        }

        fn replace_slots<'a>(
            &'a self,
            _account_id: u32,
            fence: u64,
            slots: Vec<DurableBattlePetSlotLikeCpp>,
        ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
            Box::pin(async move {
                if self.current_fence.load(Ordering::Acquire) != fence {
                    return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
                }
                self.state
                    .lock()
                    .expect("fake saga persistence poisoned")
                    .slots = slots;
                Ok(())
            })
        }
    }

    fn saga_species_store_like_cpp() -> Arc<BattlePetSpeciesStore> {
        Arc::new(BattlePetSpeciesStore::from_entries([
            BattlePetSpeciesEntry {
                id: SAGA_SPECIES,
                description: String::new(),
                source_text: String::new(),
                creature_id: 99,
                summon_spell_id: 0,
                icon_file_data_id: 0,
                pet_type_enum: 0,
                flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
                source_type_enum: 0,
                card_ui_model_scene_id: 0,
                loadout_ui_model_scene_id: 0,
            },
            BattlePetSpeciesEntry {
                id: LEGACY_UNIQUE_SPECIES,
                description: String::new(),
                source_text: String::new(),
                creature_id: 100,
                summon_spell_id: 0,
                icon_file_data_id: 0,
                pet_type_enum: 0,
                flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
                    | wow_data::BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
                source_type_enum: 0,
                card_ui_model_scene_id: 0,
                loadout_ui_model_scene_id: 0,
            },
        ]))
    }

    fn saga_stat_stores_like_cpp() -> (
        Arc<BattlePetBreedQualityStore>,
        Arc<BattlePetBreedStateStore>,
        Arc<BattlePetSpeciesStateStore>,
    ) {
        (
            Arc::new(BattlePetBreedQualityStore::from_entries([
                BattlePetBreedQualityEntry {
                    id: 1,
                    state_multiplier: 1.0,
                    quality_enum: 1,
                },
            ])),
            Arc::new(BattlePetBreedStateStore::from_entries([
                BattlePetBreedStateEntry {
                    id: 1,
                    battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                    value: 500,
                    battle_pet_breed_id: 7,
                },
                BattlePetBreedStateEntry {
                    id: 2,
                    battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                    value: 300,
                    battle_pet_breed_id: 7,
                },
                BattlePetBreedStateEntry {
                    id: 3,
                    battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                    value: 200,
                    battle_pet_breed_id: 7,
                },
            ])),
            Arc::new(BattlePetSpeciesStateStore::from_entries([
                BattlePetSpeciesStateEntry {
                    id: 1,
                    battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                    value: 100,
                    battle_pet_species_id: SAGA_SPECIES,
                },
                BattlePetSpeciesStateEntry {
                    id: 2,
                    battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                    value: 100,
                    battle_pet_species_id: LEGACY_UNIQUE_SPECIES,
                },
            ])),
        )
    }

    fn saga_selection_like_cpp(species: u32) -> BattlePetTrainerSelectionLikeCpp {
        BattlePetTrainerSelectionLikeCpp {
            species,
            breed: 7,
            quality: 1,
            display_id: 123,
            level: 1,
        }
    }

    fn saga_durable_pet_row_like_cpp(
        guid_counter: u64,
        species: u32,
        owner_guid_counter: Option<u64>,
    ) -> DurableBattlePetRowLikeCpp {
        DurableBattlePetRowLikeCpp {
            guid_counter,
            species,
            breed: 7,
            display_id: 123,
            level: 1,
            exp: 0,
            health: 100,
            quality: 1,
            flags: 0,
            name: String::new(),
            name_timestamp: 0,
            owner_guid_counter,
            declined_names: None,
        }
    }

    fn saga_trainer_guid_like_cpp() -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1)
    }

    struct SagaFixtureLikeCpp {
        session: WorldSession,
        send_rx: flume::Receiver<Vec<u8>>,
        store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
        persistence: Arc<FakeSagaPersistenceLikeCpp>,
        registry: Arc<BattlePetAccountRegistryLikeCpp>,
    }

    fn make_saga_session_like_cpp(
        player_counter: i64,
        money: u64,
    ) -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(64);
        let mut session = WorldSession::new(
            ACCOUNT_ID,
            "SagaTest".into(),
            0,
            2,
            9,
            54_261,
            vec![0; 40],
            "enUS".into(),
            pkt_rx,
            send_tx,
        );
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            ObjectGuid::create_player(1, player_counter),
            "Buyer".to_string(),
            Position::ZERO,
            0,
            1,
            1,
            80,
            0,
        ));
        session.set_battlenet_account_id(ACCOUNT_ID);
        session.set_player_gold_like_cpp(money);
        session.set_battle_pet_species_store(saga_species_store_like_cpp());
        session.set_battle_pet_purchase_selection_override_like_cpp(Some(saga_selection_like_cpp(
            SAGA_SPECIES,
        )));
        (session, send_rx)
    }

    fn saga_registry_like_cpp(
        persistence: Arc<FakeSagaPersistenceLikeCpp>,
    ) -> Arc<BattlePetAccountRegistryLikeCpp> {
        let (qualities, breed_states, species_states) = saga_stat_stores_like_cpp();
        Arc::new(
            BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
                persistence,
                saga_species_store_like_cpp(),
                qualities,
                breed_states,
                species_states,
                REALM_ID,
                VIRTUAL_REALM,
            ),
        )
    }

    async fn saga_fixture_like_cpp(
        money: u64,
        seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
    ) -> SagaFixtureLikeCpp {
        let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
        let store = Arc::new(
            FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money),
        );
        let registry = saga_registry_like_cpp(Arc::clone(&persistence));
        let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
        session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        let attachment = registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("saga account attaches");
        session.set_battle_pet_account_attachment_like_cpp(attachment);
        SagaFixtureLikeCpp {
            session,
            send_rx,
            store,
            persistence,
            registry,
        }
    }

    /// A "process restart": the old session/registry are gone and a fresh
    /// registry wraps the same durable fakes (Login/Character DB survive).
    async fn restart_saga_session_like_cpp(
        store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
        persistence: Arc<FakeSagaPersistenceLikeCpp>,
        money: u64,
    ) -> SagaFixtureLikeCpp {
        let registry = saga_registry_like_cpp(Arc::clone(&persistence));
        let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
        session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        let attachment = registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("saga account attaches after restart");
        session.set_battle_pet_account_attachment_like_cpp(attachment);
        SagaFixtureLikeCpp {
            session,
            send_rx,
            store,
            persistence,
            registry,
        }
    }

    fn store_handle_like_cpp(
        store: &Arc<FakeBattlePetPurchaseStoreLikeCpp>,
    ) -> Arc<dyn BattlePetPurchaseStoreLikeCpp> {
        store.clone()
    }

    fn saga_offer_like_cpp(price: u32) -> PreparedBattlePetTrainerOfferLikeCpp {
        PreparedBattlePetTrainerOfferLikeCpp {
            source_spell_id: SAGA_SPELL_ID,
            effective_price: price,
            species_id: SAGA_SPECIES,
        }
    }

    async fn execute_saga_purchase_like_cpp(
        fixture: &mut SagaFixtureLikeCpp,
        offer: PreparedBattlePetTrainerOfferLikeCpp,
    ) -> BattlePetPurchaseExecutionLikeCpp {
        let guard = fixture
            .session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        fixture
            .session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                offer,
            )
            .await
    }

    fn owner_of(fixture: &SagaFixtureLikeCpp) -> Arc<BattlePetAccountOwnerLikeCpp> {
        fixture
            .session
            .battle_pet_account_owner_lease_like_cpp()
            .expect("attached owner")
            .0
    }

    fn expected_pet_packet_like_cpp(
        fixture: &SagaFixtureLikeCpp,
        pet_guid: ObjectGuid,
    ) -> wow_packet::packets::misc::BattlePetJournalPet {
        owner_of(fixture)
            .pet_snapshot_like_cpp(pet_guid)
            .expect("durable pet snapshot")
            .packet_info_like_cpp(pet_guid)
    }

    fn assert_no_packets(fixture: &SagaFixtureLikeCpp) {
        assert!(
            fixture.send_rx.try_recv().is_err(),
            "no packets must be published on this path"
        );
    }

    fn expect_money_update_packet_like_cpp(fixture: &SagaFixtureLikeCpp, money: u64) -> Vec<u8> {
        wow_packet::packets::update::UpdateObject::player_money_update(
            fixture.session.player_guid().expect("player guid"),
            fixture.session.player_map_id_like_cpp(),
            money,
            None,
        )
        .to_bytes()
    }

    #[tokio::test]
    async fn purchase_success_charges_once_creates_one_pet_completes_and_publishes_once_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        let BattlePetPurchaseExecutionLikeCpp::Purchased {
            pet_guid,
            published,
        } = outcome
        else {
            panic!("purchase must succeed: {outcome:?}");
        };
        assert!(published, "the Added outcome must record publication");

        // Durable facts: one charge, one completed command, one pet, one receipt.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        let commands = fixture.store.commands_snapshot();
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert_eq!(command.price, SAGA_PRICE);
        assert_eq!(command.money_before, SAGA_MONEY);
        assert_eq!(command.money_after, 750);
        assert_eq!(command.species, SAGA_SPECIES);
        assert_eq!(command.breed, 7);
        assert_eq!(command.quality, 1);
        assert_eq!(command.display_id, 123);
        assert_eq!(command.level, 1);
        assert_eq!(command.trainer_id, TRAINER_ID);
        assert_eq!(command.spell_id, SAGA_SPELL_ID);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.persistence.receipt_count(), 1);
        assert_eq!(
            fixture
                .persistence
                .receipt(command.request_key)
                .map(|pet| pet.guid_counter),
            Some(pet_guid.counter() as u64)
        );

        // Runtime money mirrors the durable charge.
        assert_eq!(fixture.session.player_gold_like_cpp(), 750);

        // Capture fixture (success): money update, then the petAdded journal
        // update, then the dependent learned spell; no trainer visual kits.
        assert_eq!(
            fixture.send_rx.try_recv().expect("money update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&fixture, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&fixture);

        // C++ `LearnSpell(dependent=true)`: runtime-known, never durable.
        assert!(
            fixture
                .session
                .represented_dependent_known_spells_like_cpp()
                .contains(&(SAGA_SPELL_ID as i32))
        );
    }

    #[tokio::test]
    async fn purchase_survives_reload_without_replaying_charge_pet_or_publication_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        let money_before = store.money(PLAYER_COUNTER as u64);
        let pets_before = persistence.pet_count();
        drop(fixture);

        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(
            summary,
            BattlePetPurchaseRecoveryLikeCpp {
                applied: 0,
                compensated: 0,
                deferred: 0,
                terminal_failures: 0,
            }
        );
        assert_eq!(restarted.store.money(PLAYER_COUNTER as u64), money_before);
        assert_eq!(restarted.store.money_mutations(), 1);
        assert_eq!(restarted.persistence.pet_count(), pets_before);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn insufficient_money_sends_teach_failure_without_charge_command_or_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(100, Vec::new()).await;
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::InsufficientMoney
        );
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(100));
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        // Capture fixture (insufficient money): exactly
        // SMSG_TRAINER_BUY_FAILED with C++ FailReason::NotEnoughMoney.
        assert_eq!(
            fixture.send_rx.try_recv().expect("teach failure"),
            TrainerBuyFailed {
                trainer_guid: saga_trainer_guid_like_cpp(),
                spell_id: SAGA_SPELL_ID as i32,
                reason: 1,
            }
            .to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn charge_failure_before_character_commit_leaves_no_charge_and_no_command_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .store
            .fail_next_charge_pre_commit
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::ChargeDeclined);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn lost_charge_reply_after_character_commit_converges_to_paid_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .store
            .lose_next_charge_reply
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        // The reconcile attributes the committed charge through the durable
        // row and the purchase completes normally with exactly one charge.
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased {
                published: true,
                ..
            }
        ));
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated, 0);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn login_insert_failure_retries_to_exactly_one_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .fail_next_insert
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.persistence.receipt_count(), 1);
        assert_eq!(fixture.store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn lost_login_insert_reply_replays_receipt_without_a_second_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .lose_next_insert_reply
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        // The first apply committed but its reply was lost. The #160 owner
        // cannot replay it in-process (its in-memory journal lost the pet
        // with the failed insert), so the saga's terminal-failure path runs
        // and the receipt re-check completes the command instead of
        // refunding: exactly one charge, exactly one durable pet, no
        // refund, no second pet, no publication beyond the charge.
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere
        );
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.persistence.receipt_count(), 1);
        assert_eq!(fixture.store.commands_snapshot().len(), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert!(!fixture.store.commands_snapshot()[0].published);
        assert_eq!(
            fixture.send_rx.try_recv().expect("money update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert_no_packets(&fixture);

        // The completed-but-unpublished command is the recovery-publication
        // signal: after a restart the owner holds the pet again and login
        // recovery emits the one success publication, then marks it.
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        let commands = restarted.store.commands_snapshot();
        assert!(commands[0].published);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_eq!(restarted.store.money_mutations(), 1);
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            restarted
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            restarted.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            restarted
                .send_rx
                .try_recv()
                .expect("recovery learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&restarted);
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn publication_marker_failure_recovers_without_losing_delivery_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        // Fail every publication/completion mark the live path attempts.
        fixture
            .store
            .fail_marks_remaining
            .store(6, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
        );
        // Pet durable + receipt durable. Both packet enqueues succeeded before
        // the marker, while marker and completion both failed to commit.
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::PendingApplication
        );
        assert!(!fixture.store.commands_snapshot()[0].published);
        assert_eq!(
            fixture.send_rx.try_recv().expect("money update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert!(fixture.send_rx.try_recv().is_ok(), "live pet update");
        assert!(fixture.send_rx.try_recv().is_ok(), "live learned spell");
        assert_no_packets(&fixture);
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        // The receipt replay completes the command and re-sends because the
        // first enqueue could not be proven. Enqueue attempts may repeat and
        // actual delivery remains best-effort; the pet and charge remain
        // exactly-once.
        assert_eq!(summary.applied, 1);
        let commands = restarted.store.commands_snapshot();
        assert_eq!(
            commands[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert!(commands[0].published);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_eq!(restarted.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(restarted.store.money_mutations(), 1);
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            restarted
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            restarted.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            restarted
                .send_rx
                .try_recv()
                .expect("recovery learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn closed_send_channel_preserves_recovery_publication_signal_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let (_replacement_tx, replacement_rx) = flume::bounded::<Vec<u8>>(1);
        let disconnected_rx = std::mem::replace(&mut fixture.send_rx, replacement_rx);
        drop(disconnected_rx);

        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        let BattlePetPurchaseExecutionLikeCpp::Purchased { published, .. } = outcome else {
            panic!("durable purchase must still complete: {outcome:?}");
        };
        assert!(!published);
        let commands = fixture.store.commands_snapshot();
        assert_eq!(
            commands[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert!(!commands[0].published);
        assert_eq!(fixture.persistence.pet_count(), 1);
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(
            fixture
                .session
                .represented_battle_pet_unique_owned_criteria_like_cpp(),
            1
        );
        assert_eq!(
            fixture
                .session
                .represented_battle_pet_learned_new_pet_criteria_like_cpp(),
            &[SAGA_SPECIES]
        );

        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert!(restarted.store.commands_snapshot()[0].published);
        assert_eq!(
            restarted
                .session
                .represented_battle_pet_unique_owned_criteria_like_cpp(),
            1
        );
        assert_eq!(
            restarted
                .session
                .represented_battle_pet_learned_new_pet_criteria_like_cpp(),
            &[SAGA_SPECIES]
        );
        assert!(restarted.send_rx.try_recv().is_ok(), "recovery pet update");
        assert!(
            restarted.send_rx.try_recv().is_ok(),
            "recovery learned spell"
        );
        assert_no_packets(&restarted);
    }

    /// Account-wide species rows carry no owner counter (C++ only sets
    /// `owner`/`ownerRealmId` for `NotAccountWide` species); the saga species
    /// is account-wide, so capacity fixtures seed ownerless rows.
    fn seed_third_pet_into_persistence_like_cpp(fixture: &SagaFixtureLikeCpp) {
        fixture
            .persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .pets
            .push(saga_durable_pet_row_like_cpp(900, SAGA_SPECIES, None));
    }

    #[tokio::test]
    async fn capacity_reached_between_list_and_apply_compensates_exactly_once_like_cpp() {
        // Two pets of the species already exist (admission cap 3 passes),
        // then a concurrent cage fills the last slot before our apply
        // reaches the Login DB capacity lock.
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        seed_third_pet_into_persistence_like_cpp(&fixture);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);

        // Charged once, refunded once, no pet, no receipt.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        let commands = fixture.store.commands_snapshot();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert!(commands[0].failure_reason.is_some());
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
        assert_eq!(fixture.persistence.receipt_count(), 0);

        // Publication on compensation: the charge and the refund money
        // updates only — never a pet packet, never a learned spell.
        assert_eq!(
            fixture.send_rx.try_recv().expect("charge update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("refund update"),
            expect_money_update_packet_like_cpp(&fixture, SAGA_MONEY)
        );
        assert_no_packets(&fixture);

        // A second compensation attempt (replay) cannot refund again.
        assert_eq!(
            fixture
                .store
                .compensate(commands[0].request_key, test_money_commit_fence_like_cpp(),)
                .await
                .expect("replayed compensation"),
            BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                durable_money: SAGA_MONEY
            }
        );
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
    }

    #[tokio::test]
    async fn capacity_known_at_admission_is_structured_unavailable_and_wire_silent_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(3, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::Capacity
            )
        );
        // C++ stays silent on the wire (`Trainer.cpp:102-106`), but the
        // structured result keeps the failure observable; nothing is
        // charged, commanded, granted or published.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn journal_lock_held_elsewhere_is_structured_unavailable_and_wire_silent_like_cpp() {
        let first = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let store = Arc::clone(&first.store);
        let persistence = Arc::clone(&first.persistence);
        let registry = Arc::clone(&first.registry);
        // First session holds the journal lease.
        assert!(
            first
                .session
                .battle_pet_try_acquire_journal_lease_like_cpp()
                .await
        );
        let (mut second_session, _second_rx) =
            make_saga_session_like_cpp(OTHER_PLAYER_COUNTER, SAGA_MONEY);
        second_session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        second_session.set_battle_pet_account_attachment_like_cpp(
            registry
                .attach_like_cpp(ACCOUNT_ID)
                .await
                .expect("second attachment"),
        );
        let guard = second_session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        let outcome = second_session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await;
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked
            )
        );
        assert!(store.commands_snapshot().is_empty());
        assert_eq!(persistence.pet_count(), 0);
        // The first session keeps its lease and can still use the journal.
        assert!(
            first
                .session
                .battle_pet_account_owner_lease_like_cpp()
                .is_some()
        );
    }

    #[tokio::test]
    async fn compensation_pre_commit_failure_retries_then_refunds_exactly_once_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        seed_third_pet_into_persistence_like_cpp(&fixture);
        fixture
            .store
            .fail_next_compensate_pre_commit
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
    }

    #[tokio::test]
    async fn lost_compensation_reply_refunds_once_and_stays_silent_after_reload_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        seed_third_pet_into_persistence_like_cpp(&fixture);
        fixture
            .store
            .lose_next_compensate_reply
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, SAGA_MONEY).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
        assert_eq!(restarted.store.money_mutations(), 2);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn terminal_failure_when_character_row_is_missing_stops_retrying_like_cpp() {
        // The character was deleted after charging: the refund can never
        // apply, so the command must become TerminalFailure instead of
        // retrying forever or losing the charge silently.
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let mut command =
            crate::battle_pet_purchase::tests::test_command([77; 16], PLAYER_COUNTER as u64);
        command.account_id = ACCOUNT_ID;
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);
        // The character row vanishes: no money row at all.
        fixture
            .store
            .remove_money_row_for_test_like_cpp(PLAYER_COUNTER as u64);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.terminal_failures, 1);
        assert_eq!(summary.compensated, 0);
        assert_eq!(
            fixture.store.command([77; 16]).expect("command").status,
            BattlePetPurchaseStatusLikeCpp::TerminalFailure
        );
        // Exactly one compensate attempt: no hot retry loop.
        assert_eq!(fixture.store.compensate_attempts.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.store.money_mutations(), 0);
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn replayed_pending_command_from_recovery_never_duplicates_pet_or_publication_like_cpp() {
        // Crash after the Login DB commit: the receipt and pet exist, the
        // Character DB command is still pending.
        let pet_row = saga_durable_pet_row_like_cpp(5, SAGA_SPECIES, None);
        let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
        let request_key = [88; 16];
        fixture
            .persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .insert(
                BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
                (ACCOUNT_ID, pet_row),
            );
        let mut command =
            crate::battle_pet_purchase::tests::test_command(request_key, PLAYER_COUNTER as u64);
        command.account_id = ACCOUNT_ID;
        command.species = SAGA_SPECIES;
        command.breed = 7;
        command.quality = 1;
        command.display_id = 123;
        command.level = 1;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);

        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(fixture.persistence.pet_count(), 1);
        assert_eq!(fixture.persistence.receipt_count(), 1);
        let command = fixture.store.command(request_key).expect("command");
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(command.published);
        // The replayed receipt had no recorded publication, so recovery
        // emits the one success publication now — never a second pet.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert_eq!(
            fixture.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(
                    &fixture,
                    ObjectGuid::create_global(HighGuid::BattlePet, 0, 5),
                )],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("recovery learned spells"),
            LearnedSpells::single(command.spell_id as i32).to_bytes()
        );
        assert_no_packets(&fixture);

        // A second recovery finds nothing unconverged and republishes
        // nothing: the durable marker proves publication already happened.
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
        assert_eq!(fixture.persistence.pet_count(), 1);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn concurrent_sessions_charge_once_grant_once_and_compensate_once_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut first = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        let store = Arc::clone(&first.store);
        let registry = Arc::clone(&first.registry);
        let persistence = Arc::clone(&first.persistence);
        store.seed_money_like_cpp(OTHER_PLAYER_COUNTER as u64, SAGA_MONEY);
        let (mut second_session, second_rx) =
            make_saga_session_like_cpp(OTHER_PLAYER_COUNTER, SAGA_MONEY);
        second_session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        second_session.set_battle_pet_account_attachment_like_cpp(
            registry
                .attach_like_cpp(ACCOUNT_ID)
                .await
                .expect("second attachment"),
        );

        let first_purchase = async {
            let guard = first
                .session
                .begin_exclusive_player_money_persistence_like_cpp()
                .await
                .expect("money exclusivity");
            first
                .session
                .execute_battle_pet_trainer_purchase_like_cpp(
                    guard,
                    saga_trainer_guid_like_cpp(),
                    TRAINER_ID,
                    saga_offer_like_cpp(SAGA_PRICE),
                )
                .await
        };
        let second_purchase = async {
            let guard = second_session
                .begin_exclusive_player_money_persistence_like_cpp()
                .await
                .expect("money exclusivity");
            second_session
                .execute_battle_pet_trainer_purchase_like_cpp(
                    guard,
                    saga_trainer_guid_like_cpp(),
                    TRAINER_ID,
                    saga_offer_like_cpp(SAGA_PRICE),
                )
                .await
        };
        let (first_outcome, second_outcome) = tokio::join!(first_purchase, second_purchase);
        let outcomes = [first_outcome, second_outcome];
        // The #160 journal lease serializes same-account sessions: exactly
        // one session is admitted and purchases; the other receives the
        // structured journal-lock result without a charge or a command.
        let purchased = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    BattlePetPurchaseExecutionLikeCpp::Purchased {
                        published: true,
                        ..
                    }
                )
            })
            .count();
        let locked = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    BattlePetPurchaseExecutionLikeCpp::Unavailable(
                        BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked
                    )
                )
            })
            .count();
        assert_eq!((purchased, locked), (1, 1), "{outcomes:?}");

        // Exactly one new pet (third of the species), one Completed
        // command, one charge; the locked session was never charged.
        assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
        assert_eq!(persistence.receipt_count(), 1);
        let statuses: Vec<_> = store
            .commands_snapshot()
            .into_iter()
            .map(|command| command.status)
            .collect();
        assert_eq!(statuses, vec![BattlePetPurchaseStatusLikeCpp::Completed]);
        let balances: Vec<_> = [PLAYER_COUNTER as u64, OTHER_PLAYER_COUNTER as u64]
            .into_iter()
            .map(|guid| store.money(guid).expect("money row"))
            .collect();
        assert!(
            balances.contains(&750) && balances.contains(&SAGA_MONEY),
            "winner charged once, loser never charged: {balances:?}"
        );
        assert_eq!(store.money_mutations(), 1);
        drop(second_rx);

        // Releasing the winner frees the journal; the loser's retry then
        // meets the filled capacity as a structured unavailable, still
        // without a charge — two sessions can never duplicate the outcome.
        let (winning_session, mut losing_session) = if matches!(
            outcomes[0],
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ) {
            (first.session, second_session)
        } else {
            (second_session, first.session)
        };
        drop(winning_session);
        let retry_guard = losing_session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        let retry_outcome = losing_session
            .execute_battle_pet_trainer_purchase_like_cpp(
                retry_guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await;
        assert_eq!(
            retry_outcome,
            BattlePetPurchaseExecutionLikeCpp::Unavailable(
                BattlePetPurchaseAdmissionFailureLikeCpp::Capacity
            )
        );
        assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
        assert_eq!(persistence.receipt_count(), 1);
        assert_eq!(store.money_mutations(), 1);
        assert_eq!(store.commands_snapshot().len(), 1);
    }

    #[tokio::test]
    async fn lost_journal_authority_defers_then_handoff_recovers_exactly_once_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .block_next_insert
            .store(true, Ordering::SeqCst);
        let persistence = fixture.persistence.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        // The apply is mid-flight inside the owner when another process wins
        // the Login DB named lock: every further fenced insert fails.
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
            _ = persistence.insert_started.notified() => {}
        }
        persistence.simulate_process_takeover_like_cpp();
        persistence.allow_insert.notify_one();
        let outcome = purchase.await;
        assert_eq!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
        );
        // Charged once; no pet, no receipt, no completion, no publication.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::PendingApplication
        );
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_eq!(
            fixture.send_rx.try_recv().expect("charge update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert_no_packets(&fixture);
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);

        // The winning process attaches a fresh owner and recovery finishes
        // the command: one pet, one completion, one recovery publication.
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_eq!(restarted.persistence.receipt_count(), 1);
        assert_eq!(
            restarted.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert_eq!(restarted.store.money_mutations(), 1);
        let commands = restarted.store.commands_snapshot();
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            restarted
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            restarted.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            restarted
                .send_rx
                .try_recv()
                .expect("recovery learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn cancelled_during_charge_pre_commit_leaves_no_trace_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let money_tracker = fixture
            .session
            .durable_loot_money_persistence_tracker_like_cpp();
        fixture
            .store
            .block_next_charge_pre_apply
            .store(true, Ordering::SeqCst);
        let store = fixture.store.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
            _ = store.gate_started.notified() => {}
        }
        drop(purchase);
        fixture.store.allow_gate.notify_one();
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        assert!(!money_tracker.is_indeterminate_like_cpp());
        // No authority survived the cancellation: a fresh purchase works.
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
    }

    #[tokio::test]
    async fn cancelled_after_charge_commit_recovers_to_paid_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let money_tracker = fixture
            .session
            .durable_loot_money_persistence_tracker_like_cpp();
        fixture
            .store
            .block_next_charge_post_apply
            .store(true, Ordering::SeqCst);
        let store = fixture.store.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
            _ = store.gate_started.notified() => {}
        }
        drop(purchase);
        fixture.store.allow_gate.notify_one();
        // The charge committed; the command is durable; nothing else ran.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::PendingApplication
        );
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert!(money_tracker.is_indeterminate_like_cpp());
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_eq!(
            restarted.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert_eq!(restarted.store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn cancelled_during_apply_completes_through_detached_worker_and_recovery_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .block_next_insert
            .store(true, Ordering::SeqCst);
        let persistence = fixture.persistence.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
            _ = persistence.insert_started.notified() => {}
        }
        drop(purchase);
        // The #160 worker is detached from the cancelled caller: releasing
        // the gate lets it finish the durable insert exactly once.
        fixture.persistence.allow_insert.notify_one();
        let deadline = std::time::Instant::now() + StdDuration::from_secs(2);
        while fixture.persistence.receipt_count() == 0 && std::time::Instant::now() < deadline {
            tokio::time::sleep(StdDuration::from_millis(10)).await;
        }
        assert_eq!(fixture.persistence.receipt_count(), 1);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::PendingApplication
        );
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        // The receipt replay completes the command: no second pet and no
        // new charge, and because no publication was ever recorded the one
        // idempotent success publication is emitted now.
        assert_eq!(summary.applied, 1);
        assert_eq!(restarted.persistence.pet_count(), 1);
        assert_eq!(restarted.store.money_mutations(), 1);
        let commands = restarted.store.commands_snapshot();
        assert_eq!(
            commands[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert!(commands[0].published);
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            restarted
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            restarted.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            restarted
                .send_rx
                .try_recv()
                .expect("recovery learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn cancelled_before_compensation_refund_recovers_exactly_once_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        let money_tracker = fixture
            .session
            .durable_loot_money_persistence_tracker_like_cpp();
        seed_third_pet_into_persistence_like_cpp(&fixture);
        fixture
            .store
            .block_next_compensate_pre_apply
            .store(true, Ordering::SeqCst);
        let store = fixture.store.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the compensation gate: {outcome:?}"),
            _ = store.gate_started.notified() => {}
        }
        drop(purchase);
        fixture.store.allow_gate.notify_one();
        // The decision is durable, the refund is not: money still charged.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::CompensationPending
        );
        assert!(!money_tracker.is_indeterminate_like_cpp());
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);
        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.compensated, 1);
        assert_eq!(
            restarted.store.money(PLAYER_COUNTER as u64),
            Some(SAGA_MONEY)
        );
        assert_eq!(restarted.store.money_mutations(), 2);
        assert_eq!(
            restarted.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert_eq!(restarted.persistence.species_count(SAGA_SPECIES), 3);
        // Login recovery staged the runtime restore without a values packet.
        assert_eq!(restarted.session.player_gold_like_cpp(), SAGA_MONEY);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn cancelled_after_compensation_refund_stays_refunded_once_like_cpp() {
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        let money_tracker = fixture
            .session
            .durable_loot_money_persistence_tracker_like_cpp();
        seed_third_pet_into_persistence_like_cpp(&fixture);
        fixture
            .store
            .block_next_compensate_post_apply
            .store(true, Ordering::SeqCst);
        let store = fixture.store.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the compensation gate: {outcome:?}"),
            _ = store.gate_started.notified() => {}
        }
        drop(purchase);
        fixture.store.allow_gate.notify_one();
        // The refund and the status flip committed together before the
        // cancellation: exactly one refund, terminally compensated.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
        assert_eq!(
            fixture.store.commands_snapshot()[0].status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert!(money_tracker.is_indeterminate_like_cpp());
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 2);
    }

    #[tokio::test]
    async fn shutdown_drain_is_bounded_and_completes_inflight_purchase_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .block_next_insert
            .store(true, Ordering::SeqCst);
        let registry = Arc::clone(&fixture.registry);
        let persistence = fixture.persistence.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        let drained = tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
            drained = async {
                persistence.insert_started.notified().await;
                registry
                    .drain_like_cpp(StdDuration::from_millis(50))
                    .await
            } => drained,
        };
        assert!(
            !drained,
            "the bounded shutdown drain must time out while the worker is blocked"
        );
        persistence.allow_insert.notify_one();
        let outcome = purchase.await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
        assert!(
            fixture
                .registry
                .drain_like_cpp(StdDuration::from_secs(1))
                .await,
            "after releasing the worker the bounded drain completes"
        );
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.store.money_mutations(), 1);
    }

    #[tokio::test]
    async fn deterministic_selection_flows_into_command_and_pet_like_cpp() {
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .session
            .set_battle_pet_purchase_selection_override_like_cpp(Some(
                BattlePetTrainerSelectionLikeCpp {
                    species: SAGA_SPECIES,
                    breed: 9,
                    quality: 2,
                    display_id: 456,
                    level: 1,
                },
            ));
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
        let command = &fixture.store.commands_snapshot()[0];
        assert_eq!(
            (command.breed, command.quality, command.display_id),
            (9, 2, 456)
        );
        let receipt = fixture
            .persistence
            .receipt(command.request_key)
            .expect("receipt");
        assert_eq!(
            (receipt.breed, receipt.quality, receipt.display_id),
            (9, 2, 456)
        );
    }

    // ── Handler-level fixtures (CMSG_TRAINER_BUY_SPELL end to end) ────

    const SAGA_CREATURE_ENTRY: u32 = 123;
    const SAGA_SUMMON_PROPERTIES_ID: u32 = 700;
    const SAGA_SUMMON_SLOT_MINIPET_RAW: i64 = 5;
    const SAGA_SUMMON_FROM_JOURNAL_RAW: i64 = 0x0020_0000;

    fn saga_learn_effect_like_cpp(
        record_id: u32,
        wrapper_spell_id: u32,
        learned_spell_id: u32,
    ) -> wow_data::SpellAcquisitionEffectLikeCpp {
        wow_data::SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: i64::from(wrapper_spell_id),
            difficulty_id_raw: 0,
            effect_index_raw: 1,
            effect_type_raw: 36, // C++ SPELL_EFFECT_LEARN_SPELL
            effect_aura_raw: 0,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0_f32.to_bits(),
            effect_real_points_per_level_bits: 0.0_f32.to_bits(),
            effect_coefficient_bits: 0.0_f32.to_bits(),
            effect_variance_bits: 0.0_f32.to_bits(),
            effect_trigger_spell_raw: i64::from(learned_spell_id),
            effect_item_type_raw: 0,
            effect_misc_value_raw: [0, 0],
            implicit_target_raw: [1, 0],
        }
    }

    fn saga_summon_effect_like_cpp(spell_id: u32) -> wow_data::SpellAcquisitionEffectLikeCpp {
        wow_data::SpellAcquisitionEffectLikeCpp {
            record_id: 1,
            spell_id_raw: i64::from(spell_id),
            difficulty_id_raw: 0,
            effect_index_raw: 0,
            effect_type_raw: 28, // C++ SPELL_EFFECT_SUMMON
            effect_aura_raw: 0,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0_f32.to_bits(),
            effect_real_points_per_level_bits: 0.0_f32.to_bits(),
            effect_coefficient_bits: 0.0_f32.to_bits(),
            effect_variance_bits: 0.0_f32.to_bits(),
            effect_trigger_spell_raw: 0,
            effect_item_type_raw: 0,
            effect_misc_value_raw: [99, i64::from(SAGA_SUMMON_PROPERTIES_ID)],
            implicit_target_raw: [1, 0],
        }
    }

    fn insert_saga_trainer_creature_like_cpp(
        manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
        guid: ObjectGuid,
    ) {
        let mut creature = wow_entities::Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(SAGA_CREATURE_ENTRY);
        creature.unit_mut().world_mut().set_map(0, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::new(1.0, 0.0, 0.0, 0.0));
        creature.unit_mut().world_mut().set_combat_reach(1.0);
        creature.unit_mut().set_level(80);
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        creature.set_ai_identity_runtime(
            1,
            35,
            wow_constants::unit::NPCFlags1::TRAINER.bits()
                | wow_constants::unit::NPCFlags1::TRAINER_CLASS.bits()
                | wow_constants::unit::NPCFlags1::TRAINER_PROFESSION.bits(),
            0,
        );
        creature.unit_mut().world_mut().object_mut().add_to_world();
        manager
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .expect("canonical test map")
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    /// A fully rigged handler session: the trainer list/buy path runs the
    /// real admission composition (membership, gates, conditions, price,
    /// classification) before the saga. `wrapper_learned_spell` adds a
    /// `SPELL_EFFECT_LEARN_SPELL` effect so the trainer spell is castable
    /// (C++ `IsCastable()`), turning the offer into the normal wrapper
    /// acquisition that retains its battle-pet species classification.
    async fn saga_handler_fixture_like_cpp(
        money: u64,
        battle_pet_price: u32,
        wrapper_learned_spell: Option<u32>,
        seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
    ) -> SagaFixtureLikeCpp {
        let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
        let store = Arc::new(
            FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money),
        );
        let registry = saga_registry_like_cpp(Arc::clone(&persistence));
        let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
        let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_trainer_store_like_cpp(Arc::new(
            wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
                vec![wow_data::TrainerRowLikeCpp {
                    id: TRAINER_ID,
                    trainer_type: 2,
                    greeting: "Train".to_string(),
                }],
                vec![wow_data::TrainerSpellRowLikeCpp {
                    trainer_id: TRAINER_ID,
                    spell: wow_data::TrainerSpellLikeCpp {
                        spell_id: SAGA_SPELL_ID,
                        money_cost: battle_pet_price,
                        req_skill_line: 0,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        req_level: 1,
                    },
                }],
                Vec::new(),
                vec![wow_data::CreatureTrainerRowLikeCpp {
                    creature_id: SAGA_CREATURE_ENTRY,
                    trainer_id: TRAINER_ID,
                    menu_id: 0,
                    option_id: 0,
                }],
                |_| true,
                |_| true,
                |_| true,
                |_, _| true,
            )
            .store,
        ));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.set_disable_mgr(Arc::new(wow_data::DisableMgrLikeCpp::default()));
        session.set_player_aura_authority_complete_like_cpp(true);
        session.set_condition_store(Arc::new(wow_data::ConditionEntriesByTypeStore::default()));
        session.set_skill_store(Arc::new(
            wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], []),
        ));
        session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([])));
        session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
        session.set_trait_definition_store(Arc::new(
            wow_data::trait_tree::TraitDefinitionStore::from_entries([]),
        ));
        session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
        session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
        session.set_spell_custom_attribute_store(Arc::new(
            wow_data::SpellCustomAttributeStoreLikeCpp::default(),
        ));
        let mut learn_skills = wow_data::SpellLearnSkillStoreLikeCpp::default();
        learn_skills.covered_spell_ids.extend([SAGA_SPELL_ID]);
        if let Some(learned) = wrapper_learned_spell {
            learn_skills.covered_spell_ids.extend([learned]);
        }
        session.set_spell_learn_skill_store(Arc::new(learn_skills));
        if wrapper_learned_spell.is_some() {
            session.set_spell_acquisition_static_authority_like_cpp([SAGA_SPELL_ID], []);
            session.set_loot_money_persistence_test_result_like_cpp(true);
        }
        session.set_spell_learn_spell_store(Arc::new(
            wow_data::SpellLearnSpellStoreLikeCpp::default(),
        ));
        session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
        session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
        session.set_spell_pet_aura_store(Arc::new(wow_data::SpellPetAuraStoreLikeCpp::default()));
        session.set_spell_target_restrictions_store(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([]),
        ));
        session.set_spell_aura_restrictions_store(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([]),
        ));
        let mut coverage = vec![wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
            SAGA_SPELL_ID,
            0,
        )];
        let mut spell_effects = vec![saga_summon_effect_like_cpp(SAGA_SPELL_ID)];
        if let Some(learned) = wrapper_learned_spell {
            coverage.push(wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
                learned, 0,
            ));
            spell_effects.push(saga_learn_effect_like_cpp(2, SAGA_SPELL_ID, learned));
        }
        session.set_spell_acquisition_catalog(Arc::new(
            wow_data::SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                coverage,
                wow_data::EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects,
                    summon_properties: vec![wow_data::SpellAcquisitionSummonPropertiesLikeCpp {
                        record_id: SAGA_SUMMON_PROPERTIES_ID,
                        slot_raw: SAGA_SUMMON_SLOT_MINIPET_RAW,
                        flags_1_raw: SAGA_SUMMON_FROM_JOURNAL_RAW,
                    }],
                    battle_pet_species: vec![wow_data::SpellAcquisitionBattlePetSpeciesLikeCpp {
                        species_id: SAGA_SPECIES,
                        creature_id_raw: 99,
                    }],
                    ..Default::default()
                },
                wow_data::SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        session.set_known_spells_like_cpp(Vec::new());
        assert!(session.set_complete_represented_player_spell_rows_like_cpp([]));
        assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
        assert!(session.set_complete_represented_override_spells_like_cpp([]));
        assert!(
            session.set_complete_player_skill_records_like_cpp(std::collections::HashMap::new(), 0)
        );
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical player map");
        insert_saga_trainer_creature_like_cpp(&canonical, saga_trainer_guid_like_cpp());
        session.set_player_trainer_interaction_like_cpp(saga_trainer_guid_like_cpp(), TRAINER_ID);
        session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        let attachment = registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("saga account attaches");
        session.set_battle_pet_account_attachment_like_cpp(attachment);
        SagaFixtureLikeCpp {
            session,
            send_rx,
            store,
            persistence,
            registry,
        }
    }

    fn saga_buy_packet_like_cpp(spell_id: i32) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_packed_guid(&saga_trainer_guid_like_cpp());
        packet.write_int32(TRAINER_ID as i32);
        packet.write_int32(spell_id);
        packet.reset_read();
        packet
    }

    #[tokio::test]
    async fn handler_battle_pet_buy_runs_the_full_saga_like_cpp() {
        let mut fixture =
            saga_handler_fixture_like_cpp(SAGA_MONEY, SAGA_PRICE, None, Vec::new()).await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
            .await;
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        let commands = fixture.store.commands_snapshot();
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].status,
            BattlePetPurchaseStatusLikeCpp::Completed
        );
        assert_eq!(commands[0].price, SAGA_PRICE);
        assert_eq!(commands[0].trainer_id, TRAINER_ID);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.session.player_gold_like_cpp(), 750);
        // Wire order with trainer visuals suppressed: money update, petAdded
        // journal update, dependent learned spell.
        assert_eq!(
            fixture.send_rx.try_recv().expect("money update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        let commands = fixture.store.commands_snapshot();
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            fixture
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&fixture, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn handler_buy_revalidates_membership_and_current_price_like_cpp() {
        // A spell outside the trainer's current spell set is rejected with
        // the C++ generic failure before any saga state exists.
        let mut fixture =
            saga_handler_fixture_like_cpp(SAGA_MONEY, SAGA_PRICE, None, Vec::new()).await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(999_999))
            .await;
        assert_eq!(
            fixture.send_rx.try_recv().expect("teach failure"),
            TrainerBuyFailed {
                trainer_guid: saga_trainer_guid_like_cpp(),
                spell_id: 999_999,
                reason: 0,
            }
            .to_bytes()
        );
        assert_no_packets(&fixture);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));

        // The same spell at a different current store price charges the
        // current price, not a previously listed one.
        let mut fixture = saga_handler_fixture_like_cpp(SAGA_MONEY, 400, None, Vec::new()).await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
            .await;
        let commands = fixture.store.commands_snapshot();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].price, 400);
        assert_eq!(commands[0].money_after, 600);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(600));
    }

    #[tokio::test]
    async fn handler_capacity_failure_publishes_no_packets_like_cpp() {
        // Capture fixture (capacity): the structured admission failure keeps
        // the wire silent exactly like C++.
        let mut fixture = saga_handler_fixture_like_cpp(
            SAGA_MONEY,
            SAGA_PRICE,
            None,
            vec![
                saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
                saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
                saga_durable_pet_row_like_cpp(3, SAGA_SPECIES, None),
            ],
        )
        .await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
            .await;
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn recovery_publishes_exactly_one_pet_update_after_crash_before_apply_like_cpp() {
        // Capture fixture (recovery publication): a crash right after the
        // Character DB commit is resumed by login recovery, which applies
        // the pet and publishes the one allowed petAdded update.
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .store
            .block_next_charge_post_apply
            .store(true, Ordering::SeqCst);
        let store = fixture.store.clone();
        let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
            &mut fixture,
            saga_offer_like_cpp(SAGA_PRICE),
        ));
        tokio::select! {
            outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
            _ = store.gate_started.notified() => {}
        }
        drop(purchase);
        fixture.store.allow_gate.notify_one();
        while fixture.send_rx.try_recv().is_ok() {}
        let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
        drop(fixture);

        let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        let commands = restarted.store.commands_snapshot();
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            restarted
                .persistence
                .receipt(commands[0].request_key)
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            restarted.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            restarted
                .send_rx
                .try_recv()
                .expect("recovery learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&restarted);

        // A further recovery replays nothing and publishes nothing.
        let summary = restarted
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
        assert_no_packets(&restarted);
    }

    #[tokio::test]
    async fn recovery_with_receipt_after_recorded_decision_completes_without_refunding_like_cpp() {
        // The terminal-failure decision was recorded, but the Login DB
        // receipt proves the pet became durable after all: recovery must
        // complete the command, never refund a durable pet.
        let pet_row = saga_durable_pet_row_like_cpp(6, SAGA_SPECIES, None);
        let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
        let request_key = [99; 16];
        fixture
            .persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .insert(
                BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
                (ACCOUNT_ID, pet_row),
            );
        let mut command =
            crate::battle_pet_purchase::tests::test_command(request_key, PLAYER_COUNTER as u64);
        command.account_id = ACCOUNT_ID;
        command.species = SAGA_SPECIES;
        command.breed = 7;
        command.quality = 1;
        command.display_id = 123;
        command.level = 1;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
        fixture.store.seed_command(command);

        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(summary.compensated, 0);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert_eq!(fixture.persistence.pet_count(), 1);
        let command = fixture.store.command(request_key).expect("command");
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(command.published);
        // The receipt replay resolves the packet in-process, so the one
        // success publication goes out instead of the refund.
        assert_eq!(
            fixture.send_rx.try_recv().expect("recovery pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(
                    &fixture,
                    ObjectGuid::create_global(HighGuid::BattlePet, 0, 6),
                )],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("recovery learned spells"),
            LearnedSpells::single(command.spell_id as i32).to_bytes()
        );
        assert_no_packets(&fixture);
    }

    // ── Castable (wrapper) spells that still carry a battle-pet species ──

    const SAGA_WRAPPER_LEARNED_SPELL: u32 = 54_331;
    const SAGA_WRAPPER_PRICE: u32 = 25;

    #[tokio::test]
    async fn handler_castable_battle_pet_spell_fails_closed_before_money_like_cpp() {
        // C++ would charge and cast a hybrid (learn + battle-pet summon)
        // trainer spell with the silent cap and suppressed visuals. The #164
        // acquisition planner deliberately does not project SUMMON effects
        // (`BattlePetOrSummonPath`), so today the offer fails closed BEFORE
        // money, visuals, pet or saga state: the one observable packet is
        // the C++ generic buy failure. The offer type still carries the
        // species (pure-decision test in trainer_offer.rs) and the handler
        // keeps the C++-shared cap gate and visual suppression for the day
        // the planner models hybrid casts.
        let mut fixture = saga_handler_fixture_like_cpp(
            SAGA_MONEY,
            SAGA_WRAPPER_PRICE,
            Some(SAGA_WRAPPER_LEARNED_SPELL),
            Vec::new(),
        )
        .await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
            .await;
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&(SAGA_WRAPPER_LEARNED_SPELL as i32)),
            "the hybrid cast must not run while the planner rejects it"
        );
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(
            fixture.send_rx.try_recv().expect("teach failure"),
            TrainerBuyFailed {
                trainer_guid: saga_trainer_guid_like_cpp(),
                spell_id: SAGA_SPELL_ID as i32,
                reason: 0,
            }
            .to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn handler_castable_battle_pet_spell_never_reaches_the_saga_even_uncapped_like_cpp() {
        // Same fail-closed proof with journal capacity available: the
        // rejection is the planner boundary, not the battle-pet cap.
        let mut fixture = saga_handler_fixture_like_cpp(
            SAGA_MONEY,
            SAGA_WRAPPER_PRICE,
            Some(SAGA_WRAPPER_LEARNED_SPELL),
            vec![saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None)],
        )
        .await;
        fixture
            .session
            .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
            .await;
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        assert_eq!(fixture.persistence.pet_count(), 1);
        assert!(fixture.store.commands_snapshot().is_empty());
        assert_eq!(
            fixture.send_rx.try_recv().expect("teach failure"),
            TrainerBuyFailed {
                trainer_guid: saga_trainer_guid_like_cpp(),
                spell_id: SAGA_SPELL_ID as i32,
                reason: 0,
            }
            .to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn reconciled_lost_insert_reply_runs_new_pet_criteria_once_like_cpp() {
        // The Login DB insert commits but its reply is lost; the real
        // persistence reconciles the duplicate key through the receipt and
        // reports `Inserted` for this invocation. The pet was never published
        // and its C++ new-pet criteria hooks were never run, so this execution
        // owns both exactly once.
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        fixture
            .persistence
            .reconcile_next_insert_after_commit
            .store(true, Ordering::SeqCst);
        let outcome =
            execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
        let BattlePetPurchaseExecutionLikeCpp::Purchased {
            pet_guid,
            published,
        } = outcome
        else {
            panic!("reconciled insert must complete the purchase: {outcome:?}");
        };
        assert!(published, "a reconciled durable add still publishes once");
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.persistence.receipt_count(), 1);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 1);
        let command = &fixture.store.commands_snapshot()[0];
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(command.published);
        assert_eq!(
            fixture
                .session
                .represented_battle_pet_unique_owned_criteria_like_cpp(),
            1
        );
        assert_eq!(
            fixture
                .session
                .represented_battle_pet_learned_new_pet_criteria_like_cpp(),
            &[SAGA_SPECIES]
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("money update"),
            expect_money_update_packet_like_cpp(&fixture, 750)
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&fixture, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn recovery_after_account_transfer_compensates_without_applying_or_publishing_like_cpp() {
        // The character moved to another Battle.net account while the
        // purchase was pending: the pet must never be applied into the new
        // account, so the saga refunds the character (money travels with
        // it) exactly once and publishes nothing.
        let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
        let mut command =
            crate::battle_pet_purchase::tests::test_command([55; 16], PLAYER_COUNTER as u64);
        command.account_id = 999;
        command.species = SAGA_SPECIES;
        command.breed = 7;
        command.quality = 1;
        command.display_id = 123;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.compensated, 1);
        assert_eq!(summary.applied, 0);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_eq!(fixture.persistence.receipt_count(), 0);
        assert_eq!(
            fixture.store.command([55; 16]).expect("command").status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn completed_unpublished_after_account_transfer_closes_marker_without_publishing_like_cpp()
     {
        // The pet was durably created in the original account's journal
        // before the transfer; the new account's client must never receive
        // an update for a pet it does not own, so recovery just closes the
        // publication marker.
        let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
        let mut command =
            crate::battle_pet_purchase::tests::test_command([56; 16], PLAYER_COUNTER as u64);
        command.account_id = 999;
        command.species = SAGA_SPECIES;
        command.status = BattlePetPurchaseStatusLikeCpp::Completed;
        command.published = false;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(summary.compensated, 0);
        assert!(fixture.store.command([56; 16]).expect("command").published);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.persistence.pet_count(), 0);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn account_transfer_with_durable_pet_completes_without_refunding_or_publishing_like_cpp()
    {
        // Crash window + account transfer: the Login DB pet/receipt
        // committed under the ORIGINAL account before the Character DB
        // command completed. Recovery must not refund (the pet is durable)
        // and must not publish into the new account.
        let pet_row = saga_durable_pet_row_like_cpp(7, SAGA_SPECIES, None);
        let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
        let request_key = [57; 16];
        fixture
            .persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .insert(
                BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
                (999, pet_row),
            );
        let mut command =
            crate::battle_pet_purchase::tests::test_command(request_key, PLAYER_COUNTER as u64);
        command.account_id = 999;
        command.species = SAGA_SPECIES;
        command.breed = 7;
        command.quality = 1;
        command.display_id = 123;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 1);
        assert_eq!(summary.compensated, 0);
        let command = fixture.store.command(request_key).expect("command");
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(command.published);
        // No refund, no new pet, no publication into the new account.
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert_eq!(fixture.persistence.pet_count(), 1);
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn purchase_binds_the_battlenet_account_identity_like_cpp() {
        // Game account 1, Battle.net account 5: the durable command and the
        // Login DB receipt authority must both use the Battle.net identity,
        // or a later account relink would replay into the wrong journal.
        let persistence = Arc::new(FakeSagaPersistenceLikeCpp::default());
        let store = Arc::new(
            FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, SAGA_MONEY),
        );
        let registry = saga_registry_like_cpp(Arc::clone(&persistence));
        let (mut session, _send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, SAGA_MONEY);
        session.set_battlenet_account_id(5);
        session.set_battle_pet_purchase_store_like_cpp(store_handle_like_cpp(&store));
        session.set_battle_pet_account_attachment_like_cpp(
            registry
                .attach_like_cpp(5)
                .await
                .expect("attach Battle.net account 5"),
        );
        let guard = session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        let outcome = session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await;
        assert!(matches!(
            outcome,
            BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
        ));
        let command = &store.commands_snapshot()[0];
        assert_eq!(command.account_id, 5);
        let receipt_account = persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .get(&BattlePetAddRequestKeyLikeCpp::from_bytes(
                command.request_key,
            ))
            .map(|(account, _)| *account)
            .expect("receipt");
        assert_eq!(receipt_account, 5);
    }

    #[tokio::test]
    async fn completed_unpublished_with_deleted_pet_settles_marker_and_batch_continues_like_cpp() {
        // The pet and receipt committed, then another session deleted the
        // pet before this character's recovery: nothing can be published,
        // but the charge stands and the row must converge instead of
        // blocking every later login and every newer command.
        let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
        let deleted_key = [60; 16];
        let deleted_row = saga_durable_pet_row_like_cpp(8, SAGA_SPECIES, None);
        fixture
            .persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .insert(
                BattlePetAddRequestKeyLikeCpp::from_bytes(deleted_key),
                (ACCOUNT_ID, deleted_row),
            );
        let mut deleted_command =
            crate::battle_pet_purchase::tests::test_command(deleted_key, PLAYER_COUNTER as u64);
        deleted_command.account_id = ACCOUNT_ID;
        deleted_command.species = SAGA_SPECIES;
        deleted_command.breed = 7;
        deleted_command.quality = 1;
        deleted_command.display_id = 123;
        deleted_command.money_before = SAGA_MONEY;
        deleted_command.money_after = 750;
        deleted_command.status = BattlePetPurchaseStatusLikeCpp::Completed;
        deleted_command.published = false;
        fixture.store.seed_command(deleted_command);
        let mut pending_command =
            crate::battle_pet_purchase::tests::test_command([61; 16], PLAYER_COUNTER as u64);
        pending_command.account_id = ACCOUNT_ID;
        pending_command.species = SAGA_SPECIES;
        pending_command.breed = 7;
        pending_command.quality = 1;
        pending_command.display_id = 123;
        pending_command.money_before = SAGA_MONEY;
        pending_command.money_after = 750;
        fixture.store.seed_command(pending_command);

        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.applied, 2);
        assert_eq!(summary.deferred, 0);
        // The deleted pet's marker is closed without any packet; the newer
        // command behind it converges normally with its one publication.
        let deleted = fixture.store.command(deleted_key).expect("command");
        assert_eq!(deleted.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(deleted.published);
        let pending = fixture.store.command([61; 16]).expect("command");
        assert_eq!(pending.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert!(pending.published);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 0);
        let pet_guid = ObjectGuid::create_global(
            HighGuid::BattlePet,
            0,
            fixture
                .persistence
                .receipt([61; 16])
                .expect("receipt")
                .guid_counter as i64,
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(&fixture, pet_guid)],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("learned spells"),
            LearnedSpells::single(pending.spell_id as i32).to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn mismatched_refund_waits_for_original_account_fence_like_cpp() {
        // No receipt, but the original account's authority is held by a
        // still-flying driver: the snapshot absence must not refund, so the
        // command waits for the fence instead of risking pet + refund.
        let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
        let mut command =
            crate::battle_pet_purchase::tests::test_command([62; 16], PLAYER_COUNTER as u64);
        command.account_id = 999;
        command.species = SAGA_SPECIES;
        command.money_before = SAGA_MONEY;
        command.money_after = 750;
        fixture.store.seed_command(command);
        fixture
            .persistence
            .process_lease
            .store(true, Ordering::SeqCst);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.compensated, 0);
        assert!(summary.deferred >= 1);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
        assert_eq!(fixture.store.money_mutations(), 0);
        assert_eq!(
            fixture.store.command([62; 16]).expect("command").status,
            BattlePetPurchaseStatusLikeCpp::CompensationPending
        );
        // Once the original account's fence is free and the absence is
        // proven under it, the refund converges exactly once.
        fixture
            .persistence
            .process_lease
            .store(false, Ordering::SeqCst);
        let summary = fixture
            .session
            .recover_battle_pet_trainer_purchases_like_cpp()
            .await
            .expect("recovery runs");
        assert_eq!(summary.compensated, 1);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 1);
        assert_eq!(
            fixture.store.command([62; 16]).expect("command").status,
            BattlePetPurchaseStatusLikeCpp::Compensated
        );
        assert_eq!(fixture.persistence.pet_count(), 0);
    }

    #[tokio::test]
    async fn zero_price_purchase_grants_the_free_pet_like_cpp() {
        // C++ `HasEnoughMoney(0)` always passes and `ModifyMoney(-0)` is a
        // no-op: a zero-cost battle-pet trainer row must charge nothing and
        // still create the pet, the command and the one publication.
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
        let outcome = execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(0)).await;
        let BattlePetPurchaseExecutionLikeCpp::Purchased {
            pet_guid: _,
            published,
        } = outcome
        else {
            panic!("a zero-price purchase must succeed: {outcome:?}");
        };
        assert!(published);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 0);
        let command = &fixture.store.commands_snapshot()[0];
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
        assert_eq!(command.price, 0);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
        assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
        // No money update packet for a zero charge; the pet and learned
        // spell still publish once.
        assert_eq!(
            fixture.send_rx.try_recv().expect("pet update"),
            wow_packet::packets::misc::BattlePetUpdates {
                pets: vec![expected_pet_packet_like_cpp(
                    &fixture,
                    ObjectGuid::create_global(
                        HighGuid::BattlePet,
                        0,
                        fixture
                            .persistence
                            .receipt(command.request_key)
                            .expect("receipt")
                            .guid_counter as i64,
                    ),
                )],
                pet_added: true,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().expect("learned spells"),
            LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
        );
        assert_no_packets(&fixture);
    }

    #[tokio::test]
    async fn zero_price_compensation_flips_status_without_money_mutation_like_cpp() {
        // A zero-price purchase that fails terminally compensates by
        // flipping the command once; there is no refund statement to roll
        // back, so the compensation must still converge.
        let seeded = vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        ];
        let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
        seed_third_pet_into_persistence_like_cpp(&fixture);
        let outcome = execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(0)).await;
        assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
        assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
        assert_eq!(fixture.store.money_mutations(), 0);
        let command = &fixture.store.commands_snapshot()[0];
        assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Compensated);
        assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
        assert_no_packets(&fixture);
    }
}
