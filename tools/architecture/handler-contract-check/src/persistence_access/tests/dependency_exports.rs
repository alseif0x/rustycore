//! Regressions for dependency exports.

use super::*;

#[test]
fn persistence_inventory_resolves_named_type_fields_across_packages() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub struct Holder(pub wow_database::CharacterDatabase);",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent(holder: provider_alias::Holder) {
                    consume(holder.0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([(
            "consumer-b".to_owned(),
            BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
        )]),
        test: BTreeMap::new(),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
            .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_does_not_resolve_unrelated_workspace_named_types() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "unrelated-provider",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub struct Holder(pub wow_database::CharacterDatabase);",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                struct Holder(u8);
                fn clean(holder: Holder) {
                    consume(holder.0.pool());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, provider],
        &WorkspaceDependencyAliases::default(),
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_keeps_clean_dependency_field_projections_clean() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Inner {
                    pub database: wow_database::CharacterDatabase,
                    pub clean: bool,
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
                struct Outer { inner: provider_a::Inner }
                fn clean(value: Outer) {
                    consume(value.inner.clean);
                    assert!(!value.inner.clean);
                }
                fn persistent(value: Outer) { consume(value.inner.database.pool()); }
            "#,
    };
    let aliases = BTreeMap::from([("provider_a".to_owned(), "provider_a".to_owned())]);
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([("consumer-b".to_owned(), aliases.clone())]),
        test: BTreeMap::from([("consumer-b".to_owned(), aliases)]),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
            .unwrap();
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean"),
        "clean dependency field projection was tainted: {:#?}",
        baseline.accesses
    );
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_follows_sqlx_namespace_reexports() {
    let facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "facade-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use sqlx as db;",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn through_facade() {
                    facade_alias::db::query("SELECT GET_LOCK('facade', 0)");
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([(
            "consumer-b".to_owned(),
            BTreeMap::from([("facade_alias".to_owned(), "facade_a".to_owned())]),
        )]),
        test: BTreeMap::new(),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, facade], &dependencies)
            .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn through_facade"
            && row.operation == PersistenceOperation::Query
            && row.symbol == "query"
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn through_facade" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_resolves_dependency_callable_returns() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub struct Factory;
                impl Factory {
                    pub fn make() -> Holder { unreachable!() }
                }
                pub struct Constructed(pub wow_database::CharacterDatabase);
                impl Constructed {
                    pub fn new(database: wow_database::CharacterDatabase) -> Self {
                        Self(database)
                    }
                }
                pub fn make() -> Holder { unreachable!() }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn free_function() { consume(provider_alias::make().0.pool()); }
                fn associated_function() {
                    consume(provider_alias::Factory::make().0.pool());
                }
                fn self_constructor(database: wow_database::CharacterDatabase) {
                    let value = provider_alias::Constructed::new(database);
                    consume(value.0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([(
            "consumer-b".to_owned(),
            BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
        )]),
        test: BTreeMap::new(),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
            .unwrap();
    for enclosing in [
        "fn free_function",
        "fn associated_function",
        "fn self_constructor",
    ] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_propagates_dependency_macro_result_flow() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
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
                fn persistent() {
                    consume(provider_alias::hidden_database!().pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([(
            "consumer-b".to_owned(),
            BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
        )]),
        test: BTreeMap::new(),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
            .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::MacroReference
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_propagates_macros_and_values_through_facades() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                #[macro_export]
                macro_rules! hidden_database {
                    () => { wow_database::CharacterDatabase::default() };
                }
                pub static DATABASE: wow_database::CharacterDatabase = todo!();
            "#,
    };
    let named_facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub use provider_a_alias::{
                    DATABASE as EXPORTED_DATABASE,
                    hidden_database as exported_database,
                };
            "#,
    };
    let module_facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-module",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_a_alias as source;",
    };
    let glob_facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-glob",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_a_alias::*;",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-c",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn named_macro() { consume(provider_b_alias::exported_database!().pool()); }
                fn named_value() { consume(provider_b_alias::EXPORTED_DATABASE.pool()); }
                fn module_macro() {
                    consume(provider_module_alias::source::hidden_database!().pool());
                }
                fn module_value() {
                    consume(provider_module_alias::source::DATABASE.pool());
                }
                fn glob_macro() { consume(provider_glob_alias::hidden_database!().pool()); }
                fn glob_value() { consume(provider_glob_alias::DATABASE.pool()); }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([
            (
                "provider-b".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "provider-module".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "provider-glob".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "consumer-c".to_owned(),
                BTreeMap::from([
                    ("provider_b_alias".to_owned(), "provider_b".to_owned()),
                    (
                        "provider_module_alias".to_owned(),
                        "provider_module".to_owned(),
                    ),
                    ("provider_glob_alias".to_owned(), "provider_glob".to_owned()),
                ]),
            ),
        ]),
        test: BTreeMap::new(),
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, glob_facade, module_facade, named_facade, provider],
        &dependencies,
    )
    .unwrap();
    for enclosing in [
        "fn named_macro",
        "fn named_value",
        "fn module_macro",
        "fn module_value",
        "fn glob_macro",
        "fn glob_value",
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing facade flow for {enclosing}"
        );
    }
}

#[test]
fn persistence_inventory_resolves_callable_returns_through_dependency_reexports() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                struct Hidden(pub wow_database::CharacterDatabase);
                pub fn make() -> Holder { unreachable!() }
            "#,
    };
    let reexporter = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_a_alias::{make as create, Holder as ExportedHolder};",
    };
    let glob_reexporter = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-glob",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_a_alias::*;",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-c",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent() { consume(provider_b_alias::create().0.pool()); }
                fn globbed() { consume(provider_glob_alias::make().0.pool()); }
                fn named_type(holder: provider_b_alias::ExportedHolder) {
                    consume(holder.0.pool());
                }
                fn globbed_type(holder: provider_glob_alias::Holder) {
                    consume(holder.0.pool());
                }
                fn private_glob(holder: provider_glob_alias::Hidden) {
                    consume(holder.0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([
            (
                "provider-b".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "consumer-c".to_owned(),
                BTreeMap::from([
                    ("provider_b_alias".to_owned(), "provider_b".to_owned()),
                    ("provider_glob_alias".to_owned(), "provider_glob".to_owned()),
                ]),
            ),
            (
                "provider-glob".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
        ]),
        test: BTreeMap::new(),
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, reexporter, glob_reexporter, provider],
        &dependencies,
    )
    .unwrap();
    for enclosing in [
        "fn persistent",
        "fn globbed",
        "fn named_type",
        "fn globbed_type",
    ] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing pool flow for {enclosing}"
        );
    }
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn private_glob" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_imports_dependency_trait_return_registries() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Holder(pub wow_database::CharacterDatabase);
                pub trait Base {
                    fn make(&self) -> Holder;
                    fn pass<T>(&self, value: T) -> T;
                }
                pub trait Maker: Base {}
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn declared<T: provider_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
                fn inferred<T: provider_alias::Maker>(value: &T, database: wow_database::CharacterDatabase) {
                    consume(value.pass(database).pool());
                }
            "#,
    };
    let facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_alias::Maker;",
    };
    let downstream = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-c",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn reexported<T: facade_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([
            (
                "consumer-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "provider-b".to_owned(),
                BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "consumer-c".to_owned(),
                BTreeMap::from([("facade_alias".to_owned(), "provider_b".to_owned())]),
            ),
        ]),
        test: BTreeMap::new(),
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, downstream, facade, provider],
        &dependencies,
    )
    .unwrap();
    for enclosing in ["fn declared", "fn inferred", "fn reexported"] {
        assert!(baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing && row.operation == PersistenceOperation::PoolAccess
        }));
    }
}

#[test]
fn persistence_inventory_qualifies_test_only_dependency_supertraits() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                #[cfg(test)]
                pub struct Holder(pub wow_database::CharacterDatabase);
                #[cfg(test)]
                pub trait Base { fn make(&self) -> Holder; }
                #[cfg(test)]
                pub trait Maker: Base {}
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                #[cfg(test)]
                fn test_only<T: provider_alias::Maker>(value: &T) {
                    consume(value.make().0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::new(),
        test: BTreeMap::from([(
            "consumer-b".to_owned(),
            BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
        )]),
    };
    let baseline =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies)
            .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn test_only"
            && row.source_class == "test_fixture"
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_resolves_public_module_alias_named_types() {
    let provider = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-a",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                pub mod dto {
                    pub struct Holder(pub wow_database::CharacterDatabase);
                    struct Hidden(pub wow_database::CharacterDatabase);
                }
            "#,
    };
    let facade = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "provider-b",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: "pub use provider_a_alias::dto as types;",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "consumer-c",
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent(holder: facade_alias::types::Holder) {
                    consume(holder.0.pool());
                }
                fn private(holder: facade_alias::types::Hidden) {
                    consume(holder.0.pool());
                }
            "#,
    };
    let dependencies = WorkspaceDependencyAliases {
        production: BTreeMap::from([
            (
                "provider-b".to_owned(),
                BTreeMap::from([("provider_a_alias".to_owned(), "provider_a".to_owned())]),
            ),
            (
                "consumer-c".to_owned(),
                BTreeMap::from([("facade_alias".to_owned(), "provider_b".to_owned())]),
            ),
        ]),
        test: BTreeMap::new(),
    };
    let baseline = inventory_persistence_accesses_with_dependencies(
        &[consumer, facade, provider],
        &dependencies,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn private" && row.operation == PersistenceOperation::PoolAccess
    }));
}
