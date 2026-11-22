//! Durable binding between one persistent world-loot source lifetime and its
//! opaque money-claim receipt id.
//!
//! The key is the source's durable world identity (realm, map, instance and
//! database spawn id). Runtime object GUIDs and local loot generations never
//! enter this module.

use super::*;
use crate::params::SqlParam;
use sqlx::{MySql, Row, Transaction};

const INSERT_HEAD: &str = "INSERT IGNORE INTO loot_money_source_head (realm_id, map_id, instance_id, spawn_id, generation, restock_deadline) VALUES (?, ?, ?, ?, 0, NULL)";
const READ_HEAD_FOR_UPDATE: &str = "SELECT generation, restock_deadline, CAST(UNIX_TIMESTAMP() AS SIGNED) AS database_now FROM loot_money_source_head WHERE realm_id = ? AND map_id = ? AND instance_id = ? AND spawn_id = ? FOR UPDATE";
const ADVANCE_DUE_HEAD: &str = "UPDATE loot_money_source_head SET generation = ?, restock_deadline = NULL WHERE realm_id = ? AND map_id = ? AND instance_id = ? AND spawn_id = ? AND generation = ? AND restock_deadline = ? AND restock_deadline <= CAST(UNIX_TIMESTAMP() AS SIGNED)";
const INSERT_BINDING: &str = "INSERT INTO loot_money_source_binding (realm_id, map_id, instance_id, spawn_id, generation, scope_type, scope_owner, claim_id, spent) VALUES (?, ?, ?, ?, ?, ?, ?, UNHEX(REPLACE(UUID(), '-', '')), 0)";
const READ_BINDING_FOR_UPDATE: &str = "SELECT claim_id, spent FROM loot_money_source_binding WHERE realm_id = ? AND map_id = ? AND instance_id = ? AND spawn_id = ? AND generation = ? AND scope_type = ? AND scope_owner = ? FOR UPDATE";
const DISCOVER_BINDING_BY_CLAIM: &str = "SELECT realm_id, map_id, instance_id, spawn_id, generation, scope_type, scope_owner FROM loot_money_source_binding WHERE claim_id = ?";
const MARK_BINDING_SPENT: &str = "UPDATE loot_money_source_binding SET spent = 1 WHERE realm_id = ? AND map_id = ? AND instance_id = ? AND spawn_id = ? AND generation = ? AND scope_type = ? AND scope_owner = ? AND claim_id = ? AND spent = 0";
const SET_RESTOCK_DEADLINE: &str = "UPDATE loot_money_source_head SET restock_deadline = ? WHERE realm_id = ? AND map_id = ? AND instance_id = ? AND spawn_id = ? AND generation = ? AND restock_deadline IS NULL";

fn valid_source(source: WorldLootMoneySource) -> bool {
    source.realm_id != 0 && source.spawn_id != 0
}

fn valid_scope(scope_type: u8, scope_owner: u64) -> bool {
    (scope_type == 0 && scope_owner == 0) || (scope_type == 1 && scope_owner != 0)
}

fn source_params(source: WorldLootMoneySource) -> Vec<TracedParam> {
    vec![
        TracedParam::Uint {
            value: u64::from(source.realm_id),
            width_bits: 16,
        },
        TracedParam::Uint {
            value: u64::from(source.map_id),
            width_bits: 32,
        },
        TracedParam::Uint {
            value: u64::from(source.instance_id),
            width_bits: 32,
        },
        TracedParam::Uint {
            value: source.spawn_id,
            width_bits: 64,
        },
    ]
}

fn binding_params(
    source: WorldLootMoneySource,
    generation: u64,
    scope_type: u8,
    scope_owner: u64,
) -> Vec<TracedParam> {
    let mut params = source_params(source);
    params.extend([
        TracedParam::Uint {
            value: generation,
            width_bits: 64,
        },
        TracedParam::Uint {
            value: u64::from(scope_type),
            width_bits: 8,
        },
        TracedParam::Uint {
            value: scope_owner,
            width_bits: 64,
        },
    ]);
    params
}

fn record_sql_trace(
    trace: &ExplicitTransactionTrace,
    statement: &'static str,
    params: Vec<TracedParam>,
) {
    trace.statement(|| (statement.into(), params));
}

fn source_error(reason: &'static str) -> DatabaseError {
    DatabaseError::Transaction(reason.into())
}

async fn ensure_head(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    source: WorldLootMoneySource,
) -> Result<(), DatabaseError> {
    record_sql_trace(trace, INSERT_HEAD, source_params(source));
    sqlx::query(INSERT_HEAD)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn read_head_for_update(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    source: WorldLootMoneySource,
) -> Result<Option<(u64, Option<i64>, i64)>, DatabaseError> {
    record_sql_trace(trace, READ_HEAD_FOR_UPDATE, source_params(source));
    let row = sqlx::query(READ_HEAD_FOR_UPDATE)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| {
        Ok((
            row.try_get("generation")?,
            row.try_get("restock_deadline")?,
            row.try_get("database_now")?,
        ))
    })
    .transpose()
}

async fn resolve_due_generation(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    source: WorldLootMoneySource,
) -> Result<Option<u64>, DatabaseError> {
    let Some((generation, deadline, database_now)) =
        read_head_for_update(tx, trace, source).await?
    else {
        return Err(source_error(
            "world-loot source head disappeared while locked",
        ));
    };
    let Some(deadline) = deadline else {
        return Ok(Some(generation));
    };
    if deadline > database_now {
        return Ok(None);
    };
    let next_generation = generation
        .checked_add(1)
        .ok_or_else(|| source_error("world-loot source generation exhausted"))?;
    let mut params = vec![TracedParam::Uint {
        value: next_generation,
        width_bits: 64,
    }];
    params.extend(source_params(source));
    params.extend([
        TracedParam::Uint {
            value: generation,
            width_bits: 64,
        },
        TracedParam::from_param(&SqlParam::I64(deadline)),
    ]);
    record_sql_trace(trace, ADVANCE_DUE_HEAD, params);
    let advanced = sqlx::query(ADVANCE_DUE_HEAD)
        .bind(next_generation)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(deadline)
        .execute(&mut **tx)
        .await?;
    if advanced.rows_affected() != 1 {
        return Err(source_error(
            "world-loot source deadline CAS was stale or ambiguous",
        ));
    }
    Ok(Some(next_generation))
}

/// Resolve the durable source generation, advance a due restock exactly once,
/// and return or create the immutable scoped claim-id binding. The database
/// creates an opaque id only inside the inserted row; no binding is returned
/// unless its transaction commits.
pub(super) async fn resolve_binding(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    source: WorldLootMoneySource,
    personal_owner: u64,
) -> Result<Option<WorldLootMoneyBinding>, DatabaseError> {
    let scope_type = u8::from(personal_owner != 0);
    let scope_owner = personal_owner;
    if !valid_source(source) || !valid_scope(scope_type, scope_owner) {
        return Err(source_error("invalid durable world-loot source identity"));
    }
    ensure_head(tx, trace, source).await?;
    let Some(generation) = resolve_due_generation(tx, trace, source).await? else {
        return Ok(None);
    };
    let params = binding_params(source, generation, scope_type, scope_owner);
    record_sql_trace(trace, READ_BINDING_FOR_UPDATE, params);
    if let Some(existing) = sqlx::query(READ_BINDING_FOR_UPDATE)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(scope_type)
        .bind(scope_owner)
        .fetch_optional(&mut **tx)
        .await?
    {
        return read_binding(existing, generation).map(Some);
    }

    let params = binding_params(source, generation, scope_type, scope_owner);
    record_sql_trace(trace, INSERT_BINDING, params);
    match sqlx::query(INSERT_BINDING)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(scope_type)
        .bind(scope_owner)
        .execute(&mut **tx)
        .await
    {
        Ok(inserted) if inserted.rows_affected() == 1 => {}
        Ok(_) => {
            return Err(source_error(
                "world-loot source binding insert was ambiguous",
            ));
        }
        Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {}
        Err(error) => return Err(error.into()),
    }

    let params = binding_params(source, generation, scope_type, scope_owner);
    record_sql_trace(trace, READ_BINDING_FOR_UPDATE, params);
    let existing = sqlx::query(READ_BINDING_FOR_UPDATE)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(scope_type)
        .bind(scope_owner)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(existing) = existing else {
        return Err(source_error(
            "world-loot binding key conflicted without a source row",
        ));
    };
    read_binding(existing, generation).map(Some)
}

fn read_binding(
    existing: sqlx::mysql::MySqlRow,
    generation: u64,
) -> Result<WorldLootMoneyBinding, DatabaseError> {
    let claim_id: Vec<u8> = existing.try_get("claim_id")?;
    let claim_id: [u8; 16] = claim_id
        .try_into()
        .map_err(|_| source_error("world-loot source binding has an invalid claim id"))?;
    let spent = match existing.try_get::<u8, _>("spent")? {
        0 => false,
        1 => true,
        _ => {
            return Err(source_error(
                "world-loot source binding has invalid spent state",
            ));
        }
    };
    Ok(WorldLootMoneyBinding {
        generation,
        claim_id,
        spent,
    })
}

/// Fence one payout against its current source head and immutable scoped
/// binding. The claim-id lookup is deliberately nonlocking: after discovering
/// the source, this locks the head first and then re-reads the binding for
/// update, preserving one lock order for every source lifetime. `None` means
/// this is not a bound world source; `Some(false)` is a fail-closed world
/// binding rejection.
pub(super) async fn begin_payout(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    claim_id: &[u8; 16],
) -> Result<Option<bool>, DatabaseError> {
    record_sql_trace(
        trace,
        DISCOVER_BINDING_BY_CLAIM,
        vec![TracedParam::from_param(&SqlParam::Bytes(claim_id.to_vec()))],
    );
    let binding = sqlx::query(DISCOVER_BINDING_BY_CLAIM)
        .bind(claim_id.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
    let Some(binding) = binding else {
        return Ok(None);
    };
    let source = WorldLootMoneySource {
        realm_id: binding.try_get("realm_id")?,
        map_id: binding.try_get("map_id")?,
        instance_id: binding.try_get("instance_id")?,
        spawn_id: binding.try_get("spawn_id")?,
    };
    let generation: u64 = binding.try_get("generation")?;
    let scope_type: u8 = binding.try_get("scope_type")?;
    let scope_owner: u64 = binding.try_get("scope_owner")?;
    if !valid_source(source) || !valid_scope(scope_type, scope_owner) {
        return Ok(Some(false));
    }

    // The discovery query above is nonlocking. Every transaction that mutates
    // a source then locks the head before the scoped binding row.
    let Some((current_generation, restock_deadline, _)) =
        read_head_for_update(tx, trace, source).await?
    else {
        return Ok(Some(false));
    };
    if current_generation != generation || restock_deadline.is_some() {
        return Ok(Some(false));
    }

    let params = binding_params(source, generation, scope_type, scope_owner);
    record_sql_trace(trace, READ_BINDING_FOR_UPDATE, params);
    let existing = sqlx::query(READ_BINDING_FOR_UPDATE)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(scope_type)
        .bind(scope_owner)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(existing) = existing else {
        return Ok(Some(false));
    };
    let stored_id: Vec<u8> = existing.try_get("claim_id")?;
    let spent: u8 = existing.try_get("spent")?;
    if stored_id.as_slice() != claim_id || spent != 0 {
        return Ok(Some(false));
    }

    let mut params = binding_params(source, generation, scope_type, scope_owner);
    params.push(TracedParam::from_param(&SqlParam::Bytes(claim_id.to_vec())));
    record_sql_trace(trace, MARK_BINDING_SPENT, params);
    let updated = sqlx::query(MARK_BINDING_SPENT)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(generation)
        .bind(scope_type)
        .bind(scope_owner)
        .bind(claim_id.as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(Some(updated.rows_affected() == 1))
}

/// Arm a restock deadline only for the source generation observed by the
/// caller. A repeated deadline is idempotent. An older generation has already
/// retired. Reject contradictory evidence before publishing the map transition.
pub(super) async fn arm_restock_deadline(
    tx: &mut Transaction<'_, MySql>,
    trace: &ExplicitTransactionTrace,
    source: WorldLootMoneySource,
    observed_generation: u64,
    restock_deadline: i64,
    persist_respawn: bool,
) -> Result<bool, DatabaseError> {
    if !valid_source(source) || restock_deadline <= 0 {
        return Err(source_error("invalid chest restock identity or deadline"));
    }
    let Some((generation, current_deadline, _)) = read_head_for_update(tx, trace, source).await?
    else {
        return Err(source_error("missing source head for chest restock"));
    };
    if generation > observed_generation {
        return Ok(false);
    }
    if generation < observed_generation {
        return Err(source_error(
            "chest restock ahead of durable source generation",
        ));
    }
    match current_deadline {
        Some(current) if current == restock_deadline => return Ok(true),
        Some(_) => {
            return Err(source_error(
                "conflicting deadline for this chest generation",
            ));
        }
        None => {}
    }

    let mut params = vec![TracedParam::from_param(&SqlParam::I64(restock_deadline))];
    params.extend(source_params(source));
    params.push(TracedParam::Uint {
        value: observed_generation,
        width_bits: 64,
    });
    record_sql_trace(trace, SET_RESTOCK_DEADLINE, params);
    let updated = sqlx::query(SET_RESTOCK_DEADLINE)
        .bind(restock_deadline)
        .bind(source.realm_id)
        .bind(source.map_id)
        .bind(source.instance_id)
        .bind(source.spawn_id)
        .bind(observed_generation)
        .execute(&mut **tx)
        .await?;
    if updated.rows_affected() != 1 {
        return Err(source_error(
            "chest generation changed before recording restock",
        ));
    }
    if persist_respawn {
        use crate::{CharStatements, StatementDef};
        let map_id = u16::try_from(source.map_id)
            .map_err(|_| source_error("chest map exceeds the respawn table map id"))?;
        let save = CharStatements::REP_RESPAWN;
        trace.statement(|| {
            (
                save.trace_identity(),
                vec![
                    TracedParam::from_param(&SqlParam::U16(1)),
                    TracedParam::from_param(&SqlParam::U64(source.spawn_id)),
                    TracedParam::from_param(&SqlParam::I64(restock_deadline)),
                    TracedParam::from_param(&SqlParam::U16(map_id)),
                    TracedParam::from_param(&SqlParam::U32(source.instance_id)),
                ],
            )
        });
        sqlx::query(save.sql())
            .bind(1u16)
            .bind(source.spawn_id)
            .bind(restock_deadline)
            .bind(map_id)
            .bind(source.instance_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(true)
}
