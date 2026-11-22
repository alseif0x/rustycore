//! MariaDB adapter for the SQLx-free group corpse-loot money capability.

use std::sync::Arc;

mod receipt;
mod source;

use wow_persistence::{
    GroupLootMoneyPersistenceAttemptLikeCpp, GroupLootMoneyPersistenceOutcomeLikeCpp,
    GroupLootMoneyPersistencePortLikeCpp, GroupLootMoneyPersistenceRequestLikeCpp,
    GroupLootMoneyReconciliationLikeCpp, GroupLootMoneyRollbackKindLikeCpp,
    PersistenceFutureLikeCpp, WorldLootMoneyBinding, WorldLootMoneySource,
    classify_group_loot_money_reconciliation_like_cpp,
};

use crate::{
    CharStatements, CharacterDatabase, DatabaseError, StatementDef, is_database_deadlock_like_cpp,
    persistence_trace::{
        CommitOutcome, ExplicitTransactionTrace, LogicalDatabase, TracedParam,
        record_batch_not_started,
    },
    retry_deadlocked_operation_like_cpp,
};

/// Persist a canonical chest restock deadline after the map lock has been
/// released. A newer head proves the captured generation already retired.
/// Missing or contradictory evidence keeps the map waiting for confirmation.
pub(crate) async fn arm_world_chest_money(
    character_db: &CharacterDatabase,
    source: WorldLootMoneySource,
    generation: u64,
    restock_deadline: i64,
    persist_respawn: bool,
) -> Result<bool, DatabaseError> {
    let mut transaction = character_db.pool().begin().await?;
    let mut trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
    match source::arm_restock_deadline(
        &mut transaction,
        &trace,
        source,
        generation,
        restock_deadline,
        persist_respawn,
    )
    .await
    {
        Ok(true) => {
            trace.committing();
            match transaction.commit().await {
                Ok(()) => {
                    trace.committed(CommitOutcome::Committed);
                    Ok(true)
                }
                Err(error) => {
                    trace.committed(CommitOutcome::Unknown);
                    Err(error.into())
                }
            }
        }
        Ok(false) => {
            transaction.rollback().await?;
            trace.rolled_back();
            Ok(false)
        }
        Err(error) => {
            let rollback = transaction.rollback().await;
            trace.rolled_back();
            rollback?;
            Err(error)
        }
    }
}

async fn bind_world_chest_money(
    character_db: &CharacterDatabase,
    source: WorldLootMoneySource,
    personal_owner: u64,
) -> Result<Option<WorldLootMoneyBinding>, DatabaseError> {
    let mut transaction = character_db.pool().begin().await?;
    let mut trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
    match source::resolve_binding(&mut transaction, &trace, source, personal_owner).await {
        Ok(Some(binding)) => {
            trace.committing();
            match transaction.commit().await {
                Ok(()) => {
                    trace.committed(CommitOutcome::Committed);
                    Ok(Some(binding))
                }
                Err(error) => {
                    trace.committed(CommitOutcome::Unknown);
                    Err(error.into())
                }
            }
        }
        Ok(None) => {
            transaction.rollback().await?;
            trace.rolled_back();
            Ok(None)
        }
        Err(error) => {
            let rollback = transaction.rollback().await;
            trace.rolled_back();
            rollback?;
            Err(error)
        }
    }
}

const GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP: [CharStatements; 2] = [
    CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
    CharStatements::UPD_CHAR_MONEY,
];

fn payouts_in_stable_lock_order_like_cpp(
    mut payouts: Vec<wow_persistence::GroupLootMoneyPayoutLikeCpp>,
) -> Vec<wow_persistence::GroupLootMoneyPayoutLikeCpp> {
    payouts.sort_unstable_by_key(|payout| payout.recipient_guid);
    payouts
}

struct MariaDbGroupLootMoneyAttemptAdapterLikeCpp {
    character_db: Arc<CharacterDatabase>,
}

impl MariaDbGroupLootMoneyAttemptAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self { character_db }
    }
}

fn durable_outcome_like_cpp(
    recipient_guid: u64,
    before: u64,
    requested_delta: u64,
    max_money: u64,
) -> GroupLootMoneyPersistenceOutcomeLikeCpp {
    let (after, applied_delta) = before
        .checked_add(requested_delta)
        .filter(|new_money| *new_money <= max_money)
        .map_or((before, 0), |new_money| (new_money, requested_delta));
    GroupLootMoneyPersistenceOutcomeLikeCpp {
        recipient_guid,
        before,
        after,
        applied_delta,
    }
}

fn rolled_back_database_like_cpp(
    error: impl Into<DatabaseError>,
) -> GroupLootMoneyPersistenceAttemptLikeCpp {
    let error = error.into();
    GroupLootMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
        kind: GroupLootMoneyRollbackKindLikeCpp::Database,
        retryable_deadlock: is_database_deadlock_like_cpp(&error),
        reason: error.to_string(),
    }
}

impl GroupLootMoneyPersistencePortLikeCpp for MariaDbGroupLootMoneyAttemptAdapterLikeCpp {
    fn attempt_group_loot_money_like_cpp(
        &self,
        request: GroupLootMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp> {
        Box::pin(async move {
            let payouts = payouts_in_stable_lock_order_like_cpp(request.payouts);
            if payouts.is_empty()
                || (request.claim_id.is_none() && payouts.iter().any(|p| p.requested_delta != 0))
            {
                return rolled_back_database_like_cpp(DatabaseError::Transaction(
                    "invalid shared money claim".into(),
                ));
            }
            let mut transaction = match self.character_db.pool().begin().await {
                Ok(transaction) => transaction,
                Err(error) => {
                    record_batch_not_started(LogicalDatabase::Character);
                    return rolled_back_database_like_cpp(error);
                }
            };
            let mut trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
            let mut outcomes = Vec::with_capacity(payouts.len());
            if let Some(id) = request.claim_id {
                match source::begin_payout(&mut transaction, &trace, &id).await {
                    Ok(None) | Ok(Some(true)) => {}
                    Ok(Some(false)) => {
                        return GroupLootMoneyPersistenceAttemptLikeCpp::AlreadySettled;
                    }
                    Err(error) => return rolled_back_database_like_cpp(error),
                }
                match receipt::reserve(&mut transaction, &trace, &id).await {
                    Ok(true) => {}
                    Ok(false) => return GroupLootMoneyPersistenceAttemptLikeCpp::AlreadySettled,
                    Err(error) => return rolled_back_database_like_cpp(error),
                }
            }

            for payout in payouts {
                trace.statement(|| {
                    (
                        GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP[0].trace_identity(),
                        vec![TracedParam::Uint {
                            value: payout.recipient_guid,
                            width_bits: 64,
                        }],
                    )
                });
                let before = match sqlx::query_scalar::<_, u64>(
                    GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP[0].sql(),
                )
                .bind(payout.recipient_guid)
                .fetch_optional(&mut *transaction)
                .await
                {
                    Ok(Some(money)) => money,
                    Ok(None) => {
                        return GroupLootMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                            kind: GroupLootMoneyRollbackKindLikeCpp::MissingPlayer {
                                recipient_guid: payout.recipient_guid,
                            },
                            reason: "group loot-money recipient is missing".to_owned(),
                            retryable_deadlock: false,
                        };
                    }
                    Err(error) => return rolled_back_database_like_cpp(error),
                };

                let outcome = durable_outcome_like_cpp(
                    payout.recipient_guid,
                    before,
                    payout.requested_delta,
                    request.max_money,
                );
                if outcome.applied_delta != 0 {
                    trace.statement_expecting(
                        || {
                            (
                                GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP[1].trace_identity(),
                                vec![
                                    TracedParam::Uint {
                                        value: outcome.after,
                                        width_bits: 64,
                                    },
                                    TracedParam::Uint {
                                        value: payout.recipient_guid,
                                        width_bits: 64,
                                    },
                                ],
                            )
                        },
                        1,
                    );
                    let update = match sqlx::query(GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP[1].sql())
                        .bind(outcome.after)
                        .bind(payout.recipient_guid)
                        .execute(&mut *transaction)
                        .await
                    {
                        Ok(update) => update,
                        Err(error) => return rolled_back_database_like_cpp(error),
                    };
                    if update.rows_affected() != 1 {
                        return rolled_back_database_like_cpp(DatabaseError::Transaction(format!(
                            "loot-money update for character {} affected {} rows; expected exactly 1",
                            payout.recipient_guid,
                            update.rows_affected()
                        )));
                    }
                }
                outcomes.push(outcome);
            }

            if let Some(id) = request.claim_id
                && let Err(error) = receipt::record(&mut transaction, &trace, &id, &outcomes).await
            {
                return rolled_back_database_like_cpp(error);
            }
            trace.committing();
            match transaction.commit().await {
                Ok(()) => {
                    trace.committed(CommitOutcome::Committed);
                    GroupLootMoneyPersistenceAttemptLikeCpp::Applied(outcomes)
                }
                Err(error) => {
                    let error = DatabaseError::from(error);
                    if is_database_deadlock_like_cpp(&error) {
                        trace.committed(CommitOutcome::RolledBack);
                        rolled_back_database_like_cpp(error)
                    } else {
                        trace.committed(CommitOutcome::Unknown);
                        GroupLootMoneyPersistenceAttemptLikeCpp::CommitOutcomeUnknown {
                            reason: error.to_string(),
                            outcomes,
                        }
                    }
                }
            }
        })
    }

    fn reconcile_group_loot_money_like_cpp(
        &self,
        claim_id: Option<[u8; 16]>,
        outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp> {
        Box::pin(async move {
            let Some(id) = claim_id else {
                return if !outcomes.is_empty()
                    && outcomes
                        .iter()
                        .all(|row| row.before == row.after && row.applied_delta == 0)
                {
                    GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop
                } else {
                    GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None }
                };
            };
            let result: Result<_, DatabaseError> = async {
                let mut tx = self.character_db.pool().begin().await?;
                let trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
                let receipt = receipt::read(&mut tx, &trace, &id).await?;
                tx.rollback().await?;
                trace.rolled_back();
                Ok(receipt)
            }
            .await;
            match result {
                Ok(receipt) => {
                    classify_group_loot_money_reconciliation_like_cpp(&outcomes, receipt.as_deref())
                }
                Err(error) => GroupLootMoneyReconciliationLikeCpp::Indeterminate {
                    reason: Some(error.to_string()),
                },
            }
        })
    }
}

/// Concrete capability boundary. The MariaDB adapter owns its process-wide
/// deadlock retry; gameplay receives only the normalized terminal attempt.
pub struct MariaDbGroupLootMoneyPersistenceAdapterLikeCpp {
    inner: MariaDbGroupLootMoneyAttemptAdapterLikeCpp,
}

impl MariaDbGroupLootMoneyPersistenceAdapterLikeCpp {
    pub fn new(character_db: Arc<CharacterDatabase>) -> Self {
        Self {
            inner: MariaDbGroupLootMoneyAttemptAdapterLikeCpp::new(character_db),
        }
    }
}

impl GroupLootMoneyPersistencePortLikeCpp for MariaDbGroupLootMoneyPersistenceAdapterLikeCpp {
    fn bind_world_chest_money(
        &self,
        source: WorldLootMoneySource,
        personal_owner: u64,
    ) -> PersistenceFutureLikeCpp<'_, Result<Option<WorldLootMoneyBinding>, String>> {
        let character_db = Arc::clone(&self.inner.character_db);
        Box::pin(async move {
            bind_world_chest_money(&character_db, source, personal_owner)
                .await
                .map_err(|error| error.to_string())
        })
    }

    fn attempt_group_loot_money_like_cpp(
        &self,
        request: GroupLootMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp> {
        Box::pin(async move {
            match retry_deadlocked_operation_like_cpp(
                || async {
                    match self
                        .inner
                        .attempt_group_loot_money_like_cpp(request.clone())
                        .await
                    {
                        GroupLootMoneyPersistenceAttemptLikeCpp::Applied(outcomes) => Ok(outcomes),
                        other => Err(other),
                    }
                },
                |error| {
                    matches!(
                        error,
                        GroupLootMoneyPersistenceAttemptLikeCpp::DefinitelyRolledBack {
                            retryable_deadlock: true,
                            ..
                        }
                    )
                },
            )
            .await
            {
                Ok(outcomes) => GroupLootMoneyPersistenceAttemptLikeCpp::Applied(outcomes),
                Err(error) => error,
            }
        })
    }

    fn reconcile_group_loot_money_like_cpp(
        &self,
        claim_id: Option<[u8; 16]>,
        outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp> {
        self.inner
            .reconcile_group_loot_money_like_cpp(claim_id, outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_loot_money_cap_is_all_or_nothing_like_cpp() {
        assert_eq!(
            durable_outcome_like_cpp(7, 100, 5, 1_000),
            GroupLootMoneyPersistenceOutcomeLikeCpp {
                recipient_guid: 7,
                before: 100,
                after: 105,
                applied_delta: 5,
            }
        );
        assert_eq!(durable_outcome_like_cpp(7, 999, 5, 1_000).applied_delta, 0);
    }

    #[test]
    fn group_loot_money_statement_order_stays_exact_like_cpp() {
        assert_eq!(
            GROUP_LOOT_MONEY_ATTEMPT_ORDER_LIKE_CPP,
            [
                CharStatements::SEL_CHAR_MONEY_FOR_UPDATE,
                CharStatements::UPD_CHAR_MONEY,
            ]
        );
    }

    #[test]
    fn group_loot_money_recipient_locks_use_stable_guid_order_like_cpp() {
        let payouts = payouts_in_stable_lock_order_like_cpp(vec![
            wow_persistence::GroupLootMoneyPayoutLikeCpp {
                recipient_guid: 9,
                requested_delta: 2,
            },
            wow_persistence::GroupLootMoneyPayoutLikeCpp {
                recipient_guid: 3,
                requested_delta: 4,
            },
        ]);
        assert_eq!(
            payouts
                .into_iter()
                .map(|payout| payout.recipient_guid)
                .collect::<Vec<_>>(),
            vec![3, 9]
        );
    }
}
