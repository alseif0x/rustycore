use super::*;
use crate::params::SqlParam;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Transaction};
use wow_persistence::LootMoneyCriteriaProgress;

const INSERT: &str = "INSERT INTO loot_money_claim_receipt (claim_id, payouts) VALUES (?, '[]')";
const WRITE: &str = "UPDATE loot_money_claim_receipt SET payouts = ? WHERE claim_id = ?";
const READ: &str =
    "SELECT CAST(payouts AS BINARY) FROM loot_money_claim_receipt WHERE claim_id = ? FOR UPDATE";

#[derive(Serialize, Deserialize)]
struct ShareCriteria {
    counter: u64,
    date: u64,
    new_achievements: Vec<u32>,
    /// Added after legacy receipts; all awards in a claim share this time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    achievement_date: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct Share {
    recipient: u64,
    before: u64,
    after: u64,
    /// Missing in receipts written before durable criteria support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    criteria: Option<ShareCriteria>,
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
            criteria: row.criteria.as_ref().map(|criteria| ShareCriteria {
                counter: criteria.counter,
                date: criteria.date,
                new_achievements: criteria.new_achievements.clone(),
                achievement_date: criteria.achievement_date,
            }),
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
            criteria: share.criteria.map(|criteria| LootMoneyCriteriaProgress {
                counter: criteria.counter,
                date: criteria.date,
                new_achievements: criteria.new_achievements,
                achievement_date: criteria.achievement_date,
            }),
        });
    }
    Ok(Some(outcomes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_receipts_without_criteria_decode_to_the_original_money_result() {
        let legacy = br#"[{"recipient":9,"before":100,"after":105}]"#;
        let rows: Vec<Share> = serde_json::from_slice(legacy).unwrap();
        assert_eq!(
            rows.into_iter()
                .map(|row| GroupLootMoneyPersistenceOutcomeLikeCpp {
                    recipient_guid: row.recipient,
                    before: row.before,
                    after: row.after,
                    applied_delta: row.after - row.before,
                    criteria: row.criteria.map(|criteria| LootMoneyCriteriaProgress {
                        counter: criteria.counter,
                        date: criteria.date,
                        new_achievements: criteria.new_achievements,
                        achievement_date: criteria.achievement_date,
                    }),
                })
                .collect::<Vec<_>>(),
            vec![GroupLootMoneyPersistenceOutcomeLikeCpp {
                recipient_guid: 9,
                before: 100,
                after: 105,
                applied_delta: 5,
                criteria: None,
            }]
        );
    }

    #[test]
    fn current_receipts_roundtrip_criteria_and_completion_date() {
        let outcome = GroupLootMoneyPersistenceOutcomeLikeCpp {
            recipient_guid: 9,
            before: 100,
            after: 100,
            applied_delta: 0,
            criteria: Some(LootMoneyCriteriaProgress {
                counter: 1_000_000,
                date: 1_700_000_000,
                new_achievements: vec![1176],
                achievement_date: Some(1_700_000_000),
            }),
        };
        let json = serde_json::to_vec(&[Share {
            recipient: outcome.recipient_guid,
            before: outcome.before,
            after: outcome.after,
            criteria: outcome.criteria.as_ref().map(|criteria| ShareCriteria {
                counter: criteria.counter,
                date: criteria.date,
                new_achievements: criteria.new_achievements.clone(),
                achievement_date: criteria.achievement_date,
            }),
        }])
        .unwrap();
        let rows: Vec<Share> = serde_json::from_slice(&json).unwrap();
        let criteria = rows[0].criteria.as_ref().unwrap();
        assert_eq!(criteria.counter, 1_000_000);
        assert_eq!(criteria.date, 1_700_000_000);
        assert_eq!(criteria.new_achievements, [1176]);
        assert_eq!(criteria.achievement_date, Some(1_700_000_000));
    }
}
