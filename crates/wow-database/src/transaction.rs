//! SQL transaction support.

use crate::error::DatabaseError;
use crate::params::{PreparedStatement, SqlParam};
use sqlx::MySqlPool;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const DEADLOCK_MAX_RETRY_TIME_LIKE_CPP: Duration = Duration::from_secs(60);

static DEADLOCK_RETRY_LOCK_LIKE_CPP: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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
    /// On failure, all changes are rolled back. Deadlock retries are serialized
    /// under a single process-wide lock for up to 60 seconds, mirroring
    /// TrinityCore's `TransactionTask::_deadlockLock` and
    /// `DEADLOCK_MAX_RETRY_TIME_MS`.
    pub async fn commit(self, pool: &MySqlPool) -> Result<(), DatabaseError> {
        if self.statements.is_empty() {
            return Ok(());
        }

        let result = self.try_commit(pool).await;

        if !is_deadlock_like_cpp(&result) {
            return result;
        }

        let _deadlock_guard = DEADLOCK_RETRY_LOCK_LIKE_CPP.lock().await;
        let start = Instant::now();

        loop {
            if start.elapsed() > DEADLOCK_MAX_RETRY_TIME_LIKE_CPP {
                tracing::error!(
                    target: "sql.sql",
                    "Fatal deadlocked SQL Transaction, it will not be retried anymore"
                );
                return result;
            }

            let retry = self.try_commit_inner(pool).await;
            if retry.is_ok() {
                return retry;
            }

            tracing::warn!(
                target: "sql.sql",
                loop_timer_ms = start.elapsed().as_millis(),
                "Deadlocked SQL Transaction, retrying"
            );
        }
    }

    #[cfg(test)]
    pub(crate) async fn with_deadlock_retry_lock_for_test<F, T>(future: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let _deadlock_guard = DEADLOCK_RETRY_LOCK_LIKE_CPP.lock().await;
        future.await
    }

    #[cfg(test)]
    pub(crate) async fn deadlock_retry_lock_probe_for_test() -> bool {
        DEADLOCK_RETRY_LOCK_LIKE_CPP.try_lock().is_ok()
    }

    #[cfg(test)]
    pub(crate) fn deadlock_max_retry_time_like_cpp_for_test() -> Duration {
        DEADLOCK_MAX_RETRY_TIME_LIKE_CPP
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

fn is_deadlock_like_cpp(result: &Result<(), DatabaseError>) -> bool {
    match result {
        Err(DatabaseError::Query(sqlx::Error::Database(db_err))) => {
            db_err.code().as_deref() == Some("1213")
                || db_err.message().contains("Deadlock")
                || db_err.message().contains("deadlock")
        }
        _ => false,
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

#[cfg(test)]
mod tests {
    use super::SqlTransaction;
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[tokio::test]
    async fn deadlock_retry_lock_is_process_wide_like_cpp() {
        let (locked_tx, locked_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();

        let holder = tokio::spawn(async move {
            SqlTransaction::with_deadlock_retry_lock_for_test(async move {
                locked_tx.send(()).unwrap();
                release_rx.await.unwrap();
            })
            .await;
        });

        locked_rx.await.unwrap();
        assert!(!SqlTransaction::deadlock_retry_lock_probe_for_test().await);

        release_tx.send(()).unwrap();
        holder.await.unwrap();
        assert!(SqlTransaction::deadlock_retry_lock_probe_for_test().await);
    }

    #[test]
    fn deadlock_retry_window_matches_cpp() {
        assert_eq!(
            SqlTransaction::deadlock_max_retry_time_like_cpp_for_test(),
            Duration::from_secs(60)
        );
    }
}
