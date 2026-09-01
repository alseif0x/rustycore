//! MariaDB adapter for the SQLx-free stored Item loot-money capability.

use std::sync::Arc;

use wow_persistence::{
    PersistenceFutureLikeCpp, STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
    StoredItemMoneyPersistenceAttemptLikeCpp, StoredItemMoneyPersistenceOutcomeLikeCpp,
    StoredItemMoneyPersistencePortLikeCpp, StoredItemMoneyPersistenceRequestLikeCpp,
    StoredItemMoneyReconciliationLikeCpp, StoredItemMoneyRollbackKindLikeCpp,
    classify_stored_item_money_reconciliation_like_cpp,
    stored_item_money_zero_without_source_outcome_like_cpp,
};

use crate::{
    CharStatements, CharacterDatabase, DatabaseError, StatementDef, is_database_deadlock_like_cpp,
    persistence_trace::{
        CommitOutcome, ExplicitTransactionTrace, LogicalDatabase, TracedParam,
        record_batch_not_started,
    },
    retry_deadlocked_operation_like_cpp,
};

const STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP: [CharStatements; 4] = [
    CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
    CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,
    CharStatements::UPD_CHAR_MONEY,
    CharStatements::DEL_ITEMCONTAINER_MONEY,
];
const STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP: [CharStatements; 2] = [
    CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
    CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,
];

struct MariaDbStoredItemMoneyAttemptAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbStoredItemMoneyAttemptAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

fn durable_outcome_like_cpp(
    before: u64,
    notified_amount: u64,
    max_money: u64,
) -> StoredItemMoneyPersistenceOutcomeLikeCpp {
    let (after, applied_delta) = before
        .checked_add(notified_amount)
        .filter(|new_money| *new_money <= max_money)
        .map_or((before, 0), |new_money| (new_money, notified_amount));
    StoredItemMoneyPersistenceOutcomeLikeCpp {
        before,
        after,
        applied_delta,
        notified_amount,
    }
}

fn rolled_back_database_like_cpp(
    error: impl Into<DatabaseError>,
) -> StoredItemMoneyPersistenceAttemptLikeCpp {
    let error = error.into();
    StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
        kind: StoredItemMoneyRollbackKindLikeCpp::Database,
        retryable_deadlock: is_database_deadlock_like_cpp(&error),
        reason: error.to_string(),
    }
}

impl StoredItemMoneyPersistencePortLikeCpp for MariaDbStoredItemMoneyAttemptAdapterLikeCpp {
    fn attempt_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyPersistenceAttemptLikeCpp> {
        Box::pin(async move {
            let mut transaction = match self.character_db.pool().begin().await {
                Ok(transaction) => transaction,
                Err(error) => {
                    record_batch_not_started(LogicalDatabase::Character);
                    return rolled_back_database_like_cpp(error);
                }
            };
            let mut trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);

            trace.statement(|| {
                (
                    STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[0].trace_identity(),
                    vec![TracedParam::Uint {
                        value: request.player_guid,
                        width_bits: 64,
                    }],
                )
            });
            let before = match sqlx::query_scalar::<_, u64>(
                STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[0].sql(),
            )
            .bind(request.player_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(Some(money)) => money,
                Ok(None) => {
                    return StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                        kind: StoredItemMoneyRollbackKindLikeCpp::MissingPlayer,
                        reason: "stored-money character is missing".to_owned(),
                        retryable_deadlock: false,
                    };
                }
                Err(error) => return rolled_back_database_like_cpp(error),
            };

            trace.statement(|| {
                (
                    STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[1].trace_identity(),
                    vec![TracedParam::Uint {
                        value: request.item_guid,
                        width_bits: 64,
                    }],
                )
            });
            let source_money = match sqlx::query_scalar::<_, u64>(
                STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[1].sql(),
            )
            .bind(request.item_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(money) => money,
                Err(error) => return rolled_back_database_like_cpp(error),
            };

            let Some(notified_amount) = source_money else {
                if let Some(outcome) = stored_item_money_zero_without_source_outcome_like_cpp(
                    before,
                    request.cached_notified_amount,
                ) {
                    trace.rolled_back();
                    return match transaction.rollback().await {
                        Ok(()) => StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome),
                        Err(error) => rolled_back_database_like_cpp(error),
                    };
                }
                return StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                    kind: StoredItemMoneyRollbackKindLikeCpp::SourceAlreadyConsumed,
                    reason: "stored Item money source was already consumed".to_owned(),
                    retryable_deadlock: false,
                };
            };

            let outcome = durable_outcome_like_cpp(before, notified_amount, request.max_money);
            if outcome.applied_delta != 0 {
                trace.statement_expecting(
                    || {
                        (
                            STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[2].trace_identity(),
                            vec![
                                TracedParam::Uint {
                                    value: outcome.after,
                                    width_bits: 64,
                                },
                                TracedParam::Uint {
                                    value: request.player_guid,
                                    width_bits: 64,
                                },
                            ],
                        )
                    },
                    1,
                );
                let update = match sqlx::query(STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[2].sql())
                    .bind(outcome.after)
                    .bind(request.player_guid)
                    .execute(&mut *transaction)
                    .await
                {
                    Ok(update) => update,
                    Err(error) => return rolled_back_database_like_cpp(error),
                };
                if update.rows_affected() != 1 {
                    return rolled_back_database_like_cpp(DatabaseError::Transaction(format!(
                        "stored Item money update affected {} rows; expected exactly 1",
                        update.rows_affected()
                    )));
                }
            }

            trace.statement_expecting(
                || {
                    (
                        STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[3].trace_identity(),
                        vec![TracedParam::Uint {
                            value: request.item_guid,
                            width_bits: 64,
                        }],
                    )
                },
                STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
            );
            let delete = match sqlx::query(STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP[3].sql())
                .bind(request.item_guid)
                .execute(&mut *transaction)
                .await
            {
                Ok(delete) => delete,
                Err(error) => return rolled_back_database_like_cpp(error),
            };
            if delete.rows_affected() != STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP {
                return rolled_back_database_like_cpp(DatabaseError::Transaction(format!(
                    "stored Item money source delete affected {} rows; expected exactly 1",
                    delete.rows_affected()
                )));
            }

            trace.committing();
            match transaction.commit().await {
                Ok(()) => {
                    trace.committed(CommitOutcome::Committed);
                    StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome)
                }
                Err(error) => {
                    let error = DatabaseError::from(error);
                    if is_database_deadlock_like_cpp(&error) {
                        trace.committed(CommitOutcome::RolledBack);
                        rolled_back_database_like_cpp(error)
                    } else {
                        trace.committed(CommitOutcome::Unknown);
                        StoredItemMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                            reason: error.to_string(),
                            outcome,
                        }
                    }
                }
            }
        })
    }

    fn reconcile_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
        outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyReconciliationLikeCpp> {
        Box::pin(async move {
            let mut transaction = match self.character_db.pool().begin().await {
                Ok(transaction) => transaction,
                Err(error) => {
                    return StoredItemMoneyReconciliationLikeCpp::Indeterminate {
                        reason: Some(error.to_string()),
                    };
                }
            };
            let trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);

            trace.statement(|| {
                (
                    STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP[0].trace_identity(),
                    vec![TracedParam::Uint {
                        value: request.player_guid,
                        width_bits: 64,
                    }],
                )
            });
            let observed_money = match sqlx::query_scalar::<_, u64>(
                STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP[0].sql(),
            )
            .bind(request.player_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(Some(money)) => money,
                Ok(None) => {
                    return StoredItemMoneyReconciliationLikeCpp::Indeterminate {
                        reason: Some("stored-money character vanished".to_owned()),
                    };
                }
                Err(error) => {
                    return StoredItemMoneyReconciliationLikeCpp::Indeterminate {
                        reason: Some(error.to_string()),
                    };
                }
            };

            trace.statement(|| {
                (
                    STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP[1].trace_identity(),
                    vec![TracedParam::Uint {
                        value: request.item_guid,
                        width_bits: 64,
                    }],
                )
            });
            let observed_source_money = match sqlx::query_scalar::<_, u64>(
                STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP[1].sql(),
            )
            .bind(request.item_guid)
            .fetch_optional(&mut *transaction)
            .await
            {
                Ok(money) => money,
                Err(error) => {
                    return StoredItemMoneyReconciliationLikeCpp::Indeterminate {
                        reason: Some(error.to_string()),
                    };
                }
            };

            let classification = classify_stored_item_money_reconciliation_like_cpp(
                outcome,
                observed_money,
                observed_source_money,
            );

            trace.rolled_back();
            match transaction.rollback().await {
                Ok(()) => classification,
                Err(error) => StoredItemMoneyReconciliationLikeCpp::Indeterminate {
                    reason: Some(error.to_string()),
                },
            }
        })
    }
}

/// Concrete capability boundary. Deadlock retry remains inside the MariaDB
/// adapter so gameplay consumes one normalized persistence attempt and never
/// imports database retry machinery.
pub struct MariaDbStoredItemMoneyPersistenceAdapterLikeCpp {
    inner: MariaDbStoredItemMoneyAttemptAdapterLikeCpp,
}

impl MariaDbStoredItemMoneyPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self {
            inner: MariaDbStoredItemMoneyAttemptAdapterLikeCpp::new(character_db),
        }
    }
}

impl StoredItemMoneyPersistencePortLikeCpp for MariaDbStoredItemMoneyPersistenceAdapterLikeCpp {
    fn attempt_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyPersistenceAttemptLikeCpp> {
        Box::pin(async move {
            match retry_deadlocked_operation_like_cpp(
                || async {
                    match self.inner.attempt_stored_item_money_like_cpp(request).await {
                        StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome) => Ok(outcome),
                        other => Err(other),
                    }
                },
                |error| {
                    matches!(
                        error,
                        StoredItemMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                            retryable_deadlock: true,
                            ..
                        }
                    )
                },
            )
            .await
            {
                Ok(outcome) => StoredItemMoneyPersistenceAttemptLikeCpp::Applied(outcome),
                Err(error) => error,
            }
        })
    }

    fn reconcile_stored_item_money_like_cpp(
        &self,
        request: StoredItemMoneyPersistenceRequestLikeCpp,
        outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyReconciliationLikeCpp> {
        self.inner
            .reconcile_stored_item_money_like_cpp(request, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durable_outcome_preserves_the_all_or_nothing_money_cap_like_cpp() {
        assert_eq!(
            durable_outcome_like_cpp(100, 7, 1_000),
            StoredItemMoneyPersistenceOutcomeLikeCpp {
                before: 100,
                after: 107,
                applied_delta: 7,
                notified_amount: 7,
            }
        );
        assert_eq!(
            durable_outcome_like_cpp(999, 7, 1_000),
            StoredItemMoneyPersistenceOutcomeLikeCpp {
                before: 999,
                after: 999,
                applied_delta: 0,
                notified_amount: 7,
            }
        );
    }

    #[test]
    fn stored_item_money_statement_and_lock_order_stays_exact_like_cpp() {
        assert_eq!(
            STORED_ITEM_MONEY_ATTEMPT_ORDER_LIKE_CPP,
            [
                CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
                CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,
                CharStatements::UPD_CHAR_MONEY,
                CharStatements::DEL_ITEMCONTAINER_MONEY,
            ]
        );
        assert_eq!(
            STORED_ITEM_MONEY_RECONCILIATION_ORDER_LIKE_CPP,
            [
                CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
                CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE,
            ]
        );
    }
}
