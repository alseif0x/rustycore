//! Regressions for sql strings.

use super::*;

#[test]
fn persistence_inventory_classifies_compile_time_string_macros_as_static_sql() {
    let baseline = inventory(
        r#"
                fn persistent() {
                    consume(sqlx::query(concat!("SELECT ", "* FROM account")));
                }
                fn aliased() {
                    use std::concat as static_sql;
                    consume(sqlx::query(static_sql!("SELECT ", "1")));
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn persistent", "fn aliased"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::Query
        }));
        assert!(!baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::NonliteralSql
        }));
    }
}

#[test]
fn persistence_inventory_rejects_nested_external_sql_sources() {
    for source in [
        r#"fn hidden() { consume(sqlx::query(concat!(env!("QUERY"), " LIMIT 1"))); }"#,
        r#"fn hidden() { consume(sqlx::query(concat!(include_str!("query.sql"), " LIMIT 1"))); }"#,
    ] {
        assert!(inventory(source).is_err(), "{source}");
    }
}

#[test]
fn persistence_inventory_fingerprints_bound_static_sql() {
    let first = inventory(
        r#"
                const LOCK_SQL: &str = "SELECT GET_LOCK('one', 0)";
                fn local() { let sql = "SELECT 1"; consume(sqlx::query(sql)); }
                fn local_const() { const SQL: &str = "SELECT 3"; consume(sqlx::query(SQL)); }
                fn constant(db: wow_database::CharacterDatabase) { db.direct_query(LOCK_SQL); }
            "#,
    )
    .unwrap();
    let second = inventory(
        r#"
                const LOCK_SQL: &str = "SELECT GET_LOCK('two', 0)";
                fn local() { let sql = "SELECT 2"; consume(sqlx::query(sql)); }
                fn local_const() { const SQL: &str = "SELECT 4"; consume(sqlx::query(SQL)); }
                fn constant(db: wow_database::CharacterDatabase) { db.direct_query(LOCK_SQL); }
            "#,
    )
    .unwrap();
    let fingerprints = |baseline: &PersistenceAccessBaseline, enclosing: &str| {
        baseline
            .accesses
            .iter()
            .filter(|row| row.enclosing == enclosing)
            .map(|row| row.fingerprint.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_ne!(
        fingerprints(&first, "fn local"),
        fingerprints(&second, "fn local")
    );
    assert_ne!(
        fingerprints(&first, "fn constant"),
        fingerprints(&second, "fn constant")
    );
    assert_ne!(
        fingerprints(&first, "fn local_const"),
        fingerprints(&second, "fn local_const")
    );
    assert!(first.accesses.iter().any(|row| {
        row.enclosing == "fn constant" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_tracks_sql_appended_to_string_bindings() {
    let baseline = inventory(
        r#"
                fn appended() {
                    let mut sql = String::from("SELECT 1");
                    sql.push_str(" GET_LOCK('inventory', 0)");
                    consume(sqlx::query(&sql));
                }
                fn inserted() {
                    let mut sql = String::from("SELECT 1");
                    sql.insert_str(sql.len(), " GET_LOCK('inventory', 0)");
                    consume(sqlx::query(&sql));
                }
                fn replaced() {
                    let mut sql = String::from("SELECT 1");
                    sql.replace_range(.., "SELECT GET_LOCK('inventory', 0)");
                    consume(sqlx::query(&sql));
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn appended", "fn inserted", "fn replaced"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
        }));
    }
}

#[test]
fn persistence_inventory_tracks_sql_written_by_formatting_macros() {
    let baseline = inventory(
        r#"
                fn formatted() {
                    let mut sql = String::from("SELECT 1");
                    write!(&mut sql, " GET_LOCK('formatted', 0)").unwrap();
                    sqlx::query(&sql);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn formatted"
            && row.operation == PersistenceOperation::AdvisoryLock
            && row.fingerprint.contains("GET_LOCK")
    }));
}

#[test]
fn persistence_inventory_preserves_concat_argument_order() {
    // Swapping two statements changes the write order; a union of the
    // arguments would read the same either way.
    let collect = |first: &str, second: &str| {
        inventory(&format!(
            r#"
                    const SQL: &str = concat!({first:?}, {second:?});
                    fn concatenated() {{
                        sqlx::raw_sql(SQL);
                    }}
                "#
        ))
        .unwrap()
    };
    let fingerprint = |baseline: &PersistenceAccessBaseline| {
        baseline
            .accesses
            .iter()
            .find(|row| {
                row.enclosing == "fn concatenated" && row.operation == PersistenceOperation::Query
            })
            .map(|row| row.fingerprint.clone())
            .unwrap()
    };
    assert_ne!(
        fingerprint(&collect("UPDATE a SET x=1;", "DELETE FROM b;")),
        fingerprint(&collect("DELETE FROM b;", "UPDATE a SET x=1;"))
    );
}

#[test]
fn persistence_inventory_classifies_advisory_locks_by_call_not_substring() {
    let baseline = inventory(
        r#"
                const GET_LOCK_SQL: &str = "SELECT 1";
                fn remarked() {
                    sqlx::query("SELECT 1 /* GET_LOCK is intentionally not used */");
                }
                fn quoted() {
                    sqlx::query("SELECT 'GET_LOCK'");
                }
                fn named_binding() {
                    sqlx::query(GET_LOCK_SQL);
                }
                fn spaced_call() {
                    sqlx::query("select get_lock (?, ?)");
                }
                fn executed_in_comment() {
                    sqlx::query("SELECT /*!50000 GET_LOCK('live', 0) */");
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn remarked", "fn quoted", "fn named_binding"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} was classified as an advisory lock without calling one"
        );
    }
    for enclosing in ["fn spaced_call", "fn executed_in_comment"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost its advisory identity"
        );
    }
}

#[test]
fn persistence_inventory_ratchets_runtime_assembled_sql_without_claiming_content() {
    // The grammar reads pinned statements; it does not evaluate what an
    // expression would build. Every shape below assembles SQL at run time,
    // so none of them may carry a content claim — and each must still be
    // ratcheted, as interpolated or nonliteral SQL, so the reviewed
    // workflow annotation covering it cannot silently disappear.
    let baseline = inventory(
        r#"
                const LOCK_SQL: &str = "SELECT GET_LOCK('x', 0)";
                struct Statements { sql: &'static str }
                fn added() {
                    let sql = String::from("SELECT GET_") + "LOCK('x', 0)";
                    sqlx::query(&sql);
                }
                fn formatted(name: &str) {
                    let sql = format!("SELECT GET_LOCK('{}', 0)", name);
                    sqlx::query(&sql);
                }
                fn branched(flag: bool) {
                    let sql = if flag { LOCK_SQL } else { "SELECT 1" };
                    sqlx::query(sql);
                }
                fn returned() -> &'static str {
                    LOCK_SQL
                }
                fn from_helper() {
                    sqlx::query(returned());
                }
                fn projected() {
                    let statements = Statements { sql: LOCK_SQL };
                    sqlx::query(statements.sql);
                }
            "#,
    )
    .unwrap();
    for enclosing in [
        "fn added",
        "fn formatted",
        "fn branched",
        "fn from_helper",
        "fn projected",
    ] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} claimed an advisory lock it cannot prove"
        );
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && matches!(
                        row.operation,
                        PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
                    )
            }),
            "{enclosing} lost the ratchet on its assembled statement"
        );
    }
}

#[test]
fn persistence_inventory_keeps_module_constants_static() {
    let baseline = inventory(
        r#"
                const SQL: &str = "SELECT 1";
                fn pinned() {
                    sqlx::query(SQL);
                }
            "#,
    )
    .unwrap();
    // The statement is pinned, so it must not be reported as assembled at
    // run time and pushed into a workflow classification it does not need.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn pinned"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_rejects_environment_sql_inside_concat_constants() {
    let baseline = inventory(
        r#"
                const SQL: &str = concat!(env!("SQL_PREFIX"), "SELECT 1");
                fn from_environment() {
                    sqlx::query(SQL);
                }
            "#,
    );
    // The expanded prefix is outside the snapshot, so the constant is not a
    // pinned statement and the inventory must refuse it outright rather
    // than fingerprint a statement it cannot see.
    let error = baseline
        .err()
        .expect("a constant built from the environment must be rejected");
    assert!(
        error.contains("env!") && error.contains("fn from_environment"),
        "unexpected rejection: {error}"
    );
}

#[test]
fn persistence_inventory_renders_every_concat_literal() {
    let baseline = inventory(
        r#"
                fn spliced() {
                    sqlx::query(concat!("SELECT GET", 1, "_LOCK('x', 0)"));
                }
                fn joined() {
                    sqlx::query(concat!("SELECT GET_", "LOCK('x', 0)"));
                }
            "#,
    )
    .unwrap();
    // `concat!` renders the integer, so the pieces do not close over it and
    // `GET1_LOCK` is not the advisory function.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn spliced" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn joined" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_pins_imported_and_qualified_constants() {
    let facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "queries-a",
        module: "crate::queries",
        source_path: "src/queries.rs",
        inherited_cfg: &[],
        source: r#"pub const SQL: &str = "SELECT 1";"#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "queries-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::queries::SQL;
                fn imported() {
                    sqlx::query(SQL);
                }
                fn qualified() {
                    sqlx::query(crate::queries::SQL);
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[facade, consumer]).unwrap();
    // However the constant is named, the statement is pinned and must not
    // be reported as assembled at run time.
    for enclosing in ["fn imported", "fn qualified"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && matches!(
                        row.operation,
                        PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
                    )
            }),
            "{enclosing} reported a pinned constant as runtime-assembled"
        );
    }
}

#[test]
fn persistence_inventory_expands_compile_time_string_macros() {
    let baseline = inventory(
        r#"
                fn stringified() {
                    sqlx::query(concat!(stringify!(SELECT GET_LOCK), "('x', 0)"));
                }
                fn boolean_piece() {
                    sqlx::query(concat!("SELECT GET", true, "_LOCK('x', 0)"));
                }
                fn integer_piece() {
                    sqlx::query(concat!("SELECT GET", 1, "_LOCK('x', 0)"));
                }
            "#,
    )
    .unwrap();
    // `stringify!` renders its tokens, so the call it spells is executed.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn stringified" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    // A rendered piece separates the strings around it, so no call is
    // fabricated where the expansion has none.
    for enclosing in ["fn boolean_piece", "fn integer_piece"] {
        assert!(
            !baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} fabricated a call the expansion does not contain"
        );
    }
}

#[test]
fn persistence_inventory_grants_static_sql_only_to_the_standard_concat() {
    let baseline = inventory(
        r#"
                mod other {
                    macro_rules! concat { ($($piece:tt)*) => { String::new() }; }
                    pub(crate) use concat;
                }
                fn custom_macro() {
                    sqlx::query(other::concat!("SELECT 1"));
                }
                fn standard_macro() {
                    sqlx::query(concat!("SELECT ", "1"));
                }
            "#,
    )
    .unwrap();
    // Somebody's macro of the same name expands to whatever it likes, so
    // its result is not a pinned statement.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn custom_macro"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn standard_macro"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_pins_associated_constants() {
    let collect = |sql: &str| {
        inventory(&format!(
            r#"
                    struct Statements;
                    impl Statements {{
                        const SQL: &str = {sql:?};
                    }}
                    fn associated() {{
                        sqlx::query(Statements::SQL);
                    }}
                "#
        ))
        .unwrap()
    };
    let locked = collect("SELECT GET_LOCK('associated', 0)");
    let clean = collect("SELECT 1");
    let fingerprint = |baseline: &PersistenceAccessBaseline| {
        baseline
            .accesses
            .iter()
            .find(|row| {
                row.enclosing == "fn associated" && row.operation == PersistenceOperation::Query
            })
            .map(|row| row.fingerprint.clone())
            .unwrap()
    };
    assert_ne!(fingerprint(&locked), fingerprint(&clean));
    assert!(locked.accesses.iter().any(|row| {
        row.enclosing == "fn associated" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_follows_inline_string_conversions() {
    let collect = |sql: &str| {
        inventory(&format!(
            r#"
                    const SQL: &str = {sql:?};
                    fn converted(pool: sqlx::MySqlPool) {{
                        sqlx::query(SQL.to_owned().as_str()).execute(&pool);
                    }}
                "#
        ))
        .unwrap()
    };
    let fingerprint = |baseline: &PersistenceAccessBaseline| {
        baseline
            .accesses
            .iter()
            .find(|row| {
                row.enclosing == "fn converted" && row.operation == PersistenceOperation::Query
            })
            .map(|row| row.fingerprint.clone())
            .unwrap()
    };
    // A pinned statement is no less pinned for being converted on the way
    // in, so editing the constant has to move the row.
    assert_ne!(
        fingerprint(&collect("SELECT GET_LOCK('converted', 0)")),
        fingerprint(&collect("SELECT 1"))
    );
}

#[test]
fn persistence_inventory_pins_block_constants_declared_after_use() {
    let collect = |sql: &str| {
        inventory(&format!(
            r#"
                    fn run() {{
                        sqlx::query(SQL);
                        const SQL: &str = {sql:?};
                    }}
                "#
        ))
        .unwrap()
    };
    let fingerprint = |baseline: &PersistenceAccessBaseline| {
        baseline
            .accesses
            .iter()
            .find(|row| row.enclosing == "fn run" && row.operation == PersistenceOperation::Query)
            .map(|row| row.fingerprint.clone())
            .unwrap()
    };
    // A block constant is in scope throughout its block, declaration order
    // notwithstanding.
    assert_ne!(
        fingerprint(&collect("SELECT GET_LOCK('late', 0)")),
        fingerprint(&collect("SELECT 1"))
    );
}

/// Characterizes the scope a body sees. The shadows visible where the item
/// is written are threaded into its analysis; this shape already behaved
/// correctly, so the test records the contract rather than a fix.
#[test]
fn persistence_inventory_resolves_qualified_and_converted_constants() {
    let baseline = inventory(
        r#"
                trait CleanSql { const SQL: &'static str; }
                trait LockingSql { const SQL: &'static str; }
                struct Statements;
                impl CleanSql for Statements {
                    const SQL: &'static str = "SELECT 1";
                }
                impl LockingSql for Statements {
                    const SQL: &'static str = "SELECT GET_LOCK('locking', 0)";
                }
                impl Statements {
                    const INHERENT: &str = "SELECT GET_LOCK('inherent', 0)";
                }
                fn clean() {
                    sqlx::query(<Statements as CleanSql>::SQL);
                }
                fn locking() {
                    sqlx::query(<Statements as LockingSql>::SQL);
                }
                fn converted() {
                    sqlx::query(String::from(Statements::INHERENT).as_str());
                }
            "#,
    )
    .unwrap();
    // The qualification selects which impl's value is used, so a lock
    // defined by one trait must not reach a query naming the other.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    for enclosing in ["fn locking", "fn converted"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost the constant it names"
        );
    }
}

#[test]
fn persistence_inventory_requires_the_standard_string_conversion() {
    let baseline = inventory(
        r#"
                const SAFE_SQL: &str = "SELECT 1";
                struct String;
                impl String {
                    fn from(_ignored: &str) -> &'static str {
                        "SELECT GET_LOCK('substituted', 0)"
                    }
                }
                fn shadowed_conversion() {
                    sqlx::query(String::from(SAFE_SQL));
                }
            "#,
    )
    .unwrap();
    // A type of one's own named `String` returns whatever it likes, so the
    // argument's source must not be carried through it.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn shadowed_conversion"
            && row.operation == PersistenceOperation::Query
            && !row.fingerprint.contains("sql-source")
    }));
}

#[test]
fn persistence_inventory_refuses_qualified_custom_string_conversions() {
    let baseline = inventory(
        r#"
                const SAFE_SQL: &str = "SELECT 1";
                mod custom {
                    pub struct String;
                    impl String {
                        pub fn from(_ignored: &str) -> &'static str {
                            "SELECT GET_LOCK('substituted', 0)"
                        }
                    }
                }
                fn qualified_custom() {
                    sqlx::query(custom::String::from(SAFE_SQL));
                }
            "#,
    )
    .unwrap();
    // A `from` of somebody's own type returns whatever it likes.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn qualified_custom"
            && row.operation == PersistenceOperation::Query
            && !row.fingerprint.contains("sql-source")
    }));
}

#[test]
fn persistence_inventory_classifies_sql_parameters_as_nonliteral() {
    let baseline = inventory(
        r#"
                fn dynamic(db: wow_database::CharacterDatabase, query: &str) {
                    db.direct_query(query);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn dynamic" && row.operation == PersistenceOperation::NonliteralSql
    }));
}

#[test]
fn persistence_inventory_preserves_constant_visibility() {
    let baseline = inventory(
        r#"
                pub const DATABASE: Option<wow_database::CharacterDatabase> = None;
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "const DATABASE"
            && row.operation == PersistenceOperation::TypeReference
            && row.visibility == "pub"
    }));
}

#[test]
fn persistence_inventory_preserves_interpolated_sql_through_string_views() {
    let baseline = inventory(
        r#"
                fn dynamic(id: u32) {
                    sqlx::query(format!("SELECT {id}").as_str());
                    sqlx::query(format!("SELECT {id}").as_ref());
                }
            "#,
    )
    .unwrap();
    assert_eq!(
        baseline
            .accesses
            .iter()
            .filter(|row| {
                row.enclosing == "fn dynamic"
                    && row.operation == PersistenceOperation::InterpolatedSql
            })
            .count(),
        2
    );
}
