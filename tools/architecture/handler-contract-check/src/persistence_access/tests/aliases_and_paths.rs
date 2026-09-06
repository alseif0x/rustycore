//! Regressions for aliases and paths.

use super::*;

#[test]
fn same_named_nested_import_reaches_a_fixed_point() {
    let item_use: ItemUse = syn::parse_quote!(
        use bitflags::bitflags;
    );
    let mut symbols = ModuleSymbols::for_package("fixture");
    symbols.module_path = vec!["realm".to_owned()];

    assert!(apply_import_symbols(&item_use, &mut symbols));
    let aliases = symbols.path_aliases.clone();
    assert!(!apply_import_symbols(&item_use, &mut symbols));
    assert_eq!(symbols.path_aliases, aliases);
}

#[test]
fn persistence_inventory_tracks_renamed_database_extern_crates_without_false_positives() {
    let baseline = inventory(
        r#"
                extern crate wow_database as db;

                async fn leak(database: &db::CharacterDatabase) {
                    let mut tx = database.pool().begin().await.unwrap();
                    tx.rollback().await.unwrap();
                }
            "#,
    )
    .expect("renamed wow_database extern crate resolves");
    let found = operations(&baseline);
    for expected in [
        (
            PersistenceTarget::Database,
            PersistenceOperation::Import,
            "db".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::TypeReference,
            "database".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::PoolAccess,
            "pool".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Begin,
            "begin".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Rollback,
            "rollback".to_owned(),
        ),
    ] {
        assert!(
            found.contains(&expected),
            "missing {expected:?}: {found:#?}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.operation == PersistenceOperation::Import
            && row.fingerprint == "extern crate wow_database as db"
    }));

    let error = compare_persistence_access_baseline(&inventory("").unwrap(), &baseline)
        .expect_err("a renamed database extern crate must trip the non-growth ratchet");
    assert!(
        error.contains("untracked direct persistence access"),
        "{error}"
    );

    let unrelated = inventory(
        r#"
                extern crate unrelated as db;
                fn innocent(_: db::CharacterDatabase) {}
            "#,
    )
    .expect("an unrelated extern crate remains ordinary Rust syntax");
    assert!(unrelated.accesses.is_empty(), "{:#?}", unrelated.accesses);
}

#[test]
fn persistence_inventory_tracks_grouped_namespace_self_aliases() {
    let baseline = inventory(
        r#"
                async fn leak(database: &db::CharacterDatabase) {
                    database.pool().begin().await.unwrap();
                }
                use wow_database::{self as db};
            "#,
    )
    .expect("grouped self rename resolves independent of item order");
    let found = operations(&baseline);
    for expected in [
        (
            PersistenceTarget::Database,
            PersistenceOperation::Import,
            "db".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::PoolAccess,
            "pool".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::Begin,
            "begin".to_owned(),
        ),
    ] {
        assert!(
            found.contains(&expected),
            "missing {expected:?}: {found:#?}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.operation == PersistenceOperation::Import
            && row.fingerprint == "wow_database::self as db"
    }));
    assert!(
        compare_persistence_access_baseline(&inventory("").unwrap(), &baseline)
            .unwrap_err()
            .contains("untracked direct persistence access")
    );

    let unrelated = inventory(
        r#"
                use unrelated::{self as db};
                fn innocent(_: db::CharacterDatabase) {}
            "#,
    )
    .unwrap();
    assert!(unrelated.accesses.is_empty(), "{:#?}", unrelated.accesses);
}

#[test]
fn persistence_inventory_follows_module_and_type_aliases_independent_of_order() {
    let baseline = inventory(
        r#"
                use db::PgPool as Pool;
                use sqlx as db;
                type Outer = Option<std::sync::Arc<Pool>>;
                fn expose(pool: Outer) -> Outer { pool }
            "#,
    )
    .expect("aliases resolve to a fixed point");
    let found = operations(&baseline);
    assert!(found.contains(&(
        PersistenceTarget::Sqlx,
        PersistenceOperation::Import,
        "db".to_owned()
    )));
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::Import,
        "Pool".to_owned()
    )));
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::ReturnEscape,
        "pool".to_owned()
    )));
}

#[test]
fn persistence_inventory_resolves_typed_database_paths_getters_and_dynamic_sql() {
    let baseline = inventory(
        r#"
                use wow_database::{CharacterDatabase, DatabaseError as DbError, SqlTransaction};
                struct Session;
                impl Session {
                    fn character_db(&self) -> Option<&CharacterDatabase> { None }
                    async fn query(&self) {
                        let db = self.character_db().unwrap();
                        let sql = format!("SELECT {}", 1);
                        db.direct_query(&sql).await.unwrap();
                        let _tx = wow_database::SqlTransaction::new();
                        let _error = wow_database::DatabaseError::Query("x".into());
                    }
                }
                struct Store;
                impl Store {
                    fn transaction(&self) -> SqlTransaction { unreachable!() }
                    async fn commit(&self) {
                        let transaction = self.transaction();
                        transaction.commit_with_outcome_like_cpp().await.unwrap();
                    }
                }
                fn split_sql(content: &str) -> Vec<&str> { vec![content] }
                fn dynamic(content: &str) {
                    for statement in split_sql(content) {
                        sqlx::query(statement);
                    }
                }
            "#,
    )
    .expect("typed database imports, getters and qualified paths are explicit grammar");
    let found = operations(&baseline);
    for expected in [
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::DirectQuery,
            "direct_query".to_owned(),
        ),
        (
            PersistenceTarget::CharacterDatabase,
            PersistenceOperation::InterpolatedSql,
            "direct_query".to_owned(),
        ),
        (
            PersistenceTarget::SqlTransaction,
            PersistenceOperation::TransactionConstruct,
            "new".to_owned(),
        ),
        (
            PersistenceTarget::DatabaseError,
            PersistenceOperation::PathReference,
            "Query".to_owned(),
        ),
        (
            PersistenceTarget::SqlTransaction,
            PersistenceOperation::Commit,
            "commit_with_outcome_like_cpp".to_owned(),
        ),
        (
            PersistenceTarget::Sqlx,
            PersistenceOperation::NonliteralSql,
            "query".to_owned(),
        ),
    ] {
        assert!(
            found.contains(&expected),
            "missing {expected:?} from {found:#?}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.target == PersistenceTarget::DatabaseError
            && row.operation == PersistenceOperation::Import
            && row.symbol == "DbError"
    }));
}

#[test]
fn persistence_inventory_resolves_inline_module_value_aliases() {
    let baseline = inventory(
        r#"
                mod nested {
                    type Db = wow_database::CharacterDatabase;
                    static DATABASE: Db = unreachable!();
                    fn persistent() { consume(DATABASE.pool()); }
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_canonicalizes_module_level_aliases() {
    let baseline = inventory(
        r#"
                use std::concat as c;
                use sqlx::query as q;
                fn module_alias_concat() {
                    sqlx::query(c!("SELECT GET_", "LOCK('module', 0)"));
                }
                fn block_alias_call(pool: sqlx::MySqlPool) {
                    use sqlx::query as inner;
                    inner("SELECT GET_LOCK('call', 0)").execute(&pool);
                }
                fn module_alias_call() {
                    q("SELECT GET_LOCK('module-call', 0)");
                }
            "#,
    )
    .unwrap();
    for enclosing in [
        "fn module_alias_concat",
        "fn block_alias_call",
        "fn module_alias_call",
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::AdvisoryLock
            }),
            "{enclosing} lost a statement hidden behind an alias"
        );
    }
}

#[test]
fn persistence_inventory_dispatches_every_alias_to_its_constructor() {
    let baseline = inventory(
        r#"
                use sqlx::QueryBuilder as B;
                use sqlx::raw_sql as r;
                fn aliased_builder() {
                    B::new("SELECT GET_LOCK('builder', 0)");
                }
                fn aliased_raw_sql() {
                    r("SELECT GET_LOCK('raw', 0)");
                }
                fn aliased_flow(pool: sqlx::MySqlPool) {
                    use sqlx::query as q;
                    q("SELECT 1").execute(&pool);
                }
            "#,
    )
    .unwrap();
    // The constructor an alias names decides the operation, not the alias.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn aliased_builder" && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn aliased_raw_sql" && row.operation == PersistenceOperation::RawSql
    }));
    // The result of an aliased constructor keeps its query flow, so the
    // chained executor call is inventoried rather than reduced to an escape.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn aliased_flow" && row.operation == PersistenceOperation::Execute
    }));
}

#[test]
fn persistence_inventory_follows_aliased_ufcs_and_builder_flow() {
    let baseline = inventory(
        r#"
                use sqlx::QueryBuilder as B;
                use sqlx::Executor as E;
                fn chained_builder(pool: sqlx::MySqlPool, sql: String) {
                    let mut b = B::new("SELECT ");
                    b.push(sql);
                    b.build().execute(&pool);
                }
                fn ufcs_executor(pool: sqlx::MySqlPool, sql: String) {
                    E::execute(&pool, &sql);
                }
                fn ufcs_prepare(pool: sqlx::MySqlPool) {
                    sqlx::Executor::prepare(&pool, "SELECT GET_LOCK('ufcs', 0)");
                }
            "#,
    )
    .unwrap();
    // An aliased constructor still returns query flow, so what is chained
    // onto it stays in the inventory.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn chained_builder" && row.operation == PersistenceOperation::Execute
    }));
    // UFCS through an aliased trait still hands its second argument as SQL.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn ufcs_executor" && row.operation == PersistenceOperation::NonliteralSql
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn ufcs_prepare" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_resolves_chained_local_import_aliases() {
    let baseline = inventory(
        r#"
                fn chained() {
                    use mount as alias;
                    use std::include as mount;
                    alias!("db_impl.rs");
                }
            "#,
    );
    // Imports are not ordered, so an alias of an alias still names
    // `include!` and its contents remain outside the inventory.
    assert!(
        baseline
            .err()
            .is_some_and(|error| error.contains("include!")),
        "a chained alias hid an include! mount"
    );
}

#[test]
fn persistence_inventory_scopes_type_shadows_to_their_module() {
    let baseline = inventory(
        r#"
                struct String;
                mod child {
                    pub(crate) const SQL: &str = "SELECT GET_LOCK('child', 0)";
                    pub(crate) fn run() {
                        sqlx::query(String::from(SQL).as_str());
                    }
                }
            "#,
    )
    .unwrap();
    // The parent's `struct String` is not in scope under the bare name in
    // the child, so the conversion there is the standard one.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn run" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}
