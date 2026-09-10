//! Persistence-trace regressions.
//!
//! Moved out of persistence_trace.rs under #685; every test is unchanged.

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
fn moving_a_publication_before_its_commit_changes_the_trace() {
    // The issue's acceptance criterion: "a fixture that reorders
    // commit/publication ... fails". C++ gates publication on the commit
    // callback -- `CharacterHandler.cpp:907` sends the packet and inserts
    // the character-cache entry only inside `AfterComplete(success)` -- so
    // publishing first is a different durability contract, not a cosmetic
    // reordering, and the trace has to be able to tell them apart.
    let durable_then_visible = PersistenceTrace {
        events: vec![
            PersistenceEvent::Commit {
                database: LogicalDatabase::Character,
                outcome: CommitOutcome::Committed,
            },
            PersistenceEvent::Publication {
                label: "flow.client".to_owned(),
            },
        ],
    };
    let visible_then_durable = PersistenceTrace {
        events: vec![
            PersistenceEvent::Publication {
                label: "flow.client".to_owned(),
            },
            PersistenceEvent::Commit {
                database: LogicalDatabase::Character,
                outcome: CommitOutcome::Committed,
            },
        ],
    };

    assert_ne!(
        durable_then_visible, visible_then_durable,
        "publishing before the commit must not compare equal to publishing after it"
    );
    assert_ne!(
        durable_then_visible.to_golden().expect("render"),
        visible_then_durable.to_golden().expect("render"),
        "the rendered golden must distinguish them too, since that is what is committed"
    );
}

#[test]
fn a_publication_reaches_the_trace_from_production() {
    // record_publication is what production calls; before it existed the
    // variant was built only in this module's tests, so no production
    // publication could appear in any trace.
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    record_publication("flow.client");

    let trace = recorder.snapshot();
    assert_eq!(
        trace.events,
        vec![PersistenceEvent::Publication {
            label: "flow.client".to_owned()
        }],
        "the publication must be the recorded event: {trace:?}"
    );
}

#[test]
fn an_explicit_guard_cancelled_while_committing_reports_unknown() {
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    // The money paths open a transaction directly on the pool and guard it
    // with this type. Dropping it unresolved meant `Rollback`, which is
    // right up to the moment COMMIT goes out and wrong after it: the server
    // may have applied it, and a golden saying otherwise would approve a
    // retry that duplicates the write. `SqlTransaction` already made this
    // distinction; this guard is the other half.
    {
        let mut trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
        trace.committing();
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
        "a cancelled commit is unknown, not rolled back: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, PersistenceEvent::Rollback { .. })),
        "and must not also claim a rollback: {events:?}"
    );
}

#[test]
fn an_explicit_guard_cancelled_before_committing_still_reports_rollback() {
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    // Nothing was committed, so the transaction really did roll back. The
    // other direction matters as much: widening `Unknown` to cover this
    // would lose a fact the trace had.
    {
        let _trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
    }

    let events = recorder.take().events;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, PersistenceEvent::Rollback { .. })),
        "an abandoned guard before COMMIT is a rollback: {events:?}"
    );
}

#[test]
fn a_nested_recording_gives_the_outer_one_back() {
    let _serialized = capture_flag_test_lock();
    let outer = PersistenceRecorder::new();
    let outer_session = RecordingSession::install(outer.clone());

    {
        // A helper that installs its own session used to clear the ambient
        // slot on drop instead of restoring it, so everything the caller
        // recorded afterwards went nowhere and its trace ended early with
        // nothing to say it had.
        let inner = PersistenceRecorder::new();
        let _inner_session = RecordingSession::install(inner.clone());
        record_publication("inner.only");
        assert_eq!(inner.snapshot().events.len(), 1, "the inner one records");
    }

    record_publication("outer.after.nesting");
    let events = outer.take().events;
    assert_eq!(
        events,
        vec![PersistenceEvent::Publication {
            label: "outer.after.nesting".to_owned()
        }],
        "the outer recording must resume, and must not have the inner event: {events:?}"
    );
    drop(outer_session);
}

#[test]
fn the_ambient_lookup_is_free_when_nothing_is_recording() {
    // Every Database::query and execute reaches `ambient_recorder`. Taking
    // the process-wide mutex to learn there is no recorder put a global
    // serialization point on the normal production path, so the flag is
    // checked first. Asserting the answer, since asserting the absence of a
    // lock is not something a test can see directly.
    let _serialized = capture_flag_test_lock();
    assert!(
        !recording_enabled(),
        "no recording is installed at the start of this test"
    );
    assert!(
        ambient_recorder().is_none(),
        "and the lookup answers without one"
    );

    let recorder = PersistenceRecorder::new();
    let session = RecordingSession::install(recorder);
    assert!(
        ambient_recorder().is_some(),
        "the fast path must not hide a recorder that is installed"
    );
    drop(session);
    assert!(ambient_recorder().is_none(), "nor keep one that is gone");
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
    assert_eq!(statement.trace_identity(), "GENERATED_BASE:area_table:2");
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

    // The table alone is too coarse: a key-only query and a full-row query
    // against the same table produce different results, so swapping them
    // must move the trace.
    let keys_only = HotfixStatements::base("SELECT ID FROM area_table WHERE ID = ?");
    assert_ne!(statement.trace_identity(), keys_only.trace_identity());
}

#[test]
fn an_abandoned_explicit_transaction_still_records_its_end() {
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    // The whole point of the guard: an early return through `?` drops the
    // transaction, SQLx rolls it back, and no hand-written hook runs. A
    // trace that ended with an open transaction would misrepresent the
    // retry boundary.
    {
        let _trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
    }

    assert_eq!(
        recorder.take().events,
        vec![
            PersistenceEvent::TransactionBegin {
                database: LogicalDatabase::Character
            },
            PersistenceEvent::Rollback {
                database: LogicalDatabase::Character
            },
        ]
    );
}

#[test]
fn an_inert_guard_costs_nothing_and_records_nothing() {
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    // No RecordingSession: this is the production configuration.
    assert!(!recording_enabled());

    let built = std::cell::Cell::new(false);
    {
        let trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
        trace.statement(|| {
            built.set(true);
            ("UPD_CHAR_MONEY".to_owned(), Vec::new())
        });
        trace.committed(CommitOutcome::Committed);
    }

    assert!(
        !built.get(),
        "the closure must not run with capture off: building an identity and a \
         parameter vector for every payout recipient is the cost this facility \
         promises not to have"
    );
    assert!(recorder.take().events.is_empty());
}

#[test]
fn a_resolved_explicit_transaction_records_its_outcome_once() {
    let _serialized = capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    let trace = ExplicitTransactionTrace::open(LogicalDatabase::Character);
    trace.committed(CommitOutcome::Unknown);

    assert_eq!(
        recorder.take().events,
        vec![
            PersistenceEvent::TransactionBegin {
                database: LogicalDatabase::Character
            },
            PersistenceEvent::Commit {
                database: LogicalDatabase::Character,
                outcome: CommitOutcome::Unknown
            },
        ],
        "a resolved transaction must not also record a dropped rollback"
    );
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
