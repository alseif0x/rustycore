use super::*;
use crate::params::SqlParam;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Transaction};

const INSERT: &str = "INSERT INTO loot_money_claim_receipt (claim_id, payouts) VALUES (?, '[]')";
const WRITE: &str = "UPDATE loot_money_claim_receipt SET payouts = ? WHERE claim_id = ?";
const READ: &str =
    "SELECT CAST(payouts AS BINARY) FROM loot_money_claim_receipt WHERE claim_id = ? FOR UPDATE";

#[derive(Serialize, Deserialize)]
struct Share {
    recipient: u64,
    before: u64,
    after: u64,
}

fn trace_id(id: &[u8; 16]) -> TracedParam {
    TracedParam::from_param(&SqlParam::Bytes(id.to_vec()))
}

/// The INSERT serializes concurrent claimants before either touches character balances.
pub(super) async fn reserve(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    id: &[u8; 16],
) -> Result<bool, DatabaseError> {
    trace.statement(|| (INSERT.into(), vec![trace_id(id)]));
    match sqlx::query(INSERT)
        .bind(id.as_slice())
        .execute(&mut **tx)
        .await
    {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            // The other transaction has finished and released the unique key.
            // Missing or damaged evidence cannot be interpreted as a settled grant.
            let existing = read(tx, trace, id).await?;
            if existing.is_some_and(|rows| !rows.is_empty()) {
                Ok(false)
            } else {
                Err(DatabaseError::Transaction(
                    "missing shared money receipt after key conflict".into(),
                ))
            }
        }
        Err(err) => Err(err.into()),
    }
}

pub(super) async fn record(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    id: &[u8; 16],
    outcomes: &[GroupLootMoneyPersistenceOutcomeLikeCpp],
) -> Result<(), DatabaseError> {
    let shares: Vec<_> = outcomes
        .iter()
        .map(|row| Share {
            recipient: row.recipient_guid,
            before: row.before,
            after: row.after,
        })
        .collect();
    let payload =
        serde_json::to_string(&shares).map_err(|e| DatabaseError::Transaction(e.to_string()))?;
    trace.statement_expecting(
        || {
            (
                WRITE.into(),
                vec![
                    TracedParam::from_param(&SqlParam::String(payload.clone())),
                    trace_id(id),
                ],
            )
        },
        1,
    );
    let updated = sqlx::query(WRITE)
        .bind(payload)
        .bind(id.as_slice())
        .execute(&mut **tx)
        .await?;
    if updated.rows_affected() != 1 {
        return Err(DatabaseError::Transaction(
            "shared money receipt disappeared before commit".into(),
        ));
    }
    Ok(())
}

pub(super) async fn read(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    id: &[u8; 16],
) -> Result<Option<Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>>, DatabaseError> {
    trace.statement(|| (READ.into(), vec![trace_id(id)]));
    let json: Option<Vec<u8>> = sqlx::query_scalar(READ)
        .bind(id.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    let Some(json) = json else { return Ok(None) };
    let shares: Vec<Share> =
        serde_json::from_slice(&json).map_err(|e| DatabaseError::Transaction(e.to_string()))?;
    let mut outcomes = Vec::with_capacity(shares.len());
    for share in shares {
        let applied_delta = share.after.checked_sub(share.before).ok_or_else(|| {
            DatabaseError::Transaction("invalid shared money receipt balances".into())
        })?;
        outcomes.push(GroupLootMoneyPersistenceOutcomeLikeCpp {
            recipient_guid: share.recipient,
            before: share.before,
            after: share.after,
            applied_delta,
        });
    }
    Ok(Some(outcomes))
}
