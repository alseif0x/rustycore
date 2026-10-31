//! MariaDB adapter for effective PlayerCondition and UnitCondition ID presence.

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use wow_persistence::{
    ConditionPresenceLoadOutcome, ConditionPresencePersistencePort, ConditionPresenceRows,
    PersistenceFutureLikeCpp,
};

use crate::field_like_cpp::{FieldNumberLikeCpp, NativeFieldLikeCpp};
use crate::{HotfixDatabase, PreparedStatement, SqlResult};

const PLAYER_CONDITION_IDS_SQL: &str =
    "SELECT ID FROM player_condition WHERE (`VerifiedBuild` > 0) = ?";
const UNIT_CONDITION_IDS_SQL: &str =
    "SELECT ID FROM unit_condition WHERE (`VerifiedBuild` > 0) = ?";

// DB2StorageBase::LoadFromDB calls Load(false) then Load(true), and
// DB2DatabaseLoader::Load binds !custom: official VerifiedBuild=true first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConditionTable {
    Player,
    Unit,
}

const QUERY_PLAN: [(ConditionTable, bool); 4] = [
    (ConditionTable::Player, true),
    (ConditionTable::Player, false),
    (ConditionTable::Unit, true),
    (ConditionTable::Unit, false),
];

fn query_sql(table: ConditionTable) -> &'static str {
    match table {
        ConditionTable::Player => PLAYER_CONDITION_IDS_SQL,
        ConditionTable::Unit => UNIT_CONDITION_IDS_SQL,
    }
}

fn query_statement(table: ConditionTable, official: bool) -> PreparedStatement {
    let mut statement = PreparedStatement::new(query_sql(table));
    statement.set_bool(0, official);
    statement
}

fn condition_id_from_columns(columns: Vec<NativeFieldLikeCpp>, field: &'static str) -> Result<u32> {
    match columns.as_slice() {
        [] => bail!("missing {field} SQL ID column"),
        [NativeFieldLikeCpp::Null] => bail!("{field} SQL ID column is NULL"),
        [
            integer @ (NativeFieldLikeCpp::Int8(_)
            | NativeFieldLikeCpp::UInt8(_)
            | NativeFieldLikeCpp::Int16(_)
            | NativeFieldLikeCpp::UInt16(_)
            | NativeFieldLikeCpp::Int32(_)
            | NativeFieldLikeCpp::UInt32(_)
            | NativeFieldLikeCpp::Int64(_)
            | NativeFieldLikeCpp::UInt64(_)),
        ] => <u32 as FieldNumberLikeCpp>::cast_like_cpp(integer).with_context(|| {
            format!("{field} SQL ID does not round-trip through the C++ uint32 conversion")
        }),
        [
            NativeFieldLikeCpp::Float(_)
            | NativeFieldLikeCpp::Double(_)
            | NativeFieldLikeCpp::Text(_)
            | NativeFieldLikeCpp::Unsupported(_),
        ] => {
            bail!("{field} SQL ID is not an integer column")
        }
        _ => bail!(
            "{field} query returned {} columns; expected only ID",
            columns.len()
        ),
    }
}

fn condition_id(result: &SqlResult, field: &'static str) -> Result<u32> {
    condition_id_from_columns(
        result
            .native_row_like_cpp()
            .with_context(|| format!("failed to decode {field} SQL row"))?,
        field,
    )
}

async fn load_rows(db: &HotfixDatabase) -> Result<ConditionPresenceRows> {
    let mut rows = ConditionPresenceRows::default();
    for (table, official) in QUERY_PLAN {
        let field = match table {
            ConditionTable::Player => "PlayerCondition.ID",
            ConditionTable::Unit => "UnitCondition.ID",
        };
        let statement = query_statement(table, official);
        let mut result = db
            .query(&statement)
            .await
            .with_context(|| format!("failed to query {field} VerifiedBuild={official}"))?;
        if result.is_empty() {
            continue;
        }
        loop {
            let id = condition_id(&result, field)?;
            match table {
                ConditionTable::Player => rows.player_condition_ids.push(id),
                ConditionTable::Unit => rows.unit_condition_ids.push(id),
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(rows)
}

pub struct MariaDbConditionPresencePersistenceAdapter {
    hotfix_db: Arc<HotfixDatabase>,
}

impl MariaDbConditionPresencePersistenceAdapter {
    pub fn new(hotfix_db: Arc<HotfixDatabase>) -> Self {
        Self { hotfix_db }
    }
}

impl ConditionPresencePersistencePort for MariaDbConditionPresencePersistenceAdapter {
    fn load_condition_presence_rows(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ConditionPresenceLoadOutcome> {
        Box::pin(async move {
            match load_rows(&self.hotfix_db).await {
                Ok(rows) => ConditionPresenceLoadOutcome::Loaded(rows),
                Err(error) => ConditionPresenceLoadOutcome::Failed {
                    reason: error.to_string(),
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "condition_presence_adapter/tests.rs"]
mod tests;
