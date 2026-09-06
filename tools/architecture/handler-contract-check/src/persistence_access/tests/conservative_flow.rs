//! Regressions for conservative flow.

use super::*;

#[test]
fn persistence_inventory_conservatively_propagates_unmodeled_expression_children() {
    let baseline = inventory(
        r#"
                fn array(pool: sqlx::PgPool) { consume([pool]); }
                fn index(pool: sqlx::PgPool) { consume([pool][0]); }
                async fn async_block(pool: sqlx::PgPool) { consume(async { pool }.await); }
                fn loop_value(pool: sqlx::PgPool) { consume(loop { break pool }); }
                fn for_binding(databases: Vec<wow_database::CharacterDatabase>) {
                    for database in databases { database.pool(); }
                }
                fn for_capture(pool: sqlx::PgPool) { for _ in [pool] {} }
                fn standalone_async(pool: sqlx::PgPool) { async move { pool }; }
                fn async_non_tail(pool: sqlx::PgPool, flag: bool) {
                    async move {
                        if flag { pool; 0_u8 } else { 0_u8 };
                        0_u8
                    };
                }
                fn loop_standalone(pool: sqlx::PgPool) { loop { break pool; }; }
                fn array_standalone(pool: sqlx::PgPool) { [pool]; }
                fn while_binding(mut databases: Vec<wow_database::CharacterDatabase>) {
                    while let Some(database) = databases.pop() { database.pool(); }
                }
                fn scalars(value: u32) {
                    consume([value]); consume([value][0]); consume(loop { break value });
                    for _ in [value] {} async move { value };
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn array", "fn index", "fn async_block", "fn loop_value"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::PgPool
                    && row.operation == PersistenceOperation::ArgumentEscape
            }),
            "missing propagated escape for {enclosing}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn for_binding"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    for enclosing in [
        "fn for_capture",
        "fn standalone_async",
        "fn async_non_tail",
        "fn loop_standalone",
        "fn array_standalone",
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::PgPool
                    && row.operation == PersistenceOperation::ArgumentEscape
            }),
            "missing standalone wrapper escape for {enclosing}"
        );
    }
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn while_binding"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn scalars")
    );
}

#[test]
fn persistence_inventory_records_rejected_named_persistence_receiver_as_escape() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                impl Holder { fn commit(&self) {} }
                fn persistent(holder: Holder) { holder.commit(); }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "receiver:commit"
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::Commit
    }));
}

#[test]
fn persistence_inventory_rejects_relevant_globs_and_records_pool_escapes() {
    let error =
        inventory("use sqlx::*;").expect_err("glob import can hide arbitrary concrete SQLx syntax");
    assert!(error.contains("glob import sqlx::*"), "{error}");

    let baseline = inventory(
        r#"
                use sqlx::PgPool;
                struct Holder { value: usize }
                fn consume(_: &str, _: &PgPool) {}
                fn escapes(pool: PgPool, mut holder: Holder) -> PgPool {
                    consume("pool", &pool);
                    evil::clone(&pool);
                    holder.value = 1;
                    Wrapper { pool: pool.clone() };
                    pool
                }
            "#,
    )
    .expect("ordinary escapes are inventoried");
    let found = operations(&baseline);
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::ArgumentEscape,
        "consume".to_owned()
    )));
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::ArgumentEscape,
        "clone".to_owned()
    )));
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::StoreEscape,
        "pool".to_owned()
    )));
    assert!(found.contains(&(
        PersistenceTarget::PgPool,
        PersistenceOperation::ReturnEscape,
        "pool".to_owned()
    )));
}

#[test]
fn persistence_inventory_fails_closed_on_block_local_items() {
    let error = inventory(
        r#"
                fn leak(pool: Alias) {
                    type Alias = sqlx::MySqlPool;
                    pool.acquire();
                }
            "#,
    )
    .expect_err("block-local persistence alias must fail closed");
    assert!(
        error.contains("block-local item"),
        "unexpected error: {error}"
    );

    inventory(
        r#"
                fn clean() {
                    struct Local(u8);
                    let _ = Local(0);
                }
            "#,
    )
    .expect("block-local items without persistence stay allowed");
}

#[test]
fn persistence_inventory_preserves_receiver_flow_through_unmodeled_methods() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let wrapped = Some(database).iter().next();
                    consume(wrapped.unwrap().pool());
                }
                fn clean() {
                    let wrapped = Some(1_u8).iter().next();
                    consume(wrapped.unwrap());
                }
            "#,
    )
    .unwrap();

    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}
