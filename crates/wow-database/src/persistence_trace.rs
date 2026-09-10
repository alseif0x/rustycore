//! Executable record of a persistence plan's observable behaviour.
//!
//! Rows are not the contract. Crash and retry semantics are decided by
//! *statement order*, transaction boundaries, which logical database and
//! connection a step ran on, how a failed `COMMIT` was classified, and what was
//! published after it. Moving persistence behind ports can preserve every final
//! row and still break all of that, so the order has to be frozen before the
//! move rather than reconstructed afterwards.
//!
//! What is recorded is deliberately *semantic*:
//!
//! * Statements are identified by their statement-enum variant — the analogue
//!   of C++'s `CharacterDatabaseStatements` — never by SQL text. Reformatting a
//!   query, or renaming the file it lives in, must not move a trace.
//! * Parameters are recorded by type. Numbers keep their value because that is
//!   what distinguishes one plan from another; strings and blobs keep only
//!   their length and a digest, so a golden can detect a changed value without
//!   ever storing an account name, a token or a password hash.
//!
//! The recorder never holds its lock across I/O: every event is appended and
//! the guard dropped before the caller awaits anything.
//!
//! # Coverage is incomplete — do not freeze a golden on this yet
//!
//! The vocabulary and the mechanism are in place; the *coverage* of real
//! persistence paths is not, and an incomplete trace that looks complete is
//! worse than none, because absence of events reads as absence of persistence.
//! Three paths are currently invisible, all tracked in issue #213:
//!
//! * **Nine indirect statement builders** that do not name their variant
//!   inline, so `for_statement` could not be applied mechanically.
//! * **Raw pooled SQL**, which carries no logical database and is skipped
//!   rather than attributed to a guessed one.
//! * **Parameter redaction is not proof against a dictionary.** Length plus an
//!   unsalted digest can be matched for a low-entropy value, so a trace is
//!   safe to read but should not be treated as safe to publish.
//! * **Concurrent transactions cannot be correlated.** Events carry their
//!   logical database but no transaction id, so two transactions on one
//!   database interleave indistinguishably, and the ambient recorder is
//!   process-wide rather than scoped to the traced task.
//!
//! The paths that hid whole durable operations are now recorded: explicit
//! `pool().begin()` transactions, manually built statements, generated hotfix
//! statements, and the advisory-lock lifetime.
//!
//! Until those are closed, a golden built from this recorder can approve a
//! refactor that breaks the very persistence it claims to protect.

use crate::params::SqlParam;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Logical database a step ran against.
///
/// This is the ownership fact the ports must preserve; two steps on different
/// logical databases can never be made one atomic unit later.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalDatabase {
    Login,
    Character,
    World,
    Hotfix,
}

impl LogicalDatabase {
    /// Stable wire name used in goldens.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Character => "character",
            Self::World => "world",
            Self::Hotfix => "hotfix",
        }
    }
}

/// Which connection carried a step.
///
/// Independent connections cannot share a transaction, so collapsing these is
/// the exact mistake a port extraction can make invisibly.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionAffinity {
    /// Taken from the pool for this step alone.
    Pooled,
    /// The connection owned by the enclosing transaction.
    Transaction,
    /// A connection held for the lifetime of a lock, outside any pool.
    DedicatedLock,
}

/// How a commit attempt ended.
///
/// `Unknown` is the one that matters: a transport error on `COMMIT` leaves the
/// server unable to say whether the work landed, and C++ reconciles that with a
/// durable token rather than assuming either outcome.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitOutcome {
    Committed,
    RolledBack,
    Unknown,
}

/// A parameter as it appears in a trace.
///
/// Numeric parameters keep their value; text and blobs do not. A golden must be
/// able to prove that a plan bound a different value without the repository
/// storing that value, because these plans carry account names, session keys
/// and password verifiers.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TracedParam {
    Null,
    Bool {
        value: bool,
    },
    Int {
        value: i64,
        width_bits: u8,
    },
    Uint {
        value: u64,
        width_bits: u8,
    },
    /// Floats are recorded by bit pattern: a golden must not depend on decimal
    /// formatting, and `NaN` has to compare equal to itself here.
    ///
    /// The width is part of the record because sqlx sends different MySQL type
    /// metadata for a 4-byte and an 8-byte bind, so `F32(0.0)` and `F64(0.0)`
    /// are not the same bound parameter even though both are zero.
    Float {
        bits: u64,
        width_bits: u8,
    },
    Text {
        len: usize,
        digest: u64,
    },
    Bytes {
        len: usize,
        digest: u64,
    },
}

/// FNV-1a. Small, dependency-free, and stable across runs and platforms —
/// which is all a golden needs.
///
/// This detects change; it does not withstand an adversary, and the difference
/// matters enough to state plainly. A golden has to be deterministic, so the
/// same input must always produce the same digest — which means a low-entropy
/// value such as an account name can be recovered by hashing a dictionary and
/// comparing, and the recorded length narrows the search first. Salting would
/// close that and destroy determinism with it; the two properties are not
/// simultaneously available.
///
/// So the guarantee is narrower than "redacted": a value does not appear in the
/// trace, and changing it moves the trace. Traces must therefore be recorded
/// from fixtures rather than from production credentials, which is a constraint
/// on how goldens are produced, not a property this function provides.
pub(crate) fn digest(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Shape of a raw SQL statement, for traces that must not pin its formatting.
pub fn raw_statement_digest(sql: &str) -> u64 {
    digest(sql.as_bytes())
}

impl TracedParam {
    /// Project a bound parameter into its trace form.
    ///
    /// Values are replaced by shape: numerics by value and width, text and
    /// blobs by length and digest. See [`digest`] for what that does and does
    /// not protect against.
    pub fn from_param(param: &SqlParam) -> Self {
        match param {
            SqlParam::Null => Self::Null,
            SqlParam::Bool(value) => Self::Bool { value: *value },
            SqlParam::I8(value) => Self::Int {
                value: i64::from(*value),
                width_bits: 8,
            },
            SqlParam::I16(value) => Self::Int {
                value: i64::from(*value),
                width_bits: 16,
            },
            SqlParam::I32(value) => Self::Int {
                value: i64::from(*value),
                width_bits: 32,
            },
            SqlParam::I64(value) => Self::Int {
                value: *value,
                width_bits: 64,
            },
            SqlParam::U8(value) => Self::Uint {
                value: u64::from(*value),
                width_bits: 8,
            },
            SqlParam::U16(value) => Self::Uint {
                value: u64::from(*value),
                width_bits: 16,
            },
            SqlParam::U32(value) => Self::Uint {
                value: u64::from(*value),
                width_bits: 32,
            },
            SqlParam::U64(value) => Self::Uint {
                value: *value,
                width_bits: 64,
            },
            SqlParam::F32(value) => Self::Float {
                bits: u64::from(value.to_bits()),
                width_bits: 32,
            },
            SqlParam::F64(value) => Self::Float {
                bits: value.to_bits(),
                width_bits: 64,
            },
            SqlParam::String(value) => Self::Text {
                len: value.len(),
                digest: digest(value.as_bytes()),
            },
            SqlParam::Bytes(value) => Self::Bytes {
                len: value.len(),
                digest: digest(value),
            },
        }
    }
}

/// One observable step of a persistence plan.
///
/// The variants are the facts a port extraction must not silently change. They
/// are ordered by occurrence in [`PersistenceTrace`], and that order *is* the
/// contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
// Without this, dropping a field from an event would let an existing golden
// still parse — the assertion it carried would be silently discarded and the
// reduced trace would compare equal, defeating the guard exactly when contract
// information is being lost.
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum PersistenceEvent {
    /// A transaction was opened. Everything until its commit or rollback shares
    /// one connection and lands or fails together.
    TransactionBegin {
        database: LogicalDatabase,
    },
    /// A statement was appended or executed.
    Statement {
        database: LogicalDatabase,
        connection: ConnectionAffinity,
        /// Statement-enum variant, e.g. `UPD_CHARACTER_MONEY`.
        statement: String,
        params: Vec<TracedParam>,
        /// Present when the affected-row count is part of the correctness
        /// contract rather than incidental.
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_rows_affected: Option<u64>,
        /// Rows the statement actually affected, when it ran on its own pooled
        /// connection and the caller can see the number.
        ///
        /// Recorded because callers branch on it: a save that matches no
        /// character row is a different outcome from one that matches a row,
        /// and without this `Ok(0)` and `Ok(1)` trace identically.
        #[serde(skip_serializing_if = "Option::is_none")]
        observed_rows_affected: Option<u64>,
    },
    /// Raw SQL appended without a statement enum. Recorded by shape only: the
    /// text may be dynamic, and a golden that pinned it would break on
    /// reformatting.
    RawStatement {
        database: LogicalDatabase,
        connection: ConnectionAffinity,
        digest: u64,
        /// Bound values, projected exactly as a semantic statement's are.
        ///
        /// The digest covers the SQL text, which for a raw statement is a
        /// constant: the bank-slot purchase binds new money, slot count and
        /// character GUID into the same string every time, so a trace carrying
        /// only the digest is identical whatever those values are.
        params: Vec<TracedParam>,
    },
    /// Raw SQL that arrived before anything identified the transaction's
    /// database, so the trace cannot say which one it ran against.
    ///
    /// Recorded rather than dropped. A transaction built entirely from raw SQL
    /// would otherwise be absent from its own trace, and a golden asserting
    /// "this flow persists nothing" is worse than one admitting it could not
    /// attribute a statement: the first is wrong, the second is incomplete and
    /// says so.
    UnattributedRawStatement {
        connection: ConnectionAffinity,
        digest: u64,
        params: Vec<TracedParam>,
    },
    /// An advisory lock was taken or released on its own dedicated connection.
    AdvisoryLock {
        label: String,
        acquired: bool,
    },
    /// A commit attempt resolved. `Unknown` must be reconciled by the caller.
    Commit {
        database: LogicalDatabase,
        outcome: CommitOutcome,
    },
    Rollback {
        database: LogicalDatabase,
    },
    /// A commit was retried after a deadlock. C++ serializes these under one
    /// process-wide lock, so their presence and count are observable.
    DeadlockRetry {
        database: LogicalDatabase,
        attempt: u32,
    },
    /// Two logical databases appeared in one transaction.
    ///
    /// They can never be one atomic unit, so a trace that recorded a single
    /// boundary here would describe a guarantee the server cannot make. The
    /// contradiction is recorded rather than smoothed over.
    MixedLogicalDatabases {
        opened: LogicalDatabase,
        appended: LogicalDatabase,
    },
    /// A batch that was planned and then dropped without ever executing.
    ///
    /// Statements are recorded as they are appended, which is the plan the
    /// caller built. When the caller then returns without committing -- the
    /// vendor-currency turn-in does exactly this if it cannot take the money
    /// lock -- nothing reached the database, and a trace that stopped after the
    /// statements would describe writes that never happened.
    BatchAbandoned {
        database: LogicalDatabase,
    },
    /// A point the plan must not cross until prior work is durable.
    Fence {
        label: String,
    },
    /// State made visible to clients or other sessions after a commit. Its
    /// position relative to `Commit` is the crash-window contract.
    Publication {
        label: String,
    },
}

/// An ordered recording of one persistence plan.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PersistenceTrace {
    pub events: Vec<PersistenceEvent>,
}

impl PersistenceTrace {
    /// Render the trace as the golden's canonical pretty JSON.
    pub fn to_golden(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map(|mut rendered| {
                rendered.push('\n');
                rendered
            })
            .map_err(|error| format!("cannot serialize persistence trace: {error}"))
    }

    /// Parse a golden previously produced by [`Self::to_golden`].
    pub fn from_golden(source: &str) -> Result<Self, String> {
        serde_json::from_str(source)
            .map_err(|error| format!("cannot parse persistence trace golden: {error}"))
    }
}

/// Serializes tests that install a recording. Capture and the ambient
/// recorder are both process-wide, so two recording tests running in parallel
/// would see each other's events.
#[cfg(test)]
pub(crate) fn capture_flag_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The recorder every transaction picks up while a recording is installed.
///
/// Seventy-five call sites build transactions across the server. Threading a
/// recorder through all of them would be a far larger change than the contract
/// it serves, and would leave the ones nobody updated silently untraced — so
/// the recorder is ambient and `SqlTransaction::new` finds it.
static AMBIENT: Mutex<Option<PersistenceRecorder>> = Mutex::new(None);

/// The installed recorder, if a recording is in progress.
pub fn ambient_recorder() -> Option<PersistenceRecorder> {
    // Checked before the lock, not after. Every `Database::query` and
    // `Database::execute` reaches here, so taking a process-wide mutex to
    // discover that nothing is being recorded put a global serialization point
    // on the normal production path -- the one mode where the recorder is
    // supposed to cost nothing.
    //
    // Safe against a session starting or ending concurrently, because of the
    // order the two pieces of state move in: install sets the recorder and
    // *then* the flag, so a caller that sees the flag always finds the
    // recorder; drop restores the recorder and *then* the flag, so a caller in
    // between sees the flag but reads the restored slot. Neither window
    // records into a recorder that is going away.
    if !recording_enabled() {
        return None;
    }
    AMBIENT
        .lock()
        .ok()
        .and_then(|recorder| recorder.as_ref().cloned())
}

/// Installs `recorder` as ambient and enables capture until dropped.
///
/// Both pieces of state are process-wide, so tests that record must serialize
/// against each other.
#[derive(Debug)]
pub struct RecordingSession {
    /// Restored on drop, so nesting is transparent to the outer recording.
    previous: Option<PersistenceRecorder>,
    _capture: RecordingGuard,
}

impl RecordingSession {
    pub fn install(recorder: PersistenceRecorder) -> Self {
        // The previous recorder is kept, not discarded. `RecordingGuard`
        // already restores the previous *flag* on drop; clearing the recorder
        // instead of restoring it meant a nested session left the outer one
        // recording into nothing, so a helper that installs its own session
        // silently truncated the trace its caller was collecting.
        let previous = AMBIENT
            .lock()
            .ok()
            .and_then(|mut ambient| ambient.replace(recorder));
        Self {
            previous,
            _capture: RecordingGuard::enable(),
        }
    }
}

impl Drop for RecordingSession {
    fn drop(&mut self) {
        if let Ok(mut ambient) = AMBIENT.lock() {
            *ambient = self.previous.take();
        }
    }
}

/// Whether statement identities are being captured.
///
/// Deriving a statement's identity costs an allocation, and prepare sits on
/// every query path, so production does not pay for it. Recording is a test and
/// QA facility: enable it with [`RecordingGuard`], which restores the previous
/// state on drop so one test cannot leave it on for another.
static RECORDING: AtomicBool = AtomicBool::new(false);

/// Whether persistence tracing is currently capturing statement identities.
pub fn recording_enabled() -> bool {
    RECORDING.load(Ordering::Relaxed)
}

/// Enables identity capture for as long as it is held.
///
/// The flag is process-wide, so a test that needs it must also serialize
/// against other tests that read traces.
#[derive(Debug)]
pub struct RecordingGuard {
    previous: bool,
}

impl RecordingGuard {
    pub fn enable() -> Self {
        Self {
            previous: RECORDING.swap(true, Ordering::Relaxed),
        }
    }
}

impl Drop for RecordingGuard {
    fn drop(&mut self) {
        RECORDING.store(self.previous, Ordering::Relaxed);
    }
}

/// Traces an explicitly opened transaction for its whole lifetime.
///
/// SQLx rolls an unfinished transaction back when it drops, so *every* early
/// return ends the transaction — including ones added later. Annotating each
/// return site records the ones someone remembered and silently omits the
/// rest, which is how a trace ends up showing a transaction that opened and
/// never closed. A guard cannot be forgotten: if it drops without being
/// resolved, the rollback is recorded.
#[derive(Debug)]
pub struct ExplicitTransactionTrace {
    database: LogicalDatabase,
    resolved: bool,
    /// Whether COMMIT has been issued and its answer not yet seen.
    committing: bool,
}

impl ExplicitTransactionTrace {
    /// Record the boundary and start guarding it.
    pub fn open(database: LogicalDatabase) -> Self {
        if !recording_enabled() {
            // Inert: nothing to record, and Drop must not record either.
            return Self {
                database,
                resolved: true,
                committing: false,
            };
        }
        record_explicit_transaction_begin(database);
        Self {
            database,
            resolved: true,
            committing: false,
        }
        .armed()
    }

    /// Mark that COMMIT has been issued and its answer is outstanding.
    ///
    /// Called immediately before awaiting the commit, so a cancellation in that
    /// window is recorded as an unknown outcome rather than a rollback: the
    /// server may have applied it, and a trace claiming otherwise would let a
    /// retry duplicate the write.
    pub fn committing(&mut self) {
        self.committing = true;
    }

    fn armed(mut self) -> Self {
        self.resolved = false;
        self
    }

    /// The logical database this transaction runs on.
    pub fn database(&self) -> LogicalDatabase {
        self.database
    }

    /// Record a statement inside this transaction.
    ///
    /// Takes a closure because the arguments are the expensive part: an identity
    /// is a `String` and the parameters are a `Vec`, and the group payout builds
    /// both for every recipient. Evaluating them before discovering that no
    /// recorder is installed would put two allocations and a mutex acquisition
    /// on a production money path, which is exactly the cost this facility
    /// promises not to have.
    pub fn statement<F>(&self, build: F)
    where
        F: FnOnce() -> (String, Vec<TracedParam>),
    {
        if !recording_enabled() {
            return;
        }
        let (statement, params) = build();
        record_explicit_statement(self.database, &statement, params);
    }

    /// Record a statement whose asserted affected-row count is part of the
    /// contract, so dropping that assertion moves the trace.
    pub fn statement_expecting<F>(&self, build: F, expected: u64)
    where
        F: FnOnce() -> (String, Vec<TracedParam>),
    {
        if !recording_enabled() {
            return;
        }
        let (statement, params) = build();
        record_explicit_statement_expecting(self.database, &statement, params, Some(expected));
    }

    /// Record how the commit attempt resolved.
    pub fn committed(mut self, outcome: CommitOutcome) {
        record_explicit_commit(self.database, outcome);
        self.resolved = true;
    }

    /// Record a deliberate rollback.
    pub fn rolled_back(mut self) {
        record_explicit_rollback(self.database);
        self.resolved = true;
    }
}

impl Drop for ExplicitTransactionTrace {
    fn drop(&mut self) {
        if self.resolved {
            return;
        }
        if self.committing {
            // Cancelled while awaiting COMMIT. The server may have applied it,
            // so `Rollback` would state that work was undone when it may have
            // been kept -- and a golden carrying that would approve a retry
            // which duplicates it. Same distinction `SqlTransaction` makes; this
            // guard is the other half of it.
            record_explicit_commit(self.database, CommitOutcome::Unknown);
            return;
        }
        record_explicit_rollback(self.database);
    }
}

/// Record the boundary of a transaction opened directly on a pool.
///
/// Some durable workflows need `SELECT ... FOR UPDATE` inside the transaction
/// and therefore cannot be expressed as an [`SqlTransaction`]; they call
/// `pool().begin()` instead. Without these hooks their entire durable
/// operation is invisible to a trace, which is worse than not tracing them at
/// all — the trace would look complete.
pub fn record_explicit_transaction_begin(database: LogicalDatabase) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::TransactionBegin { database });
    }
}

/// Record a statement executed inside an explicitly opened transaction.
pub fn record_explicit_statement(
    database: LogicalDatabase,
    statement: &str,
    params: Vec<TracedParam>,
) {
    record_explicit_statement_expecting(database, statement, params, None);
}

/// Record such a statement together with the affected-row count its caller
/// asserts.
///
/// The count is part of the correctness contract wherever a workflow rejects
/// anything but an exact match — the money mutations all do. Without it, a
/// refactor that dropped those guards would leave a successful trace unchanged,
/// which `SqlTransaction::append_expect_rows_affected` already refuses to allow.
pub fn record_explicit_statement_expecting(
    database: LogicalDatabase,
    statement: &str,
    params: Vec<TracedParam>,
    expected_rows_affected: Option<u64>,
) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Statement {
            database,
            connection: ConnectionAffinity::Transaction,
            statement: statement.to_owned(),
            params,
            expected_rows_affected,
            observed_rows_affected: None,
        });
    }
}

/// Record how an explicitly opened transaction resolved.
pub fn record_explicit_commit(database: LogicalDatabase, outcome: CommitOutcome) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Commit { database, outcome });
    }
}

/// Record an explicitly opened transaction abandoned before any commit.
pub fn record_explicit_rollback(database: LogicalDatabase) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Rollback { database });
    }
}

/// Record the acquisition or release of an advisory lock.
///
/// The lock lives on its own dedicated connection for the life of the process,
/// so it is neither pooled nor part of any transaction, and losing it is an
/// observable persistence event.
pub fn record_advisory_lock(label: &str, acquired: bool) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::AdvisoryLock {
            label: label.to_owned(),
            acquired,
        });
    }
}

/// Records a durable workflow that never opened its transaction.
///
/// The explicitly traced money paths guard their transaction with
/// [`ExplicitTransactionTrace`], which is constructed only after
/// `pool().begin()` succeeds. When the connection cannot be acquired the guard
/// never exists and the trace stays empty, so a definite non-execution reads
/// exactly like the workflow never being reached -- and only one of those is
/// safe to retry.
pub fn record_batch_not_started(database: LogicalDatabase) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::BatchAbandoned { database });
    }
}

/// Records a point the plan must not cross until prior work is durable.
pub fn record_fence(label: &str) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Fence {
            label: label.to_owned(),
        });
    }
}

/// Records state being made visible to clients or other sessions.
///
/// The event's whole purpose is its position relative to `Commit`: that
/// ordering is the crash window. Until production called this, moving,
/// removing or duplicating a publication produced an identical trace, so a
/// golden could approve exactly the change it exists to catch.
pub fn record_publication(label: &str) {
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Publication {
            label: label.to_owned(),
        });
    }
}

/// Handle used by production code to append events.
///
/// Cloning shares one recording. The mutex is taken only to push an event and
/// is always released before the caller awaits, so no lock is ever held across
/// database I/O.
#[derive(Clone, Debug, Default)]
pub struct PersistenceRecorder {
    events: Arc<Mutex<Vec<PersistenceEvent>>>,
}

impl PersistenceRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one event. Never awaits, never blocks on I/O.
    pub fn record(&self, event: PersistenceEvent) {
        // A poisoned recorder must not take the server down: it is an
        // observation facility, and losing a trace is preferable to
        // propagating a panic through a persistence path.
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    /// Take the recording so far, leaving the recorder empty.
    pub fn take(&self) -> PersistenceTrace {
        let events = self
            .events
            .lock()
            .map(|mut events| std::mem::take(&mut *events))
            .unwrap_or_default();
        PersistenceTrace { events }
    }

    /// Read the recording without consuming it.
    pub fn snapshot(&self) -> PersistenceTrace {
        let events = self
            .events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default();
        PersistenceTrace { events }
    }
}

#[cfg(test)]
#[path = "persistence_trace/tests/mod.rs"]
mod tests;
