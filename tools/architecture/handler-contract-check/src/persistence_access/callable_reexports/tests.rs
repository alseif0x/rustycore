// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

fn source<'a>(module: &'a str, path: &'a str, text: &'a str) -> ClassifiedPersistenceSource<'a> {
    ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module,
        source_path: path,
        inherited_cfg: &[],
        source: text,
    }
}

#[test]
fn scoped_callable_reexport_preserves_statement_consumer_provenance() {
    let factory = source(
        "crate::adapter::economy",
        "src/adapter/economy.rs",
        "pub(crate) fn plan() -> Vec<wow_database::PreparedStatement> { todo!() }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use self::economy::plan;",
    );
    let consumer = source(
        "crate::vendor",
        "src/vendor.rs",
        "fn vendor() { let statements = crate::adapter::plan(); consume(statements); }",
    );
    let baseline = inventory_persistence_accesses(&[consumer, facade, factory]).unwrap();
    for operation in [
        PersistenceOperation::ValueAlias,
        PersistenceOperation::ArgumentEscape,
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == "fn vendor"
                    && row.target == PersistenceTarget::PreparedStatement
                    && row.operation == operation
            }),
            "missing {operation:?}: {:#?}",
            baseline.accesses
        );
    }
}

#[test]
fn scoped_callable_named_import_chain_preserves_pool_consumer() {
    let factory = source(
        "crate::adapter::factory",
        "src/adapter/factory.rs",
        "pub(super) fn database() -> wow_database::CharacterDatabase { todo!() }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "use self::factory::database as make; mod tests;",
    );
    let consumer = source(
        "crate::adapter::tests",
        "src/adapter/tests.rs",
        "use super::make as fixture; fn test_case() { consume(fixture().pool()); }",
    );
    let baseline = inventory_persistence_accesses(&[consumer, factory, facade]).unwrap();
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing == "fn test_case"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess
        }),
        "missing chained pool provenance: {:#?}",
        baseline.accesses
    );
}

#[test]
fn scoped_callable_parent_glob_chain_preserves_provenance_without_recursive_names() {
    let factory = source(
        "crate::adapter::factory",
        "src/adapter/factory.rs",
        "pub(super) fn database() -> wow_database::CharacterDatabase { todo!() }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "use self::factory::database;",
    );
    let tests = source(
        "crate::adapter::tests",
        "src/adapter/tests.rs",
        "use super::*;",
    );
    let scenario = source(
        "crate::adapter::tests::scenario",
        "src/adapter/tests/scenario.rs",
        "use super::*; fn scenario() { consume(database().pool()); }",
    );
    let baseline = inventory_persistence_accesses(&[scenario, tests, facade, factory]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn scenario"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn scoped_callable_glob_does_not_override_local_or_explicit_functions() {
    let factory = source(
        "crate::factory",
        "src/factory.rs",
        "pub fn database() -> wow_database::CharacterDatabase { todo!() }",
    );
    let ordinary = source("crate::ordinary", "src/ordinary.rs", "pub fn database() {}");
    let local = source(
        "crate::local",
        "src/local.rs",
        "use crate::factory::*; fn database() {} fn local() { consume(database()); }",
    );
    let named = source(
        "crate::named",
        "src/named.rs",
        "use crate::factory::*; use crate::ordinary::database; fn named() { consume(database()); }",
    );
    let baseline = inventory_persistence_accesses(&[named, local, ordinary, factory]).unwrap();
    assert!(!baseline.accesses.iter().any(|row| matches!(
        row.enclosing.as_str(),
        "fn local" | "fn named"
    ) && row.target
        == PersistenceTarget::CharacterDatabase));
}

#[test]
fn scoped_callable_cycles_and_globs_are_bounded_and_do_not_import_descendants() {
    let key = ("fixture".to_owned(), PersistenceSourceClass::Production);
    let imports = BTreeMap::from([(
        key.clone(),
        vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "a".to_owned()),
            ("tests::*".to_owned(), "*".to_owned()),
            ("tests::nested::*".to_owned(), "tests::*".to_owned()),
        ],
    )]);
    let mut caches = BTreeMap::from([(
        key.clone(),
        std::sync::Arc::new(BTreeMap::from([
            ("database".to_owned(), 1),
            ("child::hidden".to_owned(), 2),
        ])),
    )]);
    resolve_local_callable_imports(&imports, &mut caches);
    assert_eq!(
        caches[&key].as_ref(),
        &BTreeMap::from([
            ("database".to_owned(), 1),
            ("child::hidden".to_owned(), 2),
            ("tests::database".to_owned(), 1),
            ("tests::nested::database".to_owned(), 1),
        ])
    );
}

#[test]
fn scoped_callable_aliases_stay_in_the_owning_package_cache() {
    let provider = ("provider".to_owned(), PersistenceSourceClass::Production);
    let consumer = ("consumer".to_owned(), PersistenceSourceClass::Production);
    let imports = BTreeMap::from([
        (
            provider.clone(),
            vec![("restricted".to_owned(), "factory::database".to_owned())],
        ),
        (
            consumer.clone(),
            vec![("leaked".to_owned(), "provider::restricted".to_owned())],
        ),
    ]);
    let mut caches = BTreeMap::from([
        (
            provider.clone(),
            std::sync::Arc::new(BTreeMap::from([("factory::database".to_owned(), 1)])),
        ),
        (
            consumer.clone(),
            std::sync::Arc::new(BTreeMap::from([(
                "provider::factory::database".to_owned(),
                1,
            )])),
        ),
    ]);
    resolve_local_callable_imports(&imports, &mut caches);
    assert_eq!(caches[&provider].get("restricted"), Some(&1));
    assert!(!caches[&consumer].contains_key("leaked"));
    assert!(!caches[&consumer].contains_key("provider::restricted"));
}

#[test]
fn scoped_callable_reexports_do_not_leak_through_dependency_assembly() {
    let mut factory = source(
        "crate::factory",
        "src/factory.rs",
        "pub fn database() -> wow_database::CharacterDatabase { todo!() }",
    );
    factory.package = "provider";
    let mut facade = source(
        "crate",
        "src/lib.rs",
        "pub(crate) use self::factory::database;",
    );
    facade.package = "provider";
    let mut consumer = source(
        "crate",
        "src/lib.rs",
        "fn consumer() { consume(provider::database().pool()); }",
    );
    consumer.package = "consumer";
    let mut dependencies = WorkspaceDependencyAliases::default();
    dependencies.production.insert(
        "consumer".to_owned(),
        BTreeMap::from([("provider".to_owned(), "provider".to_owned())]),
    );
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, facade, factory],
        &dependencies,
    )
    .unwrap();
    // Deliberately not compiler-valid: this AST guard is not a privacy checker,
    // but its newly resolved private aliases must never become external exports.
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn consumer"
                && row.target == PersistenceTarget::CharacterDatabase)
    );
}

#[test]
fn scoped_callable_cfg_test_import_does_not_reach_production() {
    let factory = source(
        "crate::factory",
        "src/factory.rs",
        "pub fn database() -> wow_database::CharacterDatabase { todo!() }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "#[cfg(test)] pub(crate) use crate::factory::database;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        fn production() { consume(crate::adapter::database().pool()); }
        #[cfg(test)] fn test_only() { consume(crate::adapter::database().pool()); }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, factory, facade]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn test_only"
                && row.operation == PersistenceOperation::PoolAccess)
    );
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn production"
                && row.source_class == "production"
                && row.target == PersistenceTarget::CharacterDatabase)
    );
}

#[test]
fn scoped_callable_generic_return_keeps_its_parameter_substitution() {
    let factory = source(
        "crate::factory",
        "src/factory.rs",
        "pub(crate) fn identity<T>(value: T) -> T { value }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use crate::factory::identity;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        fn consumer(db: wow_database::CharacterDatabase) {
            consume(crate::adapter::identity(db).pool());
        }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, factory, facade]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn consumer"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn scoped_callable_statement_flow_matches_the_direct_builder_across_all_consumers() {
    let factory = source(
        "crate::adapter::economy",
        "src/adapter/economy.rs",
        "pub(crate) fn plan() -> Vec<wow_database::PreparedStatement> { todo!() }",
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use self::economy::plan;",
    );
    let direct = r#"
        fn vendor(transaction: &mut wow_database::SqlTransaction) {
            let statements = crate::adapter::economy::plan();
            assert_eq!(statements.len(), 3);
            inspect(&statements);
            for statement in statements { transaction.append(statement); }
        }
        fn returned() -> Vec<wow_database::PreparedStatement> {
            crate::adapter::economy::plan()
        }
    "#;
    let indirect = direct.replace("adapter::economy::plan", "adapter::plan");
    let inventory = |body: &str| {
        let consumer = source("crate::vendor", "src/vendor.rs", body);
        inventory_persistence_accesses(&[consumer, facade, factory])
            .unwrap()
            .accesses
            .into_iter()
            .filter(|row| row.source == "src/vendor.rs")
            // Token fingerprints change with the intentionally different call
            // path; every consumer target, operation, symbol and count must not.
            .map(|row| {
                (
                    row.enclosing,
                    row.target,
                    row.operation,
                    row.symbol,
                    row.count,
                )
            })
            .collect::<BTreeSet<_>>()
    };
    let expected = inventory(direct);
    assert!(expected.iter().any(|(_, target, operation, _, _)| *target
        == PersistenceTarget::PreparedStatement
        && *operation == PersistenceOperation::ArgumentEscape));
    assert_eq!(inventory(&indirect), expected);
}

#[test]
fn scoped_callable_import_preserves_mutable_argument_effects() {
    let helper = source(
        "crate::helper",
        "src/helper.rs",
        r#"
        pub(crate) fn install(slot: &mut Option<wow_database::CharacterDatabase>) {
            *slot = Some(unreachable!());
        }
    "#,
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use crate::helper::install;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        fn persistent() {
            let mut slot = None;
            crate::adapter::install(&mut slot);
            consume(slot.unwrap().pool());
        }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, helper, facade]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn persistent"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn scoped_callable_owned_result_does_not_inherit_the_input_pool() {
    let helper = source(
        "crate::helper",
        "src/helper.rs",
        r#"
        pub(crate) fn label(db: &wow_database::CharacterDatabase) -> String {
            consume(db.pool());
            "enUS".to_owned()
        }
    "#,
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use crate::helper::label;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        fn caller(db: wow_database::CharacterDatabase) {
            let label = crate::adapter::label(&db);
            consume(label);
        }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, facade, helper]).unwrap();
    assert!(baseline.accesses.iter().any(
        |row| row.enclosing == "fn label" && row.operation == PersistenceOperation::PoolAccess
    ));
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn caller"
                && row.target == PersistenceTarget::CharacterDatabase
                && row.operation == PersistenceOperation::ArgumentEscape
                && row.symbol == "label")
    );
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn caller"
                && row.target == PersistenceTarget::CharacterDatabase
                && (row.operation == PersistenceOperation::ValueAlias || row.symbol == "consume"))
    );
}

#[test]
fn scoped_callable_pool_bearing_result_and_unknown_calls_remain_conservative() {
    let helper = source(
        "crate::helper",
        "src/helper.rs",
        r#"
        pub struct Holder(pub wow_database::CharacterDatabase);
        pub(crate) fn held(db: wow_database::CharacterDatabase) -> Holder { Holder(db) }
    "#,
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use crate::helper::held;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        fn held(db: wow_database::CharacterDatabase) {
            let result = crate::adapter::held(db);
            consume(result.0.pool());
        }
        fn unknown(db: wow_database::CharacterDatabase) {
            let result = foreign_function(db);
            consume(result.pool());
        }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, facade, helper]).unwrap();
    for enclosing in ["fn held", "fn unknown"] {
        assert!(
            baseline
                .accesses
                .iter()
                .any(|row| row.source == "src/consumer.rs"
                    && row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess)
        );
    }
}

#[test]
fn scoped_callable_unresolved_async_outputs_are_conservative_until_explicitly_bound() {
    let helper = source(
        "crate::helper",
        "src/helper.rs",
        r#"
        pub(crate) async fn retry<F, Fut, T, E>(mut operation: F) -> Result<T, E>
        where F: FnMut() -> Fut, Fut: std::future::Future<Output = Result<T, E>> {
            operation().await
        }
    "#,
    );
    let facade = source(
        "crate::adapter",
        "src/adapter.rs",
        "pub(crate) use crate::helper::retry;",
    );
    let consumer = source(
        "crate::consumer",
        "src/consumer.rs",
        r#"
        async fn pool_result(db: wow_database::CharacterDatabase) {
            let result = crate::adapter::retry(|| async { Ok::<_, ()>(db) }).await.unwrap();
            consume(result.pool());
        }
        async fn scalar_result(db: wow_database::CharacterDatabase) {
            let result = crate::adapter::retry(|| async {
                consume(db.pool());
                Ok::<u64, ()>(1)
            }).await.unwrap();
            consume(result);
        }
        async fn scalar_explicit(db: wow_database::CharacterDatabase) {
            let result = crate::adapter::retry::<_, _, u64, ()>(|| async {
                consume(db.pool());
                Ok::<u64, ()>(1)
            }).await.unwrap();
            consume(result);
        }
    "#,
    );
    let baseline = inventory_persistence_accesses(&[consumer, facade, helper]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn pool_result"
                && row.operation == PersistenceOperation::PoolAccess)
    );
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn scalar_result"
                && row.operation == PersistenceOperation::PoolAccess)
    );
    // The grammar does not solve the F -> Fut -> T/E where-clause chain.
    // Keep conservative flow rather than silently losing pool_result.
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn scalar_result"
                && row.operation == PersistenceOperation::ValueAlias
                && row.symbol == "result")
    );
    // Explicitly bound outputs stay precise despite unresolved F/Fut.
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn scalar_explicit"
                && row.operation == PersistenceOperation::ValueAlias
                && row.symbol == "result"
                && row.target == PersistenceTarget::CharacterDatabase)
    );
}
