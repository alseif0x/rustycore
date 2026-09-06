//! Regressions for sql classification.

use super::*;

#[test]
fn persistence_inventory_matches_advisory_lock_sql_case_insensitively() {
    let baseline = inventory(
        r#"
                fn persistent() {
                    sqlx::query("select get_lock('k', 0)");
                }
                fn clean() {
                    sqlx::query("select 1");
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::Sqlx
            && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_reads_arithmetic_double_hyphen_as_sql() {
    let baseline = inventory(
        r#"
                fn arithmetic() {
                    sqlx::query("SELECT 1--1, GET_LOCK('live', 0)");
                }
                fn remark() {
                    sqlx::query("SELECT 1 -- GET_LOCK('inert', 0)");
                }
                fn quoted_identifier() {
                    sqlx::query("SELECT \"GET_LOCK('x', 0)\"");
                }
            "#,
    )
    .unwrap();
    // `--` opens a comment only before whitespace; here it is arithmetic.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn arithmetic" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    for enclosing in ["fn remark", "fn quoted_identifier"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} treated inert text as a call"
        );
    }
}

#[test]
fn persistence_inventory_classifies_advisory_locks_pinned_at_the_call_site() {
    let baseline = inventory(
        r#"
                const LOCK_SQL: &str = "SELECT GET_LOCK(?, 0)";
                const READ_SQL: &str = "SELECT 1";
                fn literal_at_the_call_site() {
                    sqlx::query("SELECT GET_LOCK('direct', 0)");
                }
                fn pinned_constant() {
                    sqlx::query(LOCK_SQL);
                }
                fn executable_comment() {
                    sqlx::query("/*!50000 SELECT GET_LOCK('versioned', 0) */");
                }
                fn inert_comment() {
                    sqlx::query("SELECT 1 /* GET_LOCK is intentionally not used */");
                }
                fn pinned_read() {
                    sqlx::query(READ_SQL);
                }
            "#,
    )
    .unwrap();
    for enclosing in [
        "fn literal_at_the_call_site",
        "fn pinned_constant",
        "fn executable_comment",
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost the advisory identity of a pinned statement"
        );
    }
    for enclosing in ["fn inert_comment", "fn pinned_read"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} was classified from inert text"
        );
    }
}

#[test]
fn persistence_inventory_lexes_pinned_statements_like_mysql() {
    let baseline = inventory(
        r#"
                fn escaped_quote() {
                    sqlx::query("SELECT 'it\\'s', GET_LOCK('escaped', 0)");
                }
                fn doubled_quote() {
                    sqlx::query("SELECT 'it''s', GET_LOCK('doubled', 0)");
                }
                fn concatenated() {
                    sqlx::query(concat!("SELECT GET_", "LOCK('joined', 0)"));
                }
                fn hash_without_space() {
                    sqlx::query("SELECT 1# GET_LOCK('inert', 0)");
                }
                fn quoted_span() {
                    sqlx::query("SELECT 'GET_LOCK('");
                }
            "#,
    )
    .unwrap();
    // A quote closed by an escape or a doubling does not swallow the call
    // that follows it, and `concat!` really does join its arguments.
    for enclosing in ["fn escaped_quote", "fn doubled_quote", "fn concatenated"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost a call the database makes"
        );
    }
    // `#` opens a comment with no preceding whitespace, and text inside a
    // quoted span is data.
    for enclosing in ["fn hash_without_space", "fn quoted_span"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} read inert text as a call"
        );
    }
}

#[test]
fn persistence_inventory_reads_sql_quoting_and_routine_context() {
    let baseline = inventory(
        r#"
                fn backtick_identifier() {
                    sqlx::query("SELECT `col\\`, GET_LOCK('backtick', 0)");
                }
                fn stored_routine() {
                    sqlx::query("CALL GET_LOCK('routine', 0)");
                }
                fn schema_routine() {
                    sqlx::query("SELECT app.GET_LOCK('schema', 0)");
                }
                fn builtin() {
                    sqlx::query("SELECT GET_LOCK('builtin', 0)");
                }
            "#,
    )
    .unwrap();
    // A backslash is an ordinary character inside a backtick identifier, so
    // the call after it is still the built-in.
    for enclosing in ["fn backtick_identifier", "fn builtin"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost the built-in call"
        );
    }
    // A routine of the same name is not the built-in whose connection
    // affinity this classification stands for.
    for enclosing in ["fn stored_routine", "fn schema_routine"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} classified a routine as the built-in"
        );
    }
}

#[test]
fn persistence_inventory_reads_quoting_under_either_sql_mode() {
    let baseline = inventory(
        r#"
                fn escaping_mode() {
                    sqlx::query("SELECT 'it\\'s', GET_LOCK('escaped', 0)");
                }
                fn no_backslash_escapes_mode() {
                    sqlx::query("SELECT 'x\\', GET_LOCK('literal', 0)");
                }
            "#,
    )
    .unwrap();
    // No `sql_mode` is pinned, so a statement whose quoting ends the token
    // differently under `NO_BACKSLASH_ESCAPES` keeps its identity too.
    for enclosing in ["fn escaping_mode", "fn no_backslash_escapes_mode"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost a call that one supported mode executes"
        );
    }
}

#[test]
fn persistence_inventory_reads_full_mysql_identifiers() {
    let baseline = inventory(
        r#"
                fn extended_identifier() {
                    sqlx::query("SELECT éGET_LOCK('x', 0)");
                }
                fn dollar_identifier() {
                    sqlx::query("SELECT app$GET_LOCK('x', 0)");
                }
                fn builtin() {
                    sqlx::query("SELECT GET_LOCK('x', 0)");
                }
            "#,
    )
    .unwrap();
    // MySQL identifiers admit `$` and characters beyond ASCII, so a routine
    // whose name merely ends in the built-in's is a different function.
    for enclosing in ["fn extended_identifier", "fn dollar_identifier"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} read a different routine as the built-in"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn builtin" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_reads_extended_identifier_characters() {
    let baseline = inventory(
        r#"
                fn middle_dot() {
                    sqlx::query("SELECT app\u{b7}GET_LOCK('x', 0)");
                }
                fn builtin() {
                    sqlx::query("SELECT GET_LOCK('x', 0)");
                }
            "#,
    )
    .unwrap();
    // MySQL admits extended characters in unquoted identifiers, so the
    // routine is not the built-in.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn middle_dot" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn builtin" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_recognizes_all_named_lock_functions() {
    let baseline = inventory(
        r#"
                fn locks() {
                    sqlx::query("SELECT GET_LOCK('x', 1)");
                    sqlx::query("SELECT RELEASE_LOCK('x')");
                    sqlx::query("SELECT IS_USED_LOCK('x')");
                    sqlx::query("SELECT IS_FREE_LOCK('x')");
                    sqlx::query("SELECT RELEASE_ALL_LOCKS()");
                }
            "#,
    )
    .unwrap();
    assert_eq!(
        baseline
            .accesses
            .iter()
            .filter(|row| row.enclosing == "fn locks"
                && row.operation == PersistenceOperation::AdvisoryLock)
            .count(),
        5
    );
}
