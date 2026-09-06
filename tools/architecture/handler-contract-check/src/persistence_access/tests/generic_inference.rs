//! Regressions for generic inference.

use super::*;

#[test]
fn persistence_inventory_substitutes_generic_bound_type_arguments() {
    let dto = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::dto",
        source_path: "src/dto.rs",
        inherited_cfg: &[],
        source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Holder(pub Db);
            "#,
    };
    let declaration = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::maker",
        source_path: "src/maker.rs",
        inherited_cfg: &[],
        source: r#"
                pub trait Maker<T> { fn make(&self) -> T; }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::dto::Holder;
                fn cross_file_persistent<M: crate::maker::Maker<Holder>>(factory: &M) {
                    consume(factory.make().0.pool());
                }
                fn cross_file_where<M>(factory: &M)
                where
                    M: crate::maker::Maker<Holder>,
                {
                    consume(factory.make().0.pool());
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn cross_file_persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn cross_file_where"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_substitutes_turbofish_into_recorded_returns() {
    let baseline = inventory(
        r#"
                struct Factory;
                impl Factory { fn make<T>(&self) -> T { unreachable!() } }
                fn make_selected<T>() -> T { unreachable!() }
                pub struct External;
                fn persistent_method() {
                    consume(Factory.make::<wow_database::CharacterDatabase>().pool());
                }
                fn persistent_free() {
                    consume(make_selected::<wow_database::WorldDatabase>().pool());
                }
                fn persistent_unknown(external: &External) {
                    consume(external.make::<wow_database::LoginDatabase>().pool());
                }
                fn clean() {
                    consume(Factory.make::<u8>());
                }
            "#,
    )
    .unwrap();

    let pool_row = |enclosing: &str, target: PersistenceTarget| {
        baseline.accesses.iter().any(|row| {
            row.enclosing == enclosing
                && row.target == target
                && row.operation == PersistenceOperation::PoolAccess
        })
    };
    assert!(pool_row(
        "fn persistent_method",
        PersistenceTarget::CharacterDatabase
    ));
    assert!(pool_row(
        "fn persistent_free",
        PersistenceTarget::WorldDatabase
    ));
    assert!(pool_row(
        "fn persistent_unknown",
        PersistenceTarget::LoginDatabase
    ));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_instantiates_generic_container_flow() {
    let baseline = inventory(
        r#"
                struct Holder<T>(T);
                fn persistent(database: wow_database::CharacterDatabase) {
                    let holder = Holder(database);
                    send(holder);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::ArgumentEscape
            && row.symbol == "send"
    }));
}

#[test]
fn persistence_inventory_substitutes_generic_trait_arguments_in_default_returns() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Maker<T> { fn make(&self) -> T { unreachable!() } }
                struct Factory;
                impl Maker<Holder> for Factory {}
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
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
fn persistence_inventory_propagates_inferred_generic_function_arguments() {
    let baseline = inventory(
        r#"
                fn identity<T>(value: T) -> T { value }
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(identity(database).pool());
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
fn persistence_inventory_maps_inferred_generics_to_their_formal_inputs() {
    let baseline = inventory(
        r#"
                fn first<T, U>(first: T, _second: U) -> T { first }
                fn clean(database: wow_database::CharacterDatabase) {
                    consume(first(1_u8, database).pool());
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(
        |row| row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    ));
}

#[test]
fn persistence_inventory_propagates_inferred_generic_method_arguments() {
    let baseline = inventory(
        r#"
                struct Factory;
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(factory.identity(database).pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_aligns_inferred_generic_arguments_for_ufcs_methods() {
    let baseline = inventory(
        r#"
                struct Factory;
                impl Factory { fn identity<T>(&self, value: T) -> T { value } }
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(Factory::identity(factory, database).pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_propagates_qualified_generic_function_arguments() {
    let factory = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::factory",
        source_path: "src/factory.rs",
        inherited_cfg: &[],
        source: "fn identity<T>(value: T) -> T { value }",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent(database: wow_database::CharacterDatabase) {
                    consume(crate::factory::identity(database).pool());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[consumer, factory]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_propagates_inferred_generic_trait_method_arguments() {
    let baseline = inventory(
        r#"
                trait Identity { fn identity<T>(&self, value: T) -> T { value } }
                struct Factory;
                impl Identity for Factory {}
                fn persistent(factory: &Factory, database: wow_database::CharacterDatabase) {
                    consume(factory.identity(database).pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_turbofish_to_function_item_aliases() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn make<T>() -> T { unreachable!() }
                fn persistent() {
                    let factory = make::<wow_database::CharacterDatabase>;
                    consume(factory().pool());
                }
                fn clean() {
                    let factory = make::<Clean>;
                    consume(factory().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_substitutes_associated_types_in_generic_bounds() {
    let baseline = inventory(
        r#"
                trait Maker { type Output; fn make(&self) -> Self::Output; }
                fn persistent<T: Maker<Output = wow_database::CharacterDatabase>>(value: &T) {
                    consume(value.make().pool());
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_rejects_generic_persistence_in_block_local_functions() {
    let error = inventory(
        r#"
                trait HasPool { fn pool(&self); }
                fn hidden(database: wow_database::CharacterDatabase) {
                    fn use_pool<T: HasPool>(value: T) { value.pool(); }
                    use_pool(database);
                }
            "#,
    )
    .unwrap_err();
    assert!(
        error.contains("block-local function with persistence-shaped operations (pool)"),
        "{error}"
    );
}

#[test]
fn persistence_inventory_rejects_generic_module_persistence_operations() {
    let error = inventory(
        r#"
                trait HasPool { fn pool(&self); }
                fn use_pool<T: HasPool>(value: T) { value.pool(); }
                fn hidden(database: wow_database::CharacterDatabase) { use_pool(database); }
            "#,
    )
    .unwrap_err();
    assert!(
        error.contains("fn use_pool is generic and contains persistence-shaped operations (pool)"),
        "{error}"
    );
}

#[test]
fn persistence_inventory_distinguishes_generic_from_concrete_open_calls() {
    let concrete = inventory(
        r#"
                struct Wdc4Reader;
                impl Wdc4Reader { fn open(_: &str) -> Self { Self } }
                fn load<T>(path: &str) { let _ = Wdc4Reader::open(path); }
            "#,
    );
    assert!(concrete.is_ok(), "{concrete:?}");

    let error = inventory(
        r#"
                trait Opens { fn open(_: &str) -> Self; }
                fn load<T: Opens>(path: &str) { let _ = T::open(path); }
            "#,
    )
    .unwrap_err();
    assert!(error.contains("operations (open)"), "{error}");
}

#[test]
fn persistence_inventory_infers_generic_function_item_alias_arguments() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                fn first<T, U>(value: T, _other: U) -> T { value }
                fn clean(database: wow_database::CharacterDatabase) {
                    let first_alias = first;
                    first_alias(Clean, database).pool();
                }
                fn persistent(database: wow_database::CharacterDatabase) {
                    let first_alias = first;
                    first_alias(database, Clean).pool();
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_instantiates_generic_named_type_fields() {
    let baseline = inventory(
        r#"
                struct Clean;
                impl Clean { fn pool(self) {} }
                struct Holder<T>(T);
                fn clean(holder: Holder<Clean>) { holder.0.pool(); }
                fn persistent(holder: Holder<wow_database::CharacterDatabase>) {
                    holder.0.pool();
                }
            "#,
    )
    .unwrap();
    assert!(!baseline.accesses.iter().any(|row| {
        row.enclosing == "fn clean" && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}
