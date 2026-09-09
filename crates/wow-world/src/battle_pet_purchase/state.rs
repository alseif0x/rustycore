//! Battle-pet purchase state definitions, part 1 of 1.
//!
//! Separated from the battle_pet_purchase.rs root under #656. Behaviour is preserved.

use super::*;

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

pub(super) fn record_battle_pet_purchase_publication_trace_like_cpp(name: &'static str) {
    tracing::trace!(publication = name, "battle-pet persistence publication");
}

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
pub(super) fn battle_pet_add_failure_is_retryable_like_cpp(
    error: &BattlePetAddFailureLikeCpp,
) -> bool {
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
pub(super) fn battle_pet_add_failure_is_terminal_like_cpp(
    error: &BattlePetAddFailureLikeCpp,
) -> bool {
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
pub(super) async fn retry_battle_pet_purchase_step_like_cpp<T, E, Fut, F>(
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

pub(super) fn store_error_is_retryable_like_cpp(
    error: &BattlePetPurchaseStoreErrorLikeCpp,
) -> bool {
    matches!(error, BattlePetPurchaseStoreErrorLikeCpp::Retryable(_))
}

/// Whether a compensation publishes its restored money with a values-update
/// packet (in-world live path) or only stages the runtime value (login
/// recovery, where the initial object update carries it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BattlePetPurchaseRefundPublicationLikeCpp {
    ValuesUpdatePacket,
    RuntimeOnly,
}
