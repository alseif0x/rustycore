//! Stored-item loot money attempt and lost-COMMIT reconciliation contract.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::PersistenceFutureLikeCpp;

/// One atomic attempt to credit money stored in an Item loot container.
///
/// Gameplay owns the requested amount and the money cap. The concrete adapter
/// owns row locking, statement identity, affected-row checks and COMMIT error
/// classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredItemMoneyPersistenceRequestLikeCpp {
    pub player_guid: u64,
    pub item_guid: u64,
    pub cached_notified_amount: u64,
    pub max_money: u64,
}

pub const STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredItemMoneyPersistenceOutcomeLikeCpp {
    pub before: u64,
    pub after: u64,
    pub applied_delta: u64,
    pub notified_amount: u64,
}

pub fn stored_item_money_zero_without_source_outcome_like_cpp(
    before: u64,
    cached_notified_amount: u64,
) -> Option<StoredItemMoneyPersistenceOutcomeLikeCpp> {
    (cached_notified_amount == 0).then_some(StoredItemMoneyPersistenceOutcomeLikeCpp {
        before,
        after: before,
        applied_delta: 0,
        notified_amount: 0,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredItemMoneyRollbackKindLikeCpp {
    MissingPlayer,
    SourceAlreadyConsumed,
    Database,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredItemMoneyPersistenceAttemptLikeCpp {
    Applied(StoredItemMoneyPersistenceOutcomeLikeCpp),
    DefinitelyRolledBack {
        kind: StoredItemMoneyRollbackKindLikeCpp,
        reason: String,
        retryable_deadlock: bool,
    },
    CommitOutcomeUnknown {
        reason: String,
        outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoredItemMoneyReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate { reason: Option<String> },
}

pub fn classify_stored_item_money_reconciliation_like_cpp(
    outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    observed_money: u64,
    observed_source_money: Option<u64>,
) -> StoredItemMoneyReconciliationLikeCpp {
    let all_before =
        observed_money == outcome.before && observed_source_money == Some(outcome.notified_amount);
    let all_after = observed_money == outcome.after && observed_source_money.is_none();
    match (all_before, all_after) {
        (true, false) => StoredItemMoneyReconciliationLikeCpp::RolledBack,
        (false, true) => StoredItemMoneyReconciliationLikeCpp::Committed,
        _ => StoredItemMoneyReconciliationLikeCpp::Indeterminate { reason: None },
    }
}

/// SQLx-free Characters-database capability for stored Item loot money.
pub trait StoredItemMoneyPersistencePortLikeCpp: Send + Sync {
    fn attempt_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyPersistenceAttemptLikeCpp>;

    fn reconcile_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
        outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyReconciliationLikeCpp>;
}
