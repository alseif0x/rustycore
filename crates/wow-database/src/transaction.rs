//! SQL transaction support.

use crate::error::DatabaseError;
use crate::params::{PreparedStatement, SqlParam};
use crate::persistence_trace::{
    CommitOutcome, ConnectionAffinity, LogicalDatabase, PersistenceEvent, PersistenceRecorder,
    TracedParam, raw_statement_digest,
};
use sqlx::{MySql, MySqlPool, pool::PoolConnection};
use std::future::Future;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::{
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
    time::MissedTickBehavior,
};

const DEADLOCK_MAX_RETRY_TIME_LIKE_CPP: Duration = Duration::from_secs(60);

static DEADLOCK_RETRY_LOCK_LIKE_CPP: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

const ITEM_GUID_ALLOCATOR_LOCK_PREFIX_LIKE_CPP: &str = "rustycore:item-guid:";
const ITEM_GUID_ALLOCATOR_LOCK_VERIFY_INTERVAL_LIKE_CPP: Duration = Duration::from_secs(30);

/// Retry an operation whose caller can distinguish MySQL deadlock 1213 from
/// all other failures. This extends TrinityCore's serialized 60-second retry
/// discipline to transactions that perform dynamic `SELECT ... FOR UPDATE`
/// reads and therefore cannot be represented by [`SqlTransaction`].
pub async fn retry_deadlocked_operation_like_cpp<F, Fut, T, E, IsDeadlock>(
    mut operation: F,
    is_deadlock: IsDeadlock,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    IsDeadlock: Fn(&E) -> bool,
{
    let mut result = operation().await;
    if result
        .as_ref()
        .err()
        .is_none_or(|error| !is_deadlock(error))
    {
        return result;
    }

    let _deadlock_guard = DEADLOCK_RETRY_LOCK_LIKE_CPP.lock().await;
    let start = Instant::now();
    loop {
        if start.elapsed() > DEADLOCK_MAX_RETRY_TIME_LIKE_CPP {
            tracing::error!(
                target: "sql.sql",
                "Fatal deadlocked SQL operation, it will not be retried anymore"
            );
            return result;
        }

        result = operation().await;
        if result
            .as_ref()
            .err()
            .is_none_or(|error| !is_deadlock(error))
        {
            return result;
        }
        tracing::warn!(
            target: "sql.sql",
            loop_timer_ms = start.elapsed().as_millis(),
            "Deadlocked SQL operation, retrying"
        );
    }
}

/// Dedicated MySQL connection holding the process-lifetime item GUID allocator
/// lock. `close_on_drop` is set after acquisition so early-return and panic
/// paths close the connection instead of returning a still-locked connection
/// to the pool.
#[derive(Debug)]
pub struct ItemGuidAllocatorAdvisoryLockLikeCpp {
    allocator_label: &'static str,
    stop_tx: Option<oneshot::Sender<()>>,
    loss_rx: watch::Receiver<Option<String>>,
    monitor_handle: Option<JoinHandle<Result<(), DatabaseError>>>,
}

impl ItemGuidAllocatorAdvisoryLockLikeCpp {
    pub async fn acquire_like_cpp(pool: &MySqlPool) -> Result<Self, DatabaseError> {
        Self::acquire_named_like_cpp(
            pool,
            "item",
            "character",
            ITEM_GUID_ALLOCATOR_LOCK_PREFIX_LIKE_CPP,
        )
        .await
    }

    async fn acquire_named_like_cpp(
        pool: &MySqlPool,
        allocator_label: &'static str,
        database_label: &'static str,
        lock_prefix: &'static str,
    ) -> Result<Self, DatabaseError> {
        let mut connection = pool.acquire().await.map_err(DatabaseError::from)?;
        // From this point onward even an ambiguous GET_LOCK response must not
        // return a potentially locked connection to the pool.
        connection.close_on_drop();
        let database_name = sqlx::query_scalar::<_, Option<String>>("SELECT DATABASE()")
            .fetch_one(&mut *connection)
            .await
            .map_err(DatabaseError::from)?
            .ok_or_else(|| {
                DatabaseError::Transaction(
                    format!(
                        "{allocator_label} GUID allocator lock requires a selected {database_label} database"
                    ),
                )
            })?;
        let lock_name = guid_allocator_lock_name_like_cpp(lock_prefix, &database_name);
        let acquired = sqlx::query_scalar::<_, Option<i64>>("SELECT GET_LOCK(?, 0)")
            .bind(&lock_name)
            .fetch_one(&mut *connection)
            .await
            .map_err(DatabaseError::from)?;
        // The lock lives on its own dedicated connection for the life of the
        // process, so it is neither pooled nor part of any transaction, and
        // whether it was taken is an observable persistence fact.
        crate::persistence_trace::record_advisory_lock(&lock_name, acquired == Some(1));
        if acquired != Some(1) {
            return Err(DatabaseError::Transaction(format!(
                "another world-server owns the {allocator_label} GUID allocator lock for {database_label} database {database_name}"
            )));
        }
        let (stop_tx, stop_rx) = oneshot::channel();
        let (loss_tx, loss_rx) = watch::channel(None);
        let monitor_handle = tokio::spawn(run_item_guid_allocator_lock_monitor_like_cpp(
            connection,
            lock_name,
            allocator_label,
            stop_rx,
            loss_tx,
        ));
        Ok(Self {
            allocator_label,
            stop_tx: Some(stop_tx),
            loss_rx,
            monitor_handle: Some(monitor_handle),
        })
    }

    /// Wait until the dedicated MySQL session no longer demonstrably owns the
    /// lock. A world process must treat this as fatal because its process-local
    /// allocator is no longer exclusive after that instant.
    pub async fn wait_until_lost_like_cpp(&mut self) -> Result<(), DatabaseError> {
        loop {
            if let Some(message) = self.loss_rx.borrow().clone() {
                return Err(DatabaseError::Transaction(message));
            }
            if self.loss_rx.changed().await.is_err() {
                return Err(DatabaseError::Transaction(format!(
                    "{} GUID allocator advisory-lock monitor stopped unexpectedly",
                    self.allocator_label
                )));
            }
        }
    }

    /// Release explicitly during orderly shutdown. If the query fails, the
    /// connection's close-on-drop fallback still releases this session lock.
    pub async fn release_like_cpp(mut self) -> Result<(), DatabaseError> {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        let Some(monitor_handle) = self.monitor_handle.take() else {
            return Ok(());
        };
        monitor_handle.await.map_err(|error| {
            DatabaseError::Transaction(format!(
                "{} GUID allocator advisory-lock monitor task failed: {error}",
                self.allocator_label
            ))
        })?
    }

    #[cfg(test)]
    pub(crate) fn lock_name_for_test(database_name: &str) -> String {
        item_guid_allocator_lock_name_like_cpp(database_name)
    }
}

async fn run_item_guid_allocator_lock_monitor_like_cpp(
    mut connection: PoolConnection<MySql>,
    lock_name: String,
    allocator_label: &'static str,
    mut stop_rx: oneshot::Receiver<()>,
    loss_tx: watch::Sender<Option<String>>,
) -> Result<(), DatabaseError> {
    let mut verify_interval =
        tokio::time::interval(ITEM_GUID_ALLOCATOR_LOCK_VERIFY_INTERVAL_LIKE_CPP);
    verify_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;
            _ = &mut stop_rx => {
                return release_item_guid_allocator_lock_like_cpp(
                    &mut connection,
                    &lock_name,
                    allocator_label,
                ).await;
            }
            _ = verify_interval.tick() => {
                let ownership = sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT IS_USED_LOCK(?) = CONNECTION_ID()",
                )
                .bind(&lock_name)
                .fetch_one(&mut *connection)
                .await;
                match ownership {
                    Ok(Some(1)) => {}
                    Ok(_) => {
                        let message = format!(
                            "dedicated MySQL session lost {allocator_label} GUID allocator advisory lock {lock_name}"
                        );
                        // The lock's lifetime ends here as surely as at an
                        // orderly release. Recording only the orderly path left
                        // an unexpected loss looking like a lock still held.
                        crate::persistence_trace::record_advisory_lock(&lock_name, false);
                        let _ = loss_tx.send(Some(message.clone()));
                        return Err(DatabaseError::Transaction(message));
                    }
                    Err(error) => {
                        let error = DatabaseError::from(error);
                        let message = format!(
                            "could not verify {allocator_label} GUID allocator advisory lock {lock_name}: {error}"
                        );
                        // Verification failed and the dedicated connection is
                        // closing, so ownership is over whether or not MySQL
                        // still thinks otherwise.
                        crate::persistence_trace::record_advisory_lock(&lock_name, false);
                        let _ = loss_tx.send(Some(message));
                        return Err(error);
                    }
                }
            }
        }
    }
}

async fn release_item_guid_allocator_lock_like_cpp(
    connection: &mut PoolConnection<MySql>,
    lock_name: &str,
    allocator_label: &'static str,
) -> Result<(), DatabaseError> {
    let released = sqlx::query_scalar::<_, Option<i64>>("SELECT RELEASE_LOCK(?)")
        .bind(lock_name)
        .fetch_one(&mut **connection)
        .await
        .map_err(DatabaseError::from);
    // Releasing is the other half of the lifetime: a trace that showed the
    // acquisition and never its release would describe a lock still held.
    // Recorded even when the statement errored, because the monitor's
    // close-on-drop connection goes with it and MySQL drops the session lock
    // regardless -- exiting through `?` first left exactly that false trace.
    crate::persistence_trace::record_advisory_lock(lock_name, false);
    let released = released?;
    if released != Some(1) {
        return Err(DatabaseError::Transaction(format!(
            "{allocator_label} GUID allocator advisory lock {lock_name} was not owned at shutdown"
        )));
    }
    // The monitor owns this dedicated close-on-drop connection. Returning
    // drops it immediately rather than putting it back into the shared pool.
    Ok(())
}

#[cfg(test)]
fn item_guid_allocator_lock_name_like_cpp(database_name: &str) -> String {
    guid_allocator_lock_name_like_cpp(ITEM_GUID_ALLOCATOR_LOCK_PREFIX_LIKE_CPP, database_name)
}

fn guid_allocator_lock_name_like_cpp(lock_prefix: &str, database_name: &str) -> String {
    // Stable FNV-1a avoids exposing a possibly sensitive database name while
    // keeping independent character databases in independent lock domains.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in database_name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{lock_prefix}{hash:016x}")
}

/// Whether a failed transaction is known to have rolled back or crossed the
/// point where MySQL may have committed before the connection lost the reply.
/// Consume-and-grant callers must never reopen a claim on the latter result.
#[derive(Debug)]
pub enum SqlTransactionCommitError {
    DefinitelyRolledBack(DatabaseError),
    CommitOutcomeUnknown(DatabaseError),
}

impl SqlTransactionCommitError {
    #[must_use]
    pub fn is_commit_outcome_unknown_like_cpp(&self) -> bool {
        matches!(self, Self::CommitOutcomeUnknown(_))
    }

    #[must_use]
    pub fn into_database_error(self) -> DatabaseError {
        match self {
            Self::DefinitelyRolledBack(error) | Self::CommitOutcomeUnknown(error) => error,
        }
    }
}

impl std::fmt::Display for SqlTransactionCommitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefinitelyRolledBack(error) => {
                write!(formatter, "transaction rolled back: {error}")
            }
            Self::CommitOutcomeUnknown(error) => {
                write!(formatter, "transaction COMMIT outcome is unknown: {error}")
            }
        }
    }
}

impl std::error::Error for SqlTransactionCommitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::DefinitelyRolledBack(error) | Self::CommitOutcomeUnknown(error) => error,
        })
    }
}

/// A batch of SQL statements to be executed atomically within a transaction.
///
/// Matches the TC `TransactionBase` / `Transaction<T>` pattern: collect
/// prepared statements or raw SQL strings, then commit them all at once.
/// Nothing was sent.
const TRACE_PLANNED: u8 = 0;
/// `pool.begin()` succeeded; statements are going out.
const TRACE_EXECUTING: u8 = 1;
/// COMMIT was issued and its result has not come back.
const TRACE_COMMITTING: u8 = 2;
/// A terminal event was already recorded.
const TRACE_RESOLVED: u8 = 3;

#[derive(Debug)]
pub struct SqlTransaction {
    statements: Vec<TransactionStatement>,
    cleaned_up_like_cpp: bool,
    /// Set only when a trace is being recorded. Statements are appended here
    /// in order, so the recording reflects the plan the caller built rather
    /// than the order MySQL happened to execute.
    trace: Option<PersistenceRecorder>,
    /// Set by the first appended statement. A transaction that never receives
    /// one never opened, and its trace says so.
    trace_database: Option<LogicalDatabase>,
    /// How far the batch got. A drop has to say which of three different
    /// things happened, and they are not interchangeable: whether a retry is
    /// safe depends on the answer.
    trace_progress: std::sync::atomic::AtomicU8,
    /// Raw statements appended before any database was known.
    pending_raw: Vec<PendingRawStatement>,
}

/// A raw statement held until the transaction's database is known.
#[derive(Debug)]
struct PendingRawStatement {
    digest: u64,
    params: Vec<TracedParam>,
}

#[derive(Debug)]
struct TransactionStatement {
    statement: PreparedStatement,
    expected_rows_affected: Option<u64>,
}

impl Default for SqlTransaction {
    /// Same as [`SqlTransaction::new`], deliberately not derived.
    ///
    /// A derived `Default` leaves `trace: None`, and the legacy-password
    /// migration replaces its batch with `std::mem::take` every ten thousand
    /// accounts: the first batch was traced and every one after it silently was
    /// not. Anything that can produce a transaction has to pick up the ambient
    /// recorder the same way.
    fn default() -> Self {
        Self::new()
    }
}

impl SqlTransaction {
    /// Create a new empty transaction batch.
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
            cleaned_up_like_cpp: false,
            trace_progress: std::sync::atomic::AtomicU8::new(TRACE_PLANNED),
            pending_raw: Vec::new(),
            trace: crate::persistence_trace::ambient_recorder(),
            trace_database: None,
        }
    }

    /// Record this transaction's steps into `recorder`.
    ///
    /// Opening the recording emits the transaction boundary immediately:
    /// everything appended afterwards shares one connection and lands or fails
    /// together, and that grouping is the fact a port extraction can lose.
    pub fn with_trace(mut self, recorder: PersistenceRecorder, database: LogicalDatabase) -> Self {
        recorder.record(PersistenceEvent::TransactionBegin { database });
        self.trace = Some(recorder);
        self.trace_database = Some(database);
        self
    }

    fn record_statement(&mut self, stmt: &PreparedStatement, expected_rows_affected: Option<u64>) {
        let Some(recorder) = self.trace.clone() else {
            return;
        };
        // The database comes from the first statement: `SqlTransaction::new`
        // is called in seventy-five places that do not know it, and a raw-SQL
        // append cannot supply it at all.
        let database = match (self.trace_database, stmt.trace_database()) {
            (Some(known), Some(appended)) if appended != known => {
                // Two logical databases can never be one atomic unit, so a
                // trace that recorded a single boundary here would describe a
                // guarantee the server cannot make. Record the contradiction
                // instead of smoothing it over.
                recorder.record(PersistenceEvent::MixedLogicalDatabases {
                    opened: known,
                    appended,
                });
                known
            }
            (Some(known), _) => known,
            (None, Some(first)) => {
                self.open_trace_with(&recorder, first);
                first
            }
            // Raw SQL before anything identified the transaction's database.
            // The database is not guessed, but the statement is still recorded:
            // dropping it made a transaction built entirely from raw SQL absent
            // from its own trace, and a golden that says a flow persists nothing
            // is wrong, where one that admits it could not attribute a statement
            // is merely incomplete about it.
            // Raw SQL before anything identified the transaction's database.
            // Held rather than emitted: the database may still arrive -- from a
            // later typed statement, or from the adapter that commits -- and a
            // `TransactionBegin` recorded after the statements it opens would be
            // an out-of-order trace of an in-order plan. If it never arrives,
            // `Drop` emits these unattributed, which is incomplete but true;
            // emitting nothing was the bug this replaced.
            (None, None) => {
                self.pending_raw.push(PendingRawStatement {
                    digest: raw_statement_digest(stmt.sql()),
                    params: stmt.params().iter().map(TracedParam::from_param).collect(),
                });
                return;
            }
        };
        let recorder = &recorder;
        let params = stmt.params().iter().map(TracedParam::from_param).collect();
        match stmt.trace_identity() {
            Some(statement) => recorder.record(PersistenceEvent::Statement {
                database,
                connection: ConnectionAffinity::Transaction,
                statement: statement.to_owned(),
                params,
                expected_rows_affected,
                observed_rows_affected: None,
            }),
            // Raw SQL has no statement enum behind it and its text may be
            // built at run time, so the trace keeps its shape and not its
            // formatting.
            None => recorder.record(PersistenceEvent::RawStatement {
                database,
                connection: ConnectionAffinity::Transaction,
                digest: raw_statement_digest(stmt.sql()),
                params,
            }),
        }
    }

    /// Open the trace on `database`, flushing anything held before it was known.
    ///
    /// The held statements go out after the begin and in the order they were
    /// appended, so attribution arriving late does not reorder the plan.
    fn open_trace_with(&mut self, recorder: &PersistenceRecorder, database: LogicalDatabase) {
        recorder.record(PersistenceEvent::TransactionBegin { database });
        self.trace_database = Some(database);
        for held in std::mem::take(&mut self.pending_raw) {
            recorder.record(PersistenceEvent::RawStatement {
                database,
                connection: ConnectionAffinity::Transaction,
                digest: held.digest,
                params: held.params,
            });
        }
    }

    /// Attribute a batch that never named a database, from the adapter
    /// committing it.
    ///
    /// `Database<S>` knows its logical database from `S::DATABASE` even when
    /// every statement in the batch was built raw, which is the only way those
    /// transactions get a boundary at all.
    pub(crate) fn attribute_to_like_cpp(&mut self, database: LogicalDatabase) {
        if self.trace_database.is_some() {
            return;
        }
        let Some(recorder) = self.trace.clone() else {
            return;
        };
        if self.pending_raw.is_empty() {
            return;
        }
        self.open_trace_with(&recorder, database);
    }

    fn record_commit_outcome(&self, outcome: CommitOutcome) {
        if let (Some(recorder), Some(database)) = (&self.trace, self.trace_database) {
            recorder.record(PersistenceEvent::Commit { database, outcome });
        }
        self.trace_progress
            .store(TRACE_RESOLVED, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_batch_abandoned(&self) {
        if let (Some(recorder), Some(database)) = (&self.trace, self.trace_database) {
            recorder.record(PersistenceEvent::BatchAbandoned { database });
        }
        self.trace_progress
            .store(TRACE_RESOLVED, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_rollback(&self) {
        if let (Some(recorder), Some(database)) = (&self.trace, self.trace_database) {
            recorder.record(PersistenceEvent::Rollback { database });
        }
        self.trace_progress
            .store(TRACE_RESOLVED, std::sync::atomic::Ordering::Relaxed);
    }

    fn record_retry_boundary(&self) {
        let (Some(recorder), Some(database)) = (&self.trace, self.trace_database) else {
            return;
        };
        recorder.record(PersistenceEvent::TransactionBegin { database });
        // The retry sends every statement again, so the trace has to show them
        // again. Emitting the boundary alone described an empty transaction
        // followed by a commit, which is a worse account of the attempt than
        // the missing boundary it replaced.
        for transaction_statement in &self.statements {
            let stmt = &transaction_statement.statement;
            let params = stmt.params().iter().map(TracedParam::from_param).collect();
            match stmt.trace_identity() {
                Some(statement) => recorder.record(PersistenceEvent::Statement {
                    database,
                    connection: ConnectionAffinity::Transaction,
                    statement: statement.to_owned(),
                    params,
                    expected_rows_affected: transaction_statement.expected_rows_affected,
                    observed_rows_affected: None,
                }),
                None => recorder.record(PersistenceEvent::RawStatement {
                    database,
                    connection: ConnectionAffinity::Transaction,
                    digest: raw_statement_digest(stmt.sql()),
                    params,
                }),
            }
        }
    }

    fn record_deadlock_retry(&self, attempt: u32) {
        if let (Some(recorder), Some(database)) = (&self.trace, self.trace_database) {
            recorder.record(PersistenceEvent::DeadlockRetry { database, attempt });
        }
    }
}

impl Drop for SqlTransaction {
    /// Say how far the batch got, because the three answers are not the same
    /// fact and a retry decision hangs on which one it is.
    ///
    /// Statements are recorded as they are appended, so a caller that builds a
    /// transaction and then returns -- the vendor-currency turn-in does exactly
    /// this when it cannot take the money lock -- left a trace showing writes
    /// that never reached the database.
    ///
    /// Cancellation makes that insufficient on its own. A task dropped while
    /// awaiting COMMIT has already sent it, and the server may have applied it;
    /// calling that abandoned would let a golden approve a retry that
    /// duplicates the write.
    fn drop(&mut self) {
        let Some(recorder) = self.trace.clone() else {
            return;
        };
        // Nothing ever named a database. The held statements still happened as
        // a plan, so they go out unattributed rather than vanishing.
        let Some(database) = self.trace_database else {
            for held in std::mem::take(&mut self.pending_raw) {
                recorder.record(PersistenceEvent::UnattributedRawStatement {
                    connection: ConnectionAffinity::Transaction,
                    digest: held.digest,
                    params: held.params,
                });
            }
            return;
        };
        let recorder = &recorder;
        match self
            .trace_progress
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            // Nothing was sent: the plan was built and abandoned. Deliberately
            // not `Rollback`, which would claim work was undone that was never
            // issued.
            TRACE_PLANNED => recorder.record(PersistenceEvent::BatchAbandoned { database }),
            // `pool.begin()` succeeded and statements went out. Dropping the
            // sqlx transaction rolls them back, so that is what happened.
            TRACE_EXECUTING => recorder.record(PersistenceEvent::Rollback { database }),
            // COMMIT was issued and its result never came back. Nothing here
            // can find out, and `Unknown` is the only honest answer -- it is
            // also the one that stops a retry being assumed safe.
            TRACE_COMMITTING => recorder.record(PersistenceEvent::Commit {
                database,
                outcome: CommitOutcome::Unknown,
            }),
            _ => {}
        }
    }
}

impl SqlTransaction {
    /// Append a prepared statement to this transaction.
    pub fn append(&mut self, stmt: PreparedStatement) {
        self.record_statement(&stmt, None);
        self.statements.push(TransactionStatement {
            statement: stmt,
            expected_rows_affected: None,
        });
    }

    /// Append a prepared statement whose affected-row count is part of the
    /// transaction's correctness contract.
    ///
    /// A mismatch aborts the transaction before `COMMIT`. Consume-and-grant
    /// operations must not treat an `UPDATE` or `DELETE` that matched no
    /// durable row as a durable gameplay success.
    pub fn append_expect_rows_affected(&mut self, stmt: PreparedStatement, expected: u64) {
        self.record_statement(&stmt, Some(expected));
        self.statements.push(TransactionStatement {
            statement: stmt,
            expected_rows_affected: Some(expected),
        });
    }

    /// Append a raw SQL statement like TC `TransactionBase::Append(char const*)`.
    ///
    /// Prefer prepared statements for user input. This is for C++ parity with
    /// existing raw-SQL transaction call sites and test fixtures.
    pub fn append_raw_sql_like_cpp(&mut self, sql: impl Into<String>) {
        // Recording happens in `append`: raw SQL carries no statement enum, so
        // `record_statement` already files it by shape.
        self.append(PreparedStatement::raw_sql_like_cpp(sql));
    }

    /// Clear queued statements once, mirroring TC `TransactionBase::Cleanup`.
    pub fn cleanup_like_cpp(&mut self) {
        if self.cleaned_up_like_cpp {
            return;
        }

        self.statements.clear();
        self.cleaned_up_like_cpp = true;
    }

    /// Whether [`cleanup_like_cpp`](Self::cleanup_like_cpp) already ran.
    pub fn cleaned_up_like_cpp(&self) -> bool {
        self.cleaned_up_like_cpp
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
        self.commit_with_outcome_like_cpp(pool)
            .await
            .map_err(SqlTransactionCommitError::into_database_error)
    }

    /// Commit while preserving the ambiguity of a transport error returned by
    /// `COMMIT`. Query/validation failures and MySQL deadlock 1213 are definite
    /// rollbacks; a non-deadlock COMMIT error must be reconciled by the caller.
    pub async fn commit_with_outcome_like_cpp(
        self,
        pool: &MySqlPool,
    ) -> Result<(), SqlTransactionCommitError> {
        if self.statements.is_empty() {
            return Ok(());
        }

        let result = self.try_commit(pool).await;

        if !is_outcome_deadlock_like_cpp(&result) {
            return result;
        }

        let _deadlock_guard = DEADLOCK_RETRY_LOCK_LIKE_CPP.lock().await;
        let start = Instant::now();
        let mut attempt = 0_u32;

        loop {
            if start.elapsed() > DEADLOCK_MAX_RETRY_TIME_LIKE_CPP {
                tracing::error!(
                    target: "sql.sql",
                    "Fatal deadlocked SQL Transaction, it will not be retried anymore"
                );
                return result;
            }

            attempt += 1;
            self.record_deadlock_retry(attempt);
            // `try_commit_inner` opens a new sqlx transaction, so this attempt
            // is a new boundary: the previous one rolled back and could have
            // applied nothing, while this one starts over. A trace showing a
            // single begin around several attempts describes one transaction
            // where the server saw more than one.
            self.record_retry_boundary();
            let retry = self.try_commit_inner(pool).await;
            if retry.is_ok() {
                return retry;
            }
            if !is_outcome_deadlock_like_cpp(&retry) {
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

    #[cfg(test)]
    pub(crate) fn sqls_for_test(&self) -> Vec<&str> {
        self.statements
            .iter()
            .map(|statement| statement.statement.sql())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn expected_rows_for_test(&self) -> Vec<Option<u64>> {
        self.statements
            .iter()
            .map(|statement| statement.expected_rows_affected)
            .collect()
    }

    async fn try_commit(&self, pool: &MySqlPool) -> Result<(), SqlTransactionCommitError> {
        self.try_commit_inner(pool).await
    }

    async fn try_commit_inner(&self, pool: &MySqlPool) -> Result<(), SqlTransactionCommitError> {
        // Recording happens here rather than at the caller because only this
        // scope knows whether `tx.commit()` was ever reached. A failure to
        // acquire a connection, to execute a statement, or to validate an
        // affected-row count is a rollback that never attempted a commit, and
        // conflating it with a resolved COMMIT would freeze the wrong crash
        // boundary — the one thing this contract exists to get right.
        let mut tx = match pool.begin().await {
            Ok(tx) => tx,
            Err(error) => {
                // No transaction was opened and no statement reached MySQL, so
                // this is the batch never running -- not a rollback, which says
                // work was issued and undone. Both produced the same
                // begin/statements/rollback shape, and a golden cannot tell a
                // connection that was never acquired from a transaction that
                // executed and failed.
                self.record_batch_abandoned();
                return Err(SqlTransactionCommitError::DefinitelyRolledBack(
                    DatabaseError::from(error),
                ));
            }
        };

        self.trace_progress
            .store(TRACE_EXECUTING, std::sync::atomic::Ordering::Relaxed);

        for (statement_index, transaction_statement) in self.statements.iter().enumerate() {
            let stmt = &transaction_statement.statement;
            let mut query = sqlx::query(stmt.sql());
            for param in stmt.params() {
                query = bind_param(query, param);
            }
            // Both of these abandon the transaction before `tx.commit()` is
            // ever reached, so each is a rollback and not a resolved commit.
            // The `?` shortcuts made them silent in the first pass at this.
            let result = match query.execute(&mut *tx).await {
                Ok(result) => result,
                Err(error) => {
                    self.record_rollback();
                    return Err(SqlTransactionCommitError::DefinitelyRolledBack(
                        DatabaseError::from(error),
                    ));
                }
            };
            if let Some(expected) = transaction_statement.expected_rows_affected {
                if let Err(error) =
                    validate_rows_affected(statement_index, expected, result.rows_affected())
                {
                    self.record_rollback();
                    return Err(SqlTransactionCommitError::DefinitelyRolledBack(error));
                }
            }
        }

        self.trace_progress
            .store(TRACE_COMMITTING, std::sync::atomic::Ordering::Relaxed);
        if let Err(error) = tx.commit().await {
            let error = DatabaseError::from(error);
            let outcome = if is_database_deadlock_like_cpp(&error) {
                SqlTransactionCommitError::DefinitelyRolledBack(error)
            } else {
                SqlTransactionCommitError::CommitOutcomeUnknown(error)
            };
            self.record_commit_outcome(match &outcome {
                SqlTransactionCommitError::DefinitelyRolledBack(_) => CommitOutcome::RolledBack,
                SqlTransactionCommitError::CommitOutcomeUnknown(_) => CommitOutcome::Unknown,
            });
            return Err(outcome);
        }
        self.record_commit_outcome(CommitOutcome::Committed);
        Ok(())
    }
}

fn validate_rows_affected(
    statement_index: usize,
    expected: u64,
    actual: u64,
) -> Result<(), DatabaseError> {
    if actual == expected {
        return Ok(());
    }

    Err(DatabaseError::Transaction(format!(
        "statement {statement_index} affected {actual} rows; expected exactly {expected}"
    )))
}

/// Classify a finished commit attempt for a persistence trace.
///
/// The three outcomes are not interchangeable and the distinction is the whole
/// reason this contract exists. A definite rollback means the work is gone and
/// may be replayed; an unknown outcome means the server cannot tell whether it
/// landed, and C++ reconciles that with a durable token rather than guessing.
/// Collapsing `Unknown` into either neighbour is the silent data-loss bug this
/// golden has to be able to see.
pub(crate) fn commit_outcome_like_cpp(
    result: &Result<(), SqlTransactionCommitError>,
) -> CommitOutcome {
    match result {
        Ok(()) => CommitOutcome::Committed,
        Err(SqlTransactionCommitError::DefinitelyRolledBack(_)) => CommitOutcome::RolledBack,
        Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) => CommitOutcome::Unknown,
    }
}

fn is_outcome_deadlock_like_cpp(result: &Result<(), SqlTransactionCommitError>) -> bool {
    match result {
        Err(SqlTransactionCommitError::DefinitelyRolledBack(error)) => {
            is_database_deadlock_like_cpp(error)
        }
        Err(SqlTransactionCommitError::CommitOutcomeUnknown(_)) | Ok(()) => false,
    }
}

pub fn is_database_deadlock_like_cpp(error: &DatabaseError) -> bool {
    match error {
        DatabaseError::Query(sqlx::Error::Database(db_err)) => {
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
mod trace_tests {
    use super::*;
    use crate::statements::{CharStatements, StatementDef};

    fn traced() -> (SqlTransaction, PersistenceRecorder) {
        let recorder = PersistenceRecorder::new();
        let trans = SqlTransaction::new().with_trace(recorder.clone(), LogicalDatabase::Character);
        (trans, recorder)
    }

    #[test]
    fn opening_a_trace_records_the_transaction_boundary() {
        let (_trans, recorder) = traced();
        assert_eq!(
            recorder.take().events,
            vec![PersistenceEvent::TransactionBegin {
                database: LogicalDatabase::Character
            }]
        );
    }

    #[test]
    fn appends_are_recorded_in_plan_order_with_their_identity() {
        let (mut trans, recorder) = traced();
        let mut first = PreparedStatement::for_statement(CharStatements::DEL_POOL_QUEST_SAVE)
            .with_trace_identity(CharStatements::DEL_POOL_QUEST_SAVE.trace_identity());
        first.set_u32(0, 7);
        let second = PreparedStatement::for_statement(CharStatements::INS_POOL_QUEST_SAVE)
            .with_trace_identity(CharStatements::INS_POOL_QUEST_SAVE.trace_identity());

        trans.append(first);
        trans.append_expect_rows_affected(second, 1);

        let events = recorder.take().events;
        assert_eq!(events.len(), 3, "begin + two statements: {events:?}");
        match &events[1] {
            PersistenceEvent::Statement {
                statement,
                connection,
                params,
                expected_rows_affected,
                ..
            } => {
                assert_eq!(statement, "DEL_POOL_QUEST_SAVE");
                assert_eq!(*connection, ConnectionAffinity::Transaction);
                assert_eq!(
                    params.first(),
                    Some(&TracedParam::Uint {
                        value: 7,
                        width_bits: 32
                    })
                );
                assert_eq!(*expected_rows_affected, None);
            }
            other => panic!("expected a statement, got {other:?}"),
        }
        match &events[2] {
            PersistenceEvent::Statement {
                statement,
                expected_rows_affected,
                ..
            } => {
                assert_eq!(statement, "INS_POOL_QUEST_SAVE");
                assert_eq!(
                    *expected_rows_affected,
                    Some(1),
                    "an asserted row count is part of the contract"
                );
            }
            other => panic!("expected a statement, got {other:?}"),
        }
    }

    #[test]
    fn swapping_two_appends_produces_a_different_trace() {
        // Statement order is the contract this golden exists to freeze.
        let build = |reversed: bool| {
            let (mut trans, recorder) = traced();
            let del = PreparedStatement::for_statement(CharStatements::DEL_POOL_QUEST_SAVE)
                .with_trace_identity(CharStatements::DEL_POOL_QUEST_SAVE.trace_identity());
            let ins = PreparedStatement::for_statement(CharStatements::INS_POOL_QUEST_SAVE)
                .with_trace_identity(CharStatements::INS_POOL_QUEST_SAVE.trace_identity());
            if reversed {
                trans.append(ins);
                trans.append(del);
            } else {
                trans.append(del);
                trans.append(ins);
            }
            recorder.take()
        };
        assert_ne!(build(false), build(true));
    }

    #[test]
    fn raw_sql_is_recorded_by_shape_not_text() {
        let (mut trans, recorder) = traced();
        trans.append_raw_sql_like_cpp("DELETE FROM character_pet WHERE guid = 4");
        let events = recorder.take().events;
        assert_eq!(events.len(), 2, "begin + one raw statement: {events:?}");
        match &events[1] {
            PersistenceEvent::RawStatement { digest, .. } => {
                assert_ne!(*digest, 0);
            }
            other => panic!("expected raw statement, got {other:?}"),
        }
        let rendered = serde_json::to_string(&events).expect("serialize");
        assert!(
            !rendered.contains("character_pet"),
            "raw SQL text must not reach the trace: {rendered}"
        );
    }

    fn commit_error(unknown: bool) -> SqlTransactionCommitError {
        let error = DatabaseError::Transaction("transport reset".to_owned());
        if unknown {
            SqlTransactionCommitError::CommitOutcomeUnknown(error)
        } else {
            SqlTransactionCommitError::DefinitelyRolledBack(error)
        }
    }

    #[test]
    fn the_three_commit_outcomes_stay_distinct() {
        // Collapsing `Unknown` into either neighbour is the silent data-loss
        // bug this contract exists to make visible: a definite rollback may be
        // replayed, an unknown outcome may not.
        assert_eq!(commit_outcome_like_cpp(&Ok(())), CommitOutcome::Committed);
        assert_eq!(
            commit_outcome_like_cpp(&Err(commit_error(false))),
            CommitOutcome::RolledBack
        );
        assert_eq!(
            commit_outcome_like_cpp(&Err(commit_error(true))),
            CommitOutcome::Unknown
        );
        assert_ne!(
            commit_outcome_like_cpp(&Err(commit_error(true))),
            commit_outcome_like_cpp(&Err(commit_error(false)))
        );
    }

    #[test]
    fn an_unknown_commit_is_never_treated_as_a_deadlock_retry() {
        // A deadlock is a definite rollback, so it may be retried; an ambiguous
        // COMMIT must not be, or the retry would double-apply the work.
        assert!(!is_outcome_deadlock_like_cpp(&Err(commit_error(true))));
    }

    #[test]
    fn an_installed_recording_traces_transactions_nobody_wired_up() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _session = RecordingSession::install(recorder.clone());

        // Exactly what the seventy-five untouched call sites do.
        let mut trans = SqlTransaction::new();
        let stmt = PreparedStatement::for_statement(CharStatements::DEL_POOL_QUEST_SAVE)
            .with_trace_identity(CharStatements::DEL_POOL_QUEST_SAVE.trace_identity())
            .with_trace_database(CharStatements::DEL_POOL_QUEST_SAVE.logical_database());
        trans.append(stmt);

        let events = recorder.take().events;
        assert_eq!(
            events,
            vec![
                PersistenceEvent::TransactionBegin {
                    database: LogicalDatabase::Character
                },
                PersistenceEvent::Statement {
                    database: LogicalDatabase::Character,
                    connection: ConnectionAffinity::Transaction,
                    statement: "DEL_POOL_QUEST_SAVE".to_owned(),
                    params: vec![TracedParam::Bool { value: false }],
                    expected_rows_affected: None,
                    observed_rows_affected: None,
                },
            ],
            "the boundary must open on the first statement, not at construction"
        );
    }

    #[test]
    fn a_transaction_that_never_receives_a_statement_never_opened() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _session = RecordingSession::install(recorder.clone());

        let trans = SqlTransaction::new();
        drop(trans);
        assert!(
            recorder.take().events.is_empty(),
            "an empty transaction sends nothing, so it must record nothing"
        );
    }

    #[tokio::test]
    async fn a_failure_before_commit_records_a_rollback_not_a_commit() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // Unreachable pool: `pool.begin()` fails, so `tx.commit()` is never
        // attempted. Reporting that as a resolved COMMIT would freeze the
        // wrong crash boundary — a rollback that never tried is not the same
        // event as a commit that was tried and rolled back.
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(std::time::Duration::from_millis(1))
            .connect_lazy("mysql://rustycore:rustycore@127.0.0.1:1/characters")
            .expect("syntactically valid lazy pool");

        let mut trans = SqlTransaction::new();
        trans.append(
            PreparedStatement::for_statement(CharStatements::DEL_POOL_QUEST_SAVE)
                .with_trace_identity(CharStatements::DEL_POOL_QUEST_SAVE.trace_identity())
                .with_trace_database(CharStatements::DEL_POOL_QUEST_SAVE.logical_database()),
        );
        let _ = trans.commit_with_outcome_like_cpp(&pool).await;

        let events = recorder.take().events;
        // The pool is unreachable, so `pool.begin()` fails and nothing is sent:
        // the batch never ran. `Rollback` would say work was issued and undone,
        // which is a different fact and the one a retry decision turns on.
        assert!(
            events.iter().any(|event| matches!(
                event,
                PersistenceEvent::BatchAbandoned {
                    database: LogicalDatabase::Character
                }
            )),
            "a batch whose connection never opened must say it never ran: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::Rollback { .. })),
            "and must not claim a rollback it never performed: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::Commit { .. })),
            "no commit was attempted, so none may be recorded: {events:?}"
        );
    }

    #[test]
    fn a_batch_cancelled_while_committing_reports_an_unknown_outcome() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // Cancellation after COMMIT went out. The server may have applied it and
        // nothing here can find out, so `BatchAbandoned` -- "nothing reached the
        // database" -- would be a false statement that lets a golden approve a
        // retry which duplicates the write. `Unknown` is the only answer that
        // does not.
        {
            let mut trans = SqlTransaction::new();
            trans.append(PreparedStatement::for_statement(
                CharStatements::UPD_CHAR_MONEY,
            ));
            trans
                .trace_progress
                .store(TRACE_COMMITTING, std::sync::atomic::Ordering::Relaxed);
        }

        let events = recorder.take().events;
        assert!(
            events.iter().any(|event| matches!(
                event,
                PersistenceEvent::Commit {
                    outcome: CommitOutcome::Unknown,
                    ..
                }
            )),
            "a cancelled commit must be recorded as unknown: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::BatchAbandoned { .. })),
            "it did reach the database, so it was not abandoned: {events:?}"
        );
    }

    #[test]
    fn a_batch_cancelled_while_executing_reports_a_rollback() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // Statements went out but COMMIT never did. Dropping the sqlx
        // transaction rolls them back, which is a different fact again from
        // both of the other two.
        {
            let mut trans = SqlTransaction::new();
            trans.append(PreparedStatement::for_statement(
                CharStatements::UPD_CHAR_MONEY,
            ));
            trans
                .trace_progress
                .store(TRACE_EXECUTING, std::sync::atomic::Ordering::Relaxed);
        }

        let events = recorder.take().events;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::Rollback { .. })),
            "an executed-but-uncommitted batch rolls back: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::BatchAbandoned { .. })),
            "statements were sent, so it was not abandoned: {events:?}"
        );
    }

    #[test]
    fn a_default_built_transaction_is_traced_like_a_new_one() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // The legacy-password migration replaces its batch with
        // `std::mem::take` every ten thousand accounts. With a derived
        // `Default` the replacement carried no recorder, so the first batch was
        // traced and every one after it was not -- a trace that looks complete
        // and covers a fraction of the work.
        {
            let mut trans = SqlTransaction::default();
            trans.append(PreparedStatement::for_statement(
                CharStatements::UPD_CHAR_MONEY,
            ));
        }

        assert!(
            !recorder.take().events.is_empty(),
            "a transaction from Default must pick up the ambient recorder"
        );
    }

    #[test]
    fn a_batch_built_and_then_dropped_says_it_never_ran() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // The shape of the vendor-currency turn-in: append the statements, then
        // return because the money lock could not be taken. `pool.begin()` is
        // never reached, so nothing was sent.
        {
            let mut trans = SqlTransaction::new();
            trans.append(PreparedStatement::for_statement(
                CharStatements::UPD_CHAR_MONEY,
            ));
        }

        let events = recorder.take().events;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::BatchAbandoned { .. })),
            "a planned batch that never executed must say so: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::Rollback { .. })),
            "nothing was sent, so nothing was rolled back: {events:?}"
        );
    }

    #[test]
    fn raw_statements_that_differ_only_in_their_bound_values_differ_in_the_trace() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();

        // The bank-slot purchase binds new money, slot count and character GUID
        // into one constant SQL string. Digesting only the text made every
        // purchase look identical, so a golden could not tell a change in the
        // amount from no change at all.
        let trace_of = |money: u64| {
            let recorder = PersistenceRecorder::new();
            let session = RecordingSession::install(recorder.clone());
            {
                let mut trans = SqlTransaction::new();
                let mut stmt =
                    PreparedStatement::new("UPDATE characters SET money = ? WHERE guid = ?");
                stmt.set_u64(0, money);
                stmt.set_u64(1, 42);
                trans.append(stmt);
            }
            drop(session);
            recorder.take().events
        };

        let cheap = trace_of(100);
        let dear = trace_of(999_999);
        assert_ne!(
            cheap, dear,
            "a different bound amount must produce a different trace"
        );
        assert_eq!(
            trace_of(100),
            cheap,
            "the same bound amount must produce the same trace"
        );
    }

    #[test]
    fn a_raw_only_batch_attributed_by_its_adapter_gets_a_full_boundary() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // The bank-slot purchase and the tutorial save are built entirely from
        // `PreparedStatement::new`, so nothing in the batch names a database.
        // The committing `Database<S>` does, through `S::DATABASE`, and without
        // it these traces carried a statement with no begin, commit, rollback or
        // unknown around it -- the crash boundary this recorder exists to hold.
        let mut trans = SqlTransaction::new();
        trans.append_raw_sql_like_cpp("UPDATE characters SET money = 1 WHERE guid = 2");
        trans.attribute_to_like_cpp(LogicalDatabase::Character);

        let events = recorder.snapshot().events;
        assert!(
            matches!(
                events.first(),
                Some(PersistenceEvent::TransactionBegin {
                    database: LogicalDatabase::Character
                })
            ),
            "the begin must come first, not after the statements it opens: {events:?}"
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                PersistenceEvent::RawStatement {
                    database: LogicalDatabase::Character,
                    ..
                }
            )),
            "the held statement must be attributed once the database is known: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::UnattributedRawStatement { .. })),
            "nothing should remain unattributed once the adapter supplied it: {events:?}"
        );

        drop(trans);
        let events = recorder.take().events;
        assert!(
            events
                .iter()
                .any(|event| matches!(event, PersistenceEvent::BatchAbandoned { .. })),
            "and the batch still reports how it ended: {events:?}"
        );
    }

    #[test]
    fn a_transaction_of_only_raw_sql_still_appears_in_its_own_trace() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // Nothing here identifies a database, so the transaction's database
        // genuinely cannot be attributed. It is still recorded: dropping it
        // produced an empty trace for a flow that does persist, and a golden
        // asserting "persists nothing" is wrong, where one admitting it could
        // not attribute a statement is incomplete and says so.
        // Scoped: with no database to attribute them to, raw statements are
        // held until the batch resolves, so they reach the trace when it drops
        // rather than as each one is appended. The alternative was emitting a
        // `TransactionBegin` after the statements it opens if attribution
        // arrived late.
        {
            let mut trans = SqlTransaction::new();
            trans.append_raw_sql_like_cpp("DELETE FROM something WHERE id = 1");
            trans.append_raw_sql_like_cpp("DELETE FROM something_else WHERE id = 2");
        }

        let trace = recorder.snapshot();
        let unattributed = trace
            .events
            .iter()
            .filter(|event| matches!(event, PersistenceEvent::UnattributedRawStatement { .. }))
            .count();
        assert_eq!(
            unattributed, 2,
            "both raw statements belong in the trace: {trace:?}"
        );
        assert!(
            !trace.events.is_empty(),
            "an all-raw transaction must not be invisible to its own trace"
        );
    }

    #[test]
    fn a_statement_built_from_its_variant_is_never_dropped() {
        use crate::persistence_trace::RecordingSession;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        // `PreparedStatement::new(X.sql())` throws the variant away, and a
        // statement with no identity used to be dropped silently — so a
        // transaction whose first statement was manual never opened at all.
        // `for_statement` is the form that keeps it.
        let mut trans = SqlTransaction::new();
        trans.append(PreparedStatement::for_statement(
            CharStatements::DEL_POOL_QUEST_SAVE,
        ));

        let events = recorder.take().events;
        assert_eq!(events.len(), 2, "begin + statement: {events:?}");
        assert!(matches!(
            events[0],
            PersistenceEvent::TransactionBegin {
                database: LogicalDatabase::Character
            }
        ));
        match &events[1] {
            PersistenceEvent::Statement { statement, .. } => {
                assert_eq!(statement, "DEL_POOL_QUEST_SAVE");
            }
            other => panic!("expected an identified statement, got {other:?}"),
        }
    }

    #[test]
    fn two_logical_databases_in_one_transaction_are_flagged() {
        use crate::persistence_trace::RecordingSession;
        use crate::statements::LoginStatements;

        let _serialized = crate::persistence_trace::capture_flag_test_lock();
        let recorder = PersistenceRecorder::new();
        let _recording = RecordingSession::install(recorder.clone());

        let mut trans = SqlTransaction::new();
        trans.append(PreparedStatement::for_statement(
            CharStatements::DEL_POOL_QUEST_SAVE,
        ));
        trans.append(PreparedStatement::for_statement(
            LoginStatements::SEL_REALMLIST,
        ));

        let events = recorder.take().events;
        assert!(
            events.iter().any(|event| matches!(
                event,
                PersistenceEvent::MixedLogicalDatabases {
                    opened: LogicalDatabase::Character,
                    appended: LogicalDatabase::Login,
                }
            )),
            "a transaction spanning two databases must say so: {events:?}"
        );
    }

    #[test]
    fn an_untraced_transaction_records_nothing() {
        // The recorder is opt-in; production builds the same transactions.
        let recorder = PersistenceRecorder::new();
        let mut trans = SqlTransaction::new();
        trans.append(PreparedStatement::for_statement(CharStatements::SEL_ENUM));
        assert!(recorder.take().events.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ItemGuidAllocatorAdvisoryLockLikeCpp, SqlTransaction, SqlTransactionCommitError,
        retry_deadlocked_operation_like_cpp, validate_rows_affected,
    };
    use crate::{DatabaseError, PreparedStatement};
    use std::sync::atomic::{AtomicUsize, Ordering};
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

        let attempts = AtomicUsize::new(0);
        let value = retry_deadlocked_operation_like_cpp(
            || {
                let attempt = attempts.fetch_add(1, Ordering::Relaxed);
                async move {
                    if attempt < 2 {
                        Err(DatabaseError::Transaction("synthetic deadlock".to_string()))
                    } else {
                        Ok(7u8)
                    }
                }
            },
            |error| error.to_string().contains("deadlock"),
        )
        .await
        .unwrap();
        assert_eq!(value, 7);
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn deadlock_retry_window_matches_cpp() {
        assert_eq!(
            SqlTransaction::deadlock_max_retry_time_like_cpp_for_test(),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn transaction_accepts_raw_sql_and_cleans_up_once_like_cpp() {
        let mut tx = SqlTransaction::new();
        tx.append_raw_sql_like_cpp("DELETE FROM characters WHERE guid = 7");

        assert_eq!(tx.len(), 1);
        assert_eq!(
            tx.sqls_for_test(),
            vec!["DELETE FROM characters WHERE guid = 7"]
        );
        assert!(!tx.cleaned_up_like_cpp());

        tx.cleanup_like_cpp();
        assert!(tx.is_empty());
        assert!(tx.cleaned_up_like_cpp());

        tx.append_raw_sql_like_cpp("DELETE FROM character_inventory WHERE guid = 7");
        assert_eq!(tx.len(), 1);
        tx.cleanup_like_cpp();
        assert_eq!(tx.len(), 1);
    }

    #[test]
    fn transaction_tracks_affected_row_contracts_fail_closed() {
        let mut tx = SqlTransaction::new();
        tx.append(PreparedStatement::raw_sql_like_cpp(
            "UPDATE item_instance SET count = 2 WHERE guid = 1",
        ));
        tx.append_expect_rows_affected(
            PreparedStatement::raw_sql_like_cpp(
                "DELETE FROM item_loot_items WHERE container_id = 1",
            ),
            1,
        );

        assert_eq!(tx.expected_rows_for_test(), vec![None, Some(1)]);
        assert!(validate_rows_affected(1, 1, 1).is_ok());
        let error = validate_rows_affected(1, 1, 0).unwrap_err();
        assert!(matches!(error, DatabaseError::Transaction(_)));
        assert_eq!(
            error.to_string(),
            "transaction failed: statement 1 affected 0 rows; expected exactly 1"
        );
    }

    #[test]
    fn item_guid_allocator_lock_domain_is_stable_private_and_database_scoped() {
        let first = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters");
        let same = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters");
        let other = ItemGuidAllocatorAdvisoryLockLikeCpp::lock_name_for_test("characters_qa");

        assert_eq!(first, same);
        assert_ne!(first, other);
        assert!(first.starts_with("rustycore:item-guid:"));
        assert!(!first.contains("characters"));
        assert!(
            first.len() <= 64,
            "MySQL named locks are limited to 64 bytes"
        );
    }

    #[test]
    fn commit_outcome_unknown_remains_distinct_from_definite_rollback() {
        let unknown = SqlTransactionCommitError::CommitOutcomeUnknown(DatabaseError::Transaction(
            "connection lost after COMMIT".to_string(),
        ));
        let rollback = SqlTransactionCommitError::DefinitelyRolledBack(DatabaseError::Transaction(
            "statement failed before COMMIT".to_string(),
        ));

        assert!(unknown.is_commit_outcome_unknown_like_cpp());
        assert!(!rollback.is_commit_outcome_unknown_like_cpp());
        assert!(unknown.to_string().contains("outcome is unknown"));
        assert!(rollback.to_string().contains("rolled back"));
    }
}
