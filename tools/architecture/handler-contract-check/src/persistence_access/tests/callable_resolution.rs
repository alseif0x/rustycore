//! Regressions for callable resolution.

use super::*;

#[test]
fn persistence_inventory_propagates_arbitrary_callable_result_flow() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let factory = || database;
                    consume((factory)().pool());
                }
                fn clean() {
                    let factory = || 1_u8;
                    consume((factory)());
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

#[test]
fn persistence_inventory_propagates_arguments_through_callable_results() {
    let baseline = inventory(
        r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    let identity = |value| value;
                    consume(identity(database).pool());
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
fn persistence_inventory_does_not_infer_ufcs_return_from_receiver() {
    let baseline = inventory(
        r#"
                struct Factory(wow_database::CharacterDatabase);
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn clean(factory: &Factory) {
                    consume(Factory::identity(factory, 1_u8).pool());
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_classifies_parenthesized_callables() {
    let baseline = inventory(
        r#"
                const SQL: &str = "SELECT GET_LOCK('paren', 0)";
                fn parenthesized_query(pool: sqlx::MySqlPool) {
                    (sqlx::query)(SQL).execute(&pool);
                }
                fn parenthesized_executor(pool: sqlx::MySqlPool) {
                    (sqlx::Executor::execute)(&pool, SQL);
                }
            "#,
    )
    .unwrap();
    // Parentheses do not change what a callee constructs.
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn parenthesized_query"
            && row.operation == PersistenceOperation::AdvisoryLock
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn parenthesized_executor"
            && row.operation == PersistenceOperation::Execute
    }));
}

#[test]
fn persistence_inventory_models_clean_block_local_callables() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn inferred(database: wow_database::CharacterDatabase) {
                    fn pass<T>(value: T) -> T { value }
                    let wrapped = pass(database);
                    consume(wrapped.pool());
                }
                fn explicit_persistent(database: wow_database::CharacterDatabase) {
                    fn make<T, U>(_input: U) -> T { unreachable!() }
                    consume(make::<wow_database::CharacterDatabase, _>(database).pool());
                }
                fn explicit_clean(database: wow_database::CharacterDatabase) {
                    fn make<T, U>(_input: U) -> T { unreachable!() }
                    consume(make::<Clean, _>(database).pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn inferred", "fn explicit_persistent"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing pool access for {enclosing}: {:#?}",
            baseline.accesses
        );
    }
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn explicit_clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_preserves_function_item_return_flow() {
    let baseline = inventory(
        r#"
                fn database() -> wow_database::CharacterDatabase { unreachable!() }
                struct Factory;
                impl Factory {
                    fn database() -> wow_database::CharacterDatabase { unreachable!() }
                }
                fn free_alias() {
                    let factory = crate::database;
                    consume(factory().pool());
                }
                fn associated_alias() {
                    let factory = Factory::database;
                    consume(factory().pool());
                }
            "#,
    )
    .unwrap();
    for enclosing in ["fn free_alias", "fn associated_alias"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_preserves_opaque_return_argument_flow() {
    let baseline = inventory(
        r#"
                trait HasPool { fn pool(self); }
                impl HasPool for wow_database::CharacterDatabase { fn pool(self) {} }
                fn make(database: wow_database::CharacterDatabase) -> impl HasPool { database }
                fn persistent(database: wow_database::CharacterDatabase) {
                    make(database).pool();
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));

    let error = inventory(
        r#"
                trait HasPool { fn pool(self); }
                fn make_database() -> wow_database::CharacterDatabase { unreachable!() }
                fn make() -> impl HasPool { make_database() }
                fn hidden() { make().pool(); }
            "#,
    )
    .expect_err("zero-argument opaque persistence flow must fail closed");
    assert!(error.contains("zero-argument opaque return"), "{error}");
}
