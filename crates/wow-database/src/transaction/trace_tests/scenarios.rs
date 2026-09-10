//! Transaction regressions.
//!
//! Moved out of transaction.rs under #683; every test is unchanged.

use super::*;

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
fn committing_a_typed_batch_through_the_wrong_adapter_is_recorded() {
    use crate::persistence_trace::RecordingSession;

    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let recorder = PersistenceRecorder::new();
    let _recording = RecordingSession::install(recorder.clone());

    // `SqlTransaction` erases the statement family, so a batch of Character
    // statements compiles when handed to `Database<LoginStatements>`. The
    // SQL runs on Login while the trace says Character; recording the
    // contradiction is the point of tracking connection affinity at all.
    let mut trans = SqlTransaction::new();
    trans.append(PreparedStatement::for_statement(
        CharStatements::UPD_CHAR_MONEY,
    ));
    trans.attribute_to_like_cpp(LogicalDatabase::Login);

    let events = recorder.take().events;
    assert!(
        events.iter().any(|event| matches!(
            event,
            PersistenceEvent::MixedLogicalDatabases {
                opened: LogicalDatabase::Character,
                appended: LogicalDatabase::Login,
            }
        )),
        "a batch committed through another database's adapter must say so: {events:?}"
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
            let mut stmt = PreparedStatement::new("UPDATE characters SET money = ? WHERE guid = ?");
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
    // Exclude ambient capture tests while exercising the production opt-out.
    let _serialized = crate::persistence_trace::capture_flag_test_lock();
    let mut trans = SqlTransaction::new();
    trans.append(PreparedStatement::for_statement(CharStatements::SEL_ENUM));
    assert!(crate::persistence_trace::ambient_recorder().is_none());
}
