//! Regressions for query execution.

use super::*;

#[test]
fn persistence_inventory_tracks_aliases_queries_transactions_and_pool_returns() {
    let baseline = inventory(
        r#"
                use sqlx::{query_as as load, MySqlPool as Pool};
                use std::sync::Arc;

                type SharedPool = Arc<Pool>;
                struct Adapter { pool: SharedPool }

                async fn work(adapter: &Adapter) -> SharedPool {
                    let pool = Arc::clone(&adapter.pool);
                    let mut tx = pool.begin().await.unwrap();
                    load::<_, Row>("SELECT 1")
                        .fetch_optional(&mut tx)
                        .await
                        .unwrap();
                    tx.commit().await.unwrap();
                    pool
                }
            "#,
    )
    .expect("strict persistence fixture parses");
    let found = operations(&baseline);
    for expected in [
        (
            PersistenceTarget::Sqlx,
            PersistenceOperation::Import,
            "load",
        ),
        (
            PersistenceTarget::MySqlPool,
            PersistenceOperation::Import,
            "Pool",
        ),
        (
            PersistenceTarget::MySqlPool,
            PersistenceOperation::TypeAlias,
            "SharedPool",
        ),
        (PersistenceTarget::Sqlx, PersistenceOperation::Query, "load"),
        (
            PersistenceTarget::MySqlPool,
            PersistenceOperation::Begin,
            "begin",
        ),
        (
            PersistenceTarget::Sqlx,
            PersistenceOperation::FetchOptional,
            "fetch_optional",
        ),
        (
            PersistenceTarget::MySqlPool,
            PersistenceOperation::Commit,
            "commit",
        ),
        (
            PersistenceTarget::MySqlPool,
            PersistenceOperation::ReturnEscape,
            "pool",
        ),
    ] {
        assert!(
            found.contains(&(expected.0, expected.1, expected.2.to_owned())),
            "missing {expected:?} from {found:#?}"
        );
    }
    assert!(
        !baseline.accesses.iter().any(|record| {
            record.operation == PersistenceOperation::ArgumentEscape && record.symbol == "clone"
        }),
        "the explicit Arc::clone grammar is value flow, not an unknown escape"
    );
}

#[test]
fn persistence_inventory_records_pool_escape_for_unvalidated_executor_names() {
    let baseline = inventory(
        r#"
                struct LocalExecutor;
                fn leak(local: &LocalExecutor, pool: &sqlx::PgPool) {
                    local.execute(pool);
                }
                fn valid(pool: &sqlx::PgPool) {
                    sqlx::query("SELECT 1").execute(pool);
                }
                fn bound(pool: &sqlx::PgPool) {
                    sqlx::query("SELECT ?").bind(1_u32).execute(pool);
                }
                struct LocalBinder;
                fn unrelated(local: &LocalBinder, pool: &sqlx::PgPool) {
                    local.bind(pool);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn leak"
            && row.target == PersistenceTarget::PgPool
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "execute"
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn leak"
            && row.target == PersistenceTarget::PgPool
            && row.operation == PersistenceOperation::Execute
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn bound" && row.operation == PersistenceOperation::Execute
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn unrelated"
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "bind"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn valid" && row.operation == PersistenceOperation::Execute
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn valid"
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "execute"
    }));
}

#[test]
fn persistence_inventory_records_query_macros_and_rejects_opaque_macro_escapes() {
    let baseline = inventory(
        r#"
                use sqlx::MySqlPool;
                #[derive(sqlx::FromRow)]
                struct ProjectedRow { value: u64 }

                #[sqlx::test]
                async fn adapter_contract() {}

                fn query(pool: &MySqlPool) {
                    sqlx::query!("SELECT 1").execute(pool);
                }

                fn logged(pool: &MySqlPool) { info!(?pool, "pool"); }
            "#,
    )
    .expect("known SQLx macro is explicit inventory grammar");
    assert!(operations(&baseline).contains(&(
        PersistenceTarget::Sqlx,
        PersistenceOperation::Query,
        "query".to_owned()
    )));
    assert!(operations(&baseline).contains(&(
        PersistenceTarget::Sqlx,
        PersistenceOperation::MacroReference,
        "derive".to_owned()
    )));
    assert!(operations(&baseline).contains(&(
        PersistenceTarget::Sqlx,
        PersistenceOperation::MacroReference,
        "test".to_owned()
    )));
    assert!(operations(&baseline).contains(&(
        PersistenceTarget::MySqlPool,
        PersistenceOperation::MacroReference,
        "info".to_owned()
    )));
    assert!(operations(&baseline).contains(&(
        PersistenceTarget::MySqlPool,
        PersistenceOperation::ArgumentEscape,
        "macro:info".to_owned()
    )));

    let error = inventory(
        r#"
                use sqlx::MySqlPool;
                fn hidden(pool: &MySqlPool) { hide_access!(pool); }
            "#,
    )
    .expect_err("unknown macro cannot hide a concrete pool");
    assert!(error.contains("unknown macro hide_access!"), "{error}");

    let generated = inventory(
        r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
            "#,
    )
    .expect("macro-generated persistence is an exact opaque baseline row");
    assert!(operations(&generated).contains(&(
        PersistenceTarget::Sqlx,
        PersistenceOperation::MacroReference,
        "hidden_query".to_owned()
    )));

    let error = inventory(
        r#"
                trait HiddenPort { fn pool(&self) -> sqlx::PgPool; }
            "#,
    )
    .expect_err("unsupported item grammars must fail closed");
    assert!(error.contains("unsupported item grammar"), "{error}");
}

#[test]
fn persistence_inventory_avoids_local_database_and_sqlx_variant_collisions() {
    let innocent = inventory(
        r#"
                enum LogFilter { Database }
                struct Database;
                fn local(_: Database) { let _ = LogFilter::Database; }
            "#,
    )
    .expect("local names are ordinary Rust symbols");
    assert!(innocent.accesses.is_empty(), "{:#?}", innocent.accesses);

    let sqlx = inventory(
        r#"
                fn classify(error: sqlx::Error) {
                    if let sqlx::Error::Database(inner) = error { drop(inner); }
                }
            "#,
    )
    .expect("sqlx variant paths stay in the sqlx target");
    assert!(
        sqlx.accesses
            .iter()
            .any(|row| row.target == PersistenceTarget::Sqlx)
    );
    assert!(
        !sqlx
            .accesses
            .iter()
            .any(|row| row.target == PersistenceTarget::Database),
        "{:#?}",
        sqlx.accesses
    );

    let adapter = inventory_for_package(
        "wow-database",
        "fn local(statement: PreparedStatement) -> SqlResult { todo!() }",
    )
    .expect("wow-database owns its unqualified concrete types");
    assert!(adapter.accesses.iter().any(|row| {
        row.target == PersistenceTarget::PreparedStatement
            && row.operation == PersistenceOperation::TypeReference
    }));
    let adapter_reexports = inventory_for_package(
        "wow-database",
        r#"
                pub use database::Database;
                use super::StatementDef;
            "#,
    )
    .expect("adapter-local re-exports resolve their imported leaf");
    for expected in [PersistenceTarget::Database, PersistenceTarget::StatementDef] {
        assert!(adapter_reexports.accesses.iter().any(|row| {
            row.target == expected && row.operation == PersistenceOperation::Import
        }));
    }

    let transaction_variant = inventory(
        r#"
                use sqlx::Transaction;
                use wow_database::DatabaseError;
                fn classify(error: DatabaseError) {
                    if let DatabaseError::Transaction(inner) = error { drop(inner); }
                }
            "#,
    )
    .expect("an imported sqlx type must not capture a same-named enum variant");
    assert!(
        !transaction_variant
            .accesses
            .iter()
            .any(|row| row.target == PersistenceTarget::SqlxTransaction
                && row.symbol == "Transaction"
                && row.operation != PersistenceOperation::Import),
        "{:#?}",
        transaction_variant.accesses
    );

    let generated_id = inventory(
        r#"
                use wow_database::CharStatements;
                fn allocator_seed() { let _ = CharStatements::SEL_MAX_ITEM_GUID; }
            "#,
    )
    .expect("MAX-ID statement reads are explicit inventory operations");
    assert!(generated_id.accesses.iter().any(|row| {
        row.target == PersistenceTarget::CharStatements
            && row.operation == PersistenceOperation::GeneratedIdRead
            && row.symbol == "SEL_MAX_ITEM_GUID"
    }));
}

#[test]
fn persistence_inventory_rejects_unmounted_query_files() {
    for source in [
        r#"fn hidden() { consume(sqlx::query_file!("query.sql")); }"#,
        r#"fn hidden() { consume(sqlx::query_file_as!(u8, "query.sql")); }"#,
        r#"fn hidden() { consume(sqlx::query_file_scalar!("query.sql")); }"#,
    ] {
        let error = inventory(source).unwrap_err();
        assert!(error.contains("referenced file"), "{error}");
    }
}

#[test]
fn persistence_inventory_classifies_raw_sql_constructors() {
    let baseline = inventory(
        r#"
                fn locked(pool: sqlx::MySqlPool) {
                    let sql = "SELECT GET_LOCK('inventory', 0)";
                    sqlx::raw_sql(sql).execute(&pool);
                }
                fn dynamic(sql: &str) {
                    consume(sqlx::raw_sql(sql));
                }
            "#,
    )
    .unwrap();

    for operation in [
        PersistenceOperation::Query,
        PersistenceOperation::RawSql,
        PersistenceOperation::AdvisoryLock,
    ] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn locked" && row.symbol == "raw_sql" && row.operation == operation
        }));
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn dynamic"
            && row.symbol == "raw_sql"
            && row.operation == PersistenceOperation::NonliteralSql
    }));
}

#[test]
fn persistence_inventory_fingerprints_prepared_statement_sql() {
    let baseline = inventory(
        r#"
                const LOCK_SQL: &str = "SELECT GET_LOCK('prepared', 0)";
                fn prepared() {
                    consume(wow_database::PreparedStatement::new(LOCK_SQL));
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn prepared"
            && row.target == PersistenceTarget::PreparedStatement
            && row.operation == PersistenceOperation::RawSql
            && row.fingerprint.contains("GET_LOCK")
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn prepared" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_classifies_raw_sql_executor_arguments() {
    let baseline = inventory(
        r#"
                fn raw(pool: sqlx::MySqlPool, sql: &str) {
                    pool.execute(sql);
                }
                fn inverse(pool: sqlx::MySqlPool) {
                    sqlx::query("SELECT 1").execute(&pool);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn raw"
            && row.operation == PersistenceOperation::RawSql
            && row.symbol == "execute"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn raw"
            && row.operation == PersistenceOperation::NonliteralSql
            && row.symbol == "execute"
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn inverse"
            && row.operation == PersistenceOperation::RawSql
            && row.symbol == "execute"
    }));
}

#[test]
fn persistence_inventory_classifies_raw_sql_ufcs_executor_arguments() {
    let baseline = inventory(
        r#"
                fn raw(pool: sqlx::MySqlPool) {
                    let sql = "SELECT GET_LOCK('ufcs', 0)";
                    sqlx::Executor::execute(&pool, sql);
                }
            "#,
    )
    .unwrap();
    for operation in [
        PersistenceOperation::Execute,
        PersistenceOperation::RawSql,
        PersistenceOperation::AdvisoryLock,
    ] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == "fn raw" && row.symbol == "execute" && row.operation == operation
        }));
    }
}

#[test]
fn persistence_inventory_tracks_unknown_method_transaction_escapes() {
    let baseline = inventory(
        r#"
                async fn hand_off(pool: sqlx::PgPool) {
                    let tx = pool.begin().await.unwrap();
                    tx.hand_off();
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn hand_off"
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "receiver:hand_off"
    }));
}

#[test]
fn persistence_inventory_records_query_builder_binds() {
    let baseline = inventory(
        r#"
                fn built() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT 1 WHERE id = ");
                    builder.push_bind(42);
                }
            "#,
    )
    .unwrap();
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing == "fn built"
                && row.operation == PersistenceOperation::RawSql
                && row.fingerprint.contains("push_bind")
        }),
        "a bound value can change the executed statement and must be inventoried"
    );
}

#[test]
fn persistence_inventory_classifies_only_the_statement_of_a_query_macro() {
    let baseline = inventory(
        r#"
                fn bound_value() {
                    sqlx::query!("SELECT ?", "GET_LOCK('x', 0)");
                }
                fn statement() {
                    sqlx::query!("SELECT GET_LOCK('macro', 0)");
                }
            "#,
    )
    .unwrap();
    // The second argument is a bound value; the database never executes it.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn bound_value" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn statement" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_keeps_the_statement_position_of_aliased_query_macros() {
    let baseline = inventory(
        r#"
                use sqlx::query_as as q;
                struct Row;
                fn aliased() {
                    q!(Row, "SELECT GET_LOCK('aliased', 0)");
                }
            "#,
    )
    .unwrap();
    // The alias hides which argument carries the statement; the canonical
    // macro name decides it.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn aliased" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_rejects_aliased_query_file_macros() {
    let baseline = inventory(
        r#"
                fn aliased_file() {
                    use sqlx::query_file as q;
                    q!("query.sql");
                }
            "#,
    );
    // The referenced file is outside the snapshot however the macro is
    // named, so the call must be refused rather than fingerprinted.
    let error = baseline
        .err()
        .expect("an aliased query-file macro must be rejected");
    assert!(
        error.contains("query_file") && error.contains("fn aliased_file"),
        "unexpected rejection: {error}"
    );
}

#[test]
fn persistence_inventory_does_not_reconstruct_a_builder_across_calls() {
    let baseline = inventory(
        r#"
                fn split_builder() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT GET_");
                    builder.push("LOCK('built', 0)");
                    builder.push_unseparated(" AND 1");
                }
                fn pinned_builder() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT GET_LOCK('pinned', 0)");
                    builder.push(" AND 1");
                }
            "#,
    )
    .unwrap();
    // A builder assembles its statement at run time across calls, so no
    // fragment carries a content claim — but every appended fragment is
    // still inventoried, including `push_unseparated`.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn split_builder" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert_eq!(
        baseline
            .accesses
            .iter()
            .filter(|row| {
                row.enclosing == "fn split_builder" && row.operation == PersistenceOperation::RawSql
            })
            .count(),
        2,
        "both appended fragments must be inventoried"
    );
    // A statement pinned at the constructor keeps its identity.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn pinned_builder" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_rejects_every_file_backed_query_macro() {
    for macro_name in ["query_file_unchecked", "query_file_as_unchecked"] {
        let baseline = inventory(&format!(
            r#"
                    struct Row;
                    fn from_file() {{
                        sqlx::{macro_name}!("query.sql");
                    }}
                "#
        ));
        assert!(
            baseline
                .err()
                .is_some_and(|error| error.contains("query_file")),
            "{macro_name}! was accepted with SQL outside the snapshot"
        );
    }
}

#[test]
fn persistence_inventory_classifies_prepared_statements() {
    let baseline = inventory(
        r#"
                fn prepared(pool: sqlx::MySqlPool) {
                    pool.prepare("SELECT GET_LOCK('prepared', 0)");
                }
            "#,
    )
    .unwrap();
    // `prepare` receives the statement itself, so its text belongs to the
    // inventory like any other raw SQL.
    for operation in [
        PersistenceOperation::RawSql,
        PersistenceOperation::AdvisoryLock,
    ] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn prepared" && row.operation == operation),
            "a prepared statement lost its {operation:?} row"
        );
    }
}

#[test]
fn persistence_inventory_classifies_nonstatic_prepared_sql() {
    let dynamic = inventory(
        r#"
                fn prepared(pool: sqlx::MySqlPool, sql: String) {
                    pool.prepare(&sql);
                }
            "#,
    )
    .unwrap();
    assert!(dynamic.accesses.iter().any(|row| {
        row.enclosing == "fn prepared" && row.operation == PersistenceOperation::NonliteralSql
    }));
    // SQL the snapshot cannot see is refused wherever it is prepared.
    let included = inventory(
        r#"
                fn prepared(pool: sqlx::MySqlPool) {
                    pool.prepare(include_str!("query.sql"));
                }
            "#,
    );
    assert!(
        included
            .err()
            .is_some_and(|error| error.contains("include_str!")),
        "prepared SQL from an unmounted file was accepted"
    );
}

#[test]
fn persistence_inventory_keeps_the_transaction_opened_by_ufcs_begin() {
    let baseline = inventory(
        r#"
                async fn scoped(mut connection: sqlx::MySqlConnection) {
                    let tx = sqlx::Acquire::begin(&mut connection).await.unwrap();
                    tx.commit().await.unwrap();
                }
            "#,
    )
    .unwrap();
    // The transaction the call opens must survive to its commit.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn scoped" && row.operation == PersistenceOperation::Commit
    }));
}

#[test]
fn persistence_inventory_separates_query_objects_from_raw_executor_sql() {
    let baseline = inventory(
        r#"
                fn typed(pool: sqlx::MySqlPool) {
                    pool.execute(sqlx::query("SELECT 1"));
                }
            "#,
    )
    .unwrap();
    // An executor handed a built query is not executing dynamic SQL.
    for operation in [
        PersistenceOperation::RawSql,
        PersistenceOperation::NonliteralSql,
    ] {
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn typed" && row.operation == operation),
            "typed execution was reported as {operation:?}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn typed" && row.operation == PersistenceOperation::Query
    }));
}

#[test]
fn persistence_inventory_separates_built_queries_from_ufcs_raw_sql() {
    let baseline = inventory(
        r#"
                fn typed_ufcs(pool: sqlx::MySqlPool) {
                    sqlx::Executor::execute(&pool, sqlx::query("SELECT 1"));
                }
                fn raw_ufcs(pool: sqlx::MySqlPool, sql: String) {
                    sqlx::Executor::execute(&pool, &sql);
                }
            "#,
    )
    .unwrap();
    // The second argument is the statement only when it is not a query.
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn typed_ufcs"
            && matches!(
                row.operation,
                PersistenceOperation::RawSql | PersistenceOperation::NonliteralSql
            )
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn raw_ufcs" && row.operation == PersistenceOperation::NonliteralSql
    }));
}

#[test]
fn persistence_inventory_keeps_provider_identity_and_query_modifiers() {
    let baseline = inventory(
        r#"
                use sqlx::mysql::MySqlPoolOptions as Opt;
                async fn connected() {
                    Opt::new().connect("mysql://localhost/db").await.unwrap();
                }
                fn modified(pool: sqlx::MySqlPool) {
                    sqlx::query("SELECT 1").persistent(false).execute(&pool);
                }
                fn described(pool: sqlx::MySqlPool) {
                    pool.describe("SELECT GET_LOCK('described', 0)");
                }
            "#,
    )
    .unwrap();
    // An aliased options constructor still names the provider it builds.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn connected" && row.target == PersistenceTarget::MySqlPool
    }));
    // A modifier returns the query, so the chained executor is inventoried.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn modified" && row.operation == PersistenceOperation::Execute
    }));
    // `describe` sends a statement, so its SQL is inventoried.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn described" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_keeps_executor_identity_in_a_local() {
    let baseline = inventory(
        r#"
                fn stored(pool: sqlx::MySqlPool) {
                    let run = sqlx::Executor::execute;
                    run(&pool, "SELECT GET_LOCK('stored', 0)");
                }
            "#,
    )
    .unwrap();
    // The call through the binding sends the SQL it is handed.
    for operation in [
        PersistenceOperation::Execute,
        PersistenceOperation::AdvisoryLock,
    ] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn stored" && row.operation == operation),
            "a stored executor lost its {operation:?} row"
        );
    }
}

/// Characterizes the `ControlFlow` escape rather than witnessing a fix: the
/// payload rule was added for consistency with `Ok`/`Err`, and no shape was
/// found where the row depends on it.
#[test]
fn persistence_inventory_keeps_executor_identity_through_a_block_alias() {
    let baseline = inventory(
        r#"
                fn stored(pool: sqlx::MySqlPool) {
                    use sqlx::Executor as E;
                    let run = E::execute;
                    run(&pool, "SELECT GET_LOCK('aliased', 0)");
                }
            "#,
    )
    .unwrap();
    for operation in [
        PersistenceOperation::Execute,
        PersistenceOperation::AdvisoryLock,
    ] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn stored" && row.operation == operation),
            "an executor behind a block alias lost its {operation:?} row"
        );
    }
}

#[test]
fn persistence_inventory_keeps_bound_values_out_of_sql_classification() {
    let baseline = inventory(
        r#"
                fn bound() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT 1 WHERE name = ");
                    builder.push_bind("GET_LOCK('x', 0)");
                }
            "#,
    )
    .unwrap();
    // A bound value is a parameter: it is sent, not executed.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn bound" && row.operation == PersistenceOperation::RawSql
    }));
    for operation in [
        PersistenceOperation::AdvisoryLock,
        PersistenceOperation::NonliteralSql,
    ] {
        assert!(
            !baseline
                .accesses
                .iter()
                .any(|row| row.enclosing == "fn bound" && row.operation == operation),
            "a bound parameter was classified as {operation:?}"
        );
    }
}

#[test]
fn persistence_inventory_tracks_sqlx_pool_opens_and_transaction_escapes() {
    let baseline = inventory(
        r#"
                async fn open(connection_string: &str) {
                    sqlx::mysql::MySqlPoolOptions::new()
                        .max_connections(4)
                        .idle_timeout(None)
                        .connect(connection_string)
                        .await;
                }
                async fn forget_transaction(pool: sqlx::PgPool) {
                    let tx = pool.begin().await.unwrap();
                    std::mem::forget(tx);
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn open"
            && row.target == PersistenceTarget::MySqlPool
            && row.operation == PersistenceOperation::DatabaseOpen
            && row.symbol == "connect"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn forget_transaction"
            && row.target == PersistenceTarget::PgPool
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "forget"
    }));
}

#[test]
fn persistence_inventory_fingerprints_query_builder_sql_fragments() {
    let first = inventory(
        r#"
                fn build() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT 1");
                    builder.push(" WHERE enabled = 1");
                }
            "#,
    )
    .unwrap();
    let second = inventory(
        r#"
                fn build() {
                    let mut builder = sqlx::QueryBuilder::new("SELECT 1");
                    builder.push(" WHERE enabled = 0");
                }
            "#,
    )
    .unwrap();
    let raw_sql = |baseline: &PersistenceAccessBaseline| {
        baseline
            .accesses
            .iter()
            .filter(|row| row.operation == PersistenceOperation::RawSql)
            .map(|row| row.fingerprint.clone())
            .collect::<BTreeSet<_>>()
    };
    assert_ne!(raw_sql(&first), raw_sql(&second));

    let interpolated = inventory(
        r#"
                fn build(fragment: &str) {
                    let mut builder = sqlx::QueryBuilder::new("SELECT 1");
                    builder.push(format!(" WHERE {fragment}"));
                }
            "#,
    )
    .unwrap();
    assert!(
        interpolated
            .accesses
            .iter()
            .any(|row| row.operation == PersistenceOperation::InterpolatedSql)
    );
}

#[test]
fn persistence_inventory_preserves_local_sqlx_query_alias_semantics() {
    let baseline = inventory(
        r#"
                fn persistent(sql: &str, pool: sqlx::MySqlPool) {
                    let query_fn = sqlx::query::<sqlx::MySql>;
                    query_fn(sql).execute(&pool);
                }
            "#,
    )
    .unwrap();
    for operation in [
        PersistenceOperation::Query,
        PersistenceOperation::NonliteralSql,
    ] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| { row.enclosing == "fn persistent" && row.operation == operation })
        );
    }
}

#[test]
fn persistence_inventory_classifies_query_builder_initial_sql() {
    let baseline = inventory(
        r#"
                fn dynamic(id: u32) {
                    sqlx::QueryBuilder::new(format!("SELECT {id}"));
                }
            "#,
    )
    .unwrap();
    for operation in [
        PersistenceOperation::Query,
        PersistenceOperation::InterpolatedSql,
    ] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| { row.enclosing == "fn dynamic" && row.operation == operation })
        );
    }
    let error = inventory(r#"fn rejected() { sqlx::QueryBuilder::new(env!("QUERY")); }"#)
        .expect_err("environment-sourced builder SQL must fail closed");
    assert!(error.contains("env! SQL"), "{error}");
}
