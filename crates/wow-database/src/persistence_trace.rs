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
/// which is all a golden needs. It is not a security primitive and is not used
/// as one: it exists so a changed secret moves the trace without appearing in
/// it.
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
    /// Project a bound parameter into its redacted trace form.
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
#[serde(tag = "event", rename_all = "snake_case")]
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
    },
    /// Raw SQL appended without a statement enum. Recorded by shape only: the
    /// text may be dynamic, and a golden that pinned it would break on
    /// reformatting.
    RawStatement {
        database: LogicalDatabase,
        connection: ConnectionAffinity,
        digest: u64,
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
    _capture: RecordingGuard,
}

impl RecordingSession {
    pub fn install(recorder: PersistenceRecorder) -> Self {
        if let Ok(mut ambient) = AMBIENT.lock() {
            *ambient = Some(recorder);
        }
        Self {
            _capture: RecordingGuard::enable(),
        }
    }
}

impl Drop for RecordingSession {
    fn drop(&mut self) {
        if let Ok(mut ambient) = AMBIENT.lock() {
            *ambient = None;
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
    if let Some(recorder) = ambient_recorder() {
        recorder.record(PersistenceEvent::Statement {
            database,
            connection: ConnectionAffinity::Transaction,
            statement: statement.to_owned(),
            params,
            expected_rows_affected: None,
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
mod tests {
    use super::*;

    #[test]
    fn text_and_bytes_are_recorded_without_their_content() {
        let secret = SqlParam::String("hunter2-session-key".to_owned());
        let traced = TracedParam::from_param(&secret);
        let rendered = serde_json::to_string(&traced).expect("serialize");

        assert!(
            !rendered.contains("hunter2"),
            "a traced parameter must never carry its text: {rendered}"
        );
        match traced {
            TracedParam::Text { len, .. } => assert_eq!(len, "hunter2-session-key".len()),
            other => panic!("expected redacted text, got {other:?}"),
        }
    }

    #[test]
    fn a_changed_secret_still_moves_the_trace() {
        // Redaction is worthless for a golden if two different values look the
        // same, so the digest has to separate them.
        let first = TracedParam::from_param(&SqlParam::String("account-a".to_owned()));
        let second = TracedParam::from_param(&SqlParam::String("account-b".to_owned()));
        assert_ne!(first, second);

        let same = TracedParam::from_param(&SqlParam::String("account-a".to_owned()));
        assert_eq!(first, same, "the digest must be stable across calls");
    }

    #[test]
    fn numeric_parameters_keep_their_value_and_signedness() {
        assert_eq!(
            TracedParam::from_param(&SqlParam::I32(-7)),
            TracedParam::Int {
                value: -7,
                width_bits: 32
            }
        );
        assert_eq!(
            TracedParam::from_param(&SqlParam::U32(7)),
            TracedParam::Uint {
                value: 7,
                width_bits: 32
            }
        );
        // A widening bind is a different bound parameter, exactly as with
        // floats: sqlx sends different MySQL type metadata for each width.
        assert_ne!(
            TracedParam::from_param(&SqlParam::I32(7)),
            TracedParam::from_param(&SqlParam::I64(7))
        );
        assert_ne!(
            TracedParam::from_param(&SqlParam::U8(7)),
            TracedParam::from_param(&SqlParam::U64(7))
        );
        // A signed -1 and an unsigned u64::MAX share a bit pattern but are not
        // the same bound parameter.
        assert_ne!(
            TracedParam::from_param(&SqlParam::I64(-1)),
            TracedParam::from_param(&SqlParam::U64(u64::MAX))
        );
    }

    #[test]
    fn a_float_bind_keeps_its_width() {
        // sqlx sends different MySQL type metadata for a 4-byte and an 8-byte
        // bind, so a zero is not simply a zero.
        assert_ne!(
            TracedParam::from_param(&SqlParam::F32(0.0)),
            TracedParam::from_param(&SqlParam::F64(0.0))
        );
    }

    #[test]
    fn floats_compare_by_bits_so_a_golden_survives_formatting() {
        assert_eq!(
            TracedParam::from_param(&SqlParam::F64(f64::NAN)),
            TracedParam::from_param(&SqlParam::F64(f64::NAN))
        );
        assert_ne!(
            TracedParam::from_param(&SqlParam::F64(0.0)),
            TracedParam::from_param(&SqlParam::F64(-0.0)),
            "positive and negative zero are different bound values"
        );
    }

    fn money_plan() -> PersistenceTrace {
        PersistenceTrace {
            events: vec![
                PersistenceEvent::TransactionBegin {
                    database: LogicalDatabase::Character,
                },
                PersistenceEvent::Statement {
                    database: LogicalDatabase::Character,
                    connection: ConnectionAffinity::Transaction,
                    statement: "UPD_CHARACTER_MONEY".to_owned(),
                    params: vec![TracedParam::Uint {
                        value: 100,
                        width_bits: 64,
                    }],
                    expected_rows_affected: Some(1),
                },
                PersistenceEvent::Commit {
                    database: LogicalDatabase::Character,
                    outcome: CommitOutcome::Committed,
                },
                PersistenceEvent::Publication {
                    label: "money".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn a_golden_round_trips() {
        let trace = money_plan();
        let golden = trace.to_golden().expect("render");
        assert!(golden.ends_with('\n'), "goldens are newline terminated");
        assert_eq!(
            PersistenceTrace::from_golden(&golden).expect("parse"),
            trace
        );
    }

    #[test]
    fn publishing_before_the_commit_is_a_different_trace() {
        // The whole point of freezing order: these two plans write identical
        // rows and differ only in what a crash between the steps would leave
        // behind.
        let expected = money_plan();
        let mut reordered = expected.clone();
        reordered.events.swap(2, 3);
        assert_ne!(expected, reordered);
    }

    #[test]
    fn changing_connection_affinity_is_a_different_trace() {
        let expected = money_plan();
        let mut escaped = expected.clone();
        if let Some(PersistenceEvent::Statement { connection, .. }) = escaped.events.get_mut(1) {
            *connection = ConnectionAffinity::Pooled;
        }
        assert_ne!(
            expected, escaped,
            "a statement leaving the transaction's connection must not compare equal"
        );
    }

    #[test]
    fn an_unknown_commit_is_not_a_rollback() {
        let expected = money_plan();
        let mut unknown = expected.clone();
        if let Some(PersistenceEvent::Commit { outcome, .. }) = unknown.events.get_mut(2) {
            *outcome = CommitOutcome::Unknown;
        }
        assert_ne!(expected, unknown);

        let mut rolled_back = expected.clone();
        if let Some(PersistenceEvent::Commit { outcome, .. }) = rolled_back.events.get_mut(2) {
            *outcome = CommitOutcome::RolledBack;
        }
        assert_ne!(unknown, rolled_back);
    }

    #[test]
    fn the_recorder_preserves_order_and_can_be_drained() {
        let recorder = PersistenceRecorder::new();
        for event in money_plan().events {
            recorder.record(event);
        }

        let snapshot = recorder.snapshot();
        assert_eq!(snapshot, money_plan());
        assert_eq!(
            recorder.snapshot(),
            money_plan(),
            "snapshot must not consume"
        );

        assert_eq!(recorder.take(), money_plan());
        assert!(recorder.take().events.is_empty(), "take must drain");
    }

    #[test]
    fn the_recording_guard_restores_the_previous_state() {
        let _serialized = capture_flag_test_lock();
        // Nested guards must not leave capture on for unrelated tests.
        assert!(!recording_enabled(), "capture is off by default");
        {
            let _outer = RecordingGuard::enable();
            assert!(recording_enabled());
            {
                let _inner = RecordingGuard::enable();
                assert!(recording_enabled());
            }
            assert!(recording_enabled(), "the outer guard still holds it");
        }
        assert!(
            !recording_enabled(),
            "dropping the outer guard restores off"
        );
    }

    #[test]
    fn prepare_captures_the_variant_name_only_while_recording() {
        use crate::statements::{CharStatements, StatementDef};

        let _serialized = capture_flag_test_lock();

        // The identity is the variant, not the SQL: that is what survives a
        // reformat of the query or a rename of the file holding it.
        assert_eq!(
            CharStatements::SEL_ENUM.trace_identity(),
            "SEL_ENUM",
            "identity must be the statement-enum variant"
        );
        assert_eq!(
            CharStatements::SEL_ENUM.logical_database(),
            LogicalDatabase::Character
        );
        assert!(
            !CharStatements::SEL_ENUM.trace_identity().contains("SELECT"),
            "identity must not embed SQL text"
        );

        // Production default: no capture, no allocation.
        assert!(!recording_enabled());
        let untraced = crate::params::PreparedStatement::for_statement(CharStatements::SEL_ENUM);
        assert_eq!(untraced.trace_identity(), None);

        let _capture = RecordingGuard::enable();
        let traced = crate::params::PreparedStatement::for_statement(CharStatements::SEL_ENUM)
            .with_trace_identity(CharStatements::SEL_ENUM.trace_identity());
        assert_eq!(traced.trace_identity(), Some("SEL_ENUM"));
    }

    #[test]
    fn a_generated_statement_is_identified_by_its_cpp_name_not_its_sql() {
        use crate::statements::{CharStatements, StatementDef};

        // `GENERATED_CPP` carries its SQL, so the derived `Debug` identity
        // would embed the whole query and every golden would move on a
        // reformat — the precise coupling this contract exists to avoid.
        let statement = CharStatements::cpp(
            "CHAR_SEL_CHARACTER_MONEY",
            "SELECT money FROM characters WHERE guid = ?",
        );
        assert_eq!(statement.trace_identity(), "CHAR_SEL_CHARACTER_MONEY");
        assert!(
            !statement.trace_identity().contains("SELECT"),
            "a generated statement must not be identified by its SQL"
        );

        // Reformatting the SQL must not move the identity.
        let reformatted = CharStatements::cpp(
            "CHAR_SEL_CHARACTER_MONEY",
            "SELECT  money\n  FROM characters\n  WHERE guid = ?",
        );
        assert_eq!(
            statement.trace_identity(),
            reformatted.trace_identity(),
            "identity must survive a formatting-only change"
        );

        // Two different generated statements must still be distinguishable.
        let other = CharStatements::cpp(
            "CHAR_SEL_CHARACTER_NAME",
            "SELECT name FROM characters WHERE guid = ?",
        );
        assert_ne!(statement.trace_identity(), other.trace_identity());
    }

    #[test]
    fn a_generated_hotfix_statement_is_identified_by_its_table() {
        use crate::statements::{HotfixStatements, StatementDef};

        // Same defect as `GENERATED_CPP`, in the variant I did not check the
        // first time: `GENERATED_BASE` carries its SQL.
        let statement = HotfixStatements::base("SELECT ID, Field FROM area_table WHERE ID = ?");
        assert_eq!(statement.trace_identity(), "GENERATED_BASE:area_table");
        assert!(
            !statement.trace_identity().contains("SELECT"),
            "a generated hotfix statement must not be identified by its SQL"
        );

        let reformatted =
            HotfixStatements::base("SELECT  ID, Field\n  FROM area_table\n  WHERE ID = ?");
        assert_eq!(
            statement.trace_identity(),
            reformatted.trace_identity(),
            "identity must survive a formatting-only change"
        );

        let other = HotfixStatements::base("SELECT ID FROM spell_name WHERE ID = ?");
        assert_ne!(statement.trace_identity(), other.trace_identity());
    }

    #[test]
    fn recorders_share_one_recording_when_cloned() {
        let recorder = PersistenceRecorder::new();
        let handed_to_a_plan = recorder.clone();
        handed_to_a_plan.record(PersistenceEvent::Fence {
            label: "character-save".to_owned(),
        });
        assert_eq!(recorder.snapshot().events.len(), 1);
    }
}
