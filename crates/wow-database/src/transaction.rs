//! SQL transaction support.

use crate::error::DatabaseError;
use crate::params::{PreparedStatement, SqlParam};
use sqlx::MySqlPool;

/// A batch of SQL statements to be executed atomically within a transaction.
///
/// Matches the C# `SQLTransaction` pattern: collect statements, then commit
/// them all at once.
#[derive(Debug, Default)]
pub struct SqlTransaction {
    statements: Vec<PreparedStatement>,
}

impl SqlTransaction {
    /// Create a new empty transaction batch.
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    /// Append a prepared statement to this transaction.
    pub fn append(&mut self, stmt: PreparedStatement) {
        self.statements.push(stmt);
    }

    /// Number of statements in this transaction.
    pub fn len(&self) -> usize {
        self.statements.len()
    }

    /// Returns `true` if no statements have been appended.
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    /// Commit all statements atomically.
    ///
    /// On failure, all changes are rolled back. Retries up to 5 times on
    /// deadlock (MySQL error 1213), matching the C# `TransactionTask` behavior.
    pub async fn commit(self, pool: &MySqlPool) -> Result<(), DatabaseError> {
        if self.statements.is_empty() {
            return Ok(());
        }

        let result = self.try_commit(pool).await;

        match &result {
            Err(DatabaseError::Query(sqlx::Error::Database(db_err)))
                if db_err.code().as_deref() == Some("1213") =>
            {
                // Deadlock: retry up to 5 times
                for _ in 0..5 {
                    let retry = self.try_commit_inner(pool).await;
                    if retry.is_ok() {
                        return retry;
                    }
                }
                result
            }
            _ => result,
        }
    }

    async fn try_commit(&self, pool: &MySqlPool) -> Result<(), DatabaseError> {
        self.try_commit_inner(pool).await
    }

    async fn try_commit_inner(&self, pool: &MySqlPool) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;

        for stmt in &self.statements {
            let mut query = sqlx::query(stmt.sql());
            for param in stmt.params() {
                query = bind_param(query, param);
            }
            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

/// Bind a single [`SqlParam`] to a sqlx query.
pub(crate) fn bind_param<'q>(
    query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
    param: &'q SqlParam,
) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match param {
        SqlParam::Null => query.bind(Option::<String>::None),
        SqlParam::Bool(v) => query.bind(*v),
        SqlParam::I8(v) => query.bind(*v),
        SqlParam::U8(v) => query.bind(*v),
        SqlParam::I16(v) => query.bind(*v),
        SqlParam::U16(v) => query.bind(*v),
        SqlParam::I32(v) => query.bind(*v),
        SqlParam::U32(v) => query.bind(*v),
        SqlParam::I64(v) => query.bind(*v),
        SqlParam::U64(v) => query.bind(*v),
        SqlParam::F32(v) => query.bind(*v),
        SqlParam::F64(v) => query.bind(*v),
        SqlParam::String(v) => query.bind(v.as_str()),
        SqlParam::Bytes(v) => query.bind(v.as_slice()),
    }
}
