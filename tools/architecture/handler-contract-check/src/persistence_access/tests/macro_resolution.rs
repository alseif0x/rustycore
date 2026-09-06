//! Regressions for macro resolution.

use super::*;

#[test]
fn persistence_inventory_propagates_vec_macro_result_flow() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let values = vec![database];
                    consume(values[0].pool());
                }
                fn clean() {
                    let values = vec![1_u8];
                    consume(values[0]);
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(
        |row| row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    ));
}

#[test]
fn persistence_inventory_resolves_database_paths_in_opaque_macro_tokens() {
    let baseline = inventory(
        r#"
                fn persistent() {
                    assert!(wow_database::CharacterDatabase::open("dsn").is_ok());
                }
                fn clean() {
                    assert!(true);
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::MacroReference
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_inventories_every_registered_macro_invocation() {
    let baseline = inventory(
        r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
                macro_rules! forwarded_query { ($sql:expr) => { sqlx::query($sql) } }
                const LOCK_SQL: &str = "SELECT GET_LOCK('macro', 0)";
                fn persistent() {
                    consume(hidden_query!());
                    consume(forwarded_query!(LOCK_SQL));
                }
                fn clean() {
                    consume(1_u8);
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::Sqlx
            && row.operation == PersistenceOperation::MacroReference
            && row.symbol == "hidden_query"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.operation == PersistenceOperation::AdvisoryLock
            && row.symbol == "forwarded_query"
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_preserves_join_macro_result_flow() {
    let baseline = inventory(
        r#"
                async fn persistent(database: wow_database::CharacterDatabase) {
                    let (database,) = tokio::join!(async { database });
                    consume(database.pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_resolves_registered_callables_inside_join_macros() {
    let baseline = inventory(
        r#"
                fn database() -> wow_database::CharacterDatabase { unreachable!() }
                async fn persistent() {
                    let database = tokio::join!(async { database() }).0;
                    consume(database.pool());
                }
            "#,
    )
    .unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn persistence_inventory_does_not_treat_macro_identifiers_as_function_calls() {
    let baseline = inventory(
        r#"
                fn error() -> wow_database::CharacterDatabase { unreachable!() }
                async fn clean(message: &str) {
                    let value = tokio::join!(async { tracing::error!(message) }).0;
                    consume(value.pool());
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(
        |row| row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    ));
}

#[test]
fn persistence_inventory_rejects_unmounted_include_sources() {
    let error = inventory(r#"include!("db_impl.rs");"#).unwrap_err();
    assert!(
        error.contains("include! whose Rust source is outside"),
        "{error}"
    );
    let body_error = inventory(r#"fn hidden() { include!("db_impl.rs"); }"#).unwrap_err();
    assert!(
        body_error.contains("include! whose Rust source is outside"),
        "{body_error}"
    );

    let pinned = |suffix: &str| {
        let source = format!(
            "pub mod bgs {{ pub mod protocol {{ include!(concat!(env!(\"OUT_DIR\"), {suffix:?})); }} }}"
        );
        inventory_persistence_accesses(&[ClassifiedPersistenceSource {
            classification: "direct_application_or_domain_access",
            package: "wow-proto",
            module: "crate",
            source_path: "crates/wow-proto/src/lib.rs",
            inherited_cfg: &[],
            source: &source,
        }])
    };
    pinned("/bgs.protocol.rs").expect("the exact pinned generated include is accepted");
    let changed = pinned("/unreviewed.rs").unwrap_err();
    assert!(
        changed.contains("include! whose Rust source is outside"),
        "{changed}"
    );
}

#[test]
fn persistence_inventory_rejects_unmounted_include_str_sql() {
    for source in [
        r#"fn direct() { consume(sqlx::query(include_str!("query.sql"))); }"#,
        r#"fn aliased() { let sql = include_str!("query.sql"); consume(sqlx::query(sql)); }"#,
    ] {
        let error = inventory(source).unwrap_err();
        assert!(error.contains("include_str! SQL"), "{error}");
    }
}

#[test]
fn persistence_inventory_rejects_environment_sourced_sql() {
    let error = inventory(r#"fn hidden() { consume(sqlx::query(env!("QUERY"))); }"#).unwrap_err();
    assert!(error.contains("passes env! SQL"), "{error}");
}

#[test]
fn persistence_inventory_reads_aliased_macros_through_their_canonical_name() {
    let baseline = inventory(
        r#"
                struct Row;
                fn block_local_alias() {
                    use sqlx::query_as as q;
                    q!(Row, "SELECT GET_LOCK('block', 0)");
                }
                fn aliased_concat() {
                    use std::concat as c;
                    sqlx::query(c!("SELECT GET_", "LOCK('joined', 0)"));
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn block_local_alias", "fn aliased_concat"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost a statement hidden behind an alias"
        );
    }
}

#[test]
fn persistence_inventory_resolves_only_unqualified_macro_aliases() {
    let baseline = inventory(
        r#"
                use std::concat as c;
                mod other {
                    macro_rules! c { ($($piece:tt)*) => { "SELECT 1" }; }
                    pub(crate) use c;
                }
                fn qualified_namesake() {
                    sqlx::query(other::c!("SELECT GET_", "LOCK('x', 0)"));
                }
            "#,
    )
    .unwrap();
    // `other::c!` shares a leaf name with the import but is a different
    // macro, so its arguments must not be joined into a call.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn qualified_namesake"
            && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_resolves_item_macros_before_pinning_them() {
    let baseline = inventory(
        r#"
                mod other {
                    macro_rules! concat { ($($piece:tt)*) => { "SELECT 1" }; }
                    pub(crate) use concat;
                }
                const CUSTOM: &str = other::concat!();
                const STANDARD: &str = concat!("SELECT ", "1");
                fn from_custom() {
                    sqlx::query(CUSTOM);
                }
                fn from_standard() {
                    sqlx::query(STANDARD);
                }
            "#,
    )
    .unwrap();
    // A namesake macro can expand to anything, so the constant it defines
    // is not a pinned statement.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn from_custom"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn from_standard"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_rejects_unmounted_migrations() {
    let baseline = inventory(
        r#"
                async fn migrated(pool: sqlx::MySqlPool) {
                    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
                }
            "#,
    );
    // The migration files execute SQL this inventory never reads, so the
    // invocation cannot be baselined as if it were pinned.
    let error = baseline
        .err()
        .expect("an unmounted migration directory must be rejected");
    assert!(
        error.contains("migrate!") && error.contains("fn migrated"),
        "unexpected rejection: {error}"
    );
}

#[test]
fn persistence_inventory_resolves_block_local_and_aliased_macros() {
    let baseline = inventory(
        r#"
                mod other {
                    macro_rules! concat { ($($piece:tt)*) => { "SELECT 1" }; }
                    macro_rules! stringify { ($($piece:tt)*) => { "SELECT 1" }; }
                    pub(crate) use {concat, stringify};
                }
                fn block_local_item() {
                    use other::concat;
                    const SQL: &str = concat!();
                    sqlx::query(SQL);
                }
                fn namesake_stringify() {
                    sqlx::query(other::stringify!());
                }
            "#,
    )
    .unwrap();
    // A namesake resolved through a block-local import is not the standard
    // macro, so neither statement is pinned.
    for enclosing in ["fn block_local_item", "fn namesake_stringify"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && matches!(
                        row.operation,
                        PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
                    )
            }),
            "{enclosing} pinned a statement a macro definition can change"
        );
    }
}

#[test]
fn persistence_inventory_rejects_aliased_include_mounts() {
    let baseline = inventory(
        r#"
                use std::include as mount;
                mount!("db_impl.rs");
            "#,
    );
    // An alias does not change what `include!` brings in, and its contents
    // are outside the inventory.
    assert!(
        baseline
            .err()
            .is_some_and(|error| error.contains("include!")),
        "an aliased include! was accepted"
    );
}

#[test]
fn persistence_inventory_reads_prelude_macros_inside_modules() {
    let source = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "nested-a",
        module: "crate::legacy",
        source_path: "src/legacy.rs",
        inherited_cfg: &[],
        source: r#"
                const SQL: &str = concat!("SELECT GET_LOCK('", "nested', 0)");
                fn nested() {
                    sqlx::query(SQL);
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[source]).unwrap();
    // Inside a module an unqualified builtin resolves to the module's own
    // path; that is still the prelude macro, and its statement is pinned.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn nested" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn nested" && row.operation == PersistenceOperation::NonliteralSql
    }));
}

#[test]
fn persistence_inventory_treats_a_local_namesake_macro_as_opaque() {
    let baseline = inventory(
        r#"
                macro_rules! concat { () => { "SELECT GET_LOCK('shadow', 0)" }; }
                const SQL: &str = concat!();
                fn shadowed() {
                    sqlx::query(SQL);
                }
            "#,
    )
    .unwrap();
    // A module that defines its own `concat!` shadows the prelude one, and
    // the invocation shows none of what it expands to.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn shadowed"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_shadows_a_macro_only_from_its_declaration() {
    let baseline = inventory(
        r#"
                const EARLY: &str = concat!("SELECT GET_LOCK('early', 0)");
                macro_rules! concat { () => { "SELECT 1" }; }
                const LATE: &str = concat!();
                fn before_the_shadow() {
                    sqlx::query(EARLY);
                }
                fn after_the_shadow() {
                    sqlx::query(LATE);
                }
            "#,
    )
    .unwrap();
    // A `macro_rules!` scope starts at its declaration, so `EARLY` really
    // expands with the prelude macro and its statement is pinned. Reading
    // the shadow as covering the module would have been the wrong kind of
    // caution here: an opaque source lets its literal change without moving
    // a row, which is the very signal this ratchet exists to give.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn before_the_shadow"
            && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn after_the_shadow"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_shadows_macros_only_for_bodies_below_them() {
    let baseline = inventory(
        r#"
                fn early() {
                    sqlx::query(concat!("SELECT GET_LOCK('early', 0)"));
                }
                macro_rules! concat { () => { "SELECT 1" }; }
                fn late() {
                    sqlx::query(concat!());
                }
            "#,
    )
    .unwrap();
    // A body written above the declaration resolves the prelude macro.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn early" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn late"
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_shadows_macros_inside_impl_methods() {
    let baseline = inventory(
        r#"
                macro_rules! concat { () => { "SELECT GET_LOCK('shadow', 0)" }; }
                struct Statements;
                impl Statements {
                    fn run(&self) {
                        sqlx::query(concat!());
                    }
                }
            "#,
    )
    .unwrap();
    // A method body sees the module's shadow like any other body does.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing.contains("run")
            && matches!(
                row.operation,
                PersistenceOperation::NonliteralSql | PersistenceOperation::InterpolatedSql
            )
    }));
}

#[test]
fn persistence_inventory_rejects_returning_calls_hidden_in_unknown_macros() {
    let error = inventory(
        r#"
                macro_rules! forward { ($value:expr) => { $value.pool() } }
                fn make_database() -> wow_database::CharacterDatabase { unreachable!() }
                fn hidden() { forward!(make_database()); }
            "#,
    )
    .unwrap_err();
    assert!(error.contains("unknown macro forward"), "{error}");
}

#[test]
fn persistence_inventory_does_not_import_unrelated_callables_or_macros() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "unrelated-provider",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub fn make() -> Holder { unreachable!() }
                #[macro_export]
                macro_rules! hidden_database {
                    () => { wow_database::CharacterDatabase::default() };
                }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn clean() {
                    consume(unrelated_provider::make().0.pool());
                    consume(unrelated_provider::hidden_database!().pool());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, provider],
        &WorkspaceDependencyAliases::default(),
    )
    .unwrap();
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}
